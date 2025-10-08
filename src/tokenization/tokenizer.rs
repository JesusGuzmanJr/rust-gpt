//! A module to define tokens structures and functions.

use {
    super::{Token, TokenizerModel},
    anyhow::Result,
    regex::Regex,
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
    std::{fmt, fs::File, io::Read, path::Path},
};

/// A tokenizer.
#[derive(Debug)]
pub struct Tokenizer {
    regex: Regex,
    merges: Vec<Token>,
}

impl Tokenizer {
    /// Read a tokenizer model into memory.
    pub fn from_file(path: &Path) -> Result<Self> {
        let mut buffer = Vec::new();

        File::open(path)?.read_to_end(&mut buffer)?;

        let TokenizerModel {
            pre_tokenization_regex,
            merges,
        } = TokenizerModel::from_bytes(&buffer)?;

        Ok(Self {
            regex: Regex::new(&pre_tokenization_regex)?,
            merges,
        })
    }

    /// Convert a string into token ids.
    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        let text = super::normalize_text(text)?;
        unimplemented!()
    }
}
