use {
    anyhow::{Context, Result},
    clap::Parser,
    comrak::{Arena, Options, parse_document},
    std::{path::PathBuf, time::Instant},
    thousands::Separable,
};

/// Parse a directory of markdown files, normalize them, and save them to a new
/// directory as *.md.zstd shards.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the training data directory with the markdown files.
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

    let input_dir = args.input_dir.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize input directory: {}",
            args.input_dir.display()
        )
    })?;

    println!("Collecting markdown files...");
    let start = Instant::now();

    let files = std::fs::read_dir(&input_dir)?
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "md")
        .collect::<Vec<_>>();

    println!(
        "Collected {} markdown files in {:0.2?}",
        files.len().separate_with_commas(),
        start.elapsed()
    );

    let arena = Arena::new();

    let root = parse_document(
        &arena,
        "## Places of interest\n* Moyry Castle",
        &Options::default(),
    );

    let mut file = std::fs::File::create("output.html").unwrap();
    comrak::format_commonmark(&root, &Options::default(), &mut file)
        .expect("Failed to format commonmark");

    Ok(())
}
