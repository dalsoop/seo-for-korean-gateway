//! Content analyzer — 1:1 port of the PHP Content_Analyzer's 10 checks.
//!
//! Why duplicate the logic in two languages:
//!
//!   - PHP version stays as a fallback that runs locally inside any WP host,
//!     including ones that can't reach the gateway (firewall, offline dev).
//!   - Rust version is the canonical implementation that grows beyond the
//!     PHP version's reach (lindera morphology, LLM hooks, embeddings).
//!     For V1 the two MUST agree exactly so users see identical scores
//!     whether the gateway is up or not.
//!
//! When this file diverges from `includes/modules/content-analyzer/
//! class-content-analyzer.php` in the plugin repo, the Rust version wins —
//! it's the one we keep extending. The PHP fallback should only stay
//! "good enough" to avoid surprising users when they're offline.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

const PARTICLES: &str = "을|를|이|가|은|는|에|에서|의|와|과|도|만|보다|에게|께|로|으로|로서|으로서|로써|으로써|만큼|처럼|같이|마저|조차|이나|나|이라도|라도|이라고|라고|이라며|라며";

static SCRIPT_STYLE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?si)<(script|style)[^>]*?>.*?</(?:script|style)>").unwrap());
static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());
static WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static H2: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<h2\b[^>]*>").unwrap());
static H2_INNER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<h2\b[^>]*>(.*?)</h2>").unwrap());
static P_INNER: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<p\b[^>]*>(.*?)</p>").unwrap());
static IMG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)<img\b[^>]*>").unwrap());
static IMG_ALT: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)\balt\s*=\s*"[^"]+""#).unwrap());
static NON_ASCII: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\x00-\x7F]").unwrap());
static A_HREF: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?is)<a\s+[^>]*?href\s*=\s*"([^"]+)""#).unwrap());
static SENTENCE_END: Lazy<Regex> = Lazy::new(|| Regex::new(r"[.!?。?]+\s*").unwrap());

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub focus_keyword: String,
    #[serde(default)]
    pub meta_description: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub score: u32,
    pub grade: &'static str,
    pub checks: Vec<Check>,
    /// Engine identifier echoed back so the plugin can show "via gateway"
    /// vs "via local fallback" if it wants to.
    pub engine: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub status: Status,
    pub message: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Warning,
    Fail,
    Na,
}

struct Ctx {
    title: String,
    title_length: usize,
    content_html: String,
    content_text: String,
    content_length: usize,
    slug: String,
    focus_keyword: String,
    meta_description: String,
    meta_description_length: usize,
    /// Cached so multiple link checks don't re-walk the regex.
    link_counts: LinkCounts,
}

#[derive(Debug, Clone, Copy)]
struct LinkCounts {
    internal: usize,
    outbound: usize,
}

const ENGINE: &str = "regex";

pub fn analyze(req: AnalyzeRequest) -> AnalyzeResponse {
    let ctx = normalize(req);
    let checks = vec![
        check_title_length(&ctx),
        check_meta_description_length(&ctx),
        check_focus_keyword_present(&ctx),
        check_focus_keyword_in_title(&ctx),
        check_focus_keyword_in_first_paragraph(&ctx),
        check_focus_keyword_in_content(&ctx),
        check_keyword_density(&ctx),
        check_keyword_in_meta_description(&ctx),
        check_keyword_in_h2(&ctx),
        check_keyword_in_slug(&ctx),
        check_content_length(&ctx),
        check_h2_count(&ctx),
        check_image_alt_coverage(&ctx),
        check_slug_quality(&ctx),
        check_internal_links(&ctx),
        check_outbound_links(&ctx),
        check_paragraph_length(&ctx),
        check_sentence_length(&ctx),
    ];
    let score = compute_score(&checks);
    AnalyzeResponse {
        score,
        grade: grade(score),
        checks,
        engine: ENGINE,
    }
}

fn normalize(req: AnalyzeRequest) -> Ctx {
    let title = req.title.trim().to_string();
    let content_text = strip_html(&req.content);
    let meta_desc = req.meta_description.trim().to_string();
    let link_counts = count_links(&req.content);
    Ctx {
        title_length: title.chars().count(),
        title,
        content_length: content_text.chars().count(),
        content_html: req.content,
        content_text,
        slug: req.slug.trim().to_string(),
        focus_keyword: req.focus_keyword.trim().to_string(),
        meta_description_length: meta_desc.chars().count(),
        meta_description: meta_desc,
        link_counts,
    }
}

