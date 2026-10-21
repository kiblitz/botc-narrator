//! HTTP and WebSockets: hosting and joining rooms, each phone's socket
//! (`route::PLAY`) and the bridge's (`route::BRIDGE`).
//!
//! Sockets authenticate in their first message (a player's token, a bridge's
//! pairing code), never in the URL, so no secret reaches an access log.

use std::collections;
use std::sync;
use std::sync::atomic;
use std::time;

use axum::extract;
use axum::extract::ws;
use axum::http;
use axum::response;
use axum::routing;
use botc_bridge::wire;

use crate::engine;
use crate::inbox;
use crate::room;
use crate::route;
use crate::view;

// A trait, which has to be in scope to be used.
use axum::response::IntoResponse;

/// A socket must say who it is within this long.
const HELLO: time::Duration = time::Duration::from_secs(10);
const PING: time::Duration = time::Duration::from_secs(25);
const MOST_MESSAGE_BYTES: usize = 16 * 1024;
/// Bridge replies carry a command's whole output.
const MOST_BRIDGE_MESSAGE_BYTES: usize = 256 * 1024;
const MOST_SOCKETS: usize = 512;
const SWEEP: time::Duration = time::Duration::from_secs(60);

#[derive(Clone)]
pub struct App {
    pub rooms: sync::Arc<room::Rooms>,
    pub timing: engine::Timing,
    /// Deals and random storytellers are reproducible with a seed (tests).
    pub seed: Option<u64>,
    sockets: sync::Arc<atomic::AtomicUsize>,
    bridges: sync::Arc<atomic::AtomicU64>,
    misses: sync::Arc<Misses>,
}

/// Wrong room codes, per client: past [`MOST_MISSES`] in a minute, that
/// client's joins are refused until its minute is over, so live rooms cannot
/// be found by guessing codes. Seat tokens and pairing codes are too long to
/// guess and are not limited, so a player's phone always gets back in.
#[derive(Default)]
struct Misses {
    clients: sync::Mutex<collections::HashMap<String, (time::Instant, usize)>>,
}

const MOST_MISSES: usize = 60;
const MISS_WINDOW: time::Duration = time::Duration::from_secs(60);
/// Past this many clients remembered, expired ones are forgotten.
const MOST_MISS_CLIENTS: usize = 4096;

impl Misses {
    fn allowed(&self, client: &str) -> bool {
        let clients = self
            .clients
            .lock()
            .unwrap_or_else(sync::PoisonError::into_inner);
        clients
            .get(client)
            .is_none_or(|(start, misses)| start.elapsed() > MISS_WINDOW || *misses < MOST_MISSES)
    }

    fn count(&self, client: &str) {
        let mut clients = self
            .clients
            .lock()
            .unwrap_or_else(sync::PoisonError::into_inner);
        if clients.len() > MOST_MISS_CLIENTS {
            clients.retain(|_, (start, _)| start.elapsed() <= MISS_WINDOW);
        }
        let entry = clients
            .entry(client.to_string())
            .or_insert((time::Instant::now(), 0));
        if entry.0.elapsed() > MISS_WINDOW {
            *entry = (time::Instant::now(), 0);
        }
        entry.1 += 1;
    }
}

/// Requests that came straight to the server rather than through Caddy:
/// local runs and tests, which share one client.
const DIRECT: &str = "direct";
/// IPv6 clients are told apart by their /64: one machine usually has the
/// whole block, so a single address would let it dodge every per-client limit.
const IPV6_CLIENT_SEGMENTS: usize = 4;

/// Who is asking: the client address Caddy puts in `X-Forwarded-For` (the
/// server only listens on loopback, behind Caddy). Caddy trusts no proxy in
/// front of it, so it replaces whatever header the client sent; keep it so
/// (no `trusted_proxies`), or the first entry becomes forgeable.
fn client(headers: &http::HeaderMap) -> String {
    let Some(first) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
    else {
        return DIRECT.to_string();
    };
    match first.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V6(v6)) if v6.to_ipv4_mapped().is_none() => {
            let segments = v6.segments();
            let prefix: Vec<String> = segments[..IPV6_CLIENT_SEGMENTS]
                .iter()
                .map(|s| format!("{s:x}"))
                .collect();
            format!("{}::/64", prefix.join(":"))
        }
        Ok(address) => address.to_canonical().to_string(),
        Err(_) => first.to_string(),
    }
}

