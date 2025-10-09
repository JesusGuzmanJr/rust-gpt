use {
    anyhow::Result,
    itertools::Itertools,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    std::{
        io::BufRead,
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
        time::Instant,
    },
    tracing::*,
};

/// Hash a directory of zstd-compressed files.
///
/// If either the file name or
/// contents change, the hash will change.
pub fn hash_directory(input_dir: &Path) -> Result<u64> {
    debug!(input_dir = %input_dir.display(), "Hashing directory");
    let start = Instant::now();
    let bytes_read = AtomicUsize::new(0);

    let hash = std::fs::read_dir(input_dir)?
        .filter_map(|entry| entry.ok())
        .map(|dir| dir.path())
        .filter(|path| path.is_file() && path.extension().unwrap_or_default() == "zst")
        .collect::<Vec<_>>()
        .par_iter()
        .map(|file_path| {
            let file = std::fs::File::open(file_path)?;
            // advisory that other processes should not modify files; not mandatory
            file.lock_shared()?;
            anyhow::Ok((file_path, file))
        })
        .filter_map(|result| {
            if let Err(error) = &result {
                error!("{error}");
            }
            result.ok()
        })
        .collect::<Vec<_>>()
        .par_iter()
        .map(|(file_path, file)| {
            let mut buffer = String::new();
            let mut hasher = xxhash_rust::xxh3::Xxh3::new();
            let mut reader = std::io::BufReader::new(zstd::Decoder::new(file)?);

            loop {
                // clearing is O(1).
                buffer.clear();
                let read = reader.read_line(&mut buffer)?;

                if read == 0 {
                    break;
                }

                bytes_read.fetch_add(read, Ordering::Relaxed);

                hasher.update(buffer.as_bytes());
            }

            anyhow::Ok((file_path, hasher.digest()))
        })
        .filter_map(|result| {
            if let Err(error) = &result {
                error!("{error}");
            }
            result.ok()
        })
        .collect_vec_list()
        .into_iter()
        .flatten()
        .sorted_by_key(|(file_path, _)| *file_path)
        .fold(
            xxhash_rust::xxh3::Xxh3::new(),
            |mut hasher, (file_path, hash)| {
                hasher.update(file_path.as_os_str().as_encoded_bytes());
                hasher.update(hash.to_be_bytes().as_slice());
                hasher
            },
        )
        .digest();

    debug!(
        elapsed = ?start.elapsed(),
        bytes_read = %bytesize::ByteSize::b(bytes_read.load(Ordering::Relaxed) as u64)
            .display()
            .iec(),

        "Done hashing directory"
    );

    Ok(hash)
}
