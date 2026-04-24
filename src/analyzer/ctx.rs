//! Pre-computed context passed to every check module.
//!
//! Doing this once up front means each check stays a pure function of the
//! same struct and we only walk regex/tokenize once for things multiple
//! checks need (link counts, content text).

use std::sync::Arc;

use super::helpers::{strip_html, A_HREF};
use super::keyword::KeywordCounter;
use super::types::AnalyzeRequest;

pub struct Ctx {
    pub title: String,
    pub title_length: usize,
    pub content_html: String,
    pub content_text: String,
    pub content_length: usize,
    pub slug: String,
    pub focus_keyword: String,
    pub meta_description: String,
    pub meta_description_length: usize,
    pub link_counts: LinkCounts,
    pub counter: Arc<dyn KeywordCounter>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LinkCounts {
    pub internal: usize,
    pub outbound: usize,
}

pub fn normalize(req: AnalyzeRequest, counter: Arc<dyn KeywordCounter>) -> Ctx {
    let title = req.title.trim().to_string();
    let content_text = strip_html(&req.content);
    let meta_desc = req.meta_description.trim().to_string();
    let link_counts = count_links(&req.content);
    Ctx {
        title_length: title.chars().count(),
        title,
        content_length: content_text.chars().count(),
        content_html: req.content,
        content_text,
        slug: req.slug.trim().to_string(),
        focus_keyword: req.focus_keyword.trim().to_string(),
        meta_description_length: meta_desc.chars().count(),
        meta_description: meta_desc,
        link_counts,
        counter,
    }
}

fn count_links(html: &str) -> LinkCounts {
    let mut internal = 0;
    let mut outbound = 0;
    for cap in A_HREF.captures_iter(html) {
        let href = cap[1].trim();
        if href.starts_with("http://")
            || href.starts_with("https://")
            || href.starts_with("//")
        {
            outbound += 1;
        } else if !href.is_empty()
            && !href.starts_with('#')
            && !href.starts_with("javascript:")
            && !href.starts_with("mailto:")
            && !href.starts_with("tel:")
        {
            internal += 1;
        }
    }
    LinkCounts { internal, outbound }
}
