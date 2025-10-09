use {
    anyhow::Result,
    clap::Parser,
    itertools::Itertools,
    rayon::iter::{
        IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator,
        ParallelIterator,
    },
    regex::Regex,
    rust_gpt::tokenization::{Token, TokenizerModel},
    smallvec::SmallVec,
    std::{
        fs::File,
        io::{BufRead, Seek, Write},
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::Instant,
    },
    thousands::Separable,
    tracing::*,
};

/// The pre-tokenization regex to use by default.
///
/// ---
///
/// ## Design
///
/// We want to capture a leading space to preserve spacing information.
/// E.g.
/// ```txt
/// "The cat" → ["The", " cat"]
/// "cat"     → ["cat"]
/// ```
/// The token "cat" and " cat" are **different** tokens with different IDs.
/// This helps the model understand positional context.
///  During training, neural networks learn embeddings (vector
/// representations) for each token. The model will learn that:
/// - "cat" and " cat" have similar meanings (both refer to the animal)
/// - But they have different positional contexts
///
/// Through millions of training examples, the embeddings naturally become
/// similar:
/// ```txt
/// embedding("cat") ≈ embedding(" cat")  (but not identical)
/// ```
/// The model learns this relationship automatically from data, just like it
/// learns that "cat" and "cats" are related.
///
/// Having both tokens uses more vocabulary space. But:
///
/// Typical vocabulary: 50,000 tokens
/// - ~256 base bytes
/// - ~49,744 merged tokens (learned subwords)
///
/// Having "cat" and " cat" uses 2 slots, which is negligible. The benefit
/// of capturing positional information far outweighs the cost.
const DEFAULT_PRE_TOKENIZATION_REGEX: &str = r" ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+";

fn main() {}
