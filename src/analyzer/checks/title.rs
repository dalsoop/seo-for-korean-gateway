//! Title-only checks. Currently length; future: keyword position, numbers,
//! starts-with-keyword, power words.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::keyword_regex;
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![
        title_length(ctx),
        title_keyword_position(ctx),
        title_starts_with_keyword(ctx),
    ]
}

fn title_starts_with_keyword(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() || ctx.title.is_empty() {
        return mk("title_starts_with_keyword", "제목 시작 키워드", Status::Na, String::new(), 5);
    }
    let title_l = ctx.title.to_lowercase();
    let kw_l = ctx.focus_keyword.to_lowercase();
    if title_l.starts_with(&kw_l) {
        mk("title_starts_with_keyword", "제목 시작 키워드", Status::Pass, "제목이 키워드로 시작합니다.".into(), 5)
    } else {
        mk("title_starts_with_keyword", "제목 시작 키워드", Status::Warning, "제목을 키워드로 시작하면 SEO에 더 효과적입니다.".into(), 5)
    }
}

fn title_keyword_position(ctx: &Ctx) -> Check {
    if ctx.focus_keyword.is_empty() || ctx.title_length == 0 {
        return mk("title_keyword_position", "제목 내 키워드 위치", Status::Na, String::new(), 5);
    }
    let Some(re) = keyword_regex(&ctx.focus_keyword) else {
        return mk("title_keyword_position", "제목 내 키워드 위치", Status::Na, String::new(), 5);
    };
    let Some(m) = re.find(&ctx.title) else {
        return mk("title_keyword_position", "제목 내 키워드 위치", Status::Fail, "제목에 키워드가 없습니다.".into(), 5);
    };
    // Convert byte offset to char position so % is meaningful for Korean text.
    let char_pos = ctx.title[..m.start()].chars().count();
    let percent = (char_pos as f64 / ctx.title_length as f64 * 100.0).round() as i64;
    if percent <= 30 {
        mk("title_keyword_position", "제목 내 키워드 위치", Status::Pass, format!("키워드가 제목 앞부분 ({percent}%)에 있습니다."), 5)
    } else {
        mk("title_keyword_position", "제목 내 키워드 위치", Status::Warning, format!("키워드가 제목의 {percent}% 위치에 있습니다. 앞쪽으로 옮겨보세요."), 5)
    }
}

fn title_length(ctx: &Ctx) -> Check {
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
