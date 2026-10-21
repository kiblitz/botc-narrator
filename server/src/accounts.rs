//! Who is hosting. Hosting a room needs a Kiblitz account, through the shared
//! account service (`https://accounts.kiblitz.xyz`) and its pinned session
//! library; joining a room by its code needs none.
//!
//! The server never sees a password; it keeps the library's opaque cookie and
//! takes the account's subject (to give a host one room at a time) and its
//! display name (the host's default seat name).

use std::sync;

use account_session as session;
use axum::http;

/// The production issuer, the only one `main` trusts outside development.
pub const ISSUER: &str = "https://accounts.kiblitz.xyz";
/// This app's client id at the account service.
pub const CLIENT: &str = "botc";
/// The header a [`Gate::Fixture`] session travels in: not a cookie, so no
/// browser sends one by itself.
pub const FIXTURE_HEADER: &str = "x-fixture-session";

pub struct Identity {
    pub subject: String,
    pub name: String,
}

#[derive(Clone)]
pub enum Gate {
    /// The shared account service. The only gate `main` builds.
    Shared(sync::Arc<session::Manager>),
    /// Trusts whoever sends [`FIXTURE_HEADER`], as that account. Only the
    /// tests and the end-to-end example server build one; no variable or
    /// flag makes the real server do so.
    Fixture,
}

impl Gate {
    /// The session library's own routes: start sign-in, its callback, sign out.
    pub fn router(&self) -> axum::Router {
        match self {
            Gate::Shared(manager) => sync::Arc::clone(manager).router(),
            Gate::Fixture => axum::Router::new(),
        }
    }

    /// The signed-in account a request comes from; 401 for nobody.
    pub async fn identify(&self, headers: &http::HeaderMap) -> Result<Identity, http::StatusCode> {
        match self {
            Gate::Shared(manager) => {
                let session = manager.authenticate(headers).await?;
                let identity = session.identity().await?;
                Ok(Identity {
                    subject: identity.sub,
                    name: identity.name,
                })
            }
            Gate::Fixture => {
                let subject = headers
                    .get(FIXTURE_HEADER)
                    .and_then(|v| v.to_str().ok())
                    .filter(|v| !v.is_empty())
                    .ok_or(http::StatusCode::UNAUTHORIZED)?;
                Ok(Identity {
                    subject: subject.to_string(),
                    name: subject.to_string(),
                })
            }
        }
    }
}

/// The shared gate for `origin`: the pinned issuer, or with `development`, a
/// local one over plain HTTP.
pub fn manager(
    issuer: Option<&str>,
    development: bool,
    origin: &str,
) -> anyhow::Result<session::Manager> {
    let issuer: url::Url = issuer.unwrap_or(ISSUER).parse()?;
    if development {
        return Ok(session::Manager::development(
            issuer,
            CLIENT.to_string(),
            origin.parse()?,
        )?);
    }
    // An environment variable that pointed production anywhere else would
    // hand sign-in to whoever set it.
    anyhow::ensure!(
        issuer.as_str().trim_end_matches('/') == ISSUER,
        "the production account issuer is pinned to {ISSUER}"
    );
    Ok(session::Manager::new(
        issuer,
        CLIENT.to_string(),
        origin.parse()?,
    )?)
}

#[cfg(test)]
mod tests {
    use crate::accounts;

    const ORIGIN: &str = "https://botc.kiblitz.xyz";

    #[test]
    fn production_talks_to_the_pinned_issuer_and_no_other() {
        assert!(accounts::manager(None, false, ORIGIN).is_ok());
        assert!(accounts::manager(Some(accounts::ISSUER), false, ORIGIN).is_ok());
        for elsewhere in [
            "https://accounts.evil.test",
            "http://127.0.0.1:9000",
            "https://accounts.kiblitz.xyz.evil.test",
            "https://accounts.kiblitz.xyz/elsewhere",
        ] {
            assert!(
                accounts::manager(Some(elsewhere), false, ORIGIN).is_err(),
                "{elsewhere}"
            );
        }
    }

    #[test]
    fn only_development_names_its_own_issuer() {
        assert!(
            accounts::manager(Some("http://127.0.0.1:9000"), true, "http://127.0.0.1:8081").is_ok()
        );
    }
}
