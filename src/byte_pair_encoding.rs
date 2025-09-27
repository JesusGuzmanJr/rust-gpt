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
