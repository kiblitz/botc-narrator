//! A table of bot phones for the integration tests: a server on a free
//! loopback port, a host and players joined over HTTP, each phone on its own
//! WebSocket, and a driver that answers prompts, nominates, votes and moves
//! the day along until the game ends.
#![allow(dead_code)]

use std::sync;
use std::time;

use botc_server::accounts;
use botc_server::api;
use botc_server::engine;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::json;
use tokio_tungstenite::tungstenite;

pub const HOST_ACCOUNT: &str = "host-account";
/// The host repeats "end the day" / "start the night" this often while the
/// room has not moved on: its view may lag the room, and a repeat does nothing.
const REPEAT: time::Duration = time::Duration::from_millis(500);

pub fn quick() -> engine::Timing {
    engine::Timing {
        answer: time::Duration::from_secs(5),
        decision: time::Duration::from_secs(10),
        vote: time::Duration::from_secs(5),
    }
}

/// reqwest shares the session library's TLS stack, which brings no crypto
/// provider of its own.
pub fn http() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::new()
}

/// A server on a free loopback port; returns its base URL.
pub async fn serve(timing: engine::Timing, seed: u64) -> String {
    let app = api::App::new(accounts::Gate::Fixture, timing, Some(seed));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, botc_server::app(app, None))
            .await
            .unwrap();
    });
    format!("http://{address}")
}

pub struct Phone {
    pub name: String,
    /// The latest view the server sent.
    pub view: sync::Arc<sync::Mutex<serde_json::Value>>,
    /// Other messages the server sent (errors, "gone").
    pub errors: sync::Arc<sync::Mutex<Vec<String>>>,
    outgoing: tokio::sync::mpsc::UnboundedSender<serde_json::Value>,
}

impl Phone {
    pub fn send(&self, message: serde_json::Value) {
        self.outgoing.send(message).unwrap();
    }

    pub fn view(&self) -> serde_json::Value {
        self.view.lock().unwrap().clone()
    }
}

