use {
    ahash::AHashSet,
    anyhow::{Context, Result},
    clap::Parser,
    futures::StreamExt,
    rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    rust_gpt::{tokenization::normalize_text, utils::byte_size},
    std::{io::Write, os::unix::ffi::OsStrExt, path::PathBuf},
    tracing::*,
};

/// The maximum number of concurrent downloads.
///
/// Each parquet file is ~2.5 GB.
const DOWNLOAD_CONCURRENCY: usize = 8;

/// The number of rows to read at a time from the parquet file.
const BATCH_SIZE: usize = 32;

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

#[tokio::main]
async fn main() -> Result<()> {
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

    // The English dataset is 7.54 TB and it's split into ~2.5 GB files so thats
    // ~3016 files whose paths fit in memory.
    let processed_file_indices = std::fs::read_dir(&args.output_dir)?
        .par_bridge()
        .filter_map(|result| result.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension() == Some(std::ffi::OsStr::new("parquet")))
        .filter_map(|path| {
            // path ends with "en_part_XXXXX.parquet"
            // split and get the last bytes corresponding to "XXXXX.parquet"
            path.as_os_str()
                .as_bytes()
                .split_last_chunk::<13>()
                .and_then(|(_, ending)| {
                    // ending is bytes corresponding to "XXXXX.parquet"
                    // split and get the first bytes corresponding to "XXXXX"
                    ending.split_last_chunk::<8>().map(|(number, _)| number)
                })
                .and_then(|number| String::from_utf8_lossy(number).parse::<usize>().ok())
        })
        .fold(AHashSet::default, |mut set, number| {
            set.insert(number);
            set
        })
        .reduce(AHashSet::default, |mut curr, other| {
            curr.extend(other);
            curr
        });

    // Larger than projected number of files but we'll handle 404s gracefully.
    const HIGHEST_FILE_INDEX: usize = 2; // TODO: change back to 3500;

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

    futures::stream::iter(
        (0..=HIGHEST_FILE_INDEX)
            .filter(|i| !processed_file_indices.contains(i))
            .map(|i| {
                let client = client.clone();
                async move {
                    info!(i, "Downloading parquet...");
                    (i, client.get(url(i)).send().await)
                }
            }),
    )
    .buffer_unordered(DOWNLOAD_CONCURRENCY)
    .filter_map(|(i, result)| async move {
        match result {
            Ok(response) => Some((i, response)),
            Err(error) => {
                error!(i, "{error}");
                None
            }
        }
    })
    .filter_map(|(i, response)| async move {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            info!(i, url = %response.url(), "Not found");
            None
        } else if !response.status().is_success() {
            let status_code = response.status();
            let url = response.url().clone();
            let error = response.text().await.expect("failed to get error response");
            error!(%status_code, %url, error);
            None
        } else {
            // Parquet isn't a stream based format.
            // It was designed to be written like so: content then metadata (in a footer).
            // This means the entire parquet file must be known before any data can be processed.
            // The "async" of it just means we don't have to load the entire file into memory but we need random access to the ENTIRE file.
            // Certain Cloud providers support random access of object files (with added network latencies)
            // For maximal efficiency, we'll download the entire file in memory and work with batches of rows.
            // University of Oregon NLP group chose an appropriate file size to make this approach possible.
            Some((i, response.bytes().await))
        }
    })
    .then(|(i, bytes)| async move {
        anyhow::Ok((
            i,
            parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(bytes?)?
                .with_batch_size(BATCH_SIZE)
                .build()?,
        ))
    })
    .filter_map(|result| async move {
        match result {
            Ok((i, stream)) => Some((i, stream)),
            Err(error) => {
                error!("{error}");
                None
            }
        }
    })
    .map(|(i, reader)| {
        let output_dir = args.output_dir.clone();
        async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            // bridge from IO world to CPU world
            rayon::spawn({
                let i = i;

                move || {
                    let result = || async move {
                        let file_name = format!("shard-{i}.md.zst");
                        info!(i, "Writing to {file_name}...");
                        let mut encoder = Box::new(
                            zstd::Encoder::new(
                                std::fs::File::create(output_dir.join(file_name))?,
                                // The compression level for the zstd encoder.
                                // The higher the level, the more compressed the data will be.
                                // Decompressing is same speed regardless of the level.
                                19,
                            )?
                            .auto_finish(),
                        );

                        let mut written = 0;
                        reader
                            .filter_map(|batch| {
                                if let Err(error) = &batch {
                                    error!("{error}");
                                }
                                batch.ok()
                            })
                            .map(|batch| {
                                let text = rust_gpt::utils::get_parquet_column(&batch, "text")?;

                                // parallel process the strings of text and collect as bytes
                                let bytes = (0..batch.num_rows())
                                    .map(|i| text.value(i))
                                    .collect::<Vec<_>>()
                                    .par_iter()
                                    .map(|text| {
                                        normalize_text(text)
                                            .as_bytes()
                                            .iter()
                                            .chain(rust_gpt::utils::END_OF_TEXT.iter())
                                            .cloned()
                                            .collect::<Vec<_>>()
                                    })
                                    .flatten()
                                    .collect::<Vec<_>>();

                                encoder.write_all(&bytes)?;
                                written += bytes.len();

                                if written.is_multiple_of(1000) {
                                    info!(
                                        i,
                                        written = %byte_size(written)
                                    );
                                }

                                anyhow::Ok(())
                            })
                            .for_each(|result| {
                                if let Err(error) = result {
                                    error!("{error}");
                                }
                            });

                        info!(
                            i,
                            written = %byte_size(written),
                            "Done writing"
                        );
                        anyhow::Ok(())
                    };
                    drop(tx.send(result()));
                }
            });
            rx.await.expect("channel is closed").await
        }
    })
    .for_each(|result| async {
        if let Err(error) = result.await {
            error!("{error}");
        }
    })
    .await;

    Ok(())
}

/// Get the URL of the CulturaX parquet file.
fn url(index: usize) -> String {
    // https://huggingface.co/datasets/uonlp/CulturaX
    format!(
        "https://huggingface.co/datasets/uonlp/CulturaX/resolve/main/en/en_part_{index:05}.parquet",
    )
}
