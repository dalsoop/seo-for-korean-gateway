//! Wire types — request/response/check shared between every module.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AnalyzeRequest {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub focus_keyword: String,
    #[serde(default)]
    pub meta_description: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyzeResponse {
    pub score: u32,
    pub grade: &'static str,
    pub checks: Vec<Check>,
    /// "regex" today; will become "lindera" once ko-dic morphology lands.
    pub engine: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub status: Status,
    pub message: String,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pass,
    Warning,
    Fail,
    Na,
}

pub fn mk(
    id: &'static str,
    label: &'static str,
    status: Status,
    message: String,
    weight: u32,
) -> Check {
    Check { id, label, status, message, weight }
}
