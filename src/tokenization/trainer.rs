use {
    super::Token,
    serde::{Deserialize, Serialize},
    std::path::PathBuf,
};

/// A byte-level BPE trained tokenizer model.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenizerTrainingConfig {
    /// Path to the input directory with the normalized, zstd-compressed
    /// preprocessed shards of textual content.
    pub input_dir: PathBuf,

    /// Path to save the tokenizer file.
    pub tokenizer_file: PathBuf,

    /// The target vocabulary size.
    pub vocab_size: usize,

    /// Regex for the initial split of text into words.
    pub pre_tokenization_regex: String,

    /// The merges learned by BPE.
    pub merges: Vec<Token>,
}

impl crate::Bincode for TokenizerTrainingConfig {}
