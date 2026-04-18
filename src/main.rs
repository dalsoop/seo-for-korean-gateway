//! SEO for Korean — Korean keyword matching gateway.
//!
//! HTTP service the WordPress plugin calls when it needs Korean-aware text
//! analysis. The plugin already has a PHP regex fallback that strips common
//! particles; this gateway exists so the analyzer logic lives in one place
//! and can be upgraded to real morphological analysis (lindera + ko-dic)
//! without redeploying every WP install.
//!
//! V1 ships the same regex strategy as the PHP fallback. V2 will swap the
//! `match_keyword` implementation for lindera once the ko-dic asset hosting
//! is sorted (lindera 0.32's S3 URL currently 404s).
//!
//! Endpoints:
//!   GET  /health           — liveness probe
//!   POST /keyword/contains — count keyword occurrences in text
//!                            (particle-aware: '워드프레스' matches '워드프레스를')

use std::net::SocketAddr;

use anyhow::Context;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Common Korean particles that may follow a noun. Same list as the PHP
/// fallback — keep them in sync.
const PARTICLES: &str = "을|를|이|가|은|는|에|에서|의|와|과|도|만|보다|에게|께|로|으로|로서|으로서|로써|으로써|만큼|처럼|같이|마저|조차|이나|나|이라도|라도|이라고|라고|이라며|라며";

#[derive(Deserialize)]
struct ContainsRequest {
    text: String,
    keyword: String,
}

#[derive(Serialize)]
struct ContainsResponse {
    count: usize,
    matches: Vec<String>,
    /// Engine used to do the matching. "regex" for V1, "lindera" once we
    /// finish the ko-dic dictionary integration.
    engine: &'static str,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    engine: &'static str,
}

const ENGINE: &str = "regex";

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "seo-for-korean-gateway",
        version: env!("CARGO_PKG_VERSION"),
        engine: ENGINE,
    })
}

async fn keyword_contains(
    Json(req): Json<ContainsRequest>,
) -> Result<Json<ContainsResponse>, AppError> {
    let keyword = req.keyword.trim();
    if keyword.is_empty() {
        return Ok(Json(ContainsResponse {
            count: 0,
            matches: vec![],
            engine: ENGINE,
        }));
    }

    let pattern = format!("{}(?:{})?", regex::escape(keyword), PARTICLES);
    let re = Regex::new(&pattern).map_err(|e| AppError::Internal(format!("regex: {e}")))?;

    let matches: Vec<String> = re
        .find_iter(&req.text)
        .map(|m| m.as_str().to_string())
        .collect();

    Ok(Json(ContainsResponse {
        count: matches.len(),
        matches,
        engine: ENGINE,
    }))
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

/// Sanity check the particle regex compiles at startup. If this fails the
/// process should exit immediately rather than serve broken matches.
static SANITY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!("test(?:{})?", PARTICLES)).expect("particle regex must compile")
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    Lazy::force(&SANITY);
    tracing::info!(engine = ENGINE, "particle regex compiled");

    let app = Router::new()
        .route("/health", get(health))
        .route("/keyword/contains", post(keyword_contains))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let addr: SocketAddr = bind.parse().context("parse BIND address")?;
    tracing::info!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