/// Connects a phone for `token`'s seat.
pub async fn connect(base: &str, room: &str, name: &str, token: &str) -> Phone {
    let url = format!("{}/ws/play", base.replace("http://", "ws://"));
    let (socket, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    let (mut write, mut read) = socket.split();
    let hello = json!({"type": "hello", "room": room, "token": token});
    write
        .send(tungstenite::Message::text(hello.to_string()))
        .await
        .unwrap();
    let view = sync::Arc::new(sync::Mutex::new(serde_json::Value::Null));
    let errors = sync::Arc::new(sync::Mutex::new(Vec::new()));
    let (outgoing, mut queue) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    tokio::spawn(async move {
        while let Some(message) = queue.recv().await {
            if write
                .send(tungstenite::Message::text(message.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    let (latest, others) = (sync::Arc::clone(&view), sync::Arc::clone(&errors));
    tokio::spawn(async move {
        while let Some(Ok(message)) = read.next().await {
            let tungstenite::Message::Text(text) = message else {
                continue;
            };
            let value: serde_json::Value = serde_json::from_str(&text).unwrap();
            match value["type"].as_str() {
                Some("view") => *latest.lock().unwrap() = value,
                Some(_) => others.lock().unwrap().push(value.to_string()),
                None => {}
            }
        }
    });
    Phone {
        name: name.to_string(),
        view,
        errors,
        outgoing,
    }
}

/// Hosts a room as [`HOST_ACCOUNT`] and joins `players` more; returns the
/// room code, the phones (host first) and their tokens.
pub async fn table(base: &str, players: &[&str]) -> (String, Vec<Phone>, Vec<String>) {
    let http = http();
    let hosted: serde_json::Value = http
        .post(format!("{base}/api/rooms"))
        .header(accounts::FIXTURE_HEADER, HOST_ACCOUNT)
        .json(&json!({"name": "Host"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let room = hosted["room"].as_str().unwrap().to_string();
    let mut tokens = vec![hosted["token"].as_str().unwrap().to_string()];
    let mut names = vec!["Host".to_string()];
    for name in players {
        let joined: serde_json::Value = http
            .post(format!("{base}/api/rooms/{room}/join"))
            .json(&json!({"name": name}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        tokens.push(joined["token"].as_str().unwrap().to_string());
        names.push(name.to_string());
    }
    let mut phones = Vec::new();
    for (name, token) in names.iter().zip(&tokens) {
        phones.push(connect(base, &room, name, token).await);
    }
    (room, phones, tokens)
}

/// Waits until `check` holds for the phone's view.
pub async fn until(
    phone: &Phone,
    within: time::Duration,
    check: impl Fn(&serde_json::Value) -> bool,
) {
    let deadline = time::Instant::now() + within;
    while !check(&phone.view()) {
        assert!(
            time::Instant::now() < deadline,
            "{} waited too long; view: {:#}",
            phone.name,
            phone.view()
        );
        tokio::time::sleep(time::Duration::from_millis(20)).await;
    }
}

/// What the driver saw happen.
#[derive(Default, Debug)]
pub struct Played {
    pub prompts_answered: usize,
    pub nominations: usize,
    pub votes: usize,
    pub days: usize,
    /// A player's view showed someone else's character or a storyteller
    /// entry before the game was over.
    pub leaks: Vec<String>,
}

/// Sends `message` as the host for `phase`, unless it was sent for this
/// phase less than [`REPEAT`] ago. Returns whether this is the first time.
fn repeat(
    sent: &mut Vec<(String, time::Instant)>,
    phase: &str,
    host: &Phone,
    message: serde_json::Value,
) -> bool {
    let first = !sent.iter().any(|(p, _)| p == phase);
    let due = sent
        .iter()
        .rev()
        .find(|(p, _)| p == phase)
        .is_none_or(|(_, at)| at.elapsed() > REPEAT);
    if due {
        sent.push((phase.to_string(), time::Instant::now()));
        host.send(message);
    }
    first && due
}

/// Answers every prompt with the first legal choice, has the first player
/// who may nominate nominate the first other nominee once a day, votes yes,
/// and ends each day and starts each night as the host, until the game is over.
pub async fn play(phones: &[Phone], within: time::Duration) -> (serde_json::Value, Played) {
    let deadline = time::Instant::now() + within;
    let mut played = Played::default();
    let mut answered: Vec<(String, u64)> = Vec::new();
    let mut voted: Vec<(String, String, String)> = Vec::new();
    let mut nominated_on: Vec<String> = Vec::new();
    let mut ended: Vec<(String, time::Instant)> = Vec::new();
    let mut nights: Vec<(String, time::Instant)> = Vec::new();
    loop {
        assert!(
            time::Instant::now() < deadline,
            "no winner in time; host sees {:#}",
            phones[0].view()
        );
        let host = phones[0].view();
        if host["stage"] == "over" {
            return (host, played);
        }
        for phone in phones {
            let view = phone.view();
            check_private(&phone.name, &view, &mut played.leaks);
            let phase = view["phase"].as_str().unwrap_or("").to_string();
            if let Some(prompt) = view["prompt"].as_object() {
                let id = prompt["id"].as_u64().unwrap();
                if !answered.contains(&(phone.name.clone(), id)) {
                    answered.push((phone.name.clone(), id));
                    played.prompts_answered += 1;
                    phone.send(answer(prompt));
                }
            }
            if let Some(vote) = view["vote"].as_object()
                && vote["eligible"] == true
            {
                let key = (
                    phone.name.clone(),
                    phase.clone(),
                    vote["nominee"].to_string(),
                );
                if !voted.contains(&key) {
                    voted.push(key);
                    played.votes += 1;
                    phone.send(json!({"type": "vote", "yes": true}));
                }
            }
            if view["stage"] == "day"
                && view["may_nominate"] == true
                && !nominated_on.contains(&phase)
            {
                let nominee = view["nominees"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|n| n.as_str())
                    .find(|n| *n != phone.name && alive(&view, n));
                if let Some(nominee) = nominee {
                    nominated_on.push(phase.clone());
                    played.nominations += 1;
                    phone.send(json!({"type": "nominate", "nominee": nominee}));
                }
            }
        }
        let phase = host["phase"].as_str().unwrap_or("").to_string();
        // End the day once its nomination's vote is over (or nobody could nominate).
        if host["stage"] == "day" && host["vote"].is_null() {
            let someone_may = phones.iter().any(|p| p.view()["may_nominate"] == true);
            if (nominated_on.contains(&phase) || !someone_may)
                && repeat(&mut ended, &phase, &phones[0], json!({"type": "end_day"}))
            {
                played.days += 1;
            }
        }
        if host["stage"] == "dusk" {
            repeat(
                &mut nights,
                &phase,
                &phones[0],
                json!({"type": "start_night"}),
            );
        }
        tokio::time::sleep(time::Duration::from_millis(20)).await;
    }
}

fn alive(view: &serde_json::Value, name: &str) -> bool {
    view["seats"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|s| s["name"] == name && s["alive"] == true)
}

/// The first legal answer to a prompt.
fn answer(prompt: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    let id = &prompt["id"];
    match prompt["kind"].as_str().unwrap() {
        "players" => {
            let candidates = prompt["candidates"].as_array().unwrap();
            let max = prompt["max"].as_u64().unwrap() as usize;
            let min = prompt["min"].as_u64().unwrap() as usize;
            let count = min.max(max.min(1)).min(candidates.len());
            json!({"type": "answer", "prompt": id, "players": candidates[..count]})
        }
        "character" => json!({"type": "answer", "prompt": id, "option": 0}),
        _ => json!({"type": "answer", "prompt": id, "yes": false}),
    }
}

/// Records anything in `view` that its player should not see yet.
fn check_private(name: &str, view: &serde_json::Value, leaks: &mut Vec<String>) {
    if view["stage"] == "over" || view.is_null() {
        return;
    }
    for seat in view["seats"].as_array().into_iter().flatten() {
        if seat.get("character").is_some() {
            leaks.push(format!("{name} saw {}'s character", seat["name"]));
        }
    }
    for entry in view["log"].as_array().into_iter().flatten() {
        match entry["audience"]["kind"].as_str() {
            Some("public") => {}
            Some("player") if entry["audience"]["name"] == name => {}
            _ => leaks.push(format!("{name} saw {entry}")),
        }
    }
    if view["host"] == false && view["bridge"].get("pairing").is_some() {
        leaks.push(format!("{name} saw the pairing code"));
    }
}
