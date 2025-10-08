use {
    anyhow::Result,
    clap::Parser,
    rust_gpt::tokenization::Token,
    std::{fs::File, io::Read, path::PathBuf},
    thousands::Separable,
};

/// Print the contents of a tokenizer file.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the tokenizer file.
    #[arg(short, long)]
    input_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut buffer = Vec::new();

    File::open(args.input_file)?.read_to_end(&mut buffer)?;

    let merges =
        bincode::serde::decode_from_slice::<Vec<Token>, _>(&buffer, bincode::config::standard())?.0;

    println!(
        "Vocabulary size: {}",
        (merges.len() + 256).separate_with_commas(),
    );

    for (i, merge) in merges.into_iter().enumerate() {
        println!("{:<8}: {merge:?}", (i + 256).separate_with_commas());
    }

    Ok(())
}
