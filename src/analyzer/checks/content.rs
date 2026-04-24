//! Content body structural checks. Length + heading count.
//! Future: H3 hierarchy, list usage, table presence.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::{H2, HEADING, UL_OL};
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![
        content_length(ctx),
        h2_count(ctx),
        subheading_distribution(ctx),
        has_lists(ctx),
        headings_hierarchy(ctx),
    ]
}

fn headings_hierarchy(ctx: &Ctx) -> Check {
    let levels: Vec<u8> = HEADING
        .captures_iter(&ctx.content_html)
        .filter_map(|c| c.get(1).and_then(|m| m.as_str().parse::<u8>().ok()))
        .collect();
    if levels.is_empty() {
        return mk("headings_hierarchy", "헤딩 계층", Status::Na, "헤딩이 없습니다.".into(), 5);
    }

    let h1_count = levels.iter().filter(|&&l| l == 1).count();
    if h1_count > 1 {
        return mk("headings_hierarchy", "헤딩 계층", Status::Warning, format!("본문에 H1이 {h1_count}개 있습니다. 보통 H1은 글 제목 1개만 사용합니다."), 5);
    }

    for w in levels.windows(2) {
        if w[1] > w[0] && w[1] - w[0] > 1 {
            return mk(
                "headings_hierarchy",
                "헤딩 계층",
                Status::Warning,
                format!("헤딩 단계가 건너뛰어졌습니다 (H{} → H{}). 접근성을 위해 단계별 사용 권장.", w[0], w[1]),
                5,
            );
        }
    }

    mk("headings_hierarchy", "헤딩 계층", Status::Pass, format!("헤딩 계층이 적절합니다 ({}개).", levels.len()), 5)
}

fn content_length(ctx: &Ctx) -> Check {
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

fn subheading_distribution(ctx: &Ctx) -> Check {
    if ctx.content_length < 600 {
        return mk("subheading_distribution", "헤딩 분포", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    let h2 = H2.find_iter(&ctx.content_html).count();
    if h2 == 0 {
        return mk("subheading_distribution", "헤딩 분포", Status::Warning, "헤딩이 없습니다.".into(), 5);
    }
    let avg = ctx.content_length / h2;
    if avg > 1500 {
        mk("subheading_distribution", "헤딩 분포", Status::Warning, format!("H2 사이 본문이 너무 깁니다 (평균 {avg}자). 헤딩을 더 추가하세요."), 5)
    } else {
        mk("subheading_distribution", "헤딩 분포", Status::Pass, format!("헤딩 분포가 적절합니다 (H2 사이 평균 {avg}자)."), 5)
    }
}

fn has_lists(ctx: &Ctx) -> Check {
    if ctx.content_length < 400 {
        return mk("has_lists", "리스트 사용", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    if UL_OL.is_match(&ctx.content_html) {
        mk("has_lists", "리스트 사용", Status::Pass, "본문에 리스트가 있습니다.".into(), 5)
    } else {
        mk("has_lists", "리스트 사용", Status::Warning, "리스트(ul/ol)가 없습니다. 정보 정리에 활용해 보세요.".into(), 5)
    }
}

fn h2_count(ctx: &Ctx) -> Check {
    let count = H2.find_iter(&ctx.content_html).count();
    if count == 0 {
        mk("h2_count", "H2 헤딩", Status::Warning, "H2 헤딩이 없습니다. 글이 길다면 2개 이상 추가하세요.".into(), 5)
    } else if count == 1 {
        mk("h2_count", "H2 헤딩", Status::Warning, format!("H2 헤딩이 {count}개 있습니다. 본문이 길면 더 추가하세요."), 5)
    } else {
        mk("h2_count", "H2 헤딩", Status::Pass, format!("H2 헤딩이 {count}개로 적절합니다."), 5)
    }
}