fn count_links(html: &str) -> LinkCounts {
    let mut internal = 0;
    let mut outbound = 0;
    for cap in A_HREF.captures_iter(html) {
        let href = cap[1].trim();
        if href.starts_with("http://")
            || href.starts_with("https://")
            || href.starts_with("//")
        {
            outbound += 1;
        } else if !href.is_empty()
            && !href.starts_with('#')
            && !href.starts_with("javascript:")
            && !href.starts_with("mailto:")
            && !href.starts_with("tel:")
        {
            internal += 1;
        }
    }
    LinkCounts { internal, outbound }
}

fn strip_html(html: &str) -> String {
    let stripped = SCRIPT_STYLE.replace_all(html, "");
    let no_tags = TAG.replace_all(&stripped, " ");
    WHITESPACE.replace_all(&no_tags, " ").trim().to_string()
}

fn keyword_regex(keyword: &str) -> Option<Regex> {
    if keyword.is_empty() {
        return None;
    }
    let pattern = format!("{}(?:{})?", regex::escape(keyword), PARTICLES);
    Regex::new(&pattern).ok()
}

fn keyword_count(text: &str, keyword: &str) -> usize {
    keyword_regex(keyword)
        .map(|re| re.find_iter(text).count())
        .unwrap_or(0)
}

/* ---------- checks ---------- */

fn check_title_length(ctx: &Ctx) -> Check {
    let len = ctx.title_length;
    if len == 0 {
        return mk("title_length", "제목 길이", Status::Fail, "제목이 비어 있습니다.".into(), 10);
    }
    if len < 15 {
        return mk("title_length", "제목 길이", Status::Fail, format!("제목이 너무 짧습니다 ({len}자). 최소 15자 권장."), 10);
    }
    if len > 70 {
        return mk("title_length", "제목 길이", Status::Warning, format!("제목이 너무 깁니다 ({len}자). 검색 결과에서 잘릴 수 있습니다."), 10);
    }
    if len < 30 || len > 60 {
        return mk("title_length", "제목 길이", Status::Warning, format!("제목 길이가 이상적이지 않습니다 ({len}자). 30~60자 권장."), 10);
    }
    mk("title_length", "제목 길이", Status::Pass, format!("제목 길이가 적절합니다 ({len}자)."), 10)
}

fn check_meta_description_length(ctx: &Ctx) -> Check {
    let len = ctx.meta_description_length;
    if len == 0 {
        return mk("meta_description_length", "메타 설명", Status::Warning, "메타 설명이 비어 있습니다. 80~155자 권장.".into(), 10);
    }
    if len < 40 {
        return mk("meta_description_length", "메타 설명", Status::Fail, format!("메타 설명이 너무 짧습니다 ({len}자)."), 10);
    }
    if len > 200 {
        return mk("meta_description_length", "메타 설명", Status::Warning, format!("메타 설명이 너무 깁니다 ({len}자). 검색 결과에서 잘립니다."), 10);
    }
    if len < 80 || len > 155 {
        return mk("meta_description_length", "메타 설명", Status::Warning, format!("메타 설명 길이가 이상적이지 않습니다 ({len}자). 80~155자 권장."), 10);
    }
    mk("meta_description_length", "메타 설명", Status::Pass, format!("메타 설명이 적절합니다 ({len}자)."), 10)
}

fn check_focus_keyword_present(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        mk("focus_keyword_present", "포커스 키워드 설정", Status::Fail, "포커스 키워드를 입력해 주세요.".into(), 5)
    } else {
        mk("focus_keyword_present", "포커스 키워드 설정", Status::Pass, format!("포커스 키워드: {}", ctx.focus_keyword), 5)
    }
}

fn check_focus_keyword_in_title(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Na, String::new(), 10);
    }
    if keyword_count(&ctx.title, &ctx.focus_keyword) > 0 {
        mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Pass, "제목에 포커스 키워드가 포함되어 있습니다.".into(), 10)
    } else {
        mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Fail, "제목에 포커스 키워드가 없습니다.".into(), 10)
    }
}

fn check_focus_keyword_in_first_paragraph(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Na, String::new(), 10);
    }
    let first: String = ctx.content_text.chars().take(200).collect();
    if keyword_count(&first, &ctx.focus_keyword) > 0 {
        mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Pass, "첫 단락에 포커스 키워드가 등장합니다.".into(), 10)
    } else {
        mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Warning, "첫 200자 안에 포커스 키워드가 없습니다.".into(), 10)
    }
}

