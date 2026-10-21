//! The bridge against a real server: pairing, decisions answered by a fake
//! `codex` command, and the random fallback when the command fails.

#[path = "../../server/tests/common/mod.rs"]
mod common;

use std::time;

use serde_json::json;

const FAKE: &str = env!("CARGO_BIN_EXE_fake-storyteller");
const PLAYERS: [&str; 5] = ["Ada", "Bram", "Cleo", "Dev", "Esme"];

fn fake(args: &[&str]) -> botc_bridge::Command {
    botc_bridge::Command {
        program: FAKE.to_string(),
        args: args.iter().map(|a| a.to_string()).collect(),
    }
}

/// Pairs a bridge running `command` with the host's room, in the background.
async fn pair(base: &str, host: &common::Phone, command: botc_bridge::Command) {
    common::until(host, time::Duration::from_secs(5), |v| {
        v["bridge"]["pairing"].is_string()
    })
    .await;
    let code = host.view()["bridge"]["pairing"]
        .as_str()
        .unwrap()
        .to_string();
    let base = base.to_string();
    tokio::spawn(async move {
        let mut quiet = |_: &str| {};
        botc_bridge::serve(
            &base,
            &code,
            &command,
            time::Duration::from_secs(10),
            &mut quiet,
        )
        .await
    });
    common::until(host, time::Duration::from_secs(5), |v| {
        v["bridge"]["connected"] == true
    })
    .await;
}

#[tokio::test]
async fn the_command_reads_the_prompt_and_its_output_is_the_answer() {
    let answer = fake(&["2", "0"])
        .answer("Pick one.", time::Duration::from_secs(10))
        .await;
    assert_eq!(answer.unwrap().trim(), "2 0");
    let failed = fake(&["--fail", "usage", "limit"])
        .answer("Pick one.", time::Duration::from_secs(10))
        .await;
    assert!(failed.unwrap_err().ends_with("usage limit"));
    let missing = botc_bridge::Command {
        program: "no-such-storyteller-anywhere".to_string(),
        args: Vec::new(),
    };
    assert!(
        missing
            .answer("Pick one.", time::Duration::from_secs(10))
            .await
            .is_err()
    );
}

#[test]
fn the_server_address_becomes_the_bridge_socket() {
    assert_eq!(
        botc_bridge::endpoint("https://botc.kiblitz.xyz/"),
        "wss://botc.kiblitz.xyz/ws/bridge"
    );
    assert_eq!(
        botc_bridge::endpoint("http://127.0.0.1:8081"),
        "ws://127.0.0.1:8081/ws/bridge"
    );
    assert_eq!(
        botc_bridge::endpoint("wss://botc.kiblitz.xyz"),
        "wss://botc.kiblitz.xyz/ws/bridge"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_pairing_code_is_refused() {
    let base = common::serve(common::quick(), 5).await;
    let mut quiet = |_: &str| {};
    let ended = botc_bridge::serve(
        &base,
        "AAAA-BBBB-CCCC-DDDD",
        &fake(&[]),
        time::Duration::from_secs(5),
        &mut quiet,
    )
    .await;
    assert!(matches!(ended, botc_bridge::Ended::Refused(_)), "{ended:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_paired_bridge_makes_the_storytellers_decisions() {
    let base = common::serve(common::quick(), 3).await;
    let (_room, phones, _) = common::table(&base, &PLAYERS).await;
    // Enough option numbers for any decision: the server takes the first
    // `count` legal ones.
    let picks: Vec<String> = (0..20).map(|i| i.to_string()).collect();
    let picks: Vec<&str> = picks.iter().map(String::as_str).collect();
    pair(&base, &phones[0], fake(&picks)).await;
    // Nobody but the host is told the bridge's code.
    common::until(&phones[1], time::Duration::from_secs(5), |v| {
        v["bridge"]["connected"] == true
    })
    .await;
    assert!(phones[1].view()["bridge"].get("pairing").is_none());
    phones[0].send(json!({"type": "start"}));
    let (over, played) = common::play(&phones, time::Duration::from_secs(120)).await;
    assert!(over["winner"].is_string(), "{over:#}");
    assert_eq!(played.leaks, Vec::<String>::new());
    assert!(over["bridge"]["bridged"].as_u64().unwrap() > 0, "{over:#}");
    assert_eq!(over["bridge"]["fallbacks"], 0, "{over:#}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failing_command_falls_back_to_random_and_the_game_goes_on() {
    let base = common::serve(common::quick(), 3).await;
    let (_room, phones, _) = common::table(&base, &PLAYERS).await;
    pair(
        &base,
        &phones[0],
        fake(&["--fail", "You've hit your usage limit."]),
    )
    .await;
    phones[0].send(json!({"type": "start"}));
    let (over, _) = common::play(&phones, time::Duration::from_secs(120)).await;
    assert!(over["winner"].is_string(), "{over:#}");
    assert_eq!(over["bridge"]["bridged"], 0);
    assert!(over["bridge"]["fallbacks"].as_u64().unwrap() > 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_pairing_code_drops_the_old_bridge() {
    let base = common::serve(common::quick(), 3).await;
    let (_room, phones, _) = common::table(&base, &PLAYERS).await;
    common::until(&phones[0], time::Duration::from_secs(5), |v| {
        v["bridge"]["pairing"].is_string()
    })
    .await;
    let code = phones[0].view()["bridge"]["pairing"]
        .as_str()
        .unwrap()
        .to_string();
    let serving = {
        let (base, code) = (base.clone(), code.clone());
        tokio::spawn(async move {
            let mut quiet = |_: &str| {};
            botc_bridge::serve(
                &base,
                &code,
                &fake(&[]),
                time::Duration::from_secs(5),
                &mut quiet,
            )
            .await
        })
    };
    common::until(&phones[0], time::Duration::from_secs(5), |v| {
        v["bridge"]["connected"] == true
    })
    .await;
    phones[0].send(json!({"type": "new_pairing"}));
    let ended = tokio::time::timeout(time::Duration::from_secs(5), serving)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(ended, botc_bridge::Ended::Refused(_)), "{ended:?}");
    common::until(&phones[0], time::Duration::from_secs(5), |v| {
        v["bridge"]["connected"] == false && v["bridge"]["pairing"].as_str() != Some(code.as_str())
    })
    .await;
}

#[test]
fn the_bridge_reaches_a_secure_server_over_tls_without_panicking() {
    // A TLS server that is not a botc server: the handshake needs a crypto
    // provider, then the upgrade fails and the bridge says it was refused
    // or dropped, and would reconnect; it must not crash.
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_botc-bridge"))
        .args([
            "AAAA-BBBB-CCCC-DDDD",
            "--server",
            "https://example.com",
            "--",
            FAKE,
        ])
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(time::Duration::from_secs(4));
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(
        stderr.contains("disconnected") || stderr.contains("refused"),
        "{stderr}"
    );
}
