use {
    ahash::HashMap,
    anyhow::Result,
    clap::Parser,
    regex::Regex,
    smallvec::SmallVec,
    std::{path::PathBuf, sync::LazyLock},
};

/// Pretokenize a directory of normalized, compressed markdown shards.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the input directory with the normalized, compressed markdown
    /// shards.
    ///
    /// Only *.md.zstd files will be processed. The directory will not be walked
    /// recursively.
    #[arg(short, long)]
    input_dir: PathBuf,

    /// Path to save the word count file.
    ///
    /// The file will be saved as `word-count.bincode`.
    #[arg(short, long)]
    output_dir: PathBuf,
}

/// Regex to split English text into tokens.
///
/// ---
///
/// We use optional leading space to preserve spacing information.
/// E.g.
/// ```txt
/// "The cat" → ["The", " cat"]
/// "cat"     → ["cat"]
/// ```
/// The token "cat" and " cat" are **different** tokens with different IDs. This
/// helps the model understand positional context.
///  During training, neural networks learn embeddings (vector representations)
/// for each token. The model will learn that:
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
/// Having "cat" and " cat" uses 2 slots, which is negligible. The benefit of
/// capturing positional information far outweighs the cost.
static WORD_SPLITTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r" ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+").expect("invalid regex")
});

fn main() -> Result<()> {
    let args = Args::parse();

    // Vocabulary as byte strings
    let mut vocab: HashMap<SmallVec<[u8; 32]>, u32> = HashMap::default();
    vocab.insert(SmallVec::from_slice(b"Hello"), 1);
    vocab.insert(SmallVec::from_slice(&[0x20]), 2); // space

    let line = "Hello world";
    WORD_SPLITTER
        .find_iter(line)
        .map(|w| SmallVec::<[u8; 32]>::from_slice(w.as_str().as_bytes()))
        .for_each(|w| {
            vocab.entry(w).or_insert(1);
        });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex() {
        assert_eq!(
            &WORD_SPLITTER
                .find_iter("$51 for ticket 💸.")
                .map(|m| m.as_str())
                .collect::<Vec<_>>(),
            &["$", "51", " for", " ticket", " 💸."]
        );
    }
}
