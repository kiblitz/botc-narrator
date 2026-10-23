//! Soak runner: plays random games over every script forever (or for
//! `--games N` / `--minutes M`), writing failures and a summary to `--out`.
//! See `botc_narrator::soak`.
//!
//! Usage: soak [--out DIR] [--games N] [--minutes M] [--threads T]

use botc_narrator::soak;

const DEFAULT_OUT: &str = "C:/Users/Gabriel Lee/Projects/botc-soak";
const USAGE: &str = "usage: soak [--out DIR] [--games N] [--minutes M] [--threads T]";

fn main() {
    let mut config = soak::Config {
        out: DEFAULT_OUT.into(),
        games: None,
        duration: None,
        threads: std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1).max(1)),
        summary_every: std::time::Duration::from_secs(30),
        commit: commit(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let Some(value) = args.next() else {
            usage(&format!("{flag} needs a value"));
        };
        let number = || {
            value
                .parse::<u64>()
                .unwrap_or_else(|_| usage(&format!("{flag}: not a number: {value}")))
        };
        match flag.as_str() {
            "--out" => config.out = value.clone().into(),
            "--games" => config.games = Some(number()),
            "--minutes" => config.duration = Some(std::time::Duration::from_secs(60 * number())),
            "--threads" => config.threads = number() as usize,
            _ => usage(&format!("unknown flag {flag}")),
        }
    }
    eprintln!(
        "soak: commit {}, {} threads, writing to {}",
        config.commit,
        config.threads,
        config.out.display()
    );
    match soak::run(&config) {
        Ok(summary) => eprintln!(
            "soak: {} games, {} failure signatures",
            summary.games,
            summary.signatures.len()
        ),
        Err(e) => {
            eprintln!("soak: {e}");
            std::process::exit(1);
        }
    }
}

/// The checkout's commit (with `-dirty` for uncommitted changes to tracked
/// files), or "unknown" outside a git checkout.
fn commit() -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let dirty =
        git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    match git(&["rev-parse", "--short", "HEAD"]) {
        Some(hash) if dirty => format!("{hash}-dirty"),
        Some(hash) => hash,
        None => "unknown".to_string(),
    }
}

fn usage(error: &str) -> ! {
    eprintln!("soak: {error}\n{USAGE}");
    std::process::exit(2);
}
