//! A multiplayer server: the automated storyteller runs the game while players
//! connect over WebSocket.
//!
//! Architecture — a clean split between async I/O and the blocking engine:
//!
//! * The **async coordinator** (this module, on Tokio) owns the lobby and each
//!   player's WebSocket. It never blocks on game logic.
//! * The **game thread** (one `std::thread`, [`game::run_game`]) owns the
//!   engine and runs it to completion, blocking on player commands.
//! * Two channels bridge them: a `std::sync::mpsc` carries player commands into
//!   the game thread (which blocking-`recv`s them), and a Tokio channel carries
//!   game events out to be routed to the right player's socket.
//!
//! This is why the synchronous [`Storyteller`](crate::storyteller::Storyteller)
//! needs no async: a blocking `ask` simply parks the game thread until the
//! network delivers that player's answer.

mod game;
mod protocol;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

use protocol::{ClientMsg, GameEvent, PlayerCmd, ServerMsg, Target};

const MIN_PLAYERS: usize = 5;

struct Slot {
    name: String,
    out: UnboundedSender<ServerMsg>,
}

#[derive(Default)]
struct Room {
    slots: Vec<Slot>,
    started: bool,
    to_game: Option<std::sync::mpsc::Sender<PlayerCmd>>,
}

type Shared = Arc<Mutex<Room>>;

/// Serve the game on `addr` until the process exits.
pub async fn run(addr: SocketAddr) {
    let shared: Shared = Arc::new(Mutex::new(Room::default()));
    let app = Router::new()
        .route("/", get(index))
        .route("/spectate", get(spectate))
        .route("/ws", get(ws_handler))
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    println!("botc-narrator server listening on http://{addr}");
    axum::serve(listener, app).await.expect("serve");
}

async fn index() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

/// A spectator view: connects as every player and shows all their private POVs
/// at once. Used for demo recordings.
async fn spectate() -> Html<&'static str> {
    Html(include_str!("spectate.html"))
}

async fn ws_handler(ws: WebSocketUpgrade, State(shared): State<Shared>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, shared))
}

async fn handle_socket(socket: WebSocket, shared: Shared) {
    let (mut sink, mut stream) = socket.split();
    let (out_tx, mut out_rx) = unbounded_channel::<ServerMsg>();

    // Pump this connection's outbound queue to the socket.
    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let Ok(txt) = serde_json::to_string(&msg) else {
                continue;
            };
            if sink.send(Message::Text(txt)).await.is_err() {
                break;
            }
        }
    });

    let mut my_id: Option<usize> = None;
    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else {
            continue;
        };
        let Ok(cmd) = serde_json::from_str::<ClientMsg>(&txt) else {
            continue;
        };
        match cmd {
            ClientMsg::Join { name } if my_id.is_none() => {
                let assigned = {
                    let mut room = shared.lock().unwrap();
                    if room.started {
                        let _ = out_tx.send(ServerMsg::Error {
                            message: "game already started".into(),
                        });
                        None
                    } else {
                        let id = room.slots.len();
                        room.slots.push(Slot {
                            name,
                            out: out_tx.clone(),
                        });
                        Some(id)
                    }
                };
                if let Some(id) = assigned {
                    my_id = Some(id);
                    let _ = out_tx.send(ServerMsg::Joined { you: id });
                    broadcast_lobby(&shared);
                }
            }
            ClientMsg::Start if my_id.is_some() => start_game(&shared),
            other if my_id.is_some() => {
                let room = shared.lock().unwrap();
                if let Some(tg) = &room.to_game {
                    let _ = tg.send(PlayerCmd {
                        player: my_id.unwrap(),
                        msg: other,
                    });
                }
            }
            _ => {}
        }
    }

    send_task.abort();
}

fn broadcast_lobby(shared: &Shared) {
    let room = shared.lock().unwrap();
    let names: Vec<String> = room.slots.iter().map(|s| s.name.clone()).collect();
    let can_start = names.len() >= MIN_PLAYERS;
    for slot in &room.slots {
        let _ = slot.out.send(ServerMsg::Lobby {
            players: names.clone(),
            can_start,
        });
    }
}

static GAME_SEED: AtomicU64 = AtomicU64::new(0x0BADC0DE);

fn start_game(shared: &Shared) {
    let (names, outs) = {
        let mut room = shared.lock().unwrap();
        if room.started || room.slots.len() < MIN_PLAYERS {
            return;
        }
        room.started = true;
        let names: Vec<String> = room.slots.iter().map(|s| s.name.clone()).collect();
        let outs: Vec<UnboundedSender<ServerMsg>> =
            room.slots.iter().map(|s| s.out.clone()).collect();

        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<PlayerCmd>();
        room.to_game = Some(cmd_tx);
        let (evt_tx, evt_rx) = unbounded_channel::<GameEvent>();

        let seed = GAME_SEED.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed);
        let game_names = names.clone();
        std::thread::spawn(move || game::run_game(game_names, seed, evt_tx, cmd_rx));

        // Route game events to the right sockets.
        let route_outs = outs.clone();
        tokio::spawn(async move {
            let mut evt_rx = evt_rx;
            while let Some(ev) = evt_rx.recv().await {
                match ev.to {
                    Target::One(i) => {
                        if let Some(o) = route_outs.get(i) {
                            let _ = o.send(ev.msg);
                        }
                    }
                    Target::All => {
                        for o in &route_outs {
                            let _ = o.send(ev.msg.clone());
                        }
                    }
                }
            }
        });
        (names, outs)
    };

    let _ = names;
    for o in &outs {
        let _ = o.send(ServerMsg::Started);
    }
}