fn check_focus_keyword_in_content(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Na, String::new(), 10);
    }
    let count = keyword_count(&ctx.content_text, &ctx.focus_keyword);
    if count == 0 {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Fail, "본문에 포커스 키워드가 없습니다.".into(), 10)
    } else if count >= 2 {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Pass, format!("본문에 포커스 키워드가 {count}회 등장합니다."), 10)
    } else {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Warning, "본문에 포커스 키워드가 1회만 등장합니다.".into(), 10)
    }
}

fn check_content_length(ctx: &Ctx) -> Check {
    let len = ctx.content_length;
    if len < 100 {
        return mk("content_length", "본문 길이", Status::Fail, format!("본문이 너무 짧습니다 ({len}자). 최소 600자 권장."), 10);
    }
    if len < 300 {
        return mk("content_length", "본문 길이", Status::Fail, format!("본문이 짧습니다 ({len}자). 600자 이상 권장."), 10);
    }
    if len < 600 {
        return mk("content_length", "본문 길이", Status::Warning, format!("본문이 다소 짧습니다 ({len}자). 600자 이상 권장."), 10);
    }
    mk("content_length", "본문 길이", Status::Pass, format!("본문 길이가 충분합니다 ({len}자)."), 10)
}

fn check_h2_count(ctx: &Ctx) -> Check {
    let count = H2.find_iter(&ctx.content_html).count();
    if count == 0 {
        mk("h2_count", "H2 헤딩", Status::Warning, "H2 헤딩이 없습니다. 글이 길다면 2개 이상 추가하세요.".into(), 5)
    } else if count == 1 {
        mk("h2_count", "H2 헤딩", Status::Warning, format!("H2 헤딩이 {count}개 있습니다. 본문이 길면 더 추가하세요."), 5)
    } else {
        mk("h2_count", "H2 헤딩", Status::Pass, format!("H2 헤딩이 {count}개로 적절합니다."), 5)
    }
}

fn check_image_alt_coverage(ctx: &Ctx) -> Check {
    let imgs: Vec<&str> = IMG.find_iter(&ctx.content_html).map(|m| m.as_str()).collect();
    let total = imgs.len();
    if total == 0 {
        return mk("image_alt_coverage", "이미지 alt", Status::Na, "본문에 이미지가 없습니다.".into(), 5);
    }
    let with_alt = imgs.iter().filter(|t| IMG_ALT.is_match(t)).count();
    if with_alt == total {
        mk("image_alt_coverage", "이미지 alt", Status::Pass, format!("모든 이미지({total}개)에 alt 속성이 있습니다."), 5)
    } else {
        let missing = total - with_alt;
        mk("image_alt_coverage", "이미지 alt", Status::Warning, format!("{missing}개 이미지에 alt 속성이 없습니다 (총 {total}개)."), 5)
    }
}

fn check_slug_quality(ctx: &Ctx) -> Check {
    if ctx.slug.is_empty() {
        return mk("slug_quality", "슬러그", Status::Warning, "슬러그가 비어 있습니다.".into(), 5);
    }
    if NON_ASCII.is_match(&ctx.slug) {
        return mk("slug_quality", "슬러그", Status::Warning, "슬러그에 비-ASCII 문자가 포함되어 있습니다. URL 가독성을 위해 영문 hyphen 권장.".into(), 5);
    }
    if ctx.slug.len() > 75 {
        return mk("slug_quality", "슬러그", Status::Warning, format!("슬러그가 너무 깁니다 ({}). 75자 이하 권장.", ctx.slug), 5);
    }
    mk("slug_quality", "슬러그", Status::Pass, "슬러그가 적절합니다.".into(), 5)
}

/* ---------- new checks: keyword distribution ---------- */

