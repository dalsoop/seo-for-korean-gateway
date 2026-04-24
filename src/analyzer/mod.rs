//! SEO content analyzer. Mirrors the WordPress plugin's PHP fallback;
//! the Rust version is the canonical engine going forward.
//!
//! Layout:
//!   types.rs         shared wire types (Request/Response/Check/Status)
//!   ctx.rs           pre-computed analysis context (one regex pass for all checks)
//!   helpers.rs       shared regexes + Korean particle helpers
//!   checks/<domain>  one module per SEO concern, each exposes `run(&Ctx)`
//!
//! Adding a new check: drop a function into the right `checks/<domain>.rs`
//! and append it to that module's `run()` Vec. Adding a new domain: create
//! a new file under `checks/`, register it in `checks/mod.rs`, and call its
//! `run()` from `analyze()` below.

mod checks;
mod ctx;
mod helpers;
pub mod keyword;
mod types;

use std::sync::Arc;

pub use keyword::KeywordCounter;
#[cfg(test)]
pub use keyword::RegexCounter;
pub use types::{AnalyzeRequest, AnalyzeResponse};

pub fn analyze(req: AnalyzeRequest, counter: Arc<dyn KeywordCounter>) -> AnalyzeResponse {
    let engine = counter.engine();
    let context = ctx::normalize(req, counter);

    let mut all = Vec::new();
    all.extend(checks::title::run(&context));
    all.extend(checks::meta::run(&context));
    all.extend(checks::keywords::run(&context));
    all.extend(checks::content::run(&context));
    all.extend(checks::images::run(&context));
    all.extend(checks::slug::run(&context));
    all.extend(checks::links::run(&context));
    all.extend(checks::readability::run(&context));

    let score = compute_score(&all);
    AnalyzeResponse {
        score,
        grade: grade(score),
        checks: all,
        engine,
    }
}

fn compute_score(checks: &[types::Check]) -> u32 {
    let mut total = 0u32;
    let mut earned = 0.0f64;
    for c in checks {
        if c.status == types::Status::Na {
            continue;
        }
        total += c.weight;
        match c.status {
            types::Status::Pass => earned += c.weight as f64,
            types::Status::Warning => earned += c.weight as f64 * 0.5,
            types::Status::Fail => {}
            types::Status::Na => unreachable!(),
        }
    }
    if total > 0 {
        (earned / total as f64 * 100.0).round() as u32
    } else {
        0
    }
}

fn grade(score: u32) -> &'static str {
    if score >= 85 { "great" }
    else if score >= 65 { "good" }
    else if score >= 40 { "needs_work" }
    else { "poor" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::Status;

    fn req(title: &str, content: &str, slug: &str, kw: &str, meta: &str) -> AnalyzeRequest {
        AnalyzeRequest {
            title: title.into(),
            content: content.into(),
            slug: slug.into(),
            focus_keyword: kw.into(),
            meta_description: meta.into(),
        }
    }

    fn run(req: AnalyzeRequest) -> AnalyzeResponse {
        analyze(req, RegexCounter::shared())
    }

    #[test]
    fn empty_post_is_poor() {
        let r = run(req("", "", "", "", ""));
        assert!(r.score <= 30, "got {}", r.score);
        assert_eq!(r.grade, "poor");
        assert_eq!(r.checks.len(), 35);
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
        let r = run(req(
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
        let r = run(req(
            "테스트",
            "<p>워드프레스를 사용하면 좋습니다. 워드프레스의 장점은 많습니다. 워드프레스가 인기있어요.</p>",
            "test", "워드프레스", "",
        ));
        let c = r.checks.iter().find(|c| c.id == "focus_keyword_in_content").unwrap();
        assert_eq!(c.status, Status::Pass);
        assert!(c.message.contains("3회"), "got {}", c.message);
    }

    #[test]
    fn korean_slug_warns() {
        let r = run(req("제목입니다 적당하게", "<p>본문</p>", "한국어-슬러그", "", ""));
        let c = r.checks.iter().find(|c| c.id == "slug_quality").unwrap();
        assert_eq!(c.status, Status::Warning);
    }

    #[test]
    fn h2_count_pass_at_two() {
        let r = run(req("t", "<h2>a</h2><p>b</p><h2>c</h2>", "x", "", ""));
        let c = r.checks.iter().find(|c| c.id == "h2_count").unwrap();
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn image_alt_full_coverage_passes() {
        let r = run(req("t", "<img src=x alt=\"a\" /><img src=y alt=\"b\" />", "x", "", ""));
        let c = r.checks.iter().find(|c| c.id == "image_alt_coverage").unwrap();
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn keyword_density_in_ideal_range_passes() {
        let filler = "한국어 본문이 충분히 길게 작성되어 있습니다. ".repeat(80);
        let kw_block = " 워드프레스 ".repeat(8);
        let content = format!("<p>{}{}</p>", filler, kw_block);
        let r = run(req("t", &content, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_density").unwrap();
        assert_eq!(c.status, Status::Pass, "got {:?}: {}", c.status, c.message);
    }

    #[test]
    fn keyword_density_excess_fails() {
        let content = format!("<p>{}</p>", "워드프레스 ".repeat(20));
        let r = run(req("t", &content, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_density").unwrap();
        assert_eq!(c.status, Status::Fail);
    }

    #[test]
    fn internal_and_outbound_links_counted_separately() {
        let html = r##"<p><a href="/about">about</a> <a href="https://example.com">ext</a> <a href="#top">anchor</a> <a href="mailto:x@y">m</a></p>"##;
        let r = run(req("t", html, "", "", ""));
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
        let r = run(req("t", html, "", "워드프레스", ""));
        let c = r.checks.iter().find(|c| c.id == "keyword_in_h2").unwrap();
        assert_eq!(c.status, Status::Pass);
    }

    #[test]
    fn long_paragraph_warns() {
        let long = "가".repeat(600);
        let html = format!("<p>{long}</p>");
        let r = run(req("t", &html, "", "", ""));
        let c = r.checks.iter().find(|c| c.id == "paragraph_length").unwrap();
        assert_eq!(c.status, Status::Warning);
    }
}
