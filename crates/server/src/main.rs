//! # piptable-server
//!
//! HTTP server for the piptable API.

use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};

/// Health check response.
#[derive(Serialize, Deserialize)]
pub struct Health {
    /// Server status ("ok" when healthy).
    pub status: String,
    /// Server version from Cargo.toml.
    pub version: String,
}

/// Health check endpoint handler.
pub async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create the application router.
///
/// This is separated from `main()` to allow testing.
pub fn create_router() -> Router {
    Router::new().route("/health", get(health))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router();

    let addr = "0.0.0.0:3000";
    println!("piptable-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint_status() {
        let app = create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_endpoint_body() {
        let app = create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: Health = serde_json::from_slice(&body).unwrap();

        assert_eq!(health.status, "ok");
        assert!(!health.version.is_empty());
    }

    #[tokio::test]
    async fn test_not_found() {
        let app = create_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_handler_directly() {
        let Json(health) = health().await;
        assert_eq!(health.status, "ok");
        assert_eq!(health.version, env!("CARGO_PKG_VERSION"));
    }
}
