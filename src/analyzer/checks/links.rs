//! Link analysis. Internal vs outbound counts (cached on Ctx).
//! Future: broken-link detection, dofollow ratio, link diversity.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![internal_links(ctx), outbound_links(ctx)]
}

fn internal_links(ctx: &Ctx) -> Check {
    let n = ctx.link_counts.internal;
    if n == 0 {
        mk("internal_links", "내부 링크", Status::Warning, "내부 링크가 없습니다. 관련 글로 1개 이상 링크하세요.".into(), 5)
    } else {
        mk("internal_links", "내부 링크", Status::Pass, format!("내부 링크 {n}개."), 5)
    }
}

fn outbound_links(ctx: &Ctx) -> Check {
    let n = ctx.link_counts.outbound;
    if n == 0 {
        mk("outbound_links", "외부 링크", Status::Warning, "외부 링크가 없습니다. 권위 있는 출처로 1개 이상 링크하면 신뢰도가 올라갑니다.".into(), 5)
    } else {
        mk("outbound_links", "외부 링크", Status::Pass, format!("외부 링크 {n}개."), 5)
    }
}
