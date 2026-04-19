//! Korean readability heuristics. Paragraph + sentence length today.
//! Future: transition words (그러나/따라서/한편), ending consistency
//! (해요체/합쇼체 mixing), passive voice detection.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::{strip_html, HAEYO, HAPSYO, INFORMAL, P_INNER, SENTENCE_END, TRANSITIONS};
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![
        paragraph_length(ctx),
        sentence_length(ctx),
        transition_words(ctx),
        ending_consistency(ctx),
        hanja_ratio(ctx),
        informal_text(ctx),
    ]
}

fn hanja_ratio(ctx: &Ctx) -> Check {
    if ctx.content_length < 200 {
        return mk("hanja_ratio", "한자 사용", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    // CJK Unified Ideographs: U+4E00..=U+9FFF, plus Extension A.
    let hanja: usize = ctx.content_text.chars().filter(|c| {
        let code = *c as u32;
        (0x4E00..=0x9FFF).contains(&code) || (0x3400..=0x4DBF).contains(&code)
    }).count();
    if hanja == 0 {
        return mk("hanja_ratio", "한자 사용", Status::Pass, "한자 사용 없음 (한국어 독자에게 친화적).".into(), 5);
    }
    let ratio = hanja as f64 / ctx.content_length as f64 * 100.0;
    let r = format!("{ratio:.1}");
    if ratio > 5.0 {
        mk("hanja_ratio", "한자 사용", Status::Warning, format!("한자 비율 {r}% (높음). 일반 독자에게 어려울 수 있습니다."), 5)
    } else {
        mk("hanja_ratio", "한자 사용", Status::Pass, format!("한자 비율 {r}% (적절)."), 5)
    }
}

fn informal_text(ctx: &Ctx) -> Check {
    if ctx.content_length < 100 {
        return mk("informal_text", "구어체/채팅체", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    let count = INFORMAL.find_iter(&ctx.content_text).count();
    if count == 0 {
        return mk("informal_text", "구어체/채팅체", Status::Pass, "구어체/채팅체 없음.".into(), 5);
    }
    if count >= 3 {
        return mk("informal_text", "구어체/채팅체", Status::Fail, format!("구어체/채팅체가 {count}회 등장 (ㅋㅋ/ㅠㅠ/헐 등). SEO 글에는 권장되지 않습니다."), 5);
    }
    mk("informal_text", "구어체/채팅체", Status::Warning, format!("구어체/채팅체 {count}회 등장. 정식 글에서는 자제하세요."), 5)
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

fn transition_words(ctx: &Ctx) -> Check {
    if ctx.content_length < 200 {
        return mk("transition_words", "접속어 사용", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    let count = TRANSITIONS.find_iter(&ctx.content_text).count();
    let sentences = SENTENCE_END
        .split(&ctx.content_text)
        .filter(|s| !s.trim().is_empty())
        .count();
    if sentences == 0 {
        return mk("transition_words", "접속어 사용", Status::Na, String::new(), 5);
    }
    let ratio = (count as f64 / sentences as f64 * 100.0).round() as i64;
    if count == 0 {
        return mk(
            "transition_words",
            "접속어 사용",
            Status::Warning,
            "접속어(그러나/따라서/즉 등)가 없습니다. 글의 흐름을 매끄럽게 해보세요.".into(),
            5,
        );
    }
    if ratio < 5 {
        return mk(
            "transition_words",
            "접속어 사용",
            Status::Warning,
            format!("접속어가 적습니다 ({count}회, 문장의 {ratio}%). 더 추가하면 가독성이 좋아집니다."),
            5,
        );
    }
    mk(
        "transition_words",
        "접속어 사용",
        Status::Pass,
        format!("접속어를 잘 사용했습니다 ({count}회, 문장의 {ratio}%)."),
        5,
    )
}

fn ending_consistency(ctx: &Ctx) -> Check {
    if ctx.content_length < 200 {
        return mk("ending_consistency", "어미 일관성", Status::Na, "본문이 짧아 평가 생략.".into(), 5);
    }
    let haeyo = HAEYO.find_iter(&ctx.content_text).count();
    let hapsyo = HAPSYO.find_iter(&ctx.content_text).count();
    let total = haeyo + hapsyo;
    if total < 3 {
        return mk("ending_consistency", "어미 일관성", Status::Na, "어미가 적어 평가 생략.".into(), 5);
    }
    let dominant = haeyo.max(hapsyo);
    let minor = haeyo.min(hapsyo);
    let consistency = (dominant as f64 / total as f64 * 100.0).round() as i64;
    let style = if haeyo >= hapsyo { "해요체" } else { "합쇼체" };

    if consistency >= 90 {
        mk("ending_consistency", "어미 일관성", Status::Pass, format!("{style} 일관됨 ({consistency}%)."), 5)
    } else if consistency >= 70 {
        mk("ending_consistency", "어미 일관성", Status::Warning, format!("{style} 위주이나 다른 어미 {minor}회 섞여 있습니다 ({consistency}%)."), 5)
    } else {
        mk("ending_consistency", "어미 일관성", Status::Fail, format!("해요체({haeyo})와 합쇼체({hapsyo}) 혼용. 일관된 어조 권장."), 5)
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
