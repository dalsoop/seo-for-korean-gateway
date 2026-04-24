//! Focus-keyword distribution checks. The biggest domain — anything that
//! asks "is the keyword here?" lives here.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::{strip_html, H2_INNER, NON_ASCII};
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![
        focus_keyword_present(ctx),
        focus_keyword_in_title(ctx),
        focus_keyword_in_first_paragraph(ctx),
        focus_keyword_in_content(ctx),
        keyword_density(ctx),
        keyword_in_meta_description(ctx),
        keyword_in_h2(ctx),
        keyword_in_slug(ctx),
    ]
}

fn focus_keyword_present(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        mk("focus_keyword_present", "포커스 키워드 설정", Status::Fail, "포커스 키워드를 입력해 주세요.".into(), 5)
    } else {
        mk("focus_keyword_present", "포커스 키워드 설정", Status::Pass, format!("포커스 키워드: {}", ctx.focus_keyword), 5)
    }
}

fn focus_keyword_in_title(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Na, String::new(), 10);
    }
    if ctx.counter.count(&ctx.title, &ctx.focus_keyword) > 0 {
        mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Pass, "제목에 포커스 키워드가 포함되어 있습니다.".into(), 10)
    } else {
        mk("focus_keyword_in_title", "제목에 포커스 키워드", Status::Fail, "제목에 포커스 키워드가 없습니다.".into(), 10)
    }
}

fn focus_keyword_in_first_paragraph(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Na, String::new(), 10);
    }
    let first: String = ctx.content_text.chars().take(200).collect();
    if ctx.counter.count(&first, &ctx.focus_keyword) > 0 {
        mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Pass, "첫 단락에 포커스 키워드가 등장합니다.".into(), 10)
    } else {
        mk("focus_keyword_in_first_paragraph", "첫 단락에 포커스 키워드", Status::Warning, "첫 200자 안에 포커스 키워드가 없습니다.".into(), 10)
    }
}

fn focus_keyword_in_content(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Na, String::new(), 10);
    }
    let count = ctx.counter.count(&ctx.content_text, &ctx.focus_keyword);
    if count == 0 {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Fail, "본문에 포커스 키워드가 없습니다.".into(), 10)
    } else if count >= 2 {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Pass, format!("본문에 포커스 키워드가 {count}회 등장합니다."), 10)
    } else {
        mk("focus_keyword_in_content", "본문에 포커스 키워드", Status::Warning, "본문에 포커스 키워드가 1회만 등장합니다.".into(), 10)
    }
}

fn keyword_density(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_density", "키워드 밀도", Status::Na, String::new(), 5);
    }
    if ctx.content_length == 0 {
        return mk("keyword_density", "키워드 밀도", Status::Na, "본문이 비어 있습니다.".into(), 5);
    }
    let count = ctx.counter.count(&ctx.content_text, &ctx.focus_keyword);
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

fn keyword_in_meta_description(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Na, String::new(), 5);
    }
    if ctx.meta_description_length == 0 {
        return mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Warning, "메타 설명이 비어 있습니다.".into(), 5);
    }
    if ctx.counter.count(&ctx.meta_description, &ctx.focus_keyword) > 0 {
        mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Pass, "메타 설명에 키워드가 포함되어 있습니다.".into(), 5)
    } else {
        mk("keyword_in_meta_description", "메타 설명에 키워드", Status::Warning, "메타 설명에 키워드가 없습니다.".into(), 5)
    }
}

fn keyword_in_h2(ctx: &Ctx) -> Check {
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
    let with_kw = h2s.iter().filter(|h| ctx.counter.count(h, &ctx.focus_keyword) > 0).count();
    if with_kw > 0 {
        mk("keyword_in_h2", "H2에 키워드", Status::Pass, format!("{}개 H2에 키워드가 포함되어 있습니다.", with_kw), 5)
    } else {
        mk("keyword_in_h2", "H2에 키워드", Status::Warning, "어떤 H2에도 키워드가 없습니다.".into(), 5)
    }
}

fn keyword_in_slug(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Na, String::new(), 5);
    }
    if ctx.slug.is_empty() {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Warning, "슬러그가 비어 있습니다.".into(), 5);
    }
    if NON_ASCII.is_match(&ctx.focus_keyword) {
        return mk("keyword_in_slug", "슬러그에 키워드", Status::Na, "한국어 키워드는 영문 슬러그와 직접 비교가 어렵습니다.".into(), 5);
    }
    if ctx.slug.to_lowercase().contains(&ctx.focus_keyword.to_lowercase()) {
        mk("keyword_in_slug", "슬러그에 키워드", Status::Pass, "슬러그에 키워드가 포함되어 있습니다.".into(), 5)
    } else {
        mk("keyword_in_slug", "슬러그에 키워드", Status::Warning, "슬러그에 키워드가 포함되어 있지 않습니다.".into(), 5)
    }
}
