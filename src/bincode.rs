use {
    anyhow::Result,
    serde::{Serialize, de::DeserializeOwned},
};

/// Bincode configuration for all serialization and deserialization operations.
const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

/// A trait for types that can be serialized and deserialized using bincode.
pub trait Bincode: Serialize + DeserializeOwned + Sized {
    /// Deserialize a tokenizer model from bytes.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bincode::serde::decode_from_slice::<Self, _>(bytes, BINCODE_CONFIG)?.0)
    }

    /// Serialize a tokenizer model to bytes.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serde::encode_to_vec(self, BINCODE_CONFIG)?)
    }
}