impl App {
    pub fn new(timing: engine::Timing, seed: Option<u64>) -> Self {
        App {
            rooms: sync::Arc::default(),
            timing,
            seed,
            sockets: sync::Arc::default(),
            bridges: sync::Arc::default(),
            misses: sync::Arc::default(),
        }
    }

    /// Closes idle rooms every minute, for as long as the server runs.
    pub fn sweep_rooms(&self) {
        let rooms = sync::Arc::clone(&self.rooms);
        tokio::spawn(async move {
            let mut every = tokio::time::interval(SWEEP);
            loop {
                every.tick().await;
                rooms.sweep(time::Instant::now());
            }
        });
    }
}

pub fn router(app: App) -> axum::Router {
    axum::Router::new()
        .route(route::HEALTH, routing::get(|| async { "ok" }))
        .route(
            route::SCRIPTS,
            routing::get(|| async { response::Json(view::scripts()) }),
        )
        .route(route::ROOMS, routing::post(host))
        .route(route::JOIN, routing::post(join))
        .route(route::PLAY, routing::get(play))
        .route(route::BRIDGE, routing::get(bridge))
        .with_state(app)
}

#[derive(serde::Serialize)]
struct Failure {
    error: String,
}

fn refuse(status: http::StatusCode, error: impl ToString) -> response::Response {
    (
        status,
        response::Json(Failure {
            error: error.to_string(),
        }),
    )
        .into_response()
}

fn refused(error: room::Error) -> response::Response {
    let status = match error {
        room::Error::NoSuchRoom => http::StatusCode::NOT_FOUND,
        room::Error::TooManyRooms => http::StatusCode::SERVICE_UNAVAILABLE,
        room::Error::NotHost => http::StatusCode::FORBIDDEN,
        room::Error::HostingTooMany => http::StatusCode::TOO_MANY_REQUESTS,
        _ => http::StatusCode::CONFLICT,
    };
    refuse(status, error)
}

#[derive(serde::Deserialize)]
struct Hosting {
    name: String,
}

#[derive(serde::Serialize)]
struct Seated {
    room: String,
    token: String,
}

/// Anyone opens a room and takes its first seat: no account. The seat's
/// token, kept on the host's phone, is what makes them the host.
async fn host(
    extract::State(app): extract::State<App>,
    headers: http::HeaderMap,
    response::Json(hosting): response::Json<Hosting>,
) -> response::Response {
    match app.rooms.open(&client(&headers), &hosting.name) {
        Ok((room, token)) => {
            tracing::info!(room = %room.code, "room opened");
            response::Json(Seated {
                room: room.code.clone(),
                token,
            })
            .into_response()
        }
        Err(e) => refused(e),
    }
}

#[derive(serde::Deserialize)]
struct Joining {
    name: String,
}

/// Anyone with the room's code takes a seat, until the game starts.
async fn join(
    extract::State(app): extract::State<App>,
    extract::Path(code): extract::Path<String>,
    headers: http::HeaderMap,
    response::Json(joining): response::Json<Joining>,
) -> response::Response {
    let client = client(&headers);
    if !app.misses.allowed(&client) {
        return refuse(
            http::StatusCode::TOO_MANY_REQUESTS,
            "Too many wrong codes; wait a minute.",
        );
    }
    let Some(room) = app.rooms.get(&code) else {
        app.misses.count(&client);
        return refused(room::Error::NoSuchRoom);
    };
    match room.update(|s| s.join(&joining.name)) {
        Ok(token) => {
            tracing::info!(room = %room.code, "a player joined");
            response::Json(Seated {
                room: room.code.clone(),
                token,
            })
            .into_response()
        }
        Err(e) => refused(e),
    }
}

/// Counts an open socket; refuses past [`MOST_SOCKETS`].
struct Counted(sync::Arc<atomic::AtomicUsize>);

impl Counted {
    fn take(sockets: &sync::Arc<atomic::AtomicUsize>) -> Option<Counted> {
        if sockets.fetch_add(1, atomic::Ordering::SeqCst) >= MOST_SOCKETS {
            sockets.fetch_sub(1, atomic::Ordering::SeqCst);
            return None;
        }
        Some(Counted(sync::Arc::clone(sockets)))
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_sub(1, atomic::Ordering::SeqCst);
    }
}

