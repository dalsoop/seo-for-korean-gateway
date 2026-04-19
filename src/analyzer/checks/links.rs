//! Link analysis. Internal vs outbound counts (cached on Ctx).
//! Future: broken-link detection, dofollow ratio, link diversity.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::A_HREF;
use crate::analyzer::types::{mk, Check, Status};
use once_cell::sync::Lazy;
use regex::Regex;

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![
        internal_links(ctx),
        outbound_links(ctx),
        nofollow_outbound(ctx),
        excessive_links(ctx),
    ]
}

fn excessive_links(ctx: &Ctx) -> Check {
    let total = ctx.link_counts.internal + ctx.link_counts.outbound;
    if ctx.content_length < 300 {
        return mk("excessive_links", "과도한 링크", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    if total == 0 {
        return mk("excessive_links", "과도한 링크", Status::Na, "링크가 없습니다.".into(), 5);
    }
    // Roughly 1 link per 100 chars is the soft cap for Korean text.
    let limit = std::cmp::max(5, ctx.content_length / 100);
    if total > limit {
        return mk(
            "excessive_links",
            "과도한 링크",
            Status::Warning,
            format!("링크 {total}개가 본문 {}자에 비해 많습니다 (약 {limit}개 권장). 스팸으로 보일 수 있습니다.", ctx.content_length),
            5,
        );
    }
    mk(
        "excessive_links",
        "과도한 링크",
        Status::Pass,
        format!("링크 {total}개 (본문 {}자에 적절).", ctx.content_length),
        5,
    )
}

static A_TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<a\s+([^>]*?)>").unwrap());

fn nofollow_outbound(ctx: &Ctx) -> Check {
    if ctx.link_counts.outbound == 0 {
        return mk("nofollow_outbound", "외부 링크 nofollow", Status::Na, "외부 링크가 없습니다.".into(), 5);
    }

    let mut total = 0usize;
    let mut nofollow = 0usize;
    for cap in A_TAG.captures_iter(&ctx.content_html) {
        let attrs = &cap[1];
        let Some(href_cap) = A_HREF.captures(&ctx.content_html[cap.get(0).unwrap().start()..cap.get(0).unwrap().end()]) else { continue };
        let href = href_cap[1].trim();
        let is_outbound = href.starts_with("http://")
            || href.starts_with("https://")
            || href.starts_with("//");
        if !is_outbound {
            continue;
        }
        total += 1;
        let attrs_l = attrs.to_lowercase();
        if attrs_l.contains("nofollow") || attrs_l.contains("ugc") || attrs_l.contains("sponsored") {
            nofollow += 1;
        }
    }

    if total == 0 {
        return mk("nofollow_outbound", "외부 링크 nofollow", Status::Na, "외부 링크가 없습니다.".into(), 5);
    }

    let ratio = (nofollow as f64 / total as f64 * 100.0).round() as i64;
    if ratio == 0 {
        return mk("nofollow_outbound", "외부 링크 nofollow", Status::Pass, format!("외부 링크 {total}개 모두 dofollow (추천 의미가 살아있음)."), 5);
    }
    if ratio > 80 {
        return mk("nofollow_outbound", "외부 링크 nofollow", Status::Warning, format!("외부 링크 nofollow 비율 {ratio}%로 높음. 너무 보수적이면 신뢰도 신호가 약화됩니다."), 5);
    }
    mk("nofollow_outbound", "외부 링크 nofollow", Status::Pass, format!("외부 링크 nofollow 비율 {ratio}% ({nofollow}/{total})."), 5)
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
