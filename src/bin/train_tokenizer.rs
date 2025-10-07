use {
    ahash::HashSet,
    anyhow::{Context, Result},
    clap::Parser,
    itertools::Itertools,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::{Token, utils::canonicalize_path},
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Write},
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
        time::Instant,
    },
    thousands::Separable,
    tracing::{level_filters::LevelFilter, *},
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

    let training_start = Instant::now();
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

    // num_merges is vocab_size minus 256 base bytes because we're implementing
    let bytes_read: AtomicUsize = AtomicUsize::new(0);

    let pair_frequencies = input_files
        .par_iter()
        .enumerate()
        .map(|(i, file)| {
            let mut pair_frequencies = IndexMap::<_, u64>::default();
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

                pre_tokenization_regex
                    .find_iter(&buffer)
                    .map(|m| m.as_str().as_bytes())
                    .for_each(|bytes| {
                        bytes.windows(2).for_each(|pair| {
                            *pair_frequencies
                                .entry((
                                    Token::from_slice(&[pair[0]]),
                                    Token::from_slice(&[pair[1]]),
                                ))
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

    if LevelFilter::current() == LevelFilter::TRACE {
        trace!("Pair frequencies:");
        for (i, (pair, count)) in pair_frequencies.iter().enumerate() {
            trace!(
                "  {:<8} {:>8}x → [{:?}][{:?}]",
                format!("{}:", i.separate_with_commas()),
                count.separate_with_commas(),
                String::from_utf8_lossy(pair.0.as_slice()),
                String::from_utf8_lossy(pair.1.as_slice())
            );
        }
    }

    info!(
        unique_words = pair_frequencies.len().separate_with_commas(),
        elapsed = ?start.elapsed(),
        read = %bytesize::ByteSize::b(bytes_read.load(Ordering::Relaxed) as u64)
            .display()
            .iec(),
        "Done reading",
    );

    info!(output_file = %args.output_file.display(), "Saving tokenizer");

    // File::create(args.output_file)?.write_all(&bincode::serde::encode_to_vec(
    //     &merges,
    //     bincode::config::standard(),
    // )?)?;

    info!(
        elapsed = ?training_start.elapsed(),
        "Done training tokenizer"
    );

    Ok(())
}

// /// Performs num_merges merges on the word frequencies vector in parallel.
// #[tracing::instrument(skip(word_frequencies))]
// fn perform_merges(
//     word_frequencies: IndexMap<Token, u32>,
//     num_merges: usize,
// ) -> Result<Vec<(Token, Token)>> {
//     let start = Instant::now();

//     // Break down words into bytes for byte-level BPE.
//     // b"low" -> [b"l", b"o", b"w"]
//     info!("Breaking down words into byte tokens");

//     // We want to split words into tokens to perform merges
//     let mut word_frequencies = word_frequencies
//         .par_iter()
//         .map(|(word, count)| {
//             (
//                 word.iter()
//                     .map(|b| Token::from_slice(&[*b]))
//                     .collect::<TokenVec>(),
//                 *count,
//             )
//         })
//         .collect::<Vec<_>>();

//     info!(
//         elapsed = ?start.elapsed(),
//         "Done breaking down words"
//     );

//     // Note that the model (the merges) will always fit in memory.
//     //
//     // Say we wanted to train BPE to have a vocabulary of 200k tokens.
// (GPT-4o     // has ~200k tokens.)
//     //
//     // That means we need 200,000 minus 256 merges; so 199,744 merges.
//     //
//     // Each merge is 2 tokens:
//     // 2 * 199,744 = 399,488 tokens
//     //
//     // The token size is variable. Let's take a worst case of 64 bytes per
//     // token. A 64 byte token is HUGE and very unlikely.
//     //
//     // 64 * 399,488 = 25,567,232 bytes or 25.57 MiB.
//     let mut merges = Vec::with_capacity(num_merges);

//     for _ in 0..num_merges {
//         // Video lecture by Dan Jurafsky on BPE algorithm will come in handy
// here         // https://www.youtube.com/watch?v=tOMjTCO0htA

//         if LevelFilter::current() == LevelFilter::TRACE {
//             trace!("Word frequencies:");
//             for (i, (word, count)) in word_frequencies.iter().enumerate() {
//                 trace!(
//                     "  {:<4} {:>4}x → {}",
//                     format!("{}:", i.separate_with_commas()),
//                     count.separate_with_commas(),
//                     word.iter()
//                         .map(|token| format!("[{}]",
// String::from_utf8_lossy(token.as_slice())))
// .collect::<String>()                 );
//             }
//         }

//         let (most_frequent, indices) = {
//             // compute the most frequent pair of adjacent tokens from the
// word frequencies             // token -> (count,
// indices_of_word_frequencies_set)             let pair_mapping =
// word_frequencies                 .par_iter()
//                 .enumerate()
//                 .map(|(i, (word, count))| {
//                     // map each word into a mapping of adjacent tokens to
// counts                     // (i, word, count) -> [(pair1: (count, i)),
// (pair2: (count, i)),                     // ...]
//                     //
//                     // i: 0 // index of the word in the word frequencies list
//                     // word: "hello"
//                     // count: 3
//                     //
//                     // pairs: ("h", "e"), ("e", "l"), ("l", "l"), ("l", "o")
//                     //
//                     // pair counts: ("h", "e", 3,), ("e", "l", 3), ("l", "l",
// 3), ...                     //
//                     // (0, "hello", 3) -> [(("h", "e"): (3, 0)), (("e", "l"):
// (3, 0)), ...]                     word.windows(2).fold(
//                         HashMap::<&[Token], (u32, usize)>::default(),
//                         |mut acc, pair| {
//                             acc.entry(pair).or_insert((0, i)).0 += count;
//                             acc
//                         },
//                     )
//                 })
//                 .fold(HashMap::default, |mut acc, pair_counts| {
//                     // we're working in parallel so we'll have multiple
// mappings that need merging                     // however now the mappings
// will reduce from (pair, index) to (pair, indices)                     for
// (pair, (count, i)) in pair_counts {                         let entry =
// acc.entry(pair).or_insert((0, HashSet::default()));
// entry.0 += count;                         entry.1.insert(i);
//                     }
//                     acc
//                 })
//                 .reduce(HashMap::default, |mut acc, pair_counts| {
//                     // we need to reduce the indices into a single set for
// each pair                     for (pair, (count, indices)) in pair_counts {
//                         let entry = acc.entry(pair).or_insert((0,
// HashSet::default()));                         entry.0 += count;
//                         entry.1.extend(indices);
//                     }
//                     acc
//                 });

//             let (most_frequent, (_, indices)) = pair_mapping
//                 .par_iter()
//                 .max_by_key(|(_, (count, indices))| {
//                     // if multiple pairs have the same count,
//                     // we sort by the indices to bring it into a
// deterministic order                     (*count,
// indices.iter().sorted().rev().collect::<Vec<_>>())                 })
//                 .context(
//                     "Not enough tokens to compute most frequent pair\n\
//                     You don't have enough text for the desired vocabulary
// size",                 )?;

//             if LevelFilter::current() == LevelFilter::TRACE {
//                 trace!("Pair mapping:");
//                 for (pair, (count, indices)) in &pair_mapping {
//                     trace!(
//                         "{} ([{}], [{}]) → {count:}x in {indices:?}",
//                         if pair == most_frequent { "*" } else { " " },
//                         String::from_utf8_lossy(pair[0].as_slice()),
//                         String::from_utf8_lossy(pair[1].as_slice()),
//                     );
//                 }
//             }

//             // return minimally owned data so we can later mutate word
// frequencies when             // merging the most frequent pair
//             (
//                 [most_frequent[0].clone(), most_frequent[1].clone()],
//                 indices.clone(), //
//             )
//         };

//         let max_token_size_warning = 64;
//         if most_frequent[0].len() + most_frequent[1].len() >
// max_token_size_warning {             warn!(
//                 "You are attempting to merge a pair of tokens \n\
//                 that is greater than {max_token_size_warning} bytes.\n\
//                 Tokens that big are probably not desirable. \n\
//                 Try lowering the vocabulary size."
//             );
//         }

//         merges.push((most_frequent[0].clone(), most_frequent[1].clone()));

//         if LevelFilter::current() == LevelFilter::TRACE {
//             trace!(
//                 "Merging [{}], [{}]",
//                 String::from_utf8_lossy(most_frequent[0].as_slice()),
//                 String::from_utf8_lossy(most_frequent[1].as_slice())
//             );
//         }

//         let new_token = || {
//             let mut new_token = most_frequent[0].clone();
//             new_token.extend(most_frequent[1].clone());
//             new_token
//         };

//         word_frequencies
//             .par_iter_mut()
//             .enumerate()
//             .filter(|(i, _)| indices.contains(i))
//             .map(|(_, (word, count))| (word, count))
//             .for_each(|(word, _)| {
//                 // skip next pair because it contains the rightmost token of
// the most frequent                 // pair
//                 let mut skip = false;

//                 *word = word
//                     .windows(2)
//                     .map(|pair| {
//                         if skip {
//                             // reset skip after skipping once
//                             skip = false;
//                             pair[1].clone()
//                         } else if pair == most_frequent {
//                             skip = true;
//                             new_token()
//                         } else {
//                             pair[0].clone()
//                         }
//                     })
//                     .collect::<TokenVec>();
//             });
//     }

//     info!(
//         elapsed = ?start.elapsed(),
//         num_merges = num_merges.separate_with_commas(),
//         "Done performing merges"
//     );

//     Ok(merges)
// }
