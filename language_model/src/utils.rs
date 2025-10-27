use {
    anyhow::{Context, Result},
    arrow_array::{GenericByteArray, RecordBatch, types::GenericStringType},
    serde::{Serialize, de::DeserializeOwned},
    std::path::{Path, PathBuf},
};

/// End of text character U+0003 as UTF-8 encoded bytes.
///
/// https://en.wikipedia.org/wiki/C0_and_C1_control_codes
pub const END_OF_TEXT: &[u8] = "\u{3}".as_bytes();

/// Get a reference to a column's array by name.
pub fn get_parquet_column<'a>(
    record_batch: &'a RecordBatch,
    column: &'static str,
) -> Result<&'a GenericByteArray<GenericStringType<i32>>> {
    record_batch
        .column_by_name(column)
        .with_context(|| format!("missing {column} column"))?
        .as_any()
        .downcast_ref::<arrow_array::StringArray>()
        .context("content is not a StringArray")
}

/// Format the byte size as a human readable string.
pub fn byte_size(bytes: usize) -> impl std::fmt::Display {
    bytesize::ByteSize::b(bytes as _).display().iec()
}

/// Bincode configuration for all serialization and deserialization operations.
const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

/// Set up tracing subscriber to log with relative time.
pub fn setup_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::Uptime::default())
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
}

/// A trait for types that can be serialized and deserialized using bincode.
pub trait Bincode: Serialize + DeserializeOwned + Sized {
    /// Deserialize a tokenizer model from bytes.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(
            bincode::serde::decode_from_slice::<Self, _>(bytes, BINCODE_CONFIG)
                .context("failed to deserialize")?
                .0,
        )
    }

    /// Serialize a tokenizer model to bytes.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serde::encode_to_vec(self, BINCODE_CONFIG).context("failed to serialize")
    }
}

/// A trait for config types that can be read from .ron files.
pub trait Ron: DeserializeOwned {
    /// Serialize the file into `Self`.
    fn from_file_path(file_path: &Path) -> Result<Self> {
        ron::de::from_bytes::<Self>(&std::fs::read(file_path)?)
            .with_context(|| format!("failed to parse: {}", file_path.display()))
    }
}

/// Recursively collect all files with a given extension from a directory.
pub fn collect_files_recursive(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.is_dir() {
        anyhow::bail!("input-dir is not a directory: {}", dir.display());
    }

    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // recurse!
            files.extend(collect_files_recursive(&path, extension)?);
        } else if path.is_file() && path.extension().unwrap_or_default() == extension {
            files.push(path);
        }
    }

    Ok(files)
}
