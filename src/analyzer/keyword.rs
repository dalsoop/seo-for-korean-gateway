//! Keyword counting strategy.
//!
//! `KeywordCounter` is the abstraction every check uses for "where / how often
//! does the keyword appear". Production wiring uses `LinderaCounter` (lives
//! in `crate::lindera_counter`) so this module stays lindera-free — that
//! keeps analyzer unit tests fast (no dictionary load) and the trait small.
//!
//! `RegexCounter` is the test-only baseline that mirrors the PHP plugin
//! fallback. It exists so the analyzer's 35-check suite can be exercised
//! without a tokenizer, and so a future regression caught by the PHP
//! fallback can be reproduced here verbatim.

/// Keyword matching backend. Every check that asks "where/how often does the
/// keyword appear" routes through here so we can swap engines centrally.
pub trait KeywordCounter: Send + Sync {
    fn count(&self, text: &str, keyword: &str) -> usize;

    /// Byte offset (start, end) of the first match — used by title-position
    /// % calculation. Returns None when the keyword is empty or absent.
    fn find_first(&self, text: &str, keyword: &str) -> Option<(usize, usize)>;

    /// Engine label echoed back in the analyze response.
    fn engine(&self) -> &'static str;
}

#[cfg(test)]
pub use test_support::RegexCounter;

#[cfg(test)]
mod test_support {
    use std::sync::Arc;

    use once_cell::sync::Lazy;
    use regex::Regex;

    use super::KeywordCounter;
    use crate::analyzer::helpers::PARTICLES;

    /// Particle-aware regex counter. Mirrors the PHP plugin fallback exactly.
    pub struct RegexCounter;

    impl RegexCounter {
        pub fn shared() -> Arc<dyn KeywordCounter> {
            static INSTANCE: Lazy<Arc<dyn KeywordCounter>> =
                Lazy::new(|| Arc::new(RegexCounter) as Arc<dyn KeywordCounter>);
            INSTANCE.clone()
        }

        fn pattern(keyword: &str) -> Option<Regex> {
            if keyword.is_empty() {
                return None;
            }
            let pattern = format!("{}(?:{})?", regex::escape(keyword), PARTICLES);
            Regex::new(&pattern).ok()
        }
    }

    impl KeywordCounter for RegexCounter {
        fn count(&self, text: &str, keyword: &str) -> usize {
            Self::pattern(keyword)
                .map(|re| re.find_iter(text).count())
                .unwrap_or(0)
        }

        fn find_first(&self, text: &str, keyword: &str) -> Option<(usize, usize)> {
            let re = Self::pattern(keyword)?;
            re.find(text).map(|m| (m.start(), m.end()))
        }

        fn engine(&self) -> &'static str {
            "regex"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_counter_picks_up_particles() {
        let c = RegexCounter;
        let text = "워드프레스를 사용하면 워드프레스 플러그인을 만들 수 있다. 워드프레스가 좋다.";
        assert_eq!(c.count(text, "워드프레스"), 3);
    }

    #[test]
    fn regex_counter_find_first_returns_byte_offsets() {
        let c = RegexCounter;
        let text = "한국어 SEO 가이드";
        let (s, e) = c.find_first(text, "SEO").unwrap();
        assert_eq!(&text[s..e], "SEO");
    }

    #[test]
    fn regex_counter_empty_keyword_zero() {
        let c = RegexCounter;
        assert_eq!(c.count("anything", ""), 0);
        assert_eq!(c.find_first("anything", ""), None);
    }
}
