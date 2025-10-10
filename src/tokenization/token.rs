//! A module to define tokens structures and functions.

use {
    serde::{Deserialize, Serialize},
    smallvec::SmallVec,
    std::fmt,
};

/// A byte string representation of a token.
///
/// Up to 16 bytes can be allocated on the stack before heap allocating.
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token(SmallVec<[u8; 16]>);

/// A unique identifier for a token. Up to 4,294,967,296 tokens can be
/// represented.
pub type TokenId = u32;

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
}

impl std::ops::Add for Token {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        // Extend in place to avoid unnecessary allocations
        self.0.extend(other.0);
        self
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
