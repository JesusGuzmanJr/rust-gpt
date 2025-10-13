use {
    ahash::AHashSet,
    anyhow::{Context, Result},
    clap::Parser,
    futures::stream::StreamExt,
    parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder,
    rayon::iter::{ParallelBridge, ParallelIterator},
    reqwest::Client,
    rust_gpt::{
        tokenization::normalize_text,
        utils::{END_OF_TEXT, get_parquet_column},
    },
    std::{
        fs::File,
        io::{BufWriter, Seek, SeekFrom, Write},
        os::unix::ffi::OsStrExt,
        path::PathBuf,
        sync::{
            Arc, LazyLock,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    },
    thousands::Separable,
    tokio::sync::Mutex,
    tracing::*,
};

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

    /// The desired index of the files to download and process.
    /// All files at and below the desired index will be downloaded and
    /// processed.
    #[arg(short, long)]
    index: usize,
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

    debug!(?processed_file_indices);

    let client = reqwest::ClientBuilder::new()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.append(
                "Authorization",
                format!(
                    "Bearer {}",
                    std::env::var("HF_TOKEN")
                        .context("HF_TOKEN environment variable is not set")?
                )
                .parse()
                .expect("bad header"),
            );
            headers
        })
        .build()
        .expect("failed to build client");

    let (file_tx, file_rx) = crossbeam_channel::bounded(rayon::current_num_threads());

    // indices of the files that are missing for some reason
    let queue = (0..=args.index)
        .filter(|i| !processed_file_indices.contains(i))
        .collect::<Vec<_>>();

    info!(
        files_to_request = %queue.len().separate_with_commas(),
        "Downloading missing files",
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");

    rt.spawn(async move {
        let queue = Arc::new(Mutex::new(queue));
        let should_stop = Arc::new(AtomicBool::new(false));

        for _ in 0..rayon::current_num_threads() {
            let tx = file_tx.clone();
            let client = client.clone();
            let should_stop = should_stop.clone();

            let missing = queue.clone();

            // launch IO threads to continuously download files
            tokio::spawn(async move {
                while !should_stop.load(Ordering::Acquire) {
                    let i = {
                        if let Some(i) = missing.lock().await.pop() {
                            i
                        } else {
                            should_stop.store(true, Ordering::Release);
                            break;
                        }
                    };

                    let start = Instant::now();
                    info!(i, "Downloading file...");
                    match download_file(i, client.clone()).await {
                        Ok((size, file)) => {
                            let duration = start.elapsed();
                            let size = rust_gpt::utils::byte_size(size);
                            info!(i, ?duration, %size, "Done downloading");
                            if tx.send((i, file)).is_err() {
                                should_stop.store(true, Ordering::Release);
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

    let files_processed = AtomicUsize::new(0);

    // process and compress per CPU core
    file_rx
        .into_iter()
        .par_bridge()
        .try_for_each(|(i, bytes)| -> Result<()> {
            info!(i, "Processing file...");

            let mut encoder = zstd::Encoder::new(
                BufWriter::new(tempfile()?),
                // The compression level for the zstd encoder.
                // The higher the level, the more compressed the data will be.
                // Decompressing is same speed regardless of the level.
                3,
            )?;

            let mut duration = Duration::from_secs(0);

            ParquetRecordBatchReaderBuilder::try_new(bytes)?
                .with_batch_size(128)
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

                    let bytes = (0..batch.num_rows())
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

                    let start = Instant::now();
                    encoder.write_all(&bytes)?;
                    duration += start.elapsed();
                    anyhow::Ok(())
                })
                .for_each(|result| {
                    if let Err(error) = result {
                        error!(%error);
                    }
                });

            let start = Instant::now();
            encoder.flush()?;
            let mut temp_file = encoder.finish()?.into_inner()?;

            temp_file.seek(SeekFrom::Start(0))?;

            std::io::copy(
                &mut temp_file,
                &mut File::create(args.output_dir.join(format!("shard-{i}.zst")))?,
            )?;
            duration += start.elapsed();
            info!(i, ?duration, "Done processing file");
            files_processed.fetch_add(1, Ordering::Relaxed);

            Ok(())
        })?;

    let files_processed = files_processed.load(Ordering::Relaxed);
    info!(files_processed, "Done");

    Ok(())
}

/// Download the parquet file into a temporary file.
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
async fn download_file(i: usize, client: Client) -> Result<(usize, File)> {
    // https://huggingface.co/datasets/uonlp/CulturaX
    let mut stream = client
        .get(format!(
            "https://huggingface.co/datasets/uonlp/CulturaX/resolve/main/en/en_part_{i:05}.parquet",
        ))
        .send()
        .await?
        .error_for_status()?
        .bytes_stream();

    let mut temp_file = tempfile()?;
    let mut size = 0;
    while let Some(Ok(bytes)) = stream.next().await {
        temp_file.write_all(&bytes)?;
        size += bytes.len();
    }

    temp_file.seek(SeekFrom::Start(0))?;

    Ok((size, temp_file))
}

/// Create a temporary file in the user's cache directory.
fn tempfile() -> Result<File> {
    static DIR: LazyLock<PathBuf> = LazyLock::new(|| {
        xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
            .cache_home
            .expect("failed to get cache directory")
    });

    tempfile::tempfile_in(&*DIR).context("failed to create temp file")
}