fn check_keyword_density(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_density", "키워드 밀도", Status::Na, String::new(), 5);
    }
    if ctx.content_length == 0 {
        return mk("keyword_density", "키워드 밀도", Status::Na, "본문이 비어 있습니다.".into(), 5);
    }
    let count = keyword_count(&ctx.content_text, &ctx.focus_keyword);
    let kw_chars = ctx.focus_keyword.chars().count();
    let density = count as f64 * kw_chars as f64 / ctx.content_length as f64 * 100.0;
    let d = format!("{density:.2}");
    if count == 0 {
        mk("keyword_density", "키워드 밀도", Status::Fail, "본문에 키워드가 없습니다.".into(), 5)
    } else if density > 4.0 {
        mk("keyword_density", "키워드 밀도", Status::Fail, format!("키워드 밀도가 너무 높습니다 ({d}%). 키워드 스터핑으로 보일 수 있습니다."), 5)
    } else if density > 2.5 {
        mk("keyword_density", "키워드 밀도", Status::Warning, format!("키워드 밀도가 다소 높습니다 ({d}%). 0.5~2.5% 권장."), 5)
    } else if density >= 0.5 {
        mk("keyword_density", "키워드 밀도", Status::Pass, format!("키워드 밀도가 적절합니다 ({d}%)."), 5)
    } else {
        mk("keyword_density", "키워드 밀도", Status::Warning, format!("키워드 밀도가 낮습니다 ({d}%). 0.5~2.5% 권장."), 5)
    }
}

fn check_keyword_in_meta_description(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Na, String::new(), 5);
    }
    if ctx.meta_description_length == 0 {
        return mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Warning, "메타 설명이 비어 있습니다.".into(), 5);
    }
    if keyword_count(&ctx.meta_description, &ctx.focus_keyword) > 0 {
        mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Pass, "메타 설명에 키워드가 포함되어 있습니다.".into(), 5)
    } else {
        mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Warning, "메타 설명에 키워드가 없습니다.".into(), 5)
    }
}

fn check_keyword_in_h2(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_in_h2", "H2에 키워드", Status::Na, String::new(), 5);
    }
    let h2s: Vec<String> = H2_INNER
        .captures_iter(&ctx.content_html)
        .map(|c| strip_html(&c[1]))
        .collect();
    if h2s.is_empty() {
        return mk("keyword_in_h2", "H2에 키워드", Status::Na, "H2 헤딩이 없습니다.".into(), 5);
    }
    let with_kw = h2s.iter().filter(|h| keyword_count(h, &ctx.focus_keyword) > 0).count();
    if with_kw > 0 {
        mk("keyword_in_h2", "H2에 키워드", Status::Pass, format!("{}개 H2에 키워드가 포함되어 있습니다.", with_kw), 5)
    } else {
        mk("keyword_in_h2", "H2에 키워드", Status::Warning, "어떤 H2에도 키워드가 없습니다.".into(), 5)
    }
}

fn check_keyword_in_slug(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Na, String::new(), 5);
    }
    if ctx.slug.is_empty() {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Warning, "슬러그가 비어 있습니다.".into(), 5);
    }
    // 한글 키워드면 영문 슬러그와 직접 비교 불가 — V2에서 transliteration 추가.
    if NON_ASCII.is_match(&ctx.focus_keyword) {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Na, "한국어 키워드는 영문 슬러그와 직접 비교가 어렵습니다.".into(), 5);
    }
    if ctx.slug.to_lowercase().contains(&ctx.focus_keyword.to_lowercase()) {
        mk("keyword_in_slug", "슬러그에 키워드", Status::Pass, "슬러그에 키워드가 포함되어 있습니다.".into(), 5)
    } else {
        mk("keyword_in_slug", "슬러그에 키워드", Status::Warning, "슬러그에 키워드가 포함되어 있지 않습니다.".into(), 5)
    }
}

/* ---------- new checks: links ---------- */

fn check_internal_links(ctx: &Ctx) -> Check {
    let n = ctx.link_counts.internal;
    if n == 0 {
        mk("internal_links", "내부 링크", Status::Warning, "내부 링크가 없습니다. 관련 글로 1개 이상 링크하세요.".into(), 5)
    } else {
        mk("internal_links", "내부 링크", Status::Pass, format!("내부 링크 {n}개."), 5)
    }
}

fn check_outbound_links(ctx: &Ctx) -> Check {
    let n = ctx.link_counts.outbound;
    if n == 0 {
        mk("outbound_links", "외부 링크", Status::Warning, "외부 링크가 없습니다. 권위 있는 출처로 1개 이상 링크하면 신뢰도가 올라갑니다.".into(), 5)
    } else {
        mk("outbound_links", "외부 링크", Status::Pass, format!("외부 링크 {n}개."), 5)
    }
}

/* ---------- new checks: readability ---------- */

