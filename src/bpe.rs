//! # Byte Pair Encoding
//!
//! Language models don't see text like you and I, instead they see a sequence
//! of numbers (known as tokens). Byte pair encoding (BPE) is a way of
//! converting text into tokens. It has a couple desirable properties:
//!
//! - It's reversible and lossless, so you can convert tokens back into the
//!   original text
//! - It works on arbitrary text, even text that is not in the tokenizer's
//!   training data
//! - It compresses the text: the token sequence is shorter than the bytes
//!   corresponding to the original text. On average, in practice, each token
//!   corresponds to about 4 bytes.
//! - It attempts to let the model see common subwords. For instance, "ing" is a
//!   common subword in English, so BPE encodings will often split "encoding"
//!   into tokens like "encod" and "ing" (instead of e.g. "enc" and "oding").
//!   Because the model will then see the "ing" token again and again in
//!   different contexts, it helps models generalize and better understand
//!   grammar.
//!
//!  ---
//!
//! ### References
//! - [TikToken](https://github.com/openai/tiktoken)
//! - [Tokenizers](https://github.com/huggingface/tokenizers)

#[cfg(test)]
mod tests {
    #[test]
    fn test_normalization() {
        // Normalization Form C (Canonical Composition).
        //
        // Process:
        // - Decompose everything canonically (like NFD).
        // - Recompose wherever there’s a single canonical equivalent pre-composed
        // character.
        //
        // The key is: NFC only recomposes when there exists exactly one
        // pre-composed form. So it either:
        // Shrinks (if multiple code points → 1 pre-composed code point).
        // Leaves unchanged (if no pre-composed exists).
        //
        // It never expands into more code points, because that would break canonical
        // equivalence.
        //
        // https://www.unicode.org/reports/tr15/
        let nfc = icu_normalizer::ComposingNormalizerBorrowed::new_nfc();
        assert_eq!(nfc.normalize("a\u{0308}"), "ä");
        assert!("ä".len() <= "a\u{0308}".len());
    }
}
