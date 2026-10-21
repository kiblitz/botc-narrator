//! botc-server: reads its configuration from the environment and serves.
//!
//! - `BIND_ADDR` (default `127.0.0.1:8081`), loopback behind Caddy.
//! - `PUBLIC_ORIGIN` (default `https://botc.kiblitz.xyz`), for sign-in callbacks.
//! - `LOG` (default `info`), a tracing filter.
//! - `WEB_ROOT`, a directory to serve the web client from (local runs; Caddy
//!   serves it in production).
//! - `ACCOUNTS_DEVELOPMENT=1` with `ACCOUNTS_ISSUER`: a local account service.

use std::path;
use std::sync;

use anyhow::Context;
use botc_server::accounts;
use botc_server::api;
use botc_server::engine;

const DEFAULT_BIND: &str = "127.0.0.1:8081";
const DEFAULT_ORIGIN: &str = "https://botc.kiblitz.xyz";
const DEFAULT_LOG: &str = "info";

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log = env("LOG").unwrap_or_else(|| DEFAULT_LOG.to_string());
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(&log)
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_LOG)),
        )
        .init();
    let bind = env("BIND_ADDR").unwrap_or_else(|| DEFAULT_BIND.to_string());
    let origin = env("PUBLIC_ORIGIN").unwrap_or_else(|| DEFAULT_ORIGIN.to_string());
    let development = env("ACCOUNTS_DEVELOPMENT").as_deref() == Some("1");
    let manager = accounts::manager(env("ACCOUNTS_ISSUER").as_deref(), development, &origin)
        .context("preparing shared sign-in")?;
    let app = api::App::new(
        accounts::Gate::Shared(sync::Arc::new(manager)),
        engine::Timing::default(),
        None,
    );
    app.sweep_rooms();
    let web_root = env("WEB_ROOT").map(path::PathBuf::from);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, %origin, development, "botc is listening");
    axum::serve(listener, botc_server::app(app, web_root.as_deref()))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .context("serving")?;
    Ok(())
}
