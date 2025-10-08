//! A module to define tokens structures and functions.

use {
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
    std::fmt,
    thousands::Separable,
};

/// A byte string representation of a token.
///
/// Up to 16 bytes can be allocated on the stack before heap allocating.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token(SmallVec<[u8; 16]>);

/// A unique identifier for a token.
///
/// Allows for up to 4,294,967,296 tokens.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenId(u32);

impl Token {
    /// Copy the bytes from a slice into a new token.
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Self {
        Self(SmallVec::from_slice(slice))
    }

    /// Get a slice of the token's bytes.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    /// Get the token's length in bytes.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the token is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::ops::Add for Token {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.into_iter().chain(other.0).collect())
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{:?}",
            String::from_utf8_lossy(self.as_slice())
        ))
    }
}

impl fmt::Debug for TokenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.0.separate_with_commas()))
    }
}
