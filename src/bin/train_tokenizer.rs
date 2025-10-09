use {
    ahash::AHashMap,
    anyhow::{Context, Result},
    clap::Parser,
    rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator},
    regex::Regex,
    rust_gpt::{
        tokenization::{Token, Tokenizer, TokenizerModel, TokenizerTrainingConfig},
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
    let tokenizer = Tokenizer::from_file(&config.output_file).ok();

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

    let num_merges = config.vocab_size
        - 256
        - tokenizer
            .as_ref()
            .map(|t| {
                info!(existing_vocab_size = %(t.merges.len() + 256).separate_with_commas(), "Resuming");
                t.merges.len()
            })
            .unwrap_or(0);

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
    let merges = Arc::new(Mutex::new(
        tokenizer
            .as_ref()
            .map(|t| t.merges.clone())
            .unwrap_or_else(|| Vec::<_>::with_capacity(num_merges)),
    ));

    let lock_merges = || merges.lock().expect("failed to lock merges");
    let should_stop = Arc::new(AtomicBool::new(false));

    // Upsert the tokenizer file.
    let save_tokenizer = move |merges: Vec<Token>, tokenizer_file: &Path| -> Result<()> {
        info!(output_file = %tokenizer_file.display(), "Saving tokenizer");
        File::create(tokenizer_file)?.write_all(
            &TokenizerModel {
                pre_tokenization_regex: config.pre_tokenization_regex.clone(),
                merges,
            }
            .to_bytes()?,
        )?;
        Ok(())
    };

    ctrlc::set_handler({
        let merges = merges.clone();
        let save_tokenizer = save_tokenizer.clone();
        let output_file = config.output_file.clone();
        let should_stop = should_stop.clone();
        move || {
            // no racy conditions; flag transitions from false to true only once
            // and also not synching multiple atomic operations
            should_stop.store(true, Ordering::Relaxed);
            if let Err(error) = (|| {
                let merges = merges.lock().expect("failed to lock merges").clone();
                warn!(
                    elapsed = ?start.elapsed(),
                    current_vocab_size = %(merges.len() + 256).separate_with_commas(),
                    target_vocab_size = %config.vocab_size.separate_with_commas(),
                );
                save_tokenizer(merges, &output_file)?;
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
    let mut word_frequencies = file_paths
        .par_iter()
        .map(|file| {
            let file = File::open(file)?;
            file.lock_shared()?; // advisory lock, not mandatory

            let mut word_frequencies: AHashMap<Token, u32> = AHashMap::default();
            let mut buffer = String::new();
            let mut reader = std::io::BufReader::new(zstd::Decoder::new(file)?);

            loop {
                // short circuit if the user has pressed Ctrl-C
                if should_stop.load(Ordering::Relaxed) {
                    return Ok(AHashMap::default()); // identify value to flow fast through
                }

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
        })
        .par_iter()
        .map(|(word, count)| {
            // break down words into byte tokens as starting point for byte-level BPE
            // b"low" -> [b"l", b"o", b"w"]
            (
                word.as_slice()
                    .iter()
                    .map(|b| Token::from_slice(&[*b]))
                    .collect::<TokenVec>(),
                *count,
            )
        })
        .collect::<Vec<_>>();

    if should_stop.load(Ordering::SeqCst) {
        // exit main
        return Ok(());
    }

    info!(
        unique_words = %word_frequencies.len().separate_with_commas(),
        elapsed = ?reading_start.elapsed(),
        read = %bytesize::ByteSize::b(bytes_read.load(Ordering::Relaxed) as u64)
            .display()
            .iec(),
        "Done reading",
    );

    info!("Building pair frequency map...");
    let pair_build_start = Instant::now();
    let mut pair_counts: AHashMap<[Token; 2], u32> = word_frequencies
        .par_iter()
        .fold(AHashMap::default, |mut acc, (word, count)| {
            for pair in word.windows(2) {
                *acc.entry([pair[0].clone(), pair[1].clone()]).or_default() += count;
            }
            acc
        })
        .reduce(AHashMap::default, |mut acc, map| {
            for (pair, count) in map {
                *acc.entry(pair).or_default() += count;
            }
            acc
        });

    debug!(
        elapsed = ?pair_build_start.elapsed(),
        num_unique_pairs = %pair_counts.len().separate_with_commas(),
        "Done building pair frequency map"
    );

    info!(num_merges = %num_merges.separate_with_commas(), "Finding merges...");
    for _ in 0..num_merges {
        let merges_search_start = Instant::now();
        let merges = lock_merges().clone();
        let mut durations = Vec::with_capacity(merges.len());

        assert_eq!(
            merges.len() - tokenizer.as_ref().map(|t| t.merges.len()).unwrap_or(0),
            durations.len()
        );
        if !durations.is_empty() {
            info!(
                current_merges = %merges.len().separate_with_commas(),
                target_merges = %num_merges.separate_with_commas(),
                average_duration = ?durations.iter().sum::<Duration>() / durations.len() as u32,
            );

            if merges.len().is_multiple_of(100) {
                save_tokenizer(merges, &config.output_file)?;
            }
        }

        // find the most frequent pair using the pair counts
        let most_frequent_pair = pair_counts
            .iter()
            .max_by(|(pair_a, count_a), (pair_b, count_b)| {
                // compare by count first, then by lexicographic order for determinism
                count_a.cmp(count_b).then_with(|| {
                    (pair_a[0].as_slice(), pair_a[1].as_slice())
                        .cmp(&(pair_b[0].as_slice(), pair_b[1].as_slice()))
                })
            })
            .context(
                "Not enough tokens to compute most frequent pair. \
                You don't have enough text for the desired vocabulary size",
            )?;

        let most_frequent = most_frequent_pair.0.clone();

        if should_stop.load(Ordering::SeqCst) {
            return anyhow::Ok(());
        }

        let max_token_bytes_warning = 64;
        if most_frequent[0].as_slice().len() + most_frequent[1].as_slice().len()
            > max_token_bytes_warning
        {
            warn!(
                "You are attempting to merge a pair of tokens \
                that is greater than {max_token_bytes_warning} bytes. \
                Tokens that big are probably not desirable. \
                Try lowering the vocabulary size."
            );
        }

        debug!("Merging {:?} and {:?}", most_frequent[0], most_frequent[1]);
        let merged_token = most_frequent[0].clone() + most_frequent[1].clone();
        {
            let mut merges = lock_merges();
            merges.push(merged_token.clone());
        }

        // Before merging, we need to know which pairs are disappearing so we
        // can update the global pair_counts.
        //
        // Example:
        // Word: ["l", "o", "w"] appears 5 times
        // We're about to merge ["l", "o"] → ["lo"]
        // Old pairs: ("l","o") and ("o","w") - each should lose 5 from their counts
        // New pairs: ("lo","w") - should gain 5
        let pair_deltas: AHashMap<[Token; 2], i64> = word_frequencies
            .par_iter_mut()
            .fold(AHashMap::default, |mut deltas, (word, count)| {
                // short circuit if the word does not contain the pair to merge
                let has_pair = word
                    .windows(2)
                    .any(|w| w[0] == most_frequent[0] && w[1] == most_frequent[1]);
                if !has_pair {
                    return deltas; // return empty deltas early
                }

                // remove old pairs from deltas
                for window in word.windows(2) {
                    *deltas
                        .entry([window[0].clone(), window[1].clone()])
                        .or_default() -= *count as i64; // u32 to i64 because we will be subtracting
                }

                // Build new word with merged tokens in-place using efficient iteration
                let mut new_word = TokenVec::new();
                let mut i = 0;
                while i < word.len() {
                    if i + 1 < word.len()
                        && word[i] == most_frequent[0]
                        && word[i + 1] == most_frequent[1]
                    {
                        new_word.push(merged_token.clone());
                        i += 2; // Skip both tokens
                    } else {
                        new_word.push(word[i].clone());
                        i += 1;
                    }
                }

                // Add new pairs to deltas
                for window in new_word.windows(2) {
                    *deltas
                        .entry([window[0].clone(), window[1].clone()])
                        .or_default() += *count as i64;
                }

                *word = new_word;
                deltas
            })
            .reduce(AHashMap::default, |mut acc, deltas| {
                for (pair, delta) in deltas {
                    *acc.entry(pair).or_default() += delta;
                }
                acc
            });

        for (pair, delta) in pair_deltas {
            if delta < 0 {
                let entry = pair_counts.entry(pair.clone()).or_default();
                // delta is negative, so we negate it and use saturating_sub
                *entry = entry.saturating_sub((-delta) as u32); // won't wrap around
                if *entry == 0 {
                    pair_counts.remove(&pair);
                }
            } else if delta > 0 {
                *pair_counts.entry(pair).or_default() += delta as u32;
            }
        }
        durations.push(merges_search_start.elapsed());
    }

    save_tokenizer(
        merges.lock().expect("failed to lock merges").clone(),
        &config.output_file,
    )?;

    info!(
        elapsed = ?start.elapsed(),
        "Done training tokenizer"
    );

    Ok(())
}
