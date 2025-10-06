use {
    ahash::{HashMap, HashSet},
    anyhow::{Context, Result},
    clap::Parser,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::{Token, TokenizationModel, utils::canonicalize_path},
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Write},
        path::PathBuf,
        sync::{
            LazyLock,
            atomic::{AtomicUsize, Ordering},
        },
        time::Instant,
    },
    thousands::Separable,
};

type TokenVec = SmallVec<[Token; 32]>;

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

    /// Path to save the tokenizer file.
    #[arg(short, long)]
    output_file: PathBuf,

    /// The target vocabulary size, a hyperparameter of BPE. Vocabulary size
    /// directly impacts how text is tokenized.
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
    #[arg(short, long)]
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
    let training_start = Instant::now();

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
        "Reading {} files...",
        input_files.len().separate_with_commas(),
    );

    let bytes_read: AtomicUsize = AtomicUsize::new(0);

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

                let read = reader.read_line(&mut buffer)?;

                // we are tokenizing by whitespaces (among other things)
                // so splitting lines by newline doesn't split words
                if read == 0 {
                    break;
                } else {
                    bytes_read.fetch_add(read, Ordering::Relaxed);
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
        "Read {} in {:0.2?}",
        bytesize::ByteSize::b(bytes_read.load(Ordering::Relaxed) as u64)
            .display()
            .iec(),
        start.elapsed(),
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

    // We want word_frequencies to be a vector so we can operate on its indices in
    // parallel
    let mut word_frequencies = word_frequencies
        .par_iter()
        .map(|(word, count)| {
            (
                word.iter()
                    .map(|b| Token::from_slice(&[*b]))
                    .collect::<TokenVec>(),
                *count,
            )
        })
        .collect::<Vec<_>>();

    println!("Done breaking down words in {:0.2?}", start.elapsed());

    // num_merges is vocab_size minus 256 base bytes because we're implementing
    let num_merges = args.vocab_size - 256;

    println!("Performing {} merges...", num_merges.separate_with_commas());

    let mut merges = Vec::with_capacity(num_merges);

    // The vocabulary without the 256 base bytes.
    let mut additional_vocabulary: Vec<SmallVec<[u8; 32]>> = Vec::with_capacity(num_merges);

    let start = Instant::now();

    for _ in 0..num_merges {
        // Video lecture by Dan Jurafsky on BPE algorithm will come in handy here
        // https://www.youtube.com/watch?v=tOMjTCO0htA

        let (most_frequent, indices) = {
            // compute the most frequent pair of adjacent tokens from the word frequencies
            // token -> (count, indices_of_word_frequencies_set)
            let pair_mapping = word_frequencies
                .par_iter()
                .enumerate()
                .map(|(i, (word, count))| {
                    // map each word into a mapping of adjacent tokens to counts
                    // (i, word, count) -> [(pair1: (count, i)), (pair2: (count, i)),
                    // ...]
                    //
                    // i: 0 // index of the word in the word frequencies list
                    // word: "hello"
                    // count: 3
                    //
                    // pairs: ("h", "e"), ("e", "l"), ("l", "l"), ("l", "o")
                    //
                    // pair counts: ("h", "e", 3,), ("e", "l", 3), ("l", "l", 3), ...
                    //
                    // (0, "hello", 3) -> [(("h", "e"): (3, 0)), (("e", "l"): (3, 0)), ...]
                    word.windows(2).fold(
                        HashMap::<&[Token], (u32, usize)>::default(),
                        |mut acc, pair| {
                            acc.entry(pair).or_insert((0, i)).0 += count;
                            acc
                        },
                    )
                })
                .fold(HashMap::default, |mut acc, pair_counts| {
                    // we're working in parallel so we'll have multiple mappings that need merging
                    // however now the mappings will reduce from (pair, index) to (pair, indices)
                    for (pair, (count, i)) in pair_counts {
                        let entry = acc.entry(pair).or_insert((0, HashSet::default()));
                        entry.0 += count;
                        entry.1.insert(i);
                    }
                    acc
                })
                .reduce(HashMap::default, |mut acc, pair_counts| {
                    // we need to reduce the indices into a single set for each pair
                    for (pair, (count, indices)) in pair_counts {
                        let entry = acc.entry(pair).or_insert((0, HashSet::default()));
                        entry.0 += count;
                        entry.1.extend(indices);
                    }
                    acc
                });

            let (most_frequent, (_, indices)) = pair_mapping
                .par_iter()
                .max_by_key(|(_, (count, _))| *count)
                .context(
                    "Not enough tokens to compute most frequent pair\n\
                    You don't have enough text for the desired vocabulary size",
                )?;

            // return minimally owned data so we can later mutate word frequencies when
            // merging the most frequent pair
            (
                [most_frequent[0].clone(), most_frequent[1].clone()],
                indices.clone(), //
            )
        };

        let max_token_size_warning = 64;
        if most_frequent[0].len() + most_frequent[1].len() > max_token_size_warning {
            println!(
                "You are attempting to merge a pair of tokens \n\
                that is greater than {max_token_size_warning} bytes.\n\
                Tokens that big are probably not desirable. \n\
                Try lowering the vocabulary size."
            );
        }

        merges.push((most_frequent[0].clone(), most_frequent[1].clone()));

        let new_token = || {
            let mut new_token = most_frequent[0].clone();
            new_token.extend(most_frequent[1].clone());
            new_token
        };

        word_frequencies
            .par_iter_mut()
            .enumerate()
            .filter(|(i, _)| indices.contains(i))
            .map(|(_, (word, count))| (word, count))
            .for_each(|(word, _)| {
                // skip next pair because it contains the rightmost token of the most frequent
                // pair
                let mut skip = false;

                *word = word
                    .windows(2)
                    .filter_map(|pair| {
                        if skip {
                            // reset skip after skipping once
                            skip = false;
                            return None;
                        }

                        if pair == most_frequent {
                            skip = true;
                            Some(new_token())
                        } else {
                            Some(pair[0].clone())
                        }
                    })
                    .collect::<TokenVec>();
            });

        additional_vocabulary.push(new_token());
    }

    println!(
        "Done performing {} merges in {:0.2?}",
        num_merges.separate_with_commas(),
        start.elapsed()
    );

    assert_eq!(
        additional_vocabulary.len() + 256,
        args.vocab_size,
        "vocabulary size is not equal to the target vocabulary size"
    );

    println!("Saving vocabulary to {}...", args.output_file.display());

    let model_bytes = bincode::serde::encode_to_vec(
        &TokenizationModel {
            merges,
            additional_vocabulary,
        },
        bincode::config::standard(),
    )?;

    let mut encoder = zstd::Encoder::new(
        File::create(args.output_file)?,
        // The compression level for the zstd encoder.
        // The higher the level, the more compressed the data will be.
        // Decompressing is same speed regardless of the level.
        22,
    )?
    .auto_finish();

    encoder.write_all(&model_bytes)?;

    println!("Done in {:0.2?}", training_start.elapsed());

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
