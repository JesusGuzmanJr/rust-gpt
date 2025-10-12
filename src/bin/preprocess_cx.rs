use {
    ahash::AHashSet,
    anyhow::{Context, Result},
    bytes::Bytes,
    clap::Parser,
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
    rayon::iter::{ParallelBridge, ParallelIterator},
    reqwest::Client,
    rust_gpt::{
        tokenization::normalize_text,
        utils::{END_OF_TEXT, byte_size, get_parquet_column},
    },
    std::{
        fs::File,
        io::Write,
        os::unix::ffi::OsStrExt,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
    },
    thousands::Separable,
    tokio::sync::Mutex,
    tracing::*,
};

/// The number of parquet files to process concurrently.
const PARQUET_FILE_CONCURRENCY: usize = 16;

/// Download, parse and preprocess CulturaX parquet files
/// creating shards of normalized, compressed markdown.
///
/// Requires the HUGGING_FACE_TOKEN environment variable to be set.
///
/// Dataset: https://huggingface.co/datasets/uonlp/CulturaX
///
/// Paper: https://arxiv.org/abs/2309.09400
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the output directory for the shards.
    ///
    /// If the directory does not exist, it will be created.
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    rust_gpt::utils::setup_tracing_subscriber();
    let args = Args::parse();

    // create the output directory if it doesn't exist
    if !args.output_dir.exists() {
        std::fs::create_dir_all(&args.output_dir).with_context(|| {
            format!(
                "Failed to create output directory: {}",
                args.output_dir.display()
            )
        })?;
    }

    let regex = regex::bytes::Regex::new(r"shard-(\d+)\.zst").expect("invalid regex");

    // The English dataset is 7.54 TB and it's split into ~2.5 GB files so that's
    // ~3016 files whose paths fit in memory.
    let processed_file_indices = std::fs::read_dir(&args.output_dir)?
        .par_bridge()
        .filter_map(|result| result.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension() == Some(std::ffi::OsStr::new("zst")))
        .filter_map(|path| {
            if let Some(captures) = regex.captures(path.as_os_str().as_bytes()) {
                match String::from_utf8_lossy(&captures[1]).parse::<usize>() {
                    Ok(number) => Some(number),
                    Err(error) => {
                        error!(?path, %error, "invalid file index");
                        None
                    }
                }
            } else {
                None
            }
        })
        .fold(AHashSet::default, |mut set, number| {
            set.insert(number);
            set
        })
        .reduce(AHashSet::default, |mut curr, other| {
            curr.extend(other);
            curr
        });

    let client = reqwest::ClientBuilder::new()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.append(
                "Authorization",
                format!(
                    "Bearer {}",
                    std::env::var("HUGGING_FACE_TOKEN")
                        .context("HUGGING_FACE_TOKEN environment variable is not set")?
                )
                .parse()
                .expect("bad header"),
            );
            headers
        })
        .build()
        .expect("failed to build client");

    let (tx, rx) = crossbeam_channel::bounded(PARQUET_FILE_CONCURRENCY);

    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("failed to build tokio runtime")
        .spawn(async move {
            // index of the current file
            let current_index = processed_file_indices.iter().cloned().max().unwrap_or(0);

            // indices of the files that are missing for some reason
            let missing = (0..=current_index)
                .filter(|i| !processed_file_indices.contains(i))
                .collect::<Vec<_>>();

            if !missing.is_empty() {
                warn!(
                    current_index,
                    num_missing = missing.len().separate_with_commas(),
                    "Found missing files",
                );
            }

            let missing = Arc::new(Mutex::new(missing));
            let should_stop = Arc::new(AtomicBool::new(false));
            let current_index = Arc::new(AtomicUsize::new(current_index));
            let downloaded_bytes = Arc::new(AtomicUsize::new(0));

            for _ in 0..PARQUET_FILE_CONCURRENCY {
                let tx = tx.clone();
                let client = client.clone();
                let should_stop = should_stop.clone();
                let current_index = current_index.clone();
                let missing = missing.clone();
                let downloaded_bytes = downloaded_bytes.clone();

                // launch PARQUET_FILE_CONCURRENCY IO threads to continuously download parquet
                // files
                tokio::spawn(async move {
                    while !should_stop.load(Ordering::Relaxed) {
                        let i = {
                            if let Some(i) = missing.lock().await.pop() {
                                i
                            } else {
                                current_index.fetch_add(1, Ordering::Relaxed)
                            }
                        };

                        match download_parquet(i, client.clone()).await {
                            Ok(bytes) => {
                                let downloaded = downloaded_bytes
                                    .fetch_add(bytes.len(), Ordering::Relaxed)
                                    + bytes.len();

                                if downloaded.is_multiple_of(100 * 1024 * 1024) {
                                    info!(
                                        i,
                                        downloaded = %byte_size(downloaded)
                                    );
                                }

                                if tx.send((i, bytes)).is_err() {
                                    should_stop.store(true, Ordering::Relaxed);
                                    break;
                                }
                            }
                            Err(error) => {
                                error!(%error, i);
                                should_stop.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                    }
                });
            }
        });

    let encoders = Arc::new(std::sync::Mutex::new(
        (0..rayon::current_num_threads())
            .map(|_| None)
            .collect::<Vec<_>>(),
    ));

    rx.into_iter()
        .par_bridge()
        .try_for_each(|(i, bytes)| -> Result<()> {
            info!(i, "Processing shard...");
            let thread_idx = rayon::current_thread_index().expect("thread index is not available");

            // Get or create encoder for this thread
            let mut encoders = encoders.lock().expect("failed to lock encoders");
            if encoders[thread_idx].is_none() {
                let file_name = format!("shard-{i}.zst");
                info!("Writing to {file_name}...");
                let encoder = Box::new(
                    zstd::Encoder::new(
                        File::create(args.output_dir.join(file_name))?,
                        // The compression level for the zstd encoder.
                        // The higher the level, the more compressed the data will be.
                        // Decompressing is same speed regardless of the level.
                        19,
                    )?
                    .auto_finish(),
                );
                encoders[thread_idx] = Some(encoder);
            }

            let encoder = encoders[thread_idx]
                .as_mut()
                .expect("encoder is not available")
                .as_mut();

            ParquetRecordBatchReaderBuilder::try_new(bytes)?
                .with_batch_size(1024)
                .build()?
                .filter_map(|batch| {
                    if let Err(error) = &batch {
                        error!(%error);
                    }
                    batch.ok()
                })
                .map(|batch| {
                    // see dataset card for column names
                    // https://huggingface.co/datasets/uonlp/CulturaX
                    let text = get_parquet_column(&batch, "text")?;

                    let rows = (0..batch.num_rows())
                        .map(|i| text.value(i))
                        .par_bridge()
                        .fold(Vec::<u8>::new, |mut buffer, text| {
                            buffer.extend(normalize_text(text).as_bytes());

                            // Add end-of-text character
                            // https://en.wikipedia.org/wiki/C0_and_C1_control_codes
                            buffer.extend(END_OF_TEXT);

                            buffer
                        })
                        .reduce(Vec::new, |mut curr, other| {
                            curr.extend(other);
                            curr
                        });

                    anyhow::Ok(rows)
                })
                .filter_map(|result| {
                    if let Err(error) = &result {
                        error!(%error);
                    }
                    result.ok()
                })
                .map(move |bytes| {
                    encoder.write_all(&bytes)?;
                    anyhow::Ok(())
                })
                .for_each(|result| {
                    if let Err(error) = result {
                        error!(%error);
                    }
                });

            info!(i, "Finished writing shard");

            Ok(())
        })?;

    Ok(())
}

/// Download the parquet file in its entirety.
///
/// ---
///
/// Parquet isn't a stream based format.
/// It was designed to be written like so: content, then metadata (in a footer).
/// This means the entire parquet file must be known before any data can be
/// processed. The "async" option just means we don't have to load the entire
/// file into memory but we do need random access to the entire file.
///
/// Certain Cloud providers support random access of object files (with added
/// network latencies).
///
/// For maximal efficiency, we'll download the entire file in memory
/// and work with batches of rows.
///
/// University of Oregon NLP group chose an
/// appropriate file size to make this approach possible.
async fn download_parquet(i: usize, client: Client) -> Result<Bytes> {
    // https://huggingface.co/datasets/uonlp/CulturaX
    match client
        .get(format!(
            "https://huggingface.co/datasets/uonlp/CulturaX/resolve/main/en/en_part_{i:05}.parquet",
        ))
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => Ok(response.bytes().await?),
        Err(error) => Err(error.into()),
    }
}
