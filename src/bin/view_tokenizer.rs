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

    println!("Merges: {}", merges.len().separate_with_commas(),);

    for merge in merges {
        println!("{merge:?}",);
    }

    Ok(())
}