/// What a phone sends.
#[derive(serde::Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromPhone {
    Hello {
        room: String,
        token: String,
    },
    Answer {
        prompt: u64,
        #[serde(default)]
        players: Option<Vec<String>>,
        #[serde(default)]
        option: Option<usize>,
        #[serde(default)]
        yes: Option<bool>,
    },
    Nominate {
        nominee: String,
    },
    Vote {
        yes: bool,
    },
    DayAction {
        ability: String,
    },
    // The host's:
    Reseat {
        order: Vec<String>,
    },
    Script {
        id: String,
    },
    Remove {
        name: String,
    },
    NewPairing,
    Start,
    EndDay,
    CloseVote,
    StartNight,
}

/// What a phone is sent.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ToPhone<'a> {
    View(&'a view::View),
    Error {
        message: String,
    },
    /// The room or the seat is gone: forget it.
    Gone {
        message: String,
    },
}

async fn play(
    extract::State(app): extract::State<App>,
    upgrade: ws::WebSocketUpgrade,
) -> response::Response {
    let Some(counted) = Counted::take(&app.sockets) else {
        return refuse(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "too many connections",
        );
    };
    upgrade
        .max_message_size(MOST_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            phone(app, socket).await;
            drop(counted);
        })
}

async fn send(socket: &mut ws::WebSocket, message: &ToPhone<'_>) -> bool {
    match serde_json::to_string(message) {
        Ok(text) => socket.send(ws::Message::text(text)).await.is_ok(),
        Err(_) => false,
    }
}

/// The first text message, within [`HELLO`].
async fn first_text(socket: &mut ws::WebSocket) -> Option<String> {
    let first = tokio::time::timeout(HELLO, socket.recv())
        .await
        .ok()??
        .ok()?;
    match first {
        ws::Message::Text(text) => Some(text.to_string()),
        _ => None,
    }
}

