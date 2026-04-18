//! Title-only checks. Currently length; future: keyword position, numbers,
//! starts-with-keyword, power words.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![title_length(ctx)]
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
