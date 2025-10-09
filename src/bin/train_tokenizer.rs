use {
    ahash::{AHashMap, AHashSet},
    anyhow::{Context, Result},
    clap::Parser,
    itertools::Itertools,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::{
        Bincode,
        tokenization::{Token, TokenizerModel, TokenizerTrainingConfig},
    },
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Seek, Write},
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

type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;
type TokenVec = SmallVec<[Token; 32]>;

/// Start or resume training of the tokenizer.
#[derive(Parser, Debug)]
#[command(version, long_about)]
struct Args {
    /// Path to save the training config file.
    #[arg(short, long)]
    training_config_file: PathBuf,
}

fn main() -> Result<()> {
    let start = Instant::now();

    rust_gpt::utils::setup_tracing_subscriber();
    let args = Args::parse();
    let config = TokenizerTrainingConfig::from_bytes(&std::fs::read(&args.training_config_file)?)
        .context("failed to read tokenizer file")?;

    // open files with advisory that other processes should not modify them while
    // training
    let mut input_files = std::fs::read_dir(config.input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zst")
        .collect::<Vec<_>>()
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
    let pre_tokenization_regex = Regex::new(&config.pre_tokenization_regex)?;
    let should_stop = Arc::new(AtomicBool::new(false));

    ctrlc::set_handler({
        let merges = merges.clone();
        let pre_tokenization_regex = config.pre_tokenization_regex.clone();
        let tokenizer_file = config.tokenizer_file.clone();
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
                    &tokenizer_file,
                )?;
                info!("Training interrupted");
                anyhow::Ok(())
            })() {
                error!("{error}");
            }
        }
    })
    .expect("error setting Ctrl-C handler");

    let mut bytes_read = Some(AtomicUsize::new(0));

    info!(num_merges = %num_merges.separate_with_commas(), "Finding merges...");
    for _ in 0..num_merges {
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
                &config.tokenizer_file,
            )?;
        }

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
                    if should_stop.load(Ordering::SeqCst) {
                        return anyhow::Ok((i, pair_frequencies));
                    }
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

                    pre_tokenization_regex.find_iter(&buffer).for_each(|m| {
                        let mut tokens = m
                            .as_str()
                            .as_bytes()
                            .iter()
                            .map(|b| Token::from_slice(&[*b]))
                            .collect::<TokenVec>();

                        // Apply all merges efficiently
                        apply_merges(&mut tokens, &merges);

                        // Count pairs
                        for pair in tokens.windows(2) {
                            *pair_frequencies
                                .entry((pair[0].clone(), pair[1].clone()))
                                .or_default() += 1;
                        }
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
            .fold(IndexMap::<_, u64>::default(), |mut acc, (pair, count)| {
                *acc.entry(pair).or_default() += count;
                acc
            });

        if should_stop.load(Ordering::SeqCst) {
            return anyhow::Ok(());
        }

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

            let mut merges = lock_merges();
            merges.push((
                most_frequent_pair.0.clone() + most_frequent_pair.1.clone(),
                merges_search_start.elapsed(),
            ));
        } else {
            let merges = lock_merges();
            warn!(
                target_vocab_size = config.vocab_size.separate_with_commas(),
                max_possible_vocab_size = (merges.len() + 256).separate_with_commas(),
                "Not enough training data to reach the target vocabulary size",
            );
        }
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
        &config.tokenizer_file,
    )?;

    info!(
        elapsed = ?start.elapsed(),
        "Done training tokenizer"
    );

    Ok(())
}

