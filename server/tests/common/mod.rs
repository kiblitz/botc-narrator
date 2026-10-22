//! A table of bot phones for the integration tests: a server on a free
//! loopback port, a host and players joined over HTTP, each phone on its own
//! WebSocket, and a driver that answers prompts, nominates, votes and moves
//! the day along until the game ends.
#![allow(dead_code)]

use std::sync;
use std::time;

use botc_server::api;
use botc_server::engine;
use futures_util::SinkExt;
use futures_util::StreamExt;
use serde_json::json;
use tokio_tungstenite::tungstenite;

/// The host repeats "end the day" / "start the night" this often while the
/// room has not moved on: its view may lag the room, and a repeat does nothing.
const REPEAT: time::Duration = time::Duration::from_millis(500);

pub fn quick() -> engine::Timing {
    engine::Timing {
        answer: time::Duration::from_secs(5),
        decision: time::Duration::from_secs(10),
    }
}

pub fn http() -> reqwest::Client {
    reqwest::Client::new()
}

/// A server on a free loopback port; returns its base URL.
pub async fn serve(timing: engine::Timing, seed: u64) -> String {
    let app = api::App::new(timing, Some(seed));
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

/// Hosts a room (no account) and joins `players` more; returns the
/// room code, the phones (host first) and their tokens.
pub async fn table(base: &str, players: &[&str]) -> (String, Vec<Phone>, Vec<String>) {
    let http = http();
    let hosted: serde_json::Value = http
        .post(format!("{base}/api/rooms"))
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

/// Answers every prompt with the first legal choice and, as the host,
/// nominates once a day (the first nominator, the first other living
/// nominee), raises every eligible hand, closes the vote, ends each day and
/// starts each night, until the game is over.
pub async fn play(phones: &[Phone], within: time::Duration) -> (serde_json::Value, Played) {
    let deadline = time::Instant::now() + within;
    let mut played = Played::default();
    let mut answered: Vec<(String, u64)> = Vec::new();
    let mut voted: Vec<(String, String, String)> = Vec::new();
    let mut nominated_on: Vec<String> = Vec::new();
    let mut ended: Vec<(String, time::Instant)> = Vec::new();
    let mut closed: Vec<(String, time::Instant)> = Vec::new();
    let mut nights: Vec<(String, time::Instant)> = Vec::new();
    // Who each phone saw dead when each night began: nobody may be seen
    // dying before dawn.
    let mut dead_at_dusk: Vec<(String, String, Vec<String>)> = Vec::new();
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
            if view["stage"] == "night" && phase.starts_with("Night") && view["winner"].is_null() {
                let dead: Vec<String> = view["seats"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|s| s["alive"] == false)
                    .map(|s| s["name"].as_str().unwrap().to_string())
                    .collect();
                match dead_at_dusk
                    .iter()
                    .find(|(p, n, _)| *p == phone.name && *n == phase)
                {
                    None => dead_at_dusk.push((phone.name.clone(), phase.clone(), dead)),
                    Some((_, _, before)) if *before != dead => played.leaks.push(format!(
                        "{} saw {dead:?} dead during {phase}, not {before:?}",
                        phone.name
                    )),
                    Some(_) => {}
                }
            }
            if let Some(prompt) = view["prompt"].as_object() {
                let id = prompt["id"].as_u64().unwrap();
                if !answered.contains(&(phone.name.clone(), id)) {
                    answered.push((phone.name.clone(), id));
                    played.prompts_answered += 1;
                    phone.send(answer(prompt));
                }
            }
        }
        let phase = host["phase"].as_str().unwrap_or("").to_string();
        // The host enters every nomination and hand: once a day the first
        // nominator puts up the first other living nominee.
        let nominators = names(&host["nominators"]);
        if host["stage"] == "day" && !nominated_on.contains(&phase) {
            let nominee = names(&host["nominees"])
                .into_iter()
                .find(|n| nominators.first().is_some_and(|f| f != n) && alive(&host, n));
            if let (Some(nominator), Some(nominee)) = (nominators.first(), nominee) {
                nominated_on.push(phase.clone());
                played.nominations += 1;
                phones[0]
                    .send(json!({"type": "nominate", "nominator": nominator, "nominee": nominee}));
            }
        }
        // Every hand goes up, then the host closes the vote.
        if let Some(vote) = host["vote"].as_object() {
            let raised = raised(&host);
            let mut all_up = true;
            for voter in names(&vote["eligible"]) {
                if raised.contains(&voter) {
                    continue;
                }
                all_up = false;
                let key = (voter.clone(), phase.clone(), vote["nominee"].to_string());
                if !voted.contains(&key) {
                    voted.push(key);
                    played.votes += 1;
                    phones[0].send(json!({"type": "vote", "voter": voter, "yes": true}));
                }
            }
            if all_up {
                let label = format!("{phase} {}", vote["nominee"]);
                repeat(
                    &mut closed,
                    &label,
                    &phones[0],
                    json!({"type": "close_vote"}),
                );
            }
        }
        // End the day once its nomination's vote is over (or nobody could nominate).
        if host["stage"] == "day"
            && host["vote"].is_null()
            && (nominated_on.contains(&phase) || nominators.is_empty())
            && repeat(&mut ended, &phase, &phones[0], json!({"type": "end_day"}))
        {
            played.days += 1;
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

/// The hands raised in a view's vote.
pub fn raised(view: &serde_json::Value) -> Vec<String> {
    view["vote"]["hands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|h| h[1] == true)
        .filter_map(|h| h[0].as_str())
        .map(ToString::to_string)
        .collect()
}

/// The names in a JSON array (none if it is missing).
pub fn names(list: &serde_json::Value) -> Vec<String> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n.as_str())
        .map(ToString::to_string)
        .collect()
}

fn alive(view: &serde_json::Value, name: &str) -> bool {
    view["seats"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|s| s["name"] == name && s["alive"] == true)
}

/// The first legal answer to a prompt.
pub fn answer(prompt: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
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
    if view["host"] == false && (view.get("nominators").is_some() || view.get("nominees").is_some())
    {
        leaks.push(format!("{name} was offered nominations"));
    }
}
