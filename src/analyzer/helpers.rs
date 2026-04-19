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
pub static UL_OL: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<(ul|ol)\b").unwrap());

/// Korean transition words / connectors. Combined into one regex so we
/// don't recompile per call. Boundary chars on both sides keep matches honest
/// (don't catch '그러나' inside other words).
pub static TRANSITIONS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:^|[\s,()\[\]。.!?])(그러나|그렇지만|하지만|반면|한편|따라서|그러므로|그래서|결국|결과적으로|예를\s?들어|구체적으로|말하자면|가령|또한|게다가|더불어|더욱이|즉|다시\s?말해|요컨대|우선|먼저|다음으로|마지막으로|끝으로|물론|사실|참고로|반대로|오히려|특히|즉시)"
    ).unwrap()
});

/// 해요체 endings — informal-polite. Followed by sentence-end punctuation
/// or whitespace so we don't misclassify mid-sentence morphemes.
pub static HAEYO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:해요|예요|이에요|에요|네요|어요|아요|거예요|이지요|지요|나요|ㄴ가요|는가요)[\s.!?。]"
    ).unwrap()
});

/// 합쇼체 endings — formal. Same boundary requirement.
/// Korean chat-style / casual-blog markers. Fine in DMs and Twitter,
/// noise in SEO-aimed content.
pub static INFORMAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:ㅋㅋ+|ㅎㅎ+|ㅠㅠ+|ㅜㅜ+|ㅇㅇ|ㄴㄴ|헐\b|대박\b|레알\b|개꿀\b|쩐다\b|굿굿)").unwrap()
});

/// Korean passive-voice markers. Anchored to a sentence-end so
/// '되다' inside a noun like '한정되다' on its own doesn't trigger.
pub static PASSIVE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:되었다|되었습니다|되었어요|된다|됩니다|돼요|받았다|받았습니다|받았어요|받는다|받습니다|당했다|당했습니다|당했어요|지었다|졌다|졌습니다|져요|져졌|되어졌|되어진|이루어졌|이루어진|만들어졌|만들어진|보여진다|보여졌다)[\s.!?。]"
    ).unwrap()
});

/// All headings h1-h6 with their level captured.
pub static HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)<h([1-6])\b[^>]*>").unwrap()
});

pub static HAPSYO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:합니다|입니다|습니다|됩니다|갑니다|옵니다|합니까|입니까|습니까|됩니까|십시오)[\s.!?。]"
    ).unwrap()
});

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
