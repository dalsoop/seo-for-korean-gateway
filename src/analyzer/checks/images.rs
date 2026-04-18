//! Image checks. Alt coverage today; future: filename keyword, alt keyword,
//! image-to-content ratio, dimension specification.

use crate::analyzer::ctx::Ctx;
use crate::analyzer::helpers::{IMG, IMG_ALT};
use crate::analyzer::types::{mk, Check, Status};

pub fn run(ctx: &Ctx) -> Vec<Check> {
    vec![image_alt_coverage(ctx)]
}

fn image_alt_coverage(ctx: &Ctx) -> Check {
    let imgs: Vec<&str> = IMG.find_iter(&ctx.content_html).map(|m| m.as_str()).collect();
    let total = imgs.len();
    if total == 0 {
        return mk("image_alt_coverage", "이미지 alt", Status::Na, "본문에 이미지가 없습니다.".into(), 5);
    }
    let with_alt = imgs.iter().filter(|t| IMG_ALT.is_match(t)).count();
    if with_alt == total {
        mk("image_alt_coverage", "이미지 alt", Status::Pass, format!("모든 이미지({total}개)에 alt 속성이 있습니다."), 5)
    } else {
        let missing = total - with_alt;
        mk("image_alt_coverage", "이미지 alt", Status::Warning, format!("{missing}개 이미지에 alt 속성이 없습니다 (총 {total}개)."), 5)
    }
}
