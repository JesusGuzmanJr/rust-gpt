use {
    ahash::AHashSet,
    anyhow::{Context, Result},
    clap::Parser,
    futures::StreamExt,
    rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator},
    rust_gpt::tokenization::normalize_text,
    std::{io::Write, os::unix::ffi::OsStrExt, path::PathBuf},
    tracing::*,
};

/// The maximum number of concurrent downloads.
///
/// Each parquet file is ~2.5 GB.
///
/// The Parquet stream reader requires a cursor (random access) so the
/// entire file needs to be downloaded. For the sake of simplicity, we'll load
/// the entire file in memory instead of saving it to disk.
const DOWNLOAD_CONCURRENCY: usize = 10;

/// The number of rows to read at a time from the parquet file.
const BATCH_SIZE: usize = 16;

/// Download, parse and preprocess CulturaX parquet files
/// creating shards of normalized, compressed markdown.
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
    let processed_file_numbers = std::fs::read_dir(&args.output_dir)?
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
    const HIGHEST_FILE_NUMBER: usize = 0; // TODO: change back to 3500;

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
        (0..=HIGHEST_FILE_NUMBER)
            .filter(|number| !processed_file_numbers.contains(number))
            .map(|file_number| (file_number, url(file_number)))
            .map(|(file_number, url)| {
                let client = client.clone();
                async move {
                    let file_name = url.split('/').next_back().expect("invalid URL");
                    info!(%file_name, "Downloading parquet...");
                    (file_number, client.get(&url).send().await)
                }
            }),
    )
    .buffer_unordered(DOWNLOAD_CONCURRENCY)
    .filter_map(|(file_number, result)| async move {
        match result {
            Ok(response) => Some((file_number, response)),
            Err(error) => {
                error!(file_number, "{error}");
                None
            }
        }
    })
    .filter_map(|(file_number, response)| async move {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            info!(file_number, url = %response.url(), "Not found");
            None
        } else if !response.status().is_success() {
            let status_code = response.status();
            let url = response.url().clone();
            let error = response.text().await.expect("failed to get error response");
            error!(%status_code, %url, error);
            None
        } else {
            Some((file_number, response.bytes().await))
        }
    })
    .then(|(file_number, bytes)| async move {
        anyhow::Ok((
            file_number,
            parquet::arrow::ParquetRecordBatchStreamBuilder::new(std::io::Cursor::new(bytes?))
                .await?
                .with_batch_size(BATCH_SIZE)
                .build()?,
        ))
    })
    .filter_map(|result| async move {
        match result {
            Ok((file_number, stream)) => Some((file_number, stream)),
            Err(error) => {
                error!("{error}");
                None
            }
        }
    })
    .map(|(file_number, mut stream)| {
        let output_dir = args.output_dir.clone();
        async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            rayon::spawn({
                let file_number = file_number;

                move || {
                    let result = || async move {
                        let file_name = format!("shard-{file_number}.md.zst");
                        info!("Writing to {file_name}...");
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

                        while let Some(batch) = stream.next().await {
                            let batch = batch?;
                            let text = rust_gpt::utils::get_parquet_column(&batch, "text")?;
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
                        }
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
fn url(file_number: usize) -> String {
    // https://huggingface.co/datasets/uonlp/CulturaX
    format!(
        "https://huggingface.co/datasets/uonlp/CulturaX/resolve/main/en/en_part_{file_number:05}.parquet",
    )
}
