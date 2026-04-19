mod routes;

use anyhow::Context;
use axum::{
    http::{header, HeaderName, HeaderValue, Method},
    Router,
};
use routes::{router, ApiState};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const OWNER_PUBKEY_HEADER: &str = "x-shipyard-owner-pubkey";

/// Parse a comma-separated list of allowed CORS origins from an env-var value.
/// Returns an empty Vec when the input is blank.
fn parse_cors_origins(env_val: &str) -> Vec<HeaderValue> {
    env_val
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                s.parse::<HeaderValue>().ok()
            }
        })
        .collect()
}

fn cors_allowed_headers() -> [HeaderName; 4] {
    [
        header::AUTHORIZATION,
        header::CONTENT_TYPE,
        header::ACCEPT,
        HeaderName::from_static(OWNER_PUBKEY_HEADER),
    ]
}

/// Build a [`CorsLayer`] from the `SHIPYARD_CORS_ORIGINS` environment variable.
///
/// * If the variable is absent or empty → **no origins allowed** (secure production default).
/// * If the variable contains a comma-separated list of origins → those origins are allowed.
fn build_cors_layer() -> CorsLayer {
    let raw = std::env::var("SHIPYARD_CORS_ORIGINS").unwrap_or_default();
    let origins = parse_cors_origins(&raw);

    if origins.is_empty() {
        return CorsLayer::new();
    }

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(cors_allowed_headers())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shipyard_api=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let bind_addr = std::env::var("SHIPYARD_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let state = ApiState::from_env().await?;
    let app = Router::new()
        .nest("/v1", router(state))
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .layer(build_cors_layer())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind API on {bind_addr}"))?;

    tracing::info!(%bind_addr, "shipyard-api listening");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cors_origins_returns_empty_for_blank_string() {
        assert!(parse_cors_origins("").is_empty());
        assert!(parse_cors_origins("   ").is_empty());
    }

    #[test]
    fn parse_cors_origins_parses_single_valid_origin() {
        let origins = parse_cors_origins("https://app.example.com");
        assert_eq!(origins.len(), 1);
        assert_eq!(
            origins[0],
            "https://app.example.com".parse::<HeaderValue>().unwrap()
        );
    }

    #[test]
    fn parse_cors_origins_parses_comma_separated_list() {
        let origins = parse_cors_origins("https://app.example.com,https://www.example.com");
        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn parse_cors_origins_trims_whitespace_around_entries() {
        let origins = parse_cors_origins("https://app.example.com , https://www.example.com");
        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn parse_cors_origins_ignores_empty_entries_from_double_comma() {
        let origins = parse_cors_origins("https://app.example.com,,https://www.example.com");
        assert_eq!(origins.len(), 2);
    }

    #[test]
    fn cors_allowed_headers_includes_owner_pubkey_header() {
        assert!(cors_allowed_headers().contains(&HeaderName::from_static(OWNER_PUBKEY_HEADER)));
    }

    #[test]
    fn build_cors_layer_does_not_panic_with_empty_env() {
        // Ensure we can build a restrictive layer without panicking
        std::env::remove_var("SHIPYARD_CORS_ORIGINS");
        let _layer = build_cors_layer();
    }
}
