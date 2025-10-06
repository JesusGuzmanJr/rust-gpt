use {
    anyhow::Result,
    clap::Parser,
    rust_gpt::TokenizationModel,
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

    zstd::Decoder::new(File::open(args.input_file)?)?.read_to_end(&mut buffer)?;

    let TokenizationModel {
        merges,
        additional_vocabulary,
    } = bincode::serde::decode_from_slice(&buffer, bincode::config::standard())?.0;

    println!(
        "Merges: {}\nVocabulary size: {} ({} additional tokens)",
        merges.len().separate_with_commas(),
        (additional_vocabulary.len() + 256).separate_with_commas(),
        additional_vocabulary.len().separate_with_commas()
    );

    for (l, r) in merges {
        println!(
            "{:?} + {:?}",
            String::from_utf8_lossy(l.as_slice()),
            String::from_utf8_lossy(r.as_slice())
        );
    }

    for token in additional_vocabulary {
        println!("{:?}", String::from_utf8_lossy(token.as_slice()));
    }

    Ok(())
}