async fn phone(app: App, mut socket: ws::WebSocket) {
    let hello = first_text(&mut socket)
        .await
        .and_then(|t| serde_json::from_str::<FromPhone>(&t).ok());
    let Some(FromPhone::Hello { room: code, token }) = hello else {
        return;
    };
    let seat = app.rooms.get(&code).and_then(|room| {
        room.read(|s| s.seat_of(&token).map(|seat| seat.name.clone()))
            .map(|name| (room, name))
    });
    let Some((room, name)) = seat else {
        send(
            &mut socket,
            &ToPhone::Gone {
                message: "That room or seat no longer exists.".into(),
            },
        )
        .await;
        return;
    };
    let admitted = room.update(
        |s| match s.seats.iter_mut().find(|seat| seat.name == name) {
            Some(seat) if seat.connections < room::MOST_PHONES_PER_SEAT => {
                seat.connections += 1;
                true
            }
            _ => false,
        },
    );
    if !admitted {
        let message = "Too many phones are connected as this player.".to_string();
        send(&mut socket, &ToPhone::Error { message }).await;
        return;
    }
    let mut changes = room.subscribe();
    changes.mark_changed();
    let mut ping = tokio::time::interval(PING);
    loop {
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() {
                    break;
                }
                // Removed from the lobby, or the room closed.
                let gone = room.read(|s| s.closed || !s.seats.iter().any(|seat| seat.name == name));
                if gone {
                    send(&mut socket, &ToPhone::Gone { message: "You are no longer in this room.".into() }).await;
                    break;
                }
                let view = room.read(|s| view::of(&room.code, s, &name));
                if !send(&mut socket, &ToPhone::View(&view)).await {
                    break;
                }
            }
            message = socket.recv() => {
                let text = match message {
                    Some(Ok(ws::Message::Text(text))) => text,
                    Some(Ok(ws::Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => continue,
                };
                let outcome = match serde_json::from_str::<FromPhone>(&text) {
                    Ok(message) => handle(&app, &room, &name, message),
                    Err(_) => Err("That message was not understood.".to_string()),
                };
                if let Err(message) = outcome
                    && !send(&mut socket, &ToPhone::Error { message }).await
                {
                    break;
                }
            }
            _ = ping.tick() => {
                if socket.send(ws::Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
        }
    }
    room.update(|s| {
        if let Some(seat) = s.seats.iter_mut().find(|seat| seat.name == name) {
            seat.connections = seat.connections.saturating_sub(1);
        }
    });
}

/// Checks a phone's message against the room and passes it on.
pub fn handle(
    app: &App,
    room: &sync::Arc<room::Room>,
    name: &str,
    message: FromPhone,
) -> Result<(), String> {
    let host_only = |room: &room::Room| -> Result<(), String> {
        if room.read(|s| s.is_host(name)) {
            Ok(())
        } else {
            Err(room::Error::NotHost.to_string())
        }
    };
    let stage = room.read(|s| s.game.as_ref().map(|g| g.stage));
    let not_now = || Err::<(), String>("Not now.".to_string());
    match message {
        FromPhone::Hello { .. } => Ok(()),
        FromPhone::Answer {
            prompt,
            players,
            option,
            yes,
        } => {
            let asked = room.read(|s| {
                s.game
                    .as_ref()
                    .is_some_and(|g| g.prompts.iter().any(|p| p.id == prompt && p.player == name))
            });
            let answer = match (players, option, yes) {
                (Some(players), None, None) => inbox::Answer::Players(players),
                (None, Some(option), None) => inbox::Answer::Option(option),
                (None, None, Some(yes)) => inbox::Answer::YesNo(yes),
                _ => return Err("That answer was not understood.".to_string()),
            };
            if !asked {
                return not_now();
            }
            room.send(inbox::Input::Answer {
                prompt,
                player: name.to_string(),
                answer,
            });
            Ok(())
        }
        FromPhone::Nominate { nominee } => {
            let may = room.read(|s| {
                s.game.as_ref().is_some_and(|g| {
                    g.stage == room::Stage::Day
                        && g.may_nominate.iter().any(|p| p == name)
                        && !g.nominees.contains(&nominee)
                })
            });
            if !may {
                return not_now();
            }
            room.send(inbox::Input::Nominate {
                nominator: name.to_string(),
                nominee,
            });
            Ok(())
        }
        FromPhone::Vote { yes } => {
            let eligible = room.read(|s| {
                s.game
                    .as_ref()
                    .and_then(|g| g.vote.as_ref())
                    .is_some_and(|v| v.eligible.iter().any(|e| e == name))
            });
            if !eligible {
                return not_now();
            }
            room.send(inbox::Input::Vote {
                player: name.to_string(),
                yes,
            });
            Ok(())
        }
        FromPhone::DayAction { ability } => {
            let may = room.read(|s| {
                s.game.as_ref().is_some_and(|g| {
                    g.stage == room::Stage::Day
                        && g.day_actions
                            .iter()
                            .any(|a| a.player == name && a.ability == ability)
                })
            });
            if !may {
                return not_now();
            }
            room.send(inbox::Input::DayAction {
                player: name.to_string(),
                ability,
            });
            Ok(())
        }
        FromPhone::Reseat { order } => {
            host_only(room)?;
            room.update(|s| s.reseat(&order)).map_err(|e| e.to_string())
        }
        FromPhone::Script { id } => {
            host_only(room)?;
            room.update(|s| s.choose_script(&id))
                .map_err(|e| e.to_string())
        }
        FromPhone::Remove { name: removed } => {
            host_only(room)?;
            room.update(|s| s.remove(&removed))
                .map_err(|e| e.to_string())
        }
        FromPhone::NewPairing => {
            host_only(room)?;
            room.update(room::State::new_pairing);
            Ok(())
        }
        FromPhone::Start => {
            host_only(room)?;
            engine::start(room, app.timing, app.seed).map_err(|e| e.to_string())
        }
        FromPhone::EndDay => {
            host_only(room)?;
            if stage != Some(room::Stage::Day) {
                return not_now();
            }
            let first = room.update(|s| {
                s.game
                    .as_mut()
                    .is_some_and(|g| !std::mem::replace(&mut g.end_requested, true))
            });
            if first {
                room.send(inbox::Input::EndDay);
            }
            Ok(())
        }
        FromPhone::CloseVote => {
            host_only(room)?;
            if stage != Some(room::Stage::Vote) {
                return not_now();
            }
            room.send(inbox::Input::CloseVote);
            Ok(())
        }
        FromPhone::StartNight => {
            host_only(room)?;
            if stage != Some(room::Stage::Dusk) {
                return not_now();
            }
            let first = room.update(|s| {
                s.game
                    .as_mut()
                    .is_some_and(|g| !std::mem::replace(&mut g.night_requested, true))
            });
            if first {
                room.send(inbox::Input::StartNight);
            }
            Ok(())
        }
    }
}

async fn bridge(
    extract::State(app): extract::State<App>,
    upgrade: ws::WebSocketUpgrade,
) -> response::Response {
    let Some(counted) = Counted::take(&app.sockets) else {
        return refuse(
            http::StatusCode::SERVICE_UNAVAILABLE,
            "too many connections",
        );
    };
    upgrade
        .max_message_size(MOST_BRIDGE_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            bridge_socket(app, socket).await;
            drop(counted);
        })
}

async fn send_bridge(socket: &mut ws::WebSocket, message: &wire::ToBridge) -> bool {
    match serde_json::to_string(message) {
        Ok(text) => socket.send(ws::Message::text(text)).await.is_ok(),
        Err(_) => false,
    }
}

async fn bridge_socket(app: App, mut socket: ws::WebSocket) {
    let pair = first_text(&mut socket)
        .await
        .and_then(|t| serde_json::from_str::<wire::ToServer>(&t).ok());
    let room = match pair {
        Some(wire::ToServer::Pair { code }) => app.rooms.paired(&code),
        _ => None,
    };
    let Some(room) = room else {
        let reason = "That pairing code is not one the host was shown.".to_string();
        send_bridge(&mut socket, &wire::ToBridge::Refused { reason }).await;
        return;
    };
    let connection = app.bridges.fetch_add(1, atomic::Ordering::SeqCst);
    let (requests, mut decisions) = tokio::sync::mpsc::unbounded_channel();
    // A new bridge replaces the old one, whose channel then closes.
    room.update(|s| {
        s.bridge = Some(room::Bridge {
            connection,
            requests,
        })
    });
    tracing::info!(room = %room.code, connection, "bridge paired");
    if !send_bridge(
        &mut socket,
        &wire::ToBridge::Paired {
            room: room.code.clone(),
        },
    )
    .await
    {
        return forget_bridge(&room, connection, Vec::new());
    }
    let mut waiting: Vec<u64> = Vec::new();
    let mut ping = tokio::time::interval(PING);
    loop {
        tokio::select! {
            decision = decisions.recv() => {
                let Some(decision) = decision else {
                    let reason = "Another bridge paired, or the host made a new pairing code.".to_string();
                    send_bridge(&mut socket, &wire::ToBridge::Refused { reason }).await;
                    break;
                };
                if let wire::ToBridge::Decide { id, .. } = &decision {
                    waiting.push(*id);
                }
                if !send_bridge(&mut socket, &decision).await {
                    break;
                }
            }
            message = socket.recv() => {
                let text = match message {
                    Some(Ok(ws::Message::Text(text))) => text,
                    Some(Ok(ws::Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => continue,
                };
                if let Ok(wire::ToServer::Reply { id, text, error }) = serde_json::from_str(&text)
                    && let Some(i) = waiting.iter().position(|w| *w == id)
                {
                    waiting.remove(i);
                    let reply = text.ok_or_else(|| error.unwrap_or_else(|| "no reply".to_string()));
                    room.send(inbox::Input::Decided { request: id, reply });
                }
            }
            _ = ping.tick() => {
                if socket.send(ws::Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
        }
    }
    forget_bridge(&room, connection, waiting);
}

/// The bridge went: unpair it (unless a newer one took its place), and fail
/// the decisions it still owed, so the game picks at random at once.
fn forget_bridge(room: &room::Room, connection: u64, waiting: Vec<u64>) {
    room.update(|s| {
        if s.bridge
            .as_ref()
            .is_some_and(|b| b.connection == connection)
        {
            s.bridge = None;
        }
    });
    for request in waiting {
        room.send(inbox::Input::Decided {
            request,
            reply: Err("the bridge disconnected".to_string()),
        });
    }
    tracing::info!(room = %room.code, connection, "bridge gone");
}
