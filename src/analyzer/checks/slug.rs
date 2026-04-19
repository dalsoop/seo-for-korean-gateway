//! URL slug checks. Quality + length.
//! Future: slug structure (dashes vs underscores), stop words.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::NON_ASCII;
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![slug_quality(ctx), slug_uses_dashes(ctx)]
}

fn slug_uses_dashes(ctx: &Ctx) -> Check {
    if ctx.slug.is_empty() {
        return mk("slug_uses_dashes", "슬러그 구분자", Status::Na, String::new(), 5);
    }
    if ctx.slug.contains('_') {
        return mk("slug_uses_dashes", "슬러그 구분자", Status::Warning, "슬러그에 언더스코어(_)가 있습니다. 검색엔진은 하이픈(-)을 권장합니다.".into(), 5);
    }
    mk("slug_uses_dashes", "슬러그 구분자", Status::Pass, "슬러그 구분자가 적절합니다.".into(), 5)
}

fn slug_quality(ctx: &Ctx) -> Check {
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
