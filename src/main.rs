#![deny(unused_crate_dependencies)]
mod common;
mod config;
mod error;
mod model;
mod schema;
mod upload;

use crate::common::*;
use crate::error::{Error, Result};
use crate::upload::upload;

use axum::{
    Router,
    http::HeaderMap,
    routing::{get, post},
};
use chrono::Utc;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use pq_sys as _;
use tokio::net::TcpListener;
use tracing::info;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

async fn healthz(headers: HeaderMap) -> Result<String> {
    let timestamp = format_timestamp_from_datetime(Utc::now().to_utc());

    info!(
        timestamp = %timestamp,
        user_agent = %headers.get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown"),
        "Health check requested"
    );

    Ok(format!(
        "{{\"status\":\"healthy\",\"timestamp\":\"{}\",\"service\":\"trunk-processor\"}}",
        timestamp
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trunk_processor=info,tower_http=debug".into()),
        )
        .init();

    info!("Initializing trunk-processor");

    let config = config::initialize()?;
    if config.filter.enabled() {
        info!(
            group = config.filter.group().join(", "),
            tgid = config.filter.tgid().join(", "),
            "Filter values provided"
        );
    } else {
        info!("Filtering disabled");
    }

    run_migrations(
        MIGRATIONS,
        &mut config
            .db_pool
            .clone()
            .get()
            .map_err(|e| Error::Database(e.to_string()))?,
    )?;

    let app = Router::new()
        .route("/upload", post(upload).with_state(config))
        .route("/healthz", get(healthz));

    let bind_addr = "0.0.0.0:3000";
    info!(addr = %bind_addr, "Starting HTTP server");

    let listener = TcpListener::bind(bind_addr)
        .await
        .map_err(Error::ServerInit)?;
    axum::serve(listener, app)
        .await
        .map_err(Error::ServerInit)?;

    Ok(())
}
