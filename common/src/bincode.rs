use {
    anyhow::{Context, Result},
    bincode::config::Configuration,
    serde::{Serialize, de::DeserializeOwned},
};

const CONFIG: Configuration = bincode::config::standard();

/// Serialize the given value into bytes.
pub fn serialize<T>(input: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    bincode::serde::encode_to_vec(input, CONFIG).context("failed to encode into bytes")
}

/// Deserialize bytes into a value.
pub fn deserialize<T>(bytes: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    bincode::serde::borrow_decode_from_slice(bytes, CONFIG)
        .map(|(value, _bytes_read)| value)
        .context("failed to decode from bytes")
}
