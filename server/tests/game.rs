//! Whole games over real sockets: a host and players on bot phones, the
//! engine as storyteller with the random fallback (no bridge).

mod common;

use std::time;

use serde_json::json;

const PLAYERS: [&str; 5] = ["Ada", "Bram", "Cleo", "Dev", "Esme"];

#[tokio::test(flavor = "multi_thread")]
async fn a_table_of_phones_plays_to_a_winner_and_nobody_sees_more_than_they_should() {
    for seed in [1, 2, 3] {
        let base = common::serve(common::quick(), seed).await;
        let (_room, phones, _) = common::table(&base, &PLAYERS).await;
        common::until(&phones[0], time::Duration::from_secs(5), |v| {
            v["seats"].as_array().is_some_and(|s| s.len() == 6)
        })
        .await;
        phones[0].send(json!({"type": "start"}));
        let (over, played) = common::play(&phones, time::Duration::from_secs(120)).await;
        assert!(over["winner"].is_string(), "seed {seed}: {over:#}");
        assert!(over["failure"].is_null(), "seed {seed}: {over:#}");
        assert!(played.prompts_answered > 0, "seed {seed}: {played:?}");
        assert!(
            played.nominations > 0 && played.votes > 0,
            "seed {seed}: {played:?}"
        );
        assert_eq!(played.leaks, Vec::<String>::new(), "seed {seed}");
        // Once it is over, the grimoire is revealed to everyone.
        for seat in over["seats"].as_array().unwrap() {
            assert!(seat["character"].is_string(), "{seat}");
        }
        let storyteller = over["log"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["audience"]["kind"] == "storyteller")
            .count();
        assert!(storyteller > 0);
        assert_eq!(over["bridge"]["bridged"], 0);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn anyone_can_host_or_join_without_an_account() {
    let base = common::serve(common::quick(), 7).await;
    let http = common::http();
    let hosted = http
        .post(format!("{base}/api/rooms"))
        .json(&json!({"name": "Nobody"}))
        .send()
        .await
        .unwrap();
    assert_eq!(hosted.status(), 200);
    let body: serde_json::Value = hosted.json().await.unwrap();
    let host = common::connect(
        &base,
        body["room"].as_str().unwrap(),
        "Nobody",
        body["token"].as_str().unwrap(),
    )
    .await;
    common::until(&host, time::Duration::from_secs(5), |v| v["host"] == true).await;

    let (room, _phones, _) = common::table(&base, &["Ada"]).await;
    let taken = http
        .post(format!("{base}/api/rooms/{room}/join"))
        .json(&json!({"name": "ada"}))
        .send()
        .await
        .unwrap();
    assert_eq!(taken.status(), 409);
    let missing = http
        .post(format!("{base}/api/rooms/QQQQQ/join"))
        .json(&json!({"name": "Bram"}))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_host_seats_everyone_and_nobody_else_can() {
    let base = common::serve(common::quick(), 7).await;
    let (room, phones, _) = common::table(&base, &PLAYERS).await;
    common::until(&phones[0], time::Duration::from_secs(5), |v| {
        v["seats"].as_array().is_some_and(|s| s.len() == 6)
    })
    .await;
    let order = ["Esme", "Host", "Dev", "Ada", "Cleo", "Bram"];
    phones[0].send(json!({"type": "reseat", "order": order}));
    let seated = |v: &serde_json::Value| -> Vec<String> {
        v["seats"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|s| s["name"].as_str().unwrap().to_string())
            .collect()
    };
    common::until(&phones[3], time::Duration::from_secs(5), |v| {
        seated(v) == order
    })
    .await;
    // A player who is not the host cannot.
    phones[1]
        .send(json!({"type": "reseat", "order": ["Host", "Ada", "Bram", "Cleo", "Dev", "Esme"]}));
    phones[1].send(json!({"type": "start"}));
    tokio::time::sleep(time::Duration::from_millis(300)).await;
    assert_eq!(seated(&phones[0].view()), order);
    assert_eq!(phones[0].view()["stage"], "lobby");
    assert!(
        phones[1]
            .errors
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.contains("Only the host"))
    );
    // Only the host sees the pairing code and the script list.
    assert!(phones[0].view()["bridge"]["pairing"].is_string());
    assert!(
        phones[0].view()["scripts"]
            .as_array()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(phones[1].view()["bridge"].get("pairing").is_none());
    assert!(phones[1].view().get("scripts").is_none());

    phones[0].send(json!({"type": "start"}));
    common::until(&phones[1], time::Duration::from_secs(5), |v| {
        v["me"].is_object()
    })
    .await;
    // The game seats players as the host arranged them, and the lobby is shut.
    assert_eq!(seated(&phones[1].view()), order);
    let late = common::http()
        .post(format!("{base}/api/rooms/{room}/join"))
        .json(&json!({"name": "Late"}))
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), 409);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_phone_that_reconnects_keeps_its_seat_and_character() {
    let base = common::serve(common::quick(), 11).await;
    let (room, phones, tokens) = common::table(&base, &PLAYERS).await;
    common::until(&phones[0], time::Duration::from_secs(5), |v| {
        v["seats"].as_array().is_some_and(|s| s.len() == 6)
    })
    .await;
    phones[0].send(json!({"type": "start"}));
    common::until(&phones[2], time::Duration::from_secs(5), |v| {
        v["me"].is_object()
    })
    .await;
    let character = phones[2].view()["me"]["character"].clone();
    let again = common::connect(&base, &room, "Bram", &tokens[2]).await;
    common::until(&again, time::Duration::from_secs(5), |v| {
        v["me"].is_object()
    })
    .await;
    assert_eq!(again.view()["you"], "Bram");
    assert_eq!(again.view()["me"]["character"], character);

    let stranger = common::connect(&base, &room, "Nobody", "not-a-token").await;
    tokio::time::sleep(time::Duration::from_millis(300)).await;
    assert!(stranger.view().is_null());
    assert!(
        stranger
            .errors
            .lock()
            .unwrap()
            .iter()
            .any(|e| e.contains("gone"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn guessing_codes_is_stopped_and_seats_take_few_phones() {
    let base = common::serve(common::quick(), 13).await;
    let (room, phones, tokens) = common::table(&base, &PLAYERS).await;
    // A fourth phone on one seat is turned away.
    let extra = [
        common::connect(&base, &room, "Ada", &tokens[1]).await,
        common::connect(&base, &room, "Ada", &tokens[1]).await,
        common::connect(&base, &room, "Ada", &tokens[1]).await,
    ];
    tokio::time::sleep(time::Duration::from_millis(300)).await;
    let refused = extra
        .iter()
        .filter(|p| {
            p.errors
                .lock()
                .unwrap()
                .iter()
                .any(|e| e.contains("Too many phones"))
        })
        .count();
    assert_eq!(refused, 1);
    // One client's wrong room codes run out, then even a right one waits;
    // other clients, and phones that already have a seat, are unaffected.
    let http = common::http();
    let join = |client: &'static str, code: String, name: &'static str| {
        http.post(format!("{base}/api/rooms/{code}/join"))
            .header("x-forwarded-for", client)
            .json(&json!({"name": name}))
            .send()
    };
    let mut statuses = Vec::new();
    for _ in 0..61 {
        statuses.push(
            join("203.0.113.9", "ZZZZZ".into(), "Guess")
                .await
                .unwrap()
                .status()
                .as_u16(),
        );
    }
    assert!(statuses[..60].iter().all(|s| *s == 404), "{statuses:?}");
    assert_eq!(statuses[60], 429);
    assert_eq!(
        join("203.0.113.9", room.clone(), "Late")
            .await
            .unwrap()
            .status(),
        429
    );
    assert_eq!(
        join("198.51.100.7", room.clone(), "Other")
            .await
            .unwrap()
            .status(),
        200
    );
    let back = common::connect(&base, &room, "Bram", &tokens[2]).await;
    common::until(&back, time::Duration::from_secs(5), |v| v["you"] == "Bram").await;
    drop(phones);
}

#[tokio::test(flavor = "multi_thread")]
async fn one_address_hosts_only_a_few_rooms() {
    let base = common::serve(common::quick(), 17).await;
    let host = |client: &'static str| {
        common::http()
            .post(format!("{base}/api/rooms"))
            .header("x-forwarded-for", client)
            .json(&json!({"name": "Host"}))
            .send()
    };
    for _ in 0..botc_server::room::MOST_ROOMS_PER_CLIENT {
        assert_eq!(host("203.0.113.9").await.unwrap().status(), 200);
    }
    let refused = host("203.0.113.9").await.unwrap();
    assert_eq!(refused.status(), 429);
    assert_eq!(host("198.51.100.7").await.unwrap().status(), 200);
    // An IPv6 machine owns its whole /64, so it is one client.
    for last in 1..=botc_server::room::MOST_ROOMS_PER_CLIENT {
        let address: &'static str = format!("2001:db8:1:2::{last}").leak();
        assert_eq!(host(address).await.unwrap().status(), 200);
    }
    assert_eq!(host("2001:db8:1:2::ff").await.unwrap().status(), 429);
    assert_eq!(host("2001:db8:1:3::1").await.unwrap().status(), 200);
    // An IPv4 address written as IPv6 is still that IPv4 client.
    assert_eq!(host("::ffff:203.0.113.9").await.unwrap().status(), 429);
}
