//! botc-bridge: lets this computer be the storyteller's brain for a room.
//!
//! Usage: botc-bridge <PAIRING-CODE> [--server URL] [--timeout SECONDS] [-- COMMAND...]
//!
//! The host's phone shows the pairing code. Each storyteller decision is a
//! prompt written to COMMAND's stdin; its stdout is the answer. COMMAND
//! defaults to `codex exec --skip-git-repo-check --ephemeral --sandbox=read-only -`;
//! `-- claude -p` uses Claude Code instead.

use std::time;

const USAGE: &str =
    "usage: botc-bridge <PAIRING-CODE> [--server URL] [--timeout SECONDS] [-- COMMAND...]";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut code: Option<String> = None;
    let mut server = botc_bridge::DEFAULT_SERVER.to_string();
    let mut timeout = botc_bridge::COMMAND_TIMEOUT;
    let mut command = botc_bridge::Command::codex();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => server = args.next().unwrap_or_else(|| fail("--server needs a URL")),
            "--timeout" => {
                let seconds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| fail("--timeout needs a number of seconds"));
                timeout = time::Duration::from_secs(seconds);
            }
            "--" => {
                let rest: Vec<String> = args.by_ref().collect();
                let Some((program, rest)) = rest.split_first() else {
                    fail("-- needs a command")
                };
                command = botc_bridge::Command {
                    program: program.clone(),
                    args: rest.to_vec(),
                };
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            other if code.is_none() && !other.starts_with('-') => code = Some(other.to_string()),
            other => fail(&format!("unexpected argument {other:?}")),
        }
    }
    let code = code.unwrap_or_else(|| fail("the pairing code is missing"));
    let runtime = tokio::runtime::Runtime::new().expect("a tokio runtime");
    runtime.block_on(async {
        let mut log = |line: &str| eprintln!("[bridge] {line}");
        log(&format!(
            "answering with: {} {}",
            command.program,
            command.args.join(" ")
        ));
        loop {
            match botc_bridge::serve(&server, &code, &command, timeout, &mut log).await {
                botc_bridge::Ended::Refused(reason) => {
                    log(&format!("refused: {reason}"));
                    std::process::exit(1);
                }
                botc_bridge::Ended::Dropped(reason) => {
                    log(&format!("disconnected ({reason}); reconnecting"));
                    tokio::time::sleep(botc_bridge::RECONNECT).await;
                }
            }
        }
    });
}

fn fail(message: &str) -> ! {
    eprintln!("botc-bridge: {message}\n{USAGE}");
    std::process::exit(2);
}