// Optimized merge application that modifies tokens in place where possible
fn apply_merges(tokens: &mut TokenVec, merges: &[(Token, Duration)]) {
    for (merge, _) in merges {
        let mut i = 0;
        while i + 1 < tokens.len() {
            if merge == &(tokens[i].clone() + tokens[i + 1].clone()) {
                tokens[i] = merge.clone();
                tokens.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }
}

// Upsert the tokenizer file.
fn save_tokenizer(tokenizer: &TokenizerModel, tokenizer_file: &Path) -> Result<()> {
    info!(output_file = %tokenizer_file.display(), "Saving tokenizer");
    File::create(tokenizer_file)?.write_all(&tokenizer.to_bytes()?)?;
    Ok(())
}

/// Performs num_merges merges on the word frequencies vector in parallel.
#[tracing::instrument(skip(word_frequencies))]
fn perform_merges(
    word_frequencies: AHashMap<Token, u32>,
    num_merges: usize,
) -> Result<Vec<(Token, Token)>> {
    let start = Instant::now();

    // Break down words into bytes for byte-level BPE.
    // b"low" -> [b"l", b"o", b"w"]
    info!("Breaking down words into byte tokens");

    // We want to split words into tokens to perform merges
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

    info!(
        elapsed = ?start.elapsed(),
        "Done breaking down words"
    );

    // Note that the model (the merges) will always fit in memory.
    //
    // Say we wanted to train BPE to have a vocabulary of 200k tokens. (GPT-4o has
    // ~200k tokens.)
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
    let mut merges = Vec::with_capacity(num_merges);

    for _ in 0..num_merges {
        // Video lecture by Dan Jurafsky on BPE algorithm will come in handy here
        // https://www.youtube.com/watch?v=tOMjTCO0htA

        trace!("Word frequencies:");
        for (i, (word, count)) in word_frequencies.iter().enumerate() {
            trace!(
                "  {:<4} {:>4}x → {}",
                format!("{}:", i.separate_with_commas()),
                count.separate_with_commas(),
                word.iter()
                    .map(|token| format!("[{}]", String::from_utf8_lossy(token.as_slice())))
                    .collect::<String>()
            );
        }

        let (most_frequent, indices) = {
            // compute the most frequent pair of adjacent tokens from the word frequencies
            // token -> (count, indices_of_word_frequencies_set)
            let pair_mapping = word_frequencies
                .par_iter()
                .enumerate()
                .map(|(i, (word, count))| {
                    // map each word into a mapping of adjacent tokens to counts
                    // (i, word, count) -> [(pair1: (count, i)), (pair2: (count, i)), ...]
                    //
                    // i: 0 // index of the word in the word frequencies list
                    // word: "hello"
                    // count: 3
                    //
                    // pairs: ("h", "e"), ("e", "l"), ("l", "l"), ("l", "o")
                    //
                    // pair counts: ("h", "e", 3,), ("e", "l", 3), ("l", "l", 3), ...
                    // // (0, "hello", 3) -> [(("h", "e"): (3, 0)), (("e", "l"):
                    // (3, 0)), ...]
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

            let (most_frequent, (_, indices)) = pair_mapping
                .par_iter()
                .max_by_key(|(_, (count, indices))| {
                    // if multiple pairs have the same count,
                    // we sort by the indices to bring it into a deterministic order
                    (*count, indices.iter().sorted().rev().collect::<Vec<_>>())
                })
                .context(
                    "Not enough tokens to compute most frequent pair\
                    You don't have enough text for the desired vocabulary size",
                )?;

            trace!("Pair mapping:");
            for (pair, (count, indices)) in &pair_mapping {
                trace!(
                    "{} ([{}], [{}]) → {count:}x in {indices:?}",
                    if pair == most_frequent { "*" } else { " " },
                    String::from_utf8_lossy(pair[0].as_slice()),
                    String::from_utf8_lossy(pair[1].as_slice()),
                );
            }

            // return minimally owned data so we can later mutate word frequencies when
            // merging the most frequent pair
            (
                [most_frequent[0].clone(), most_frequent[1].clone()],
                indices.clone(), //
            )
        };

        let max_token_size_warning = 64;
        if most_frequent[0].len() + most_frequent[1].len() > max_token_size_warning {
            warn!(
                "You are attempting to merge a pair of tokens \n\
                that is greater than {max_token_size_warning} bytes.\n\
                Tokens that big are probably not desirable. \n\
                Try lowering the vocabulary size."
            );
        }

        merges.push((most_frequent[0].clone(), most_frequent[1].clone()));

        trace!(
            "Merging [{}], [{}]",
            String::from_utf8_lossy(most_frequent[0].as_slice()),
            String::from_utf8_lossy(most_frequent[1].as_slice())
        );

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
                    .map(|pair| {
                        if skip {
                            // reset skip after skipping once
                            skip = false;
                            pair[1].clone()
                        } else if pair == most_frequent {
                            skip = true;
                            new_token()
                        } else {
                            pair[0].clone()
                        }
                    })
                    .collect::<TokenVec>();
            });
    }

    info!(
        elapsed = ?start.elapsed(),
        num_merges = num_merges.separate_with_commas(),
        "Done performing merges"
    );

    Ok(merges)
}
