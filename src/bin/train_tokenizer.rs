use {
    anyhow::Result,
    clap::Parser,
    itertools::Itertools,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::{
        Bincode,
        tokenization::{Token, TokenizerModel},
    },
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Seek, Write},
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::Instant,
    },
    thousands::Separable,
    tracing::*,
};

type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
type TokenVec = SmallVec<[Token; 32]>;

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
    let start = Instant::now();

    let args = Args::parse();
    rust_gpt::utils::setup_tracing_subscriber();

    if args.vocab_size < 256 {
        anyhow::bail!("vocab-size must be greater than 256 for byte-level BPE");
    }

    let pre_tokenization_regex = Regex::new(&args.pre_tokenization_regex)?;

    let input_files = std::fs::read_dir(args.input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zst")
        .collect::<Vec<_>>();

    info!(
        num_files = input_files.len().separate_with_commas(),
        %args.pre_tokenization_regex,
        "Reading files",
    );

    // open files with advisory that other processes should not modify them while
    // training
    let mut input_files = input_files
        .par_iter()
        .map(|file_path| {
            let file = File::open(file_path)?;
            file.lock_shared()?; // advisory, not mandatory
            anyhow::Ok(file)
        })
        .filter_map(|result| {
            if let Err(error) = &result {
                error!("{error}");
            }
            result.ok()
        })
        .collect::<Vec<_>>();

    let num_merges = args.vocab_size - 256;

    // Note that the model (the merges) will always fit in memory.
    //
    // Say we wanted to train BPE to have a vocabulary of 200k tokens. (GPT-4o
    // has ~200k tokens.)
    //
    // That means we need 200,000 minus 256 merges; so 199,744 merges.
    //
    // Each merge is 2 tokens:
    // 2 * 199,744 = 399,488 tokens
    //
    // The token size is variable. Let's take a worst case of 64 bytes per
    // token. A 64 byte token is HUGE and very unlikely.
    //
    // 64 * 399,488 = 25,567,232 bytes or 25.57 MiB.
    let mut merges = Vec::<Token>::with_capacity(num_merges);

    let mut bytes_read = Some(AtomicUsize::new(0));

    for _ in 0..num_merges {
        // we assume that the files are not modified during the training
        let pair_frequencies = input_files
            .par_iter_mut()
            .enumerate()
            .map(|(i, file)| {
                let mut pair_frequencies = IndexMap::<_, u64>::default();
                let mut buffer = String::new();
                file.seek(std::io::SeekFrom::Start(0))?; // reset cursor
                let mut reader = std::io::BufReader::new(zstd::Decoder::new(file)?);

                loop {
                    // clearing is O(1).
                    buffer.clear();

                    let read = reader.read_line(&mut buffer)?;

                    // we are tokenizing by whitespaces (among other things)
                    // so splitting lines by newline doesn't split words
                    if read == 0 {
                        break;
                    } else if let Some(bytes_read) = &bytes_read {
                        bytes_read.fetch_add(read, Ordering::Relaxed);
                    }

                    pre_tokenization_regex
                        .find_iter(&buffer)
                        .map(|m| {
                            m.as_str()
                                .as_bytes()
                                .iter()
                                .map(|b| Token::from_slice(&[*b]))
                                .collect::<TokenVec>()
                        })
                        .for_each(|mut tokens| {
                            // need to apply all the merges in order
                            for merge in &merges {
                                let mut skip = false;
                                let mut processed_tokens = tokens
                                    .windows(2)
                                    .filter_map(|window| {
                                        if skip {
                                            skip = false;
                                            return None;
                                        }

                                        // if this is the merge, push the entire merge
                                        if merge == &(window[0].clone() + window[1].clone()) {
                                            skip = true;
                                            Some(merge.clone())
                                        } else {
                                            Some(window[0].clone())
                                        }
                                    })
                                    .collect::<TokenVec>();

                                if !skip {
                                    processed_tokens.extend(tokens.last().cloned());
                                }

                                tokens = processed_tokens;
                            }
                            tokens.windows(2).for_each(|pair| {
                                *pair_frequencies
                                    .entry((pair[0].clone(), pair[1].clone()))
                                    .or_default() += 1;
                            });
                        });
                }

                anyhow::Ok((i, pair_frequencies))
            })
            .filter_map(|result| {
                if let Err(error) = &result {
                    error!("{error}");
                }
                result.ok()
            })
            .collect::<Vec<_>>()
            .into_iter()
            .sorted_by_key(|(i, _)| *i)
            .flat_map(|(_, pairs)| pairs)
            .collect::<IndexMap<_, _>>();

        if let Some(read) = &bytes_read {
            info!(
                read = %bytesize::ByteSize::b(read.load(Ordering::Relaxed) as u64)
                    .display()
                    .iec(),
                "Done reading all files"
            );
            bytes_read = None;
        }

        trace!("Pair frequencies:");
        for (i, (pair, count)) in pair_frequencies.iter().enumerate() {
            trace!(
                "  {:<8} {:>8}x → [{:?}][{:?}]",
                format!("{}:", i.separate_with_commas()),
                count.separate_with_commas(),
                pair.0,
                pair.1
            );
        }
        if let Some((most_frequent_pair, count)) = pair_frequencies
            .iter()
            .rev()
            .max_by_key(|(_, count)| *count)
        {
            trace!(
                "Most frequent pair: [{:?}][{:?}] ({}x)",
                most_frequent_pair.0,
                most_frequent_pair.1,
                count.separate_with_commas()
            );

            merges.push(most_frequent_pair.0.clone() + most_frequent_pair.1.clone());
        } else {
            warn!(
                target_vocab_size = args.vocab_size.separate_with_commas(),
                max_possible_vocab_size = (merges.len() + 256).separate_with_commas(),
                "Not enough training data to reach the target vocabulary size",
            );
        }
    }

    info!(output_file = %args.output_file.display(), "Saving tokenizer");

    File::create(args.output_file)?.write_all(
        &TokenizerModel {
            pre_tokenization_regex: args.pre_tokenization_regex,
            merges,
        }
        .to_bytes()?,
    )?;

    info!(
        elapsed = ?start.elapsed(),
        "Done training tokenizer"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use rust_gpt::{
        Bincode,
        tokenization::{Token, TokenizerModel},
    };

    #[test]
    fn test_train_tokenizer() {
        let regex = r"\p{L}+ ?";
        let output = std::process::Command::new("just")
            .arg("run")
            .arg("train_tokenizer")
            .arg("--input-dir")
            .arg("training-data")
            .arg("--output-file")
            .arg("target/test/test-tokenizer")
            .arg("--vocab-size")
            .arg("264")
            .arg("--regex")
            .arg(regex)
            .output()
            .expect("failed to run train_tokenizer");

        assert!(&output.status.success());

        let control_model = TokenizerModel {
            pre_tokenization_regex: regex.to_string(),
            merges: vec![
                Token::from_slice(b"er"),
                Token::from_slice(b"er "),
                Token::from_slice(b"ne"),
                Token::from_slice(b"new"),
                Token::from_slice(b"lo"),
                Token::from_slice(b"low"),
                Token::from_slice(b"newer "),
                Token::from_slice(b"low "),
            ],
        };

        let model =
            TokenizerModel::from_bytes(&std::fs::read("target/test/test-tokenizer").unwrap())
                .unwrap();

        assert_eq!(model, control_model);
    }
}
