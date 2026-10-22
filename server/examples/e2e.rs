//! The server the Playwright tests drive (`server/e2e`): the real app and web
//! client, but deals are seeded and players have little time to answer.
//!
//! Never deployed: examples are not part of the release build.
//!
//! Usage: cargo run -p botc_server --example e2e -- <port> <seed>

use std::path;
use std::time;

use botc_server::api;
use botc_server::engine;

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().and_then(|p| p.parse().ok()).unwrap_or(8091);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let timing = engine::Timing {
        answer: time::Duration::from_secs(30),
        decision: time::Duration::from_secs(10),
    };
    let app = api::App::new(timing, Some(seed));
    let root = path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .expect("a free port");
    eprintln!("e2e server on http://127.0.0.1:{port} (seed {seed})");
    axum::serve(listener, botc_server::app(app, Some(&root)))
        .await
        .expect("serving");
}
