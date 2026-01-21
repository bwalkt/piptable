//! # piptable-server
//!
//! HTTP server for the piptable API.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: String,
    version: String,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/health", get(health));

    let addr = "0.0.0.0:3000";
    println!("piptable-server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
