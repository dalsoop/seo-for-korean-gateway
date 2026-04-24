//! Lindera-backed KeywordCounter — the production path when the morphology
//! dictionary is loaded.
//!
//! Counting strategy: tokenize both keyword and text via mecab-ko-dic, then
//! slide-match the keyword's surface sequence over the text's surface
//! sequence. lindera splits particles off as separate tokens, so '워드프레스를'
//! becomes [워드프레스, 를] and a search for '워드프레스' matches naturally
//! without the regex particle list ever being consulted.
//!
//! find_first returns the byte_start/byte_end of the matched span in the
//! original text by reading lindera's Token::byte_start/byte_end fields —
//! preserves Korean character boundaries cleanly.

use std::sync::Arc;

use lindera::tokenizer::Tokenizer;

use crate::analyzer::keyword::KeywordCounter;

pub struct LinderaCounter {
    tokenizer: Arc<Tokenizer>,
}

impl LinderaCounter {
    pub fn new(tokenizer: Arc<Tokenizer>) -> Self {
        Self { tokenizer }
    }

    /// Tokenize and return owned (surface, byte_start, byte_end) tuples.
    /// Tokenization failures yield an empty vec — callers fall through to
    /// "0 matches" / "no first match", which is the right behavior for SEO
    /// scoring (no spurious passes).
    fn tokens(&self, text: &str) -> Vec<(String, usize, usize)> {
        match self.tokenizer.tokenize(text) {
            Ok(toks) => toks
                .into_iter()
                .map(|t| (t.surface.to_string(), t.byte_start, t.byte_end))
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

impl KeywordCounter for LinderaCounter {
    fn count(&self, text: &str, keyword: &str) -> usize {
        if keyword.is_empty() || text.is_empty() {
            return 0;
        }
        let key = self.tokens(keyword);
        if key.is_empty() {
            return 0;
        }
        let hay = self.tokens(text);
        if hay.len() < key.len() {
            return 0;
        }

        let mut matches = 0usize;
        let mut i = 0usize;
        while i + key.len() <= hay.len() {
            let mut hit = true;
            for j in 0..key.len() {
                if hay[i + j].0 != key[j].0 {
                    hit = false;
                    break;
                }
            }
            if hit {
                matches += 1;
                i += key.len();
            } else {
                i += 1;
            }
        }
        matches
    }

    fn find_first(&self, text: &str, keyword: &str) -> Option<(usize, usize)> {
        if keyword.is_empty() || text.is_empty() {
            return None;
        }
        let key = self.tokens(keyword);
        if key.is_empty() {
            return None;
        }
        let hay = self.tokens(text);
        if hay.len() < key.len() {
            return None;
        }

        let mut i = 0usize;
        while i + key.len() <= hay.len() {
            let mut hit = true;
            for j in 0..key.len() {
                if hay[i + j].0 != key[j].0 {
                    hit = false;
                    break;
                }
            }
            if hit {
                return Some((hay[i].1, hay[i + key.len() - 1].2));
            }
            i += 1;
        }
        None
    }

    fn engine(&self) -> &'static str {
        "lindera"
    }
}
