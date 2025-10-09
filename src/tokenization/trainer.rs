use {
    super::Token,
    anyhow::Result,
    serde::{Deserialize, Serialize},
};

/// A byte-level BPE trained tokenizer model.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trainer {
    /// The regex used to pre-tokenize the text before BPE.
    pub pre_tokenization_regex: String,

    /// The merges learned by BPE.
    pub merges: Vec<Token>,
}

impl Trainer {
    const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

    /// Deserialize a tokenizer model from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(bincode::serde::decode_from_slice::<Self, _>(bytes, Self::BINCODE_CONFIG)?.0)
    }

    /// Serialize a tokenizer model to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(bincode::serde::encode_to_vec(self, Self::BINCODE_CONFIG)?)
    }
}
