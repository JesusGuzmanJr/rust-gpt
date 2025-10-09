use {
    ahash::{AHashMap, AHashSet},
    anyhow::{Context, Result},
    clap::Parser,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::{
        tokenization::{Token, TokenizerModel, TokenizerTrainingConfig},
        utils::{Bincode, Ron},
    },
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Write},
        path::{Path, PathBuf},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    },
    thousands::Separable,
    tracing::*,
};

type TokenVec = SmallVec<[Token; 32]>;

/// Start or resume training the tokenizer using approximate byte-level byte
/// pair encoding (BPE).
#[derive(Parser, Debug)]
#[command(version, long_about)]
struct Args {
    /// Path to the tokenizer training config file.
    #[arg(short, long)]
    config: PathBuf,
}

fn main() -> Result<()> {
    let start = Instant::now();

    rust_gpt::utils::setup_tracing_subscriber();

    let config = TokenizerTrainingConfig::from_file_path(&Args::parse().config)?;

    if config.vocab_size < 256 {
        anyhow::bail!("vocab-size must be greater than 256 for byte-level BPE");
    }

    // validate the pre-tokenization regex
    let pre_tokenization_regex =
        Regex::new(&config.pre_tokenization_regex).context("invalid pre-tokenization regex")?;

    // ensure output_file is not a directory
    if config.output_file.is_dir() {
        anyhow::bail!(
            "output-file is a directory: {}",
            config.output_file.display()
        );
    }

    let num_merges = config.vocab_size - 256;

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
    let merges = Arc::new(Mutex::new(Vec::<(Token, Duration)>::with_capacity(
        num_merges,
    )));

    let lock_merges = || merges.lock().expect("failed to lock merges");
    let should_stop = Arc::new(AtomicBool::new(false));

    // Upsert the tokenizer file.
    let save_tokenizer = |tokenizer: &TokenizerModel, tokenizer_file: &Path| -> Result<()> {
        info!(output_file = %tokenizer_file.display(), "Saving tokenizer");
        File::create(tokenizer_file)?.write_all(&tokenizer.to_bytes()?)?;
        Ok(())
    };

    ctrlc::set_handler({
        let merges = merges.clone();
        let pre_tokenization_regex = config.pre_tokenization_regex.clone();
        let output_file = config.output_file.clone();
        let should_stop = should_stop.clone();
        move || {
            should_stop.store(true, Ordering::SeqCst);
            if let Err(error) = (|| {
                let merges = merges.lock().expect("failed to lock merges");
                warn!(
                    elapsed = ?start.elapsed(),
                    current_vocab_size = %(merges.len() + 256).separate_with_commas(),
                    target_vocab_size = %config.vocab_size.separate_with_commas(),
                );
                save_tokenizer(
                    &TokenizerModel {
                        pre_tokenization_regex: pre_tokenization_regex.clone(),
                        merges: merges
                            .clone()
                            .into_iter()
                            .map(|(token, _duration)| token)
                            .collect(),
                    },
                    &output_file,
                )?;
                info!("Training interrupted");
                anyhow::Ok(())
            })() {
                error!("{error}");
            }
        }
    })
    .expect("error setting Ctrl-C handler");

    let file_paths = std::fs::read_dir(config.input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zst")
        .collect::<Vec<_>>();

    info!(
        num_files = %file_paths.len().separate_with_commas(),
        "Reading files",
    );

    let bytes_read: AtomicUsize = AtomicUsize::new(0);
    let reading_start = Instant::now();
    let word_frequencies = file_paths
        .par_iter()
        .map(|file| {
            let file = File::open(file)?;
            file.lock_shared()?; // advisory lock, not mandatory

            let mut word_frequencies: AHashMap<Token, u32> = AHashMap::default();
            let mut buffer = String::new();
            let mut reader = std::io::BufReader::new(zstd::Decoder::new(file)?);

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
                    .map(|w| Token::from_slice(w.as_str().as_bytes()))
                    .for_each(|w| {
                        *word_frequencies.entry(w).or_default() += 1;
                    });
            }

            anyhow::Ok(word_frequencies)
        })
        .filter_map(|result| {
            if let Err(error) = &result {
                error!("{error}");
            }
            result.ok()
        })
        .reduce(AHashMap::default, |mut vocab, v| {
            for (word, count) in v {
                *vocab.entry(word).or_default() += count;
            }
            vocab
        });

    info!(
        unique_words = %word_frequencies.len().separate_with_commas(),
        elapsed = ?reading_start.elapsed(),
        read = %bytesize::ByteSize::b(bytes_read.load(Ordering::Relaxed) as u64)
            .display()
            .iec(),
        "Done reading",
    );

    // Break down words into bytes for byte-level BPE.
    // b"low" -> [b"l", b"o", b"w"]
    debug!("Breaking down words into byte tokens");

    // We want to split words into tokens to perform merges
    let breakdown_start = Instant::now();
    let mut word_frequencies = word_frequencies
        .par_iter()
        .map(|(word, count)| {
            (
                word.as_slice()
                    .iter()
                    .map(|b| Token::from_slice(&[*b]))
                    .collect::<TokenVec>(),
                *count,
            )
        })
        .collect::<Vec<_>>();

    debug!(
        elapsed = ?breakdown_start.elapsed(),
        "Done breaking down words"
    );

    info!(num_merges = %num_merges.separate_with_commas(), "Finding merges...");
    for _ in 0..num_merges {
        // Video lecture by Dan Jurafsky on BPE algorithm will come in handy here
        // https://www.youtube.com/watch?v=tOMjTCO0htA

        let merges_search_start = Instant::now();
        let merges = lock_merges().clone();

        if !merges.is_empty() {
            info!(
                current_merges = %merges.len().separate_with_commas(),
                target_merges = %num_merges.separate_with_commas(),
                average_duration = ?merges.iter().map(|(_, duration)| *duration).sum::<Duration>() / merges.len() as u32,
            );

            save_tokenizer(
                &TokenizerModel {
                    pre_tokenization_regex: config.pre_tokenization_regex.clone(),
                    merges: merges
                        .iter()
                        .map(|(token, _duration)| token)
                        .cloned()
                        .collect(),
                },
                &config.output_file,
            )?;
        }

        let most_frequent = {
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
                        AHashMap::<&[Token], (u32, usize)>::default(),
                        |mut acc, pair| {
                            acc.entry(pair).or_insert((0, i)).0 += count;
                            acc
                        },
                    )
                })
                .fold(AHashMap::default, |mut acc, pair_counts| {
                    // we're working in parallel so we'll have multiple mappings that need merging
                    // however now the mappings will reduce from (pair, index) to (pair, indices)
                    for (pair, (count, i)) in pair_counts {
                        let entry = acc.entry(pair).or_insert((0, AHashSet::default()));
                        entry.0 += count;
                        entry.1.insert(i);
                    }
                    acc
                })
                .reduce(AHashMap::default, |mut acc, pair_counts| {
                    // we need to reduce the indices into a single set for each pair
                    for (pair, (count, indices)) in pair_counts {
                        let entry = acc.entry(pair).or_insert((0, AHashSet::default()));
                        entry.0 += count;
                        entry.1.extend(indices);
                    }
                    acc
                });

            trace!("Pair mapping:");
            for (pair, (count, indices)) in &pair_mapping {
                trace!(
                    "({:?}, {:?}) → count: {count}, indices: {indices:?}",
                    pair[0], pair[1],
                );
            }

            let (most_frequent, _) = pair_mapping
                .par_iter()
                .max_by(|(pair_a, (count_a, _)), (pair_b, (count_b, _))| {
                    // compare by count, then by lexicographic order of the pair's bytes
                    // to ensure deterministic results when counts are equal
                    // this isn't exactly BPE but it's a close enough approximation
                    count_a.cmp(count_b).then_with(|| {
                        (pair_a[0].as_slice(), pair_a[1].as_slice())
                            .cmp(&(pair_b[0].as_slice(), pair_b[1].as_slice()))
                    })
                })
                .context(
                    "Not enough tokens to compute most frequent pair \
                    You don't have enough text for the desired vocabulary size",
                )?;

            // return minimally owned data so we can later mutate word frequencies when
            // merging the most frequent pair
            [most_frequent[0].clone(), most_frequent[1].clone()]
        };

        if should_stop.load(Ordering::SeqCst) {
            return anyhow::Ok(());
        }

        let max_token_size_warning = 64;
        if most_frequent[0].as_slice().len() + most_frequent[1].as_slice().len()
            > max_token_size_warning
        {
            warn!(
                "You are attempting to merge a pair of tokens \
                that is greater than {max_token_size_warning} bytes.\
                Tokens that big are probably not desirable. \
                Try lowering the vocabulary size."
            );
        }

        debug!("Merging {:?} and {:?}", most_frequent[0], most_frequent[1]);
        let merged_token = most_frequent[0].clone() + most_frequent[1].clone();
        {
            let mut merges = lock_merges();
            merges.push((merged_token.clone(), merges_search_start.elapsed()));
        }

        word_frequencies.par_iter_mut().for_each(|(word, _)| {
            let mut skip = false;
            let mut new_word = word
                .windows(2)
                .filter_map(|window| {
                    if skip {
                        skip = false;
                        return None;
                    }

                    if window[0] == most_frequent[0] && window[1] == most_frequent[1] {
                        skip = true;
                        Some(merged_token.clone())
                    } else {
                        Some(window[0].clone())
                    }
                })
                .collect::<TokenVec>();

            // need to add the rightmost token if it wasn't part of the merge
            if !skip {
                new_word.extend(word.last().cloned());
            }

            *word = new_word;
        });
    }

    save_tokenizer(
        &TokenizerModel {
            pre_tokenization_regex: config.pre_tokenization_regex,
            merges: merges
                .lock()
                .expect("failed to lock merges")
                .clone()
                .into_iter()
                .map(|(token, _duration)| token)
                .collect(),
        },
        &config.output_file,
    )?;

    info!(
        elapsed = ?start.elapsed(),
        "Done training tokenizer"
    );

    Ok(())
}
