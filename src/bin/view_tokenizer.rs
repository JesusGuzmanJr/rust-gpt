use {
    anyhow::Result,
    clap::Parser,
    rust_gpt::{tokenization::TokenizerModel, utils::Bincode},
    std::{fs::File, io::Read, path::PathBuf},
    thousands::Separable,
};

/// Print the contents of a tokenizer file.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the tokenizer model file.
    #[arg(short, long)]
    tokenizer_file: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut buffer = Vec::new();

    File::open(args.tokenizer_file)?.read_to_end(&mut buffer)?;

    let model = TokenizerModel::from_bytes(&buffer)?;

    println!(
        "Pre-tokenization regex: {:?}\nVocabulary size: {}",
        model.pre_tokenization_regex,
        (model.merges.len() + 256).separate_with_commas(),
    );

    for (i, merge) in model.merges.into_iter().enumerate() {
        println!("{:<8}: {merge:?}", (i + 256).separate_with_commas());
    }

    Ok(())
}
