//! A module to define tokens structures and functions.

use {
    super::{Token, TokenizerModel},
    crate::tokenization::TokenId,
    anyhow::Result,
    regex::Regex,
    smallvec::SmallVec,
    std::{fs::File, io::Read, path::Path},
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

    /// Convert a string into token IDs.
    pub fn encode(&self, text: &str) -> Vec<TokenId> {
        let text = super::normalize_text(text);

        self.regex
            .find_iter(&text)
            .map(|m| {
                m.as_str()
                    .as_bytes()
                    .iter()
                    .map(|b| (Token::from_slice(&[*b]), *b as TokenId))
                    .collect::<SmallVec<[_; 32]>>()
            })
            .flat_map(|mut tokens| {
                for (i, merge) in self.merges.iter().enumerate() {
                    let i = i as TokenId + 256;
                    let mut skip = false;
                    let mut processed_tokens = tokens
                        .windows(2)
                        .filter_map(|window| {
                            if skip {
                                skip = false;
                                return None;
                            }

                            // if this is the merge, push the entire merge
                            if merge == &(window[0].0.clone() + window[1].0.clone()) {
                                skip = true;
                                Some((merge.clone(), i))
                            } else {
                                Some(window[0].clone())
                            }
                        })
                        .collect::<SmallVec<[_; 32]>>();

                    if !skip {
                        processed_tokens.extend(tokens.last().cloned());
                    }

                    tokens = processed_tokens;
                }
                tokens
            })
            .map(|(_, id)| id)
            .collect::<Vec<_>>()
    }

    /// Convert token IDs into a string.
    pub fn decode(&self, ids: &[TokenId]) -> String {
        String::from_utf8_lossy(
            &ids.iter()
                .filter_map(|id| self.merges.get(*id as usize).map(Token::as_slice))
                .flatten()
                .cloned()
                .collect::<Vec<_>>(),
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let tokenizer = Tokenizer {
            regex: Regex::new(r"\p{L}+ ?").unwrap(),
            merges: vec![
                Token::from_slice(b"er"),     // 256
                Token::from_slice(b"er "),    // 257
                Token::from_slice(b"ne"),     // 258
                Token::from_slice(b"new"),    // 259
                Token::from_slice(b"lo"),     // 260
                Token::from_slice(b"low"),    // 261
                Token::from_slice(b"newer "), // 262
                Token::from_slice(b"low "),   // 263
            ],
        };

        let ids = tokenizer.encode("Hello world");
        #[rustfmt::skip]
        assert_eq!(
            ids,
            [
                'H' as u32,
                'e' as u32,
                'l' as u32,
                260, // merge of 'l' and 'o'
                ' ' as u32,
                'w' as u32,
                'o' as u32,
                'r' as u32,
                'l' as u32,
                'd' as u32,
            ]
            .iter()
            .map(|id| *id as TokenId)
            .collect::<Vec<_>>()
        );
    }
}
