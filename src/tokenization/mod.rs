use {icu_normalizer::ComposingNormalizerBorrowed, std::sync::LazyLock};

mod model;
mod token;
mod tokenizer;
mod trainer;

pub use {model::*, token::*, tokenizer::*, trainer::*};

/// Unicode NFC normalizer.
static NFC: LazyLock<ComposingNormalizerBorrowed<'_>> =
    LazyLock::new(ComposingNormalizerBorrowed::new_nfc);

/// Remove control characters and normalize the string using NFC.
///
/// ---
///
/// ## Normalization Form C (Canonical Composition).
/// Process:
/// - Decompose everything canonically (like NFD).
/// - Recompose wherever there’s a single canonical equivalent pre-composed
///   character.
///
/// The key is: NFC only recomposes when there exists exactly one
/// pre-composed form. So it either:
/// Shrinks (if multiple code points → 1 pre-composed code point).
/// Leaves unchanged (if no pre-composed exists).
///
/// It never expands into more code points, because that would break
/// canonical equivalence.
///
/// See [Unicode normalization forms](https://www.unicode.org/reports/tr15).
pub fn normalize_text(text: &str) -> String {
    // Remove control characters and normalize whitespace
    let text = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n') // Keep newlines
        .collect::<String>();

    // Apply Unicode NFC normalization
    let mut string = String::with_capacity(text.len());
    NFC.normalize_to(&text, &mut string)
        .expect("failed to normalize text");

    string
}
