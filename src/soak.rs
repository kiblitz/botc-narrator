//! Continuous play-testing: plays random simulator games ([`sim::simulate`])
//! over every script and legal player count on several threads, checks each
//! with [`sim::problems`], and writes failures and a summary to a directory.
//!
//! Output (in [`Config::out`]):
//! - `failures.jsonl`: one [`Failure`] per line, appended; at most
//!   [`MAX_PER_SIGNATURE`] lines per [`signature`] (counted across runs);
//! - `summary.json` / `summary.md`: rewritten every [`Config::summary_every`]
//!   and at the end: games, win rates and days per script, and failures
//!   grouped by signature with a count and one repro. A new run starts a new
//!   summary.
//!
//! Every failure reproduces with `sim::simulate(seed, players, script)`.

use std::collections;
use std::io::Write as _;
use std::sync::atomic;

use rand::Rng;
use serde::Serialize;

use crate::character;
use crate::game;
use crate::script;
use crate::sim;

/// Lines kept in `failures.jsonl` per distinct signature.
pub const MAX_PER_SIGNATURE: usize = 20;

pub struct Config {
    pub out: std::path::PathBuf,
    /// Stop after this many games (`None`: no limit).
    pub games: Option<u64>,
    /// Stop after this long (`None`: no limit).
    pub duration: Option<std::time::Duration>,
    pub threads: usize,
    pub summary_every: std::time::Duration,
    /// The engine's git commit, recorded with each failure.
    pub commit: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Panic,
    /// No winner by the simulator's day limit ([`sim::MAX_DAYS`]).
    Stuck,
    Invariant,
}

/// One failing game, as written to `failures.jsonl`.
#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    /// Unix seconds.
    pub time: u64,
    pub commit: String,
    pub script: &'static str,
    pub players: usize,
    pub seed: u64,
    pub kind: Kind,
    pub message: String,
    pub signature: String,
}

/// The result of one game.
pub struct Game {
    pub script: &'static str,
    pub players: usize,
    pub seed: u64,
    pub outcome: Outcome,
}

pub enum Outcome {
    Finished {
        winner: character::Alignment,
        days: u32,
    },
    Failed(Kind, String),
}

thread_local! {
    /// Whether this thread is playing a game in [`play`].
    static IN_GAME: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The last panic's message and location in a game on this thread.
    static LAST_PANIC: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Installs (once per process) a panic hook that records the message and
/// location of a panic in [`play`] instead of printing it; other panics go
/// to the previous hook.
pub fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_GAME.get() {
                return previous(info);
            }
            let payload = info.payload();
            let message = payload
                .downcast_ref::<&str>()
                .map(ToString::to_string)
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "non-string panic".to_string());
            let location = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default();
            LAST_PANIC.set(Some(format!("{message} at {location}")));
        }));
    });
}

/// Plays and judges one game (call [`install_panic_hook`] first to get
/// panic messages).
pub fn play(seed: u64, players: usize, script: &script::Script) -> Game {
    Game {
        script: script.id,
        players,
        seed,
        outcome: outcome(|| sim::simulate(seed, players, script)),
    }
}

/// Runs a game, catching a panic, and judges its trace.
pub fn outcome(game: impl FnOnce() -> sim::Trace) -> Outcome {
    LAST_PANIC.take();
    IN_GAME.set(true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(game));
    IN_GAME.set(false);
    match result {
        Err(_) => {
            let message = LAST_PANIC
                .take()
                .unwrap_or_else(|| "panic (no soak panic hook)".to_string());
            Outcome::Failed(Kind::Panic, message)
        }
        Ok(trace) => judge(&trace),
    }
}

