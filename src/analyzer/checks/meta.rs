//! Meta description checks. Length only for now; future: keyword position,
//! starts-with-keyword, sentiment.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![meta_description_length(ctx)]
}

fn meta_description_length(ctx: &Ctx) -> Check {
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
