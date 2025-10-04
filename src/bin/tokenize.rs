use {
    ahash::{AHashSet, HashMap},
    anyhow::{Context, Result},
    clap::Parser,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::utils::canonicalize_path,
    smallvec::SmallVec,
    std::{fs::File, io::BufRead, path::PathBuf, sync::LazyLock, time::Instant},
    thousands::Separable,
};

// Up to 32 bytes can be allocated on the stack before heap allocating.
type Token = SmallVec<[u8; 32]>;

/// Tokenize a directory of normalized, compressed markdown shards using
/// byte-level byte pair encoding (BPE).
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the input directory with the normalized, zstd-compressed
    /// preprocessed shards of textual content.
    ///
    /// Only *.zstd files will be processed. The directory will not be walked
    /// recursively.
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Path to save the tokenizer *.zstd file.
    #[arg(short, long)]
    output_file: PathBuf,

    /// The target vocabulary size.
    ///
    /// For English, 50,000 tokens is a good default.
    ///
    /// A larger vocabulary produces bigger pieces per token which means fewer
    /// tokens per text or shorter token sequences. This translates to faster
    /// training/inference per sequence at the cost of slower tokenization,
    /// overfiting, etc.
    ///
    /// A smaller vocabulary produces smaller pieces per token which means more
    /// tokens per text or longer token sequences. This translates to slower
    /// training/inference per sequence but with the benefit of better subword
    /// sharing across rarer forms, smaller embeddings, etc.
    #[arg(short, long, default_value = "50000")]
    vocab_size: usize,
}

/// Regex to split text into tokens.
///
/// ---
///
/// ## Design Notes
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
static WORD_BOUNDARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+").expect("invalid regex")
});

fn main() -> Result<()> {
    let args = Args::parse();

    if args.vocab_size < 256 {
        anyhow::bail!("vocab-size must be greater than 256 for byte-level BPE");
    }

    let input_dir = canonicalize_path(&args.input_dir)?;

    let input_files = std::fs::read_dir(&input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zstd")
        .collect::<Vec<_>>();

    println!(
        "Collected {} files\nComputing word frequencies...",
        input_files.len().separate_with_commas(),
    );

    let start = Instant::now();
    let word_frequencies = input_files
        .par_iter()
        .map(|file| {
            let mut word_frequencies: HashMap<Token, u32> = HashMap::default();
            let mut buffer = String::new();
            let mut reader = std::io::BufReader::new(zstd::Decoder::new(File::open(file)?)?);

            loop {
                // Clearing is O(1).
                buffer.clear();

                // we are tokenizing by whitespaces (among other things)
                // so splitting lines by newline doesn't split words
                if reader.read_line(&mut buffer)? == 0 {
                    break;
                }

                WORD_BOUNDARY_RE
                    .find_iter(&buffer)
                    .map(|w| Token::from_slice(w.as_str().as_bytes()))
                    .for_each(|w| {
                        *word_frequencies.entry(w).or_default() += 1;
                    });
            }

            anyhow::Ok(word_frequencies)
        })
        .filter_map(|result| {
            if let Err(error) = &result {
                eprintln!("{error}");
            }
            result.ok()
        })
        .reduce(HashMap::default, |mut vocab, v| {
            for (word, count) in v {
                *vocab.entry(word).or_default() += count;
            }
            vocab
        });

    println!(
        "Done computing word frequencies in {:0.2?} for {} files",
        start.elapsed(),
        input_files.len().separate_with_commas(),
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
                "{:>16} → {:?}",
                count.separate_with_commas(),
                String::from_utf8_lossy(word.as_slice()),
            );
        }
    }

    // Break down words into bytes for byte-level BPE.
    // b"low" -> [b"l", b"o", b"w"]
    let start = Instant::now();
    println!("Breaking down words into byte tokens...");

    let mut word_frequencies = word_frequencies
        .par_iter()
        .map(|(word, count)| {
            (
                word.iter()
                    .map(|b| Token::from_slice(&[*b]))
                    .collect::<SmallVec<[Token; 32]>>(),
                *count,
            )
        })
        .collect::<Vec<_>>();

    println!("Done breaking down words in {:0.2?}", start.elapsed());

    let mut vocabulary = (0x00u8..=0xff).collect::<AHashSet<_>>();

    // .len() is O(1)
    while vocabulary.len() < args.vocab_size {
        // Video lecture by Dan Jurafsky on BPE algorithm: https://www.youtube.com/watch?v=tOMjTCO0htA

        let start = Instant::now();
        println!("Computing most frequent pair of adjacent tokens...");

        let top_pair = *word_frequencies
            .par_iter()
            .map(|(word, count)| {
                word.windows(2)
                    .fold(HashMap::<&[Token], u32>::default(), |mut acc, pair| {
                        *acc.entry(pair).or_default() += count;
                        acc
                    })
            })
            .reduce(HashMap::default, |mut acc, pair_counts| {
                for (pair, count) in pair_counts {
                    *acc.entry(pair).or_default() += count;
                }
                acc
            })
            .par_iter()
            .max_by_key(|(_, count)| **count)
            .context("not enough tokens to compute most frequent pair")?
            .0;

        println!(
            "Done computing most frequent pair in {:0.2?}",
            start.elapsed()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        assert_eq!(
            &WORD_BOUNDARY_RE
                .find_iter("$51 for ticket 💸.")
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            &["$", "51", " for", " ticket", " 💸."]
        );
    }
}
