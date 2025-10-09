use {
    anyhow::{Context, Result},
    clap::Parser,
    regex::Regex,
    rust_gpt::{Bincode, tokenization::TokenizerTrainingConfig},
    std::{
        fs::File,
        io::Write,
        path::{Path, PathBuf},
    },
};

/// The pre-tokenization regex to use by default.
///
/// ---
///
/// ## Design
///
/// We want to capture a leading space to preserve spacing information.
/// E.g.
/// ```txt
/// "The cat" → ["The", " cat"]
/// "cat"     → ["cat"]
/// ```
/// The token "cat" and " cat" are **different** tokens with different IDs.
/// This helps the model understand positional context.
///  During training, neural networks learn embeddings (vector
/// representations) for each token. The model will learn that:
/// - "cat" and " cat" have similar meanings (both refer to the animal)
/// - But they have different positional contexts
///
/// Through millions of training examples, the embeddings naturally become
/// similar:
/// ```txt
/// embedding("cat") ≈ embedding(" cat")  (but not identical)
/// ```
/// The model learns this relationship automatically from data, just like it
/// learns that "cat" and "cats" are related.
///
/// Having both tokens uses more vocabulary space. But:
///
/// Typical vocabulary: 50,000 tokens
/// - ~256 base bytes
/// - ~49,744 merged tokens (learned subwords)
///
/// Having "cat" and " cat" uses 2 slots, which is negligible. The benefit
/// of capturing positional information far outweighs the cost.
const DEFAULT_PRE_TOKENIZATION_REGEX: &str = r" ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+";

/// Tokenize a directory of normalized, compressed markdown shards using
/// byte-level byte pair encoding (BPE).
#[derive(Parser, Debug)]
#[command(version, long_about)]
struct Args {
    /// Path to the input directory with the normalized, zstd-compressed
    /// preprocessed shards of textual content.
    ///
    /// A word should not be split across shard boundaries. The shards should
    /// not be modified during training.
    ///
    /// Only *.zst files will be processed. The directory will not be walked
    /// recursively.
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Path to save the tokenizer file.
    #[arg(short, long)]
    output_file: PathBuf,

    /// The target vocabulary size, a hyperparameter of BPE. Vocabulary size
    /// directly impacts how text is tokenized.
    ///
    /// Must be greater than or equal to 256 because this is byte-level BPE.
    ///
    /// Larger Vocabulary
    ///
    /// A larger vocabulary produces bigger pieces per token which means
    /// fewer tokens per text or shorter token sequences. This translates to
    /// faster training/inference per sequence at the cost of slower
    /// tokenization, overfiting, etc.
    ///
    /// - Fewer tokens per sentence (longer subwords or even whole words are
    ///   represented as single tokens).
    ///
    /// - Better for capturing rare words or linguistic nuances. Increases
    ///   memory and computational costs for embedding and training.
    ///
    /// - Increases memory and computational costs for embedding and training.
    ///
    /// Smaller Vocabulary
    ///
    /// A smaller vocabulary produces smaller pieces per token which means more
    /// tokens per text or longer token sequences. This translates to slower
    /// training/inference per sequence but with the benefit of better subword
    /// sharing across rarer forms, smaller embeddings, etc.
    ///
    /// - More tokens per sentence (each token represents a smaller unit like a
    ///   character or short subword).
    ///
    /// - Since token length for sentences is very high, the tokens may not fit
    ///   in context length of models. This may lead to loss of context and poor
    ///   model training.
    ///
    /// Examples in industry
    ///
    /// - GPT-2 has a vocabulary size of 50257
    ///
    /// - GPT-4 has a vocabulary size of ~100,000
    #[arg(short = 's', long)]
    vocab_size: usize,

    /// Regex for the initial split of text into words. The regex should capture
    /// what you want the words to contain.
    #[arg(
        short = 'r',
        long = "regex",
        default_value = DEFAULT_PRE_TOKENIZATION_REGEX
    )]
    pre_tokenization_regex: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.vocab_size < 256 {
        anyhow::bail!("vocab-size must be greater than 256 for byte-level BPE");
    }

    // validate the pre-tokenization regex
    Regex::new(&args.pre_tokenization_regex).context("invalid pre-tokenization regex")?;

    if Path::exists(&args.output_file) {
        anyhow::bail!("output-file already exists: {}", args.output_file.display());
    }

    File::create(&args.output_file)?.write_all(
        &TokenizerTrainingConfig {
            input_dir: args.input_dir,
            output_file: args.output_file,
            vocab_size: args.vocab_size,
            pre_tokenization_regex: args.pre_tokenization_regex,
            merges: Vec::with_capacity(0), // no need to allocate
        }
        .to_bytes()?,
    )?;

    Ok(())
}