fn check_paragraph_length(ctx: &Ctx) -> Check {
    let lengths: Vec<usize> = P_INNER
        .captures_iter(&ctx.content_html)
        .map(|c| strip_html(&c[1]).chars().count())
        .filter(|&l| l > 0)
        .collect();
    if lengths.is_empty() {
        return mk("paragraph_length", "문단 길이", Status::Na, "문단이 없습니다.".into(), 5);
    }
    let max = *lengths.iter().max().unwrap();
    let too_long = lengths.iter().filter(|&&l| l > 500).count();
    if too_long > 0 {
        mk("paragraph_length", "문단 길이", Status::Warning, format!("{}개 문단이 500자보다 깁니다 (최대 {}자). 가독성을 위해 분할하세요.", too_long, max), 5)
    } else {
        mk("paragraph_length", "문단 길이", Status::Pass, format!("문단 길이가 적절합니다 (최대 {}자).", max), 5)
    }
}

fn check_sentence_length(ctx: &Ctx) -> Check {
    if ctx.content_length == 0 {
        return mk("sentence_length", "문장 길이", Status::Na, String::new(), 5);
    }
    let sentences: Vec<&str> = SENTENCE_END
        .split(&ctx.content_text)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if sentences.is_empty() {
        return mk("sentence_length", "문장 길이", Status::Na, String::new(), 5);
    }
    let lengths: Vec<usize> = sentences.iter().map(|s| s.chars().count()).collect();
    let avg = lengths.iter().sum::<usize>() / lengths.len();
    let over = lengths.iter().filter(|&&l| l > 80).count();
    let total = sentences.len();
    if over > total / 4 && over > 0 {
        mk("sentence_length", "문장 길이", Status::Warning, format!("긴 문장이 많습니다 ({}/{} 문장이 80자 초과). 평균 {}자.", over, total, avg), 5)
    } else {
        mk("sentence_length", "문장 길이", Status::Pass, format!("문장 길이가 적절합니다 (평균 {}자, 총 {} 문장).", avg, total), 5)
    }
}

/* ---------- score / grade ---------- */

fn compute_score(checks: &[Check]) -> u32 {
    let mut total = 0u32;
    let mut earned = 0.0f64;
    for c in checks {
        if c.status == Status::Na {
            continue;
        }
        total += c.weight;
        match c.status {
            Status::Pass => earned += c.weight as f64,
            Status::Warning => earned += c.weight as f64 * 0.5,
            Status::Fail => {}
            Status::Na => unreachable!(),
        }
    }
    if total > 0 {
        (earned / total as f64 * 100.0).round() as u32
    } else {
        0
    }
}

fn grade(score: u32) -> &'static str {
    if score >= 85 {
        "great"
    } else if score >= 65 {
        "good"
    } else if score >= 40 {
        "needs_work"
    } else {
        "poor"
    }
}

fn mk(id: &'static str, label: &'static str, status: Status, message: String, weight: u32) -> Check {
    Check { id, label, status, message, weight }
}

/* ---------- tests ---------- */

#[cfg(test)]
mod tests {
    use super::*;

    fn req(title: &str, content: &str, slug: &str, kw: &str, meta: &str) -> AnalyzeRequest {
        AnalyzeRequest {
            title: title.into(),
            content: content.into(),
            slug: slug.into(),
            focus_keyword: kw.into(),
            meta_description: meta.into(),
        }
    }

    #[test]
    fn empty_post_is_poor() {
        let r = analyze(req("", "", "", "", ""));
        assert!(r.score <= 30, "got {}", r.score);
        assert_eq!(r.grade, "poor");
        assert_eq!(r.checks.len(), 18);
    }

