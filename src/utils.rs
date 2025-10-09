use {
    anyhow::{Context, Result},
    serde::{Serialize, de::DeserializeOwned},
    std::path::Path,
};

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
