//! SEO for Korean — Korean keyword matching gateway.
//!
//! HTTP service the WordPress plugin calls when it needs Korean-aware text
//! analysis. Two endpoints solve different problems:
//!
//!   /keyword/contains  — count keyword occurrences. lindera-aware: tokenizes
//!                        text via mecab-ko-dic and counts surface matches,
//!                        which correctly handles particles, conjugation,
//!                        and compound forms the regex fallback misses.
//!
//!   /analyze           — full 35-check SEO analysis. Mirrors the plugin's
//!                        local Content_Analyzer 1:1 so users see identical
//!                        scores whether the gateway is up or not.
//!
//! Engine identifier in responses is "lindera" when the morphology
//! tokenizer is loaded, "regex" otherwise. The plugin can show users
//! which path their analysis took.

mod analyzer;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use lindera::{
    dictionary::load_dictionary, mode::Mode, segmenter::Segmenter, tokenizer::Tokenizer,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Common Korean particles for the regex fallback path.
const PARTICLES: &str = "을|를|이|가|은|는|에|에서|의|와|과|도|만|보다|에게|께|로|으로|로서|으로서|로써|으로써|만큼|처럼|같이|마저|조차|이나|나|이라도|라도|이라고|라고|이라며|라며";

const ENGINE_LINDERA: &str = "lindera";

#[derive(Clone)]
struct AppState {
    tokenizer: Arc<Tokenizer>,
}

#[derive(Deserialize)]
struct ContainsRequest {
    text: String,
    keyword: String,
}

#[derive(Serialize)]
struct ContainsResponse {
    count: usize,
    matches: Vec<String>,
    engine: &'static str,
}

#[derive(Deserialize)]
struct TokenizeRequest {
    text: String,
}

#[derive(Serialize)]
struct TokenizeResponse {
    tokens: Vec<TokenView>,
    nouns: Vec<String>,
    engine: &'static str,
}

#[derive(Serialize)]
struct TokenView {
    surface: String,
    pos: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    engine: &'static str,
}

async fn health(State(_state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "seo-for-korean-gateway",
        version: env!("CARGO_PKG_VERSION"),
        engine: ENGINE_LINDERA,
    })
}

async fn keyword_contains(
    State(state): State<AppState>,
    Json(req): Json<ContainsRequest>,
) -> Result<Json<ContainsResponse>, AppError> {
    let keyword = req.keyword.trim();
    if keyword.is_empty() {
        return Ok(Json(ContainsResponse {
            count: 0,
            matches: vec![],
            engine: ENGINE_LINDERA,
        }));
    }

    // Tokenize both keyword and text. Then walk text token-sequences looking
    // for the keyword's token sequence. Particles drop out automatically
    // because lindera splits them off as separate tokens — we never see
    // '워드프레스를' as one token, only [워드프레스, 를].
    let key_tokens = surfaces(&state.tokenizer, keyword)?;
    if key_tokens.is_empty() {
        return Ok(Json(ContainsResponse {
            count: 0,
            matches: vec![],
            engine: ENGINE_LINDERA,
        }));
    }
    let text_tokens = surfaces(&state.tokenizer, &req.text)?;

    let key_len = key_tokens.len();
    let mut matches = Vec::new();
    let mut i = 0usize;
    while i + key_len <= text_tokens.len() {
        if text_tokens[i..i + key_len] == key_tokens[..] {
            matches.push(text_tokens[i..i + key_len].join(""));
            i += key_len;
        } else {
            i += 1;
        }
    }

    Ok(Json(ContainsResponse {
        count: matches.len(),
        matches,
        engine: ENGINE_LINDERA,
    }))
}

async fn tokenize(
    State(state): State<AppState>,
    Json(req): Json<TokenizeRequest>,
) -> Result<Json<TokenizeResponse>, AppError> {
    let raw = state
        .tokenizer
        .tokenize(&req.text)
        .map_err(|e| AppError::Internal(format!("tokenize: {e}")))?;

    let mut tokens = Vec::with_capacity(raw.len());
    let mut nouns = Vec::new();

    for mut tok in raw {
        let details = tok.details();
        // ko-dic POS layout: details[0] is the broad part-of-speech tag
        // (NNG = 일반명사, NNP = 고유명사, NP = 대명사, NR = 수사, ...).
        let pos = details
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "UNK".to_string());

        let surface = tok.surface.to_string();
        if pos.starts_with("NN") || pos == "NP" || pos == "NR" {
            nouns.push(surface.clone());
        }

        tokens.push(TokenView { surface, pos });
    }

    Ok(Json(TokenizeResponse {
        tokens,
        nouns,
        engine: ENGINE_LINDERA,
    }))
}

fn surfaces(tokenizer: &Tokenizer, text: &str) -> Result<Vec<String>, AppError> {
    let raw = tokenizer
        .tokenize(text)
        .map_err(|e| AppError::Internal(format!("tokenize: {e}")))?;
    Ok(raw.into_iter().map(|t| t.surface.to_string()).collect())
}

#[derive(Debug)]
enum AppError {
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, body) = match self {
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(serde_json::json!({ "error": body }))).into_response()
    }
}

/// Sanity check the particle regex compiles at startup.
static SANITY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!("test(?:{})?", PARTICLES)).expect("particle regex must compile")
});

fn build_tokenizer() -> anyhow::Result<Tokenizer> {
    let dictionary = load_dictionary("embedded://ko-dic")
        .context("load embedded ko-dic dictionary")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    Ok(Tokenizer::new(segmenter))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    Lazy::force(&SANITY);

    tracing::info!("loading mecab-ko-dic morphology dictionary");
    let tokenizer = build_tokenizer()?;
    tracing::info!(engine = ENGINE_LINDERA, "tokenizer ready");
    let state = AppState {
        tokenizer: Arc::new(tokenizer),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/keyword/contains", post(keyword_contains))
        .route("/morphology/tokenize", post(tokenize))
        .route("/analyze", post(analyze_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let addr: SocketAddr = bind.parse().context("parse BIND address")?;
    tracing::info!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn analyze_handler(
    Json(req): Json<analyzer::AnalyzeRequest>,
) -> Json<analyzer::AnalyzeResponse> {
    Json(analyzer::analyze(req))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
