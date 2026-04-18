//! Korean readability heuristics. Paragraph + sentence length today.
//! Future: transition words (그러나/따라서/한편), ending consistency
//! (해요체/합쇼체 mixing), passive voice detection.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::{strip_html, P_INNER, SENTENCE_END};
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![paragraph_length(ctx), sentence_length(ctx)]
}

fn paragraph_length(ctx: &Ctx) -> Check {
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

fn sentence_length(ctx: &Ctx) -> Check {
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
