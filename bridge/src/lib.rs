//! The bridge: runs on a computer at (or away from) the table, pairs with a
//! room, and answers the room's storyteller decisions by running a command
//! (Codex by default) with each prompt on stdin.

pub mod wire;

use std::process;
use std::time;

use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::tungstenite;

/// `codex exec` reading its prompt from stdin, outside any repository, with
/// no session saved and no permission to change files.
/// `server/static/bridge.mjs` mirrors this, [`COMMAND_TIMEOUT`] and
/// [`RECONNECT`]: keep them in step.
pub const DEFAULT_COMMAND: [&str; 6] = [
    "codex",
    "exec",
    "--skip-git-repo-check",
    "--ephemeral",
    "--sandbox=read-only",
    "-",
];
pub const DEFAULT_SERVER: &str = "https://botc.kiblitz.xyz";
/// A command that has not answered by now is stopped. Shorter than the
/// server's wait (90s), so a slow answer never delays the next decision past
/// the server's patience as well.
pub const COMMAND_TIMEOUT: time::Duration = time::Duration::from_secs(80);
/// How long to wait before reconnecting after the connection drops.
pub const RECONNECT: time::Duration = time::Duration::from_secs(3);

/// The program to run and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
}

impl Command {
    pub fn codex() -> Self {
        Command {
            program: DEFAULT_COMMAND[0].to_string(),
            args: DEFAULT_COMMAND[1..].iter().map(|a| a.to_string()).collect(),
        }
    }

    fn spawn(&self) -> std::io::Result<tokio::process::Child> {
        let mut command = self.process();
        command
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .kill_on_drop(true);
        command.spawn()
    }

    /// On Windows, `codex` and `claude` are npm `.cmd` shims, which only a
    /// shell can start; elsewhere the program runs directly.
    fn process(&self) -> tokio::process::Command {
        if cfg!(windows) && !self.program.to_ascii_lowercase().ends_with(".exe") {
            let mut command = tokio::process::Command::new("cmd");
            command.arg("/C").arg(&self.program).args(&self.args);
            command
        } else {
            let mut command = tokio::process::Command::new(&self.program);
            command.args(&self.args);
            command
        }
    }

    /// Runs the command with `prompt` on stdin; its stdout is the answer.
    pub async fn answer(&self, prompt: &str, timeout: time::Duration) -> Result<String, String> {
        let mut child = self
            .spawn()
            .map_err(|e| format!("could not start {}: {e}", self.program))?;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| format!("could not write the prompt: {e}"))?;
        drop(stdin);
        let output = tokio::time::timeout(timeout, child.wait_with_output())
            .await
            .map_err(|_| format!("{} took longer than {timeout:?}", self.program))?
            .map_err(|e| format!("{} failed: {e}", self.program))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let last = stderr.lines().rev().find(|l| !l.trim().is_empty());
            return Err(format!(
                "{} exited with {}: {}",
                self.program,
                output.status,
                last.unwrap_or("").trim()
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// How the bridge connection ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// The server refused the pairing code: reconnecting will not help.
    Refused(String),
    /// The connection dropped; reconnecting may.
    Dropped(String),
}

/// The bridge WebSocket's address on `server` (`https://host` or `wss://host`).
pub fn endpoint(server: &str) -> String {
    let server = server.trim_end_matches('/');
    let server = if let Some(rest) = server.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = server.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        server.to_string()
    };
    format!("{server}/ws/bridge")
}

/// Pairs with the room and answers decisions until the connection ends.
/// Decisions are answered one at a time, in order.
pub async fn serve(
    server: &str,
    code: &str,
    command: &Command,
    timeout: time::Duration,
    log: &mut impl FnMut(&str),
) -> Ended {
    // Which provider TLS uses cannot be left to the build's features: with
    // more than one compiled in, rustls refuses to guess and panics.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let url = endpoint(server);
    let (mut socket, _) = match tokio_tungstenite::connect_async(url.as_str()).await {
        Ok(connected) => connected,
        Err(e) => return Ended::Dropped(format!("could not connect to {url}: {e}")),
    };
    let pair = wire::ToServer::Pair {
        code: code.to_string(),
    };
    if let Err(e) = send(&mut socket, &pair).await {
        return Ended::Dropped(e);
    }
    while let Some(message) = socket.next().await {
        let text = match message {
            Ok(tungstenite::Message::Text(text)) => text,
            Ok(tungstenite::Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => return Ended::Dropped(e.to_string()),
        };
        let Ok(message) = serde_json::from_str::<wire::ToBridge>(&text) else {
            log(&format!(
                "ignoring a message the bridge does not understand: {text}"
            ));
            continue;
        };
        match message {
            wire::ToBridge::Paired { room } => log(&format!("paired with room {room}")),
            wire::ToBridge::Refused { reason } => return Ended::Refused(reason),
            wire::ToBridge::Decide { id, prompt } => {
                log(&format!("decision {id}: asking {}", command.program));
                let started = time::Instant::now();
                let reply = match command.answer(&prompt, timeout).await {
                    Ok(text) => {
                        log(&format!(
                            "decision {id}: answered in {:.1}s: {}",
                            started.elapsed().as_secs_f32(),
                            text.trim()
                        ));
                        wire::ToServer::Reply {
                            id,
                            text: Some(text),
                            error: None,
                        }
                    }
                    Err(e) => {
                        log(&format!("decision {id}: {e}"));
                        wire::ToServer::Reply {
                            id,
                            text: None,
                            error: Some(e),
                        }
                    }
                };
                if let Err(e) = send(&mut socket, &reply).await {
                    return Ended::Dropped(e);
                }
            }
        }
    }
    Ended::Dropped("the server closed the connection".to_string())
}

async fn send<S>(socket: &mut S, message: &wire::ToServer) -> Result<(), String>
where
    S: futures_util::Sink<tungstenite::Message, Error = tungstenite::Error> + Unpin,
{
    let text = serde_json::to_string(message).map_err(|e| e.to_string())?;
    socket
        .send(tungstenite::Message::text(text))
        .await
        .map_err(|e| e.to_string())
}
