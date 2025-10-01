use {
    anyhow::{Context, Result},
    clap::Parser,
    rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    rust_gpt::utils::canonicalize_path,
    std::{fs::File, io::Write, path::PathBuf, time::Instant},
    thousands::Separable,
};

/// Parse a directory of markdown files, normalize them, and save them to a new
/// directory as *.md.zstd compressed shards.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the training data directory with the markdown files.
    ///
    /// A file should be about a single topic.
    /// Bad files will be skipped.
    ///
    /// Only *.md files will be processed. The directory will not be walked
    /// recursively.
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Path to the output directory for the shards.
    ///
    /// If the directory does not exist, it will be created.
    #[arg(short, long)]
    output_dir: PathBuf,
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

    let input_dir = canonicalize_path(&args.input_dir)?;

    println!("Collecting markdown files...");
    let start = Instant::now();

    let input_files = std::fs::read_dir(&input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "md")
        .collect::<Vec<_>>();

    println!(
        "Collected {} markdown files in {:0.2?}",
        input_files.len().separate_with_commas(),
        start.elapsed()
    );

    let thread_count = rayon::current_num_threads();
    println!("Using {thread_count} threads");

    let chunk_size = input_files.len().div_ceil(thread_count);

    println!(
        "Chunking files into {} chunks",
        chunk_size.separate_with_commas()
    );

    let start = Instant::now();
    input_files
        .chunks(chunk_size)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .par_iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut file = File::create(args.output_dir.join(format!("shard-{i}.md.zstd")))?;

            let mut encoder = zstd::Encoder::new(
                &mut file,
                // The compression level for the zstd encoder.
                // The higher the level, the more compressed the data will be.
                // Decompressing is same speed regardless of the level.
                19,
            )?
            .auto_finish();

            let markdown_options = comrak::Options::default();

            // Normalization Form C (Canonical Composition).
            //
            // Process:
            // - Decompose everything canonically (like NFD).
            // - Recompose wherever there’s a single canonical equivalent pre-composed
            // character.
            //
            // The key is: NFC only recomposes when there exists exactly one
            // pre-composed form. So it either:
            // Shrinks (if multiple code points → 1 pre-composed code point).
            // Leaves unchanged (if no pre-composed exists).
            //
            // It never expands into more code points, because that would break
            // canonical equivalence.
            //
            // https://www.unicode.org/reports/tr15/
            let nfc = icu_normalizer::ComposingNormalizerBorrowed::new_nfc();

            chunk
                .iter()
                .map(std::fs::read_to_string)
                .filter_map(|result| {
                    if let Err(error) = &result {
                        eprintln!("{error}");
                    }
                    result.ok()
                })
                .map(|mut buffer| {
                    {
                        // filter out files that are less than 20 lines
                        if buffer.lines().count() < 20 {
                            return Ok(());
                        }

                        // remove control characters
                        let cleaned = buffer
                            .chars()
                            .filter(|c| !c.is_control())
                            .collect::<String>();

                        assert!(
                            cleaned.len() <= buffer.len(),
                            "expected cleaned string to contain ≤ original"
                        );

                        nfc.normalize_to(&cleaned, &mut buffer)?;
                    }

                    // parse Markdown - create a new arena for each file to avoid accumulating
                    // memory
                    let arena = comrak::Arena::new();
                    let parsed = comrak::parse_document(&arena, &buffer, &markdown_options);

                    comrak::format_commonmark(parsed, &markdown_options, &mut encoder)?;
                    encoder.write_all("\u{3}".as_bytes())?; // end-of-text character

                    anyhow::Ok(())
                })
                .for_each(|result| {
                    if let Err(error) = result {
                        eprintln!("{error}");
                    }
                });

            anyhow::Ok(())
        })
        .for_each(|result| {
            if let Err(error) = result {
                eprintln!("{error}");
            }
        });

    println!(
        "Processed {} markdown files in {:0.2?}",
        input_files.len().separate_with_commas(),
        start.elapsed()
    );
    Ok(())
}