    #[test]
    fn keyword_density_in_ideal_range_passes() {
        // ~2000자 본문 + 워드프레스(5자) 8회 = 40/2000 = 2.0% (pass range)
        let filler = "한국어 본문이 충분히 길게 작성되어 있습니다. ".repeat(80);
        let kw_block = " 워드프레스 ".repeat(8);
        let content = format!("<p>{}{}</p>", filler, kw_block);
        let r = analyze(req("t", &content, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_density").unwrap();
        assert_eq!(c.status, Status::Pass, "got {:?}: {}", c.status, c.message);
    }

    #[test]
    fn keyword_density_excess_fails() {
        let content = format!("<p>{}</p>", "워드프레스 ".repeat(20));
        let r = analyze(req("t", &content, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_density").unwrap();
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn internal_and_outbound_links_counted_separately() {
        let html = r##"<p><a href="/about">about</a> <a href="https://example.com">ext</a> <a href="#top">anchor</a> <a href="mailto:x@y">m</a></p>"##;
        let r = analyze(req("t", html, "", "", ""));
        let i = r.checks.iter().find(|c| c.id == "internal_links").unwrap();
        let o = r.checks.iter().find(|c| c.id == "outbound_links").unwrap();
        assert_eq!(i.status, Status::Pass);
        assert!(i.message.contains("1개"));
        assert_eq!(o.status, Status::Pass);
        assert!(o.message.contains("1개"));
    }

    #[test]
    fn keyword_in_h2_passes_when_present() {
        let html = "<h2>워드프레스 입문</h2><p>본문</p><h2>설치</h2>";
        let r = analyze(req("t", html, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_in_h2").unwrap();
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn long_paragraph_warns() {
        let long = "가".repeat(600);
        let html = format!("<p>{long}</p>");
        let r = analyze(req("t", &html, "", "", ""));
        let c = r.checks.iter().find(|c| c.id == "paragraph_length").unwrap();
        assert_eq!(c.status, Status::Warning);
    }

    #[test]
    fn well_formed_korean_post_is_at_least_good() {
        let content = "<h1>워드프레스 입문</h1>\
<p>워드프레스는 가장 널리 쓰이는 콘텐츠 관리 시스템입니다. \
워드프레스를 사용하면 누구나 쉽게 블로그나 웹사이트를 만들 수 있습니다. \
오픈소스이며 무료로 사용할 수 있고 한국어 지원도 잘 되어 있습니다. \
본 글에서는 워드프레스를 처음 접하는 분들을 위해 설치부터 운영까지 단계별로 자세히 안내합니다.</p>\
<h2>워드프레스의 장점</h2>\
<p>오픈소스이며 자유롭게 커스터마이징할 수 있습니다. 수많은 테마와 플러그인이 있어 확장성이 뛰어납니다. \
한국어 자료도 풍부해서 학습 곡선이 완만합니다. 워드프레스의 활발한 커뮤니티가 큰 강점입니다.</p>\
<h2>설치 방법</h2>\
<p>호스팅 업체에서 원클릭 설치를 제공하거나 wordpress.org에서 직접 받을 수 있습니다. \
워드프레스를 처음 다루는 분들은 원클릭 설치가 무난합니다. \
설치 후에는 기본 테마를 활성화하고 필요한 플러그인을 추가합니다.</p>\
<img src=\"d.jpg\" alt=\"워드프레스 대시보드\" />\
<p>대시보드에서 글 작성, 미디어 업로드, 댓글 관리 등 모든 작업을 할 수 있습니다. \
워드프레스는 이 단순함과 확장성을 동시에 제공한다는 점에서 매력적입니다.</p>";
        let r = analyze(req(
            "워드프레스 입문 가이드: 한국어 블로그를 시작하는 가장 쉬운 방법",
            content,
            "wordpress-guide-korean-blog",
            "워드프레스",
            "워드프레스로 한국어 블로그를 처음 만드는 분들을 위한 입문 가이드. 호스팅 선택부터 테마 적용, 플러그인 설치까지 단계별로 자세히 설명합니다.",
        ));
        assert!(r.score >= 65, "got {}", r.score);
        assert!(["good", "great"].contains(&r.grade));
    }

    #[test]
    fn korean_particles_caught_via_regex() {
        let r = analyze(req(
            "테스트",
            "<p>워드프레스를 사용하면 좋습니다. 워드프레스의 장점은 많습니다. 워드프레스가 인기있어요.</p>",
            "test",
            "워드프레스",
            "",
        ));
        let c = r.checks.iter().find(|c| c.id == "focus_keyword_in_content").unwrap();
        assert_eq!(c.status, Status::Pass);
        assert!(c.message.contains("3회"), "got {}", c.message);
    }

    #[test]
    fn korean_slug_warns() {
        let r = analyze(req("제목입니다 적당하게", "<p>본문</p>", "한국어-슬러그", "", ""));
        let c = r.checks.iter().find(|c| c.id == "slug_quality").unwrap();
        assert_eq!(c.status, Status::Warning);
    }

    #[test]
    fn h2_count_pass_at_two() {
        let r = analyze(req("t", "<h2>a</h2><p>b</p><h2>c</h2>", "x", "", ""));
        let c = r.checks.iter().find(|c| c.id == "h2_count").unwrap();
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn image_alt_full_coverage_passes() {
        let r = analyze(req("t", "<img src=x alt=\"a\" /><img src=y alt=\"b\" />", "x", "", ""));
        let c = r.checks.iter().find(|c| c.id == "image_alt_coverage").unwrap();
        assert_eq!(c.status, Status::Pass);
    }
}
