use {
    anyhow::Result,
    clap::Parser,
    language_model::{tokenization::Tokenizer, utils::Bincode},
    std::{fs::File, io::Write, path::PathBuf},
};

/// Export a tokenizer to JSON format for the web viewer.
#[derive(Parser, Debug)]
#[command(version, long_about)]
struct Args {
    /// Path to the tokenizer binary file.
    #[arg(short, long)]
    input: PathBuf,

    /// Path to the output JSON file.
    #[arg(short, long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load the tokenizer
    let tokenizer = Tokenizer::from_file(&args.input)?;

    // Convert merges to base64 or hex arrays for JSON
    let merges_json: Vec<Vec<u8>> = tokenizer
        .merges
        .iter()
        .map(|token| token.as_slice().to_vec())
        .collect();

    // Get the pre-tokenization regex pattern
    // We need to access the model to get the regex pattern
    let model = language_model::tokenization::TokenizerModel::from_bytes(&std::fs::read(&args.input)?)?;

    // Create JSON structure
    let json_data = serde_json::json!({
        "pre_tokenization_regex": model.pre_tokenization_regex,
        "merges": merges_json,
        "vocab_size": 256 + tokenizer.merges.len(),
    });

    // Write to file
    let json_string = serde_json::to_string_pretty(&json_data)?;
    File::create(&args.output)?.write_all(json_string.as_bytes())?;

    println!("Exported tokenizer to: {}", args.output.display());
    println!("Vocabulary size: {}", 256 + tokenizer.merges.len());

    Ok(())
}
