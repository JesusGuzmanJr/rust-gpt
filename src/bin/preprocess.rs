use {
    anyhow::{Context, Result},
    arrow_array::{GenericByteArray, RecordBatch, StringArray, types::GenericStringType},
    clap::Parser,
    icu_normalizer::ComposingNormalizerBorrowed,
    itertools::Itertools,
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
    rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    regex::Regex,
    rust_gpt::utils::canonicalize_path,
    std::{fs::File, io::Write, path::PathBuf, sync::LazyLock},
};

/// Parse a Stanford Oval Wikipedia parquet file creating shards of normalized,
/// compressed markdown.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the Stanford Oval Wikipedia parquet file.
    #[arg(short, long)]
    input_file: PathBuf,

    /// Path to the output directory for the shards.
    ///
    /// If the directory does not exist, it will be created.
    #[arg(short, long)]
    output_dir: PathBuf,
}

/// A Wikipedia article.
struct Article {
    /// The title of the article.
    document_title: String,

    /// The sections of the article.
    sections: Vec<Section>,
}

/// A section of a Wikipedia article.
struct Section {
    /// The title of the section. This may be None.
    title: Option<String>,

    /// The content of the section.
    content: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // create the output directory if it doesn't exist
    if !args.output_dir.exists() {
        std::fs::create_dir_all(&args.output_dir).with_context(|| {
            format!(
                "Failed to create output directory: {}",
                args.output_dir.display()
            )
        })?;
    }

    // Buffer up to 64 articles that need to be further processed and sharded
    let (tx, rx) = crossbeam_channel::bounded(64);

    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&canonicalize_path(
        &args.input_file,
    )?)?)?
    .with_batch_size(rayon::current_num_threads() * 16)
    .with_limit(50)
    .build()?;

    // Reads rows from the parquet file and produces assembled Wikipedia articles.
    std::thread::spawn(move || {
        reader
            .filter_map(|batch| {
                if let Err(error) = &batch {
                    eprintln!("{error}");
                }
                batch.ok()
            })
            .map(|batch| {
                // see dataset card for column names
                // https://huggingface.co/datasets/stanford-oval/wikipedia
                let document_title = column(&batch, "document_title")?;
                let section_title = column(&batch, "section_title")?;
                let content = column(&batch, "content")?;

                let rows = (0..batch.num_rows())
                    .map(|i| {
                        let document_title = document_title.value(i);
                        let section_title = section_title.value(i);
                        let content = content.value(i);
                        (document_title, section_title, content)
                    })
                    .filter(|(document_title, _, content)| {
                        !document_title.is_empty() && !content.is_empty()
                    })
                    .map(|(document_title, section_title, content)| {
                        (
                            document_title,
                            if section_title.is_empty() {
                                None
                            } else {
                                Some(section_title)
                            },
                            content,
                        )
                    })
                    .collect::<Vec<_>>();

                // process the batched rows in parallel
                let rows = rows
                    .par_iter()
                    .map(|(document_title, section_title, content)| {
                        anyhow::Ok((
                            process_text(document_title)?,
                            if let Some(section_title) = section_title {
                                Some(process_text(section_title)?)
                            } else {
                                None
                            },
                            process_text(content)?,
                        ))
                    })
                    .filter_map(|result| {
                        if let Err(error) = &result {
                            eprintln!("{error}");
                        }
                        result.ok()
                    })
                    .collect::<Vec<_>>();

                anyhow::Ok(rows)
            })
            .filter_map(|result| {
                if let Err(error) = &result {
                    eprintln!("{error}");
                }
                result.ok()
            })
            .flatten()
            .chunk_by(
                // Groups consecutive rows that have the same document_title
                |(document_title, ..)| document_title.to_owned(),
            )
            .into_iter()
            .for_each(|(document_title, group)| {
                let article = Article {
                    document_title,
                    sections: group
                        .map(|(_, section_title, content)| Section {
                            title: section_title,
                            content,
                        })
                        .collect::<Vec<_>>(),
                };

                // Block until the channel is ready to receive the article
                tx.send(article).expect("channel is not open");
            });
    });

    rx.into_iter()
        .enumerate()
        .par_bridge()
        .map(|(i, article)| {
            let mut file = File::create(args.output_dir.join(format!("shard-{i}.md.zstd")))?;

            let mut encoder = zstd::Encoder::new(
                &mut file,
                // The compression level for the zstd encoder.
                // The higher the level, the more compressed the data will be.
                // Decompressing is same speed regardless of the level.
                19,
            )?
            .auto_finish();

            // Build markdown document
            let mut markdown = String::new();

            // Add document title as H1
            markdown.push_str("# ");
            markdown.push_str(&article.document_title);
            markdown.push_str("\n\n");

            // Add each section
            for section in &article.sections {
                // Add section title as H2 if it exists and is not empty
                if let Some(title) = &section.title
                    && !title.is_empty()
                {
                    markdown.push_str("## ");
                    markdown.push_str(title);
                    markdown.push_str("\n\n");
                }

                // Add section content
                markdown.push_str(&section.content);
                markdown.push_str("\n\n");
            }

            let arena = comrak::Arena::new();
            let options = comrak::Options::default();
            let root = comrak::parse_document(&arena, &markdown, &options);

            // Format to a string
            let mut buffer = String::new();
            comrak::format_commonmark(root, &options, &mut buffer)?;

            // Write to encoder
            encoder.write_all(buffer.as_bytes())?;

            // Add end-of-text character
            // https://en.wikipedia.org/wiki/C0_and_C1_control_codes
            encoder.write_all("\u{3}".as_bytes())?;

            anyhow::Ok(())
        })
        .for_each(|result| {
            if let Err(error) = result {
                eprintln!("{error}");
            }
        });

    Ok(())
}

/// Get a reference to a column's array by name.
fn column<'a>(
    record_batch: &'a RecordBatch,
    column: &'static str,
) -> Result<&'a GenericByteArray<GenericStringType<i32>>> {
    record_batch
        .column_by_name(column)
        .with_context(|| format!("missing {column} column"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .context("content is not a StringArray")
}

/// Remove control characters and normalize the string using NFC.
///
/// ---
///
/// ## Normalization Form C (Canonical Composition).
/// Process:
/// - Decompose everything canonically (like NFD).
/// - Recompose wherever there’s a single canonical equivalent pre-composed
///   character.
///
/// The key is: NFC only recomposes when there exists exactly one
/// pre-composed form. So it either:
/// Shrinks (if multiple code points → 1 pre-composed code point).
/// Leaves unchanged (if no pre-composed exists).
///
/// It never expands into more code points, because that would break
/// canonical equivalence.
///
/// See [Unicode normalization forms](https://www.unicode.org/reports/tr15).
fn process_text(text: &str) -> Result<String> {
    static NFC: LazyLock<ComposingNormalizerBorrowed<'_>> =
        LazyLock::new(ComposingNormalizerBorrowed::new_nfc);

    // Remove control characters and normalize whitespace
    let text = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n') // Keep newlines
        .collect::<String>();

    static TABLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<Table>.*?</Table>").expect("invalid regex"));

    let text = TABLE_RE.replace_all(&text, " ");

    // Apply Unicode NFC normalization
    let mut buffer = String::with_capacity(text.len());
    NFC.normalize_to(&text, &mut buffer)?;

    Ok(buffer)
}
