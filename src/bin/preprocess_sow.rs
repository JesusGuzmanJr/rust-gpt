use {
    anyhow::{Context, Result},
    clap::Parser,
    itertools::Itertools,
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
    rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    regex::Regex,
    rust_gpt::{tokenization::normalize_text, utils::get_parquet_column},
    std::{
        fs::File,
        io::Write,
        path::PathBuf,
        sync::{
            Arc, LazyLock, Mutex,
            atomic::{AtomicU32, Ordering},
        },
    },
    thousands::Separable,
    tracing::*,
};

/// Parse a Stanford Oval Wikipedia parquet file creating shards of normalized,
/// compressed markdown.
///
/// Dataset: https://huggingface.co/datasets/stanford-oval/wikipedia
#[derive(Parser, Debug)]
#[command(version, about)]
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
    rust_gpt::utils::setup_tracing_subscriber();
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

    let batch_size = rayon::current_num_threads() * 64;

    let (tx, rx) = crossbeam_channel::bounded(batch_size);

    let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(&args.input_file)?)?
        .with_batch_size(batch_size)
        .build()?;

    info!(batch_size = %batch_size.separate_with_commas(), "Reading batched rows from the parquet file...");

    rayon::spawn(move || {
        reader
            .filter_map(|batch| {
                if let Err(error) = &batch {
                    error!("{error}");
                }
                batch.ok()
            })
            .map(|batch| {
                // see dataset card for column names
                // https://huggingface.co/datasets/stanford-oval/wikipedia
                let document_title = get_parquet_column(&batch, "document_title")?;
                let section_title = get_parquet_column(&batch, "section_title")?;
                let content = get_parquet_column(&batch, "content")?;

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

                let rows = rows
                    .par_iter()
                    .map(|(document_title, section_title, content)| {
                        anyhow::Ok((
                            clean_text(document_title)?,
                            if let Some(section_title) = section_title {
                                Some(clean_text(section_title)?)
                            } else {
                                None
                            },
                            clean_text(content)?,
                        ))
                    })
                    .filter_map(|result| {
                        if let Err(error) = &result {
                            error!("{error}");
                        }
                        result.ok()
                    })
                    .collect::<Vec<_>>();

                anyhow::Ok(rows)
            })
            .filter_map(|result| {
                if let Err(error) = &result {
                    error!("{error}");
                }
                result.ok()
            })
            .flatten()
            .chunk_by(
                // Groups consecutive rows that have the same document_title
                |(document_title, ..)| document_title.to_owned(),
            )
            .into_iter()
            .for_each(move |(document_title, group)| {
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
        info!("Finished reading parquet file");
    });

    let encoders = Arc::new(Mutex::new(
        (0..rayon::current_num_threads())
            .map(|_| None)
            .collect::<Vec<_>>(),
    ));

    let article_count = AtomicU32::new(0);

    rx.into_iter()
        .par_bridge()
        .try_for_each(|article| -> Result<()> {
            let processed = article_count.fetch_add(1, Ordering::Relaxed) + 1;

            if processed.is_multiple_of(1000) {
                info!("Processed {} articles...", processed.separate_with_commas());
            }

            let markdown = article.to_markdown()?;
            let thread_idx = rayon::current_thread_index().expect("thread index is not available");

            // Get or create encoder for this thread
            let mut encoders = encoders.lock().expect("failed to lock encoders");
            if encoders[thread_idx].is_none() {
                let file_name = format!("shard-{}.md.zst", thread_idx);
                info!("Writing to {file_name}...");
                let encoder = Box::new(
                    zstd::Encoder::new(
                        File::create(args.output_dir.join(file_name))?,
                        // The compression level for the zstd encoder.
                        // The higher the level, the more compressed the data will be.
                        // Decompressing is same speed regardless of the level.
                        19,
                    )?
                    .auto_finish(),
                );
                encoders[thread_idx] = Some(encoder);
            }

            let encoder = encoders[thread_idx]
                .as_mut()
                .expect("encoder is not available")
                .as_mut();

            encoder.write_all(markdown.as_bytes())?;

            // Add end-of-text character
            // https://en.wikipedia.org/wiki/C0_and_C1_control_codes
            encoder.write_all(rust_gpt::utils::END_OF_TEXT)?;

            Ok(())
        })?;

    info!(
        "Finished processing {} articles",
        article_count.load(Ordering::Relaxed).separate_with_commas()
    );

    Ok(())
}

fn clean_text(text: &str) -> Result<String> {
    Ok(remove_tables(&normalize_text(text)))
}

fn remove_tables(text: &str) -> String {
    static TABLE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?s)<Table>.*?</Table>").expect("invalid regex"));

    TABLE_RE.replace_all(text, " ").to_string()
}

impl Article {
    /// Return a string of properly formatted markdown.
    fn to_markdown(&self) -> Result<String> {
        // Add 20% extra
        let mut markdown = String::with_capacity(self.len() + self.len() / 5);

        // Add document title as H1
        markdown.push_str("# ");
        markdown.push_str(&self.document_title);
        markdown.push_str("\n\n");

        // Add each section
        for section in &self.sections {
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

        let mut formatted = String::with_capacity((markdown.len() as f32 * 1.2) as usize);
        comrak::format_commonmark(root, &options, &mut formatted)?;

        Ok(formatted)
    }

    /// Get the byte length of the article.
    fn len(&self) -> usize {
        self.document_title.len()
            + self
                .sections
                .iter()
                .map(|section| section.len())
                .sum::<usize>()
    }
}

impl Section {
    /// Get the byte length of the section.
    fn len(&self) -> usize {
        self.title.as_ref().map(|title| title.len()).unwrap_or(0) + self.content.len()
    }
}
