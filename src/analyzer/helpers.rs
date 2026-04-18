//! Shared regex utilities. Compiled once at startup, used across check modules.

use once_cell::sync::Lazy;
use regex::Regex;

/// Common Korean particles appended after nouns. Naive — V2 uses morphology.
pub const PARTICLES: &str = "을|를|이|가|은|는|에|에서|의|와|과|도|만|보다|에게|께|로|으로|로서|으로서|로써|으로써|만큼|처럼|같이|마저|조차|이나|나|이라도|라도|이라고|라고|이라며|라며";

pub static SCRIPT_STYLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?si)<(script|style)[^>]*?>.*?</(?:script|style)>").unwrap());
pub static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());
pub static WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
pub static H2: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<h2\b[^>]*>").unwrap());
pub static H2_INNER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<h2\b[^>]*>(.*?)</h2>").unwrap());
pub static P_INNER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<p\b[^>]*>(.*?)</p>").unwrap());
pub static IMG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<img\b[^>]*>").unwrap());
pub static IMG_ALT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)\balt\s*=\s*"[^"]+""#).unwrap());
pub static NON_ASCII: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\x00-\x7F]").unwrap());
pub static A_HREF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<a\s+[^>]*?href\s*=\s*"([^"]+)""#).unwrap());
pub static SENTENCE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?。?]+\s*").unwrap());

pub fn strip_html(html: &str) -> String {
    let stripped = SCRIPT_STYLE.replace_all(html, "");
    let no_tags = TAG.replace_all(&stripped, " ");
    WHITESPACE.replace_all(&no_tags, " ").trim().to_string()
}

pub fn keyword_regex(keyword: &str) -> Option<Regex> {
    if keyword.is_empty() {
        return None;
    }
    let pattern = format!("{}(?:{})?", regex::escape(keyword), PARTICLES);
    Regex::new(&pattern).ok()
}

pub fn keyword_count(text: &str, keyword: &str) -> usize {
    keyword_regex(keyword)
        .map(|re| re.find_iter(text).count())
        .unwrap_or(0)
}
