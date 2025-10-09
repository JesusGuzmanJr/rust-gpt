use {
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

    if rust_gpt::hash::hash_directory(&config.input_dir)? != config.hash {
        anyhow::bail!(
            "Training data has changed. Re-run create_tokenizer to create a new tokenizer model.",
        );
    }

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
                            for (merge, _duration) in &merges {
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

// Upsert the tokenizer file.
fn save_tokenizer(tokenizer: &TokenizerModel, tokenizer_file: &Path) -> Result<()> {
    info!(output_file = %tokenizer_file.display(), "Saving tokenizer");
    File::create(tokenizer_file)?.write_all(&tokenizer.to_bytes()?)?;
    Ok(())
}
