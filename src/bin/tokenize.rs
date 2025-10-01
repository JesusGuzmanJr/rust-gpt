//! # Byte Pair Encoding
//!
//! Language models don't see text like you and I, instead they see a sequence
//! of numbers (known as tokens). Byte pair encoding (BPE) is a way of
//! converting text into tokens. It has a couple desirable properties:
//!
//! - It's reversible and lossless, so you can convert tokens back into the
//!   original text
//! - It works on arbitrary text, even text that is not in the tokenizer's
//!   training data
//! - It compresses the text: the token sequence is shorter than the bytes
//!   corresponding to the original text. On average, in practice, each token
//!   corresponds to about 4 bytes.
//! - It attempts to let the model see common subwords. For instance, "ing" is a
//!   common subword in English, so BPE encodings will often split "encoding"
//!   into tokens like "encod" and "ing" (instead of e.g. "enc" and "oding").
//!   Because the model will then see the "ing" token again and again in
//!   different contexts, it helps models generalize and better understand
//!   grammar.
//!
//!  ---
//!
//! ### References
//! - [TikToken](https://github.com/openai/tiktoken)
//! - [Tokenizers](https://github.com/huggingface/tokenizers)
use {
    ahash::HashMap,
    anyhow::Result,
    clap::Parser,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    regex::Regex,
    rust_gpt::utils::canonicalize_path,
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, BufReader},
        path::PathBuf,
        sync::LazyLock,
        time::Instant,
    },
    thousands::Separable,
};

// a byte string is a small vector of bytes. Up to 32 bytes can be allocated on
// the stack.
type ByteString = SmallVec<[u8; 32]>;

/// Tokenize a directory of normalized, compressed markdown shards using
/// byte-level byte pair encoding (BPE).
///
/// Byte-level BPE is a subtype of BPE that uses bytes instead of characters as
/// basic token component.
///
/// ---
///
/// # Byte Pair Encoding
///
/// Language models don't see text like you and I, instead they see a sequence
/// of numbers (known as tokens). Byte pair encoding (BPE) is a way of
/// converting text into tokens. It has a couple desirable properties:
///
/// - It's reversible and lossless, so you can convert tokens back into the
///   original text
/// - It works on arbitrary text, even text that is not in the tokenizer's
///   training data
/// - It compresses the text: the token sequence is shorter than the bytes
///   corresponding to the original text. On average, in practice, each token
///   corresponds to about 4 bytes.
/// - It attempts to let the model see common subwords. For instance, "ing" is a
///   common subword in English, so BPE encodings will often split "encoding"
///   into tokens like "encod" and "ing" (instead of e.g. "enc" and "oding").
///   Because the model will then see the "ing" token again and again in
///   different contexts, it helps models generalize and better understand
///   grammar.
///
///  ---
///
/// ### References
/// - [TikToken](https://github.com/openai/tiktoken)
/// - [Tokenizers](https://github.com/huggingface/tokenizers)
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the input directory with the normalized, compressed preprocessed
    /// shards.
    ///
    /// Only *.md.zstd files will be processed. The directory will not be walked
    /// recursively.
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Path to save the tokenizer *.bincode file.
    #[arg(short, long)]
    output_file: PathBuf,
}

/// Regex to split text into tokens.
///
/// ---
///
/// We use optional leading space to preserve spacing information.
/// E.g.
/// ```txt
/// "The cat" → ["The", " cat"]
/// "cat"     → ["cat"]
/// ```
/// The token "cat" and " cat" are **different** tokens with different IDs. This
/// helps the model understand positional context.
///  During training, neural networks learn embeddings (vector representations)
/// for each token. The model will learn that:
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
/// Having "cat" and " cat" uses 2 slots, which is negligible. The benefit of
/// capturing positional information far outweighs the cost.
static WORD_SPLITTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+").expect("invalid regex")
});

fn main() -> Result<()> {
    let args = Args::parse();

    let input_dir = canonicalize_path(&args.input_dir)?;

    println!("Collecting markdown files...");
    let start = Instant::now();

    let input_files = std::fs::read_dir(&input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zstd")
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
        "Chunking files into chunks of {}",
        chunk_size.separate_with_commas()
    );

    let start = Instant::now();
    let word_frequencies = input_files
        .chunks(chunk_size)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .par_iter()
        .map(|chunk| {
            let mut word_frequencies: HashMap<ByteString, u32> = HashMap::default();
            let mut line = String::new();

            chunk
                .iter()
                .map(|file| {
                    let mut reader = BufReader::new(zstd::Decoder::new(File::open(file)?)?);

                    loop {
                        line.clear();
                        // we are tokenizing by whitespaces (among other things)
                        // so splitting lines by newline doesn't split words
                        if reader.read_line(&mut line)? == 0 {
                            break;
                        }

                        WORD_SPLITTER
                            .find_iter(&line)
                            .map(|w| ByteString::from_slice(w.as_str().as_bytes()))
                            .for_each(|w| {
                                *word_frequencies.entry(w).or_default() += 1;
                            });
                    }

                    anyhow::Ok(())
                })
                .for_each(|result| {
                    if let Err(error) = result {
                        eprintln!("{error}");
                    }
                });

            word_frequencies
        })
        .reduce(HashMap::default, |mut vocab, v| {
            for (word, count) in v {
                *vocab.entry(word).or_default() += count;
            }
            vocab
        });

    println!(
        "Processed {} markdown files in {:0.2?}",
        input_files.len().separate_with_commas(),
        start.elapsed()
    );

    println!(
        "Unique words: {}",
        word_frequencies.len().separate_with_commas()
    );

    {
        println!("Sorting by frequency...");
        let start = Instant::now();

        let mut word_frequencies = word_frequencies.iter().collect::<Vec<_>>();
        word_frequencies.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

        println!("Sorted by frequency in {:0.2?}", start.elapsed());

        const TOP_N: usize = 20;
        println!("First {TOP_N} most frequent words:");
        for (word, count) in word_frequencies.iter().take(TOP_N) {
            println!(
                "`{}`: {}",
                String::from_utf8_lossy(word.as_slice()),
                count.separate_with_commas()
            );
        }
    }

    // TODO: implement BPE

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        assert_eq!(
            &WORD_SPLITTER
                .find_iter("$51 for ticket 💸.")
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            &["$", "51", " for", " ticket", " 💸."]
        );
    }
}
