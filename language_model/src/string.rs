use {
    crate::tokenization::normalize_text,
    derive_more::Display,
    serde::{Deserialize, Serialize},
    std::ops::Deref,
};

/// A container for a normalized string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)]
pub struct NormString(String);

impl NormString {
    /// Create a new normalized string from a string slice.
    pub fn new(text: impl AsRef<str>) -> Self {
        let mut text = normalize_text(text.as_ref().trim());
        text.shrink_to_fit();
        Self(text)
    }

    /// Returns the length in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the length is zero bytes.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for NormString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for NormString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