/// A trace's outcome: its first broken invariant, else stuck, else its winner.
pub fn judge(trace: &sim::Trace) -> Outcome {
    if let Some(problem) = sim::problems(trace).into_iter().next() {
        return Outcome::Failed(Kind::Invariant, problem);
    }
    let days = trace
        .steps
        .iter()
        .filter_map(|s| match s.phase {
            game::Phase::Day(n) => Some(n),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    match trace.winner {
        Some(winner) => Outcome::Finished { winner, days },
        None => Outcome::Failed(
            Kind::Stuck,
            format!("no winner after {} days", sim::MAX_DAYS),
        ),
    }
}

/// Groups failures that are the same bug: the kind and message with seat
/// names and numbers blanked out (the simulator's default seat names, which
/// [`sim::simulate`] uses).
pub fn signature(kind: Kind, message: &str) -> String {
    let mut text = message.to_string();
    for name in sim::seat_names(&[]) {
        text = replace_word(&text, &name, "<player>");
    }
    let mut blanked = String::new();
    for c in text.chars() {
        if !c.is_ascii_digit() {
            blanked.push(c);
        } else if !blanked.ends_with('#') {
            blanked.push('#');
        }
    }
    let text = blanked;
    let kind = serde_json::to_value(kind).expect("kinds serialize");
    format!("{}: {text}", kind.as_str().unwrap_or_default())
}

/// `text` with every whole-word `word` replaced by `with`.
fn replace_word(text: &str, word: &str, with: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find(word) {
        let before = rest[..i].chars().last().or(out.chars().last());
        let after = rest[i + word.len()..].chars().next();
        out.push_str(&rest[..i]);
        let whole = before.is_none_or(|c| !c.is_alphanumeric())
            && after.is_none_or(|c| !c.is_alphanumeric());
        out.push_str(if whole { with } else { word });
        rest = &rest[i + word.len()..];
    }
    out.push_str(rest);
    out
}

#[derive(Default, Serialize)]
pub struct ScriptStats {
    pub games: u64,
    pub good_wins: u64,
    pub evil_wins: u64,
    pub failures: u64,
    /// Days summed over finished games (for the average).
    pub days: u64,
}

#[derive(Serialize)]
pub struct SignatureStats {
    pub kind: Kind,
    pub count: u64,
    /// The first failing game: `sim::simulate(seed, players, script)`.
    pub script: &'static str,
    pub players: usize,
    pub seed: u64,
    pub message: String,
}

#[derive(Default, Serialize)]
pub struct Summary {
    pub commit: String,
    pub started: u64,
    pub updated: u64,
    pub games: u64,
    pub scripts: collections::BTreeMap<&'static str, ScriptStats>,
    pub signatures: collections::BTreeMap<String, SignatureStats>,
}

impl Summary {
    /// Counts a game; returns its failure if it failed.
    pub fn add(&mut self, game: &Game, commit: &str) -> Option<Failure> {
        self.games += 1;
        let stats = self.scripts.entry(game.script).or_default();
        stats.games += 1;
        match &game.outcome {
            Outcome::Finished { winner, days } => {
                match winner {
                    character::Alignment::Good => stats.good_wins += 1,
                    character::Alignment::Evil => stats.evil_wins += 1,
                }
                stats.days += u64::from(*days);
                None
            }
            Outcome::Failed(kind, message) => {
                stats.failures += 1;
                let signature = signature(*kind, message);
                self.signatures
                    .entry(signature.clone())
                    .or_insert_with(|| SignatureStats {
                        kind: *kind,
                        count: 0,
                        script: game.script,
                        players: game.players,
                        seed: game.seed,
                        message: message.clone(),
                    })
                    .count += 1;
                Some(Failure {
                    time: now(),
                    commit: commit.to_string(),
                    script: game.script,
                    players: game.players,
                    seed: game.seed,
                    kind: *kind,
                    message: message.clone(),
                    signature,
                })
            }
        }
    }

    /// The summary for people: a table per script, then failures by
    /// signature, most frequent first.
    pub fn markdown(&self) -> String {
        let mut md = format!(
            "# Soak summary\n\nCommit `{}`, {} games, started {} and updated {} (unix seconds).\n\n\
             | Script | Games | Good wins | Evil wins | Failures | Avg days |\n\
             |---|---|---|---|---|---|\n",
            self.commit, self.games, self.started, self.updated
        );
        for (id, s) in &self.scripts {
            let finished = (s.good_wins + s.evil_wins).max(1) as f64;
            md += &format!(
                "| {id} | {} | {:.0}% | {:.0}% | {} | {:.1} |\n",
                s.games,
                100.0 * s.good_wins as f64 / finished,
                100.0 * s.evil_wins as f64 / finished,
                s.failures,
                s.days as f64 / finished
            );
        }
        md += "\n## Failures by signature\n\n";
        if self.signatures.is_empty() {
            md += "None.\n";
        }
        let mut signatures: Vec<(&String, &SignatureStats)> = self.signatures.iter().collect();
        signatures.sort_by_key(|(_, s)| std::cmp::Reverse(s.count));
        for (signature, s) in signatures {
            md += &format!(
                "- x{} `{signature}`\n  - repro: `sim::simulate({}, {}, &script::by_id(\"{}\").unwrap())`\n  - {}\n",
                s.count, s.seed, s.players, s.script, s.message
            );
        }
        md
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Lines already in `failures.jsonl` per signature, so the cap holds across runs.
fn logged_signatures(path: &std::path::Path) -> collections::BTreeMap<String, usize> {
    let mut counts = collections::BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return counts;
    };
    for line in text.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
            && let Some(signature) = value.get("signature").and_then(|s| s.as_str())
        {
            *counts.entry(signature.to_string()).or_insert(0) += 1;
        }
    }
    counts
}

fn write_summary(out: &std::path::Path, summary: &mut Summary) -> std::io::Result<()> {
    summary.updated = now();
    let json = serde_json::to_string_pretty(summary).expect("summaries serialize");
    std::fs::write(out.join("summary.json"), json)?;
    std::fs::write(out.join("summary.md"), summary.markdown())
}

/// Plays games on `config.threads` threads until the config's limits
/// (forever without any), returning the final summary. Installs the soak
/// panic hook for the whole process.
pub fn run(config: &Config) -> std::io::Result<Summary> {
    std::fs::create_dir_all(&config.out)?;
    install_panic_hook();
    let scripts: Vec<(script::Script, usize)> = script::all()
        .into_iter()
        .map(|s| {
            let max = sim::max_players(&s);
            (s, max)
        })
        .collect();
    let failures_path = config.out.join("failures.jsonl");
    let mut logged = logged_signatures(&failures_path);
    let mut failures = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&failures_path)?;
    let mut summary = Summary {
        commit: config.commit.clone(),
        started: now(),
        ..Summary::default()
    };
    let start = std::time::Instant::now();
    let stop = atomic::AtomicBool::new(false);
    let started = atomic::AtomicU64::new(0);
    let (send, receive) = std::sync::mpsc::channel::<Game>();
    let collected = std::thread::scope(|scope| {
        for _ in 0..config.threads.max(1) {
            let send = send.clone();
            let (scripts, stop, started) = (&scripts, &stop, &started);
            scope.spawn(move || {
                let mut rng = rand::rng();
                while !stop.load(atomic::Ordering::Relaxed) {
                    let n = started.fetch_add(1, atomic::Ordering::Relaxed);
                    if config.games.is_some_and(|g| n >= g) {
                        break;
                    }
                    let (script, max) = &scripts[rng.random_range(0..scripts.len())];
                    let players = rng.random_range(sim::MIN_PLAYERS..=*max);
                    if send.send(play(rng.random(), players, script)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(send);
        let mut last_summary = std::time::Instant::now();
        let mut collect = || -> std::io::Result<()> {
            loop {
                match receive.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(game) => {
                        if let Some(failure) = summary.add(&game, &config.commit) {
                            let count = logged.entry(failure.signature.clone()).or_insert(0);
                            if *count < MAX_PER_SIGNATURE {
                                *count += 1;
                                let line =
                                    serde_json::to_string(&failure).expect("failures serialize");
                                writeln!(failures, "{line}")?;
                            }
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
                }
                if config.duration.is_some_and(|d| start.elapsed() >= d) {
                    stop.store(true, atomic::Ordering::Relaxed);
                }
                if last_summary.elapsed() >= config.summary_every {
                    write_summary(&config.out, &mut summary)?;
                    last_summary = std::time::Instant::now();
                }
            }
        };
        let collected = collect();
        // On a write error, stop the workers too (they end at their next game).
        stop.store(true, atomic::Ordering::Relaxed);
        collected
    });
    collected?;
    write_summary(&config.out, &mut summary)?;
    Ok(summary)
}
