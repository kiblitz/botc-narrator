//! Multiplayer rooms for Blood on the Clocktower at a real table: everyone
//! plays on their own phone, the engine is the storyteller, and a paired
//! bridge (Codex on someone's computer) makes the storyteller's choices.

pub mod accounts;
pub mod api;
pub mod bridged;
pub mod engine;
pub mod inbox;
pub mod phone;
pub mod room;
pub mod route;
pub mod view;

/// The whole app: the API and sockets, plus the web client from `web_root`
/// when given (in production Caddy serves the client instead).
pub fn app(app: api::App, web_root: Option<&std::path::Path>) -> axum::Router {
    let router = api::router(app);
    match web_root {
        Some(root) => router.fallback_service(tower_http::services::ServeDir::new(root).fallback(
            tower_http::services::ServeFile::new(root.join("index.html")),
        )),
        None => router,
    }
}
