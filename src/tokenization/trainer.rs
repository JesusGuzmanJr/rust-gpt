use {
    super::Token,
    anyhow::Result,
    serde::{Deserialize, Serialize},
};

/// A byte-level BPE trained tokenizer model.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrainerConfig {
    /// The regex used to pre-tokenize the text before BPE.
    pub pre_tokenization_regex: String,

    /// The merges learned by BPE.
    pub merges: Vec<Token>,
}

impl crate::Bincode for TrainerConfig {}
