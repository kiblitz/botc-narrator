mod common;

use rand::SeedableRng;

use botc_narrator::character;
use botc_narrator::script;
use botc_narrator::sim;

fn simulate(seed: u64, players: usize) -> sim::Trace {
    sim::simulate(seed, players, &common::trouble_brewing())
}

fn simulate_json(seed: u64, players: usize) -> String {
    sim::simulate_json(seed, players, "trouble_brewing").expect("known script")
}

fn counts(seats: &[(botc_narrator::player::Id, character::Ref)]) -> [usize; 4] {
    let mut counts = [0; 4];
    for (_, c) in seats {
        counts[c.kind() as usize] += 1;
    }
    counts
}

#[test]
fn setups_follow_trouble_brewing_counts() {
    let expected = [
        (5, [3, 0, 1, 1]),
        (6, [3, 1, 1, 1]),
        (7, [5, 0, 1, 1]),
        (8, [5, 1, 1, 1]),
        (9, [5, 2, 1, 1]),
        (10, [7, 0, 2, 1]),
        (12, [7, 2, 2, 1]),
        (13, [9, 0, 3, 1]),
        (15, [9, 2, 3, 1]),
    ];
    let mut drunk_seen = false;
    for seed in 0..40 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for (players, [t, o, m, d]) in expected {
            let seats = sim::setup(&mut rng, players, &common::trouble_brewing());
            assert_eq!(seats.len(), players);
            let has_baron = seats.iter().any(|(_, c)| c.id() == "baron");
            let shift = if has_baron { 2 } else { 0 };
            assert_eq!(
                counts(&seats),
                [t - shift, o + shift, m, d],
                "{players} players, seed {seed}"
            );
            let mut ids: Vec<&str> = seats.iter().map(|(_, c)| c.id()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), players, "duplicate character");
            drunk_seen |= ids.contains(&"drunk");
        }
    }
    assert!(drunk_seen, "the Drunk is part of setups");
}

#[test]
fn simulated_games_are_well_formed_and_deterministic() {
    let mut finished = 0;
    for seed in 0..60 {
        let players = sim::MIN_PLAYERS + (seed as usize % 11);
        let trace = simulate(seed, players);
        assert_eq!(trace.players.len(), players);
        assert!(!trace.steps.is_empty());
        for step in &trace.steps {
            assert!(step.snapshot < trace.snapshots.len());
            if matches!(
                step.kind,
                sim::Kind::Tell | sim::Kind::Ask | sim::Kind::Wake | sim::Kind::Sleep
            ) {
                assert!(step.player.is_some(), "{step:?}");
            }
        }
        if trace.winner.is_some() {
            finished += 1;
        }
        assert_eq!(simulate_json(seed, players), simulate_json(seed, players));
    }
    assert!(
        finished >= 50,
        "only {finished} of 60 games reached a winner"
    );
}

#[test]
fn every_death_is_announced_publicly() {
    for seed in 0..60 {
        let trace = simulate(seed, sim::MIN_PLAYERS + (seed as usize % 11));
        let final_seats = trace.snapshots.last().expect("snapshots");
        for seat in final_seats.iter().filter(|s| !s.alive) {
            let announced = trace.steps.iter().any(|s| {
                s.kind == sim::Kind::Death
                    && matches!(s.audience, sim::Audience::Public)
                    && s.player.as_deref() == Some(seat.name.as_str())
            });
            assert!(announced, "seed {seed}: {} never announced dead", seat.name);
        }
    }
}

#[test]
fn demon_bluffs_are_recorded_with_the_step_they_were_told() {
    // 7+ players always get demon info on night 1.
    let trace = simulate(7, 8);
    let bluffs = trace.bluffs.expect("bluffs");
    assert_eq!(bluffs.characters.len(), 3);
    let step = &trace.steps[bluffs.step];
    assert_eq!(step.kind, sim::Kind::Tell);
    assert_eq!(step.player.as_deref(), Some(bluffs.demon.as_str()));
    assert!(step.text.starts_with("Bluffs: "));
    // Too few players: no demon info, so no bluffs.
    assert!(simulate(7, 5).bluffs.is_none());
}

#[test]
fn vote_steps_name_the_nominee_and_voters() {
    let trace = simulate(7, 8);
    let votes: Vec<_> = trace
        .steps
        .iter()
        .filter(|s| s.kind == sim::Kind::Vote)
        .collect();
    assert!(!votes.is_empty());
    for step in votes {
        assert!(step.player.is_some());
        let count: usize = step.text.split(' ').next().unwrap().parse().unwrap();
        assert_eq!(step.voters.len(), count, "{}", step.text);
        let alive = &trace.snapshots[step.snapshot];
        for voter in &step.voters {
            assert!(
                alive.iter().any(|s| &s.name == voter && s.alive),
                "{voter} cannot vote"
            );
        }
    }
}

#[test]
fn information_steps_carry_the_players_and_characters_they_name() {
    let trace = simulate(36, 8);
    // Ada (Washerwoman) learns "Gus and Cleo: one is the Empath".
    let step = trace
        .steps
        .iter()
        .find(|s| s.kind == sim::Kind::Tell && s.text.contains("one is the Empath"))
        .expect("washerwoman info");
    assert_eq!(step.players, ["Gus", "Cleo"]);
    assert_eq!(step.characters, ["Empath"]);
    // A choice names the chosen players.
    let ask = trace
        .steps
        .iter()
        .find(|s| s.kind == sim::Kind::Ask)
        .expect("an ask");
    assert!(!ask.players.is_empty());
    for name in &ask.players {
        assert!(ask.text.contains(name.as_str()));
    }
}

#[test]
fn scripts_are_listed_and_unknown_ones_rejected() {
    let listed: Vec<serde_json::Value> = serde_json::from_str(&sim::scripts_json()).unwrap();
    assert_eq!(listed.len(), script::all().len());
    assert_eq!(listed[0]["id"], "trouble_brewing");
    assert_eq!(listed[0]["name"], "Trouble Brewing");
    assert!(sim::simulate_json(1, 8, "no_such_script").is_err());
    let trace: serde_json::Value = serde_json::from_str(&simulate_json(1, 8)).unwrap();
    assert_eq!(trace["script"], "trouble_brewing");
}

#[test]
fn communication_steps_say_which_way_they_go() {
    let trace = simulate(36, 8);
    for step in &trace.steps {
        let me = || sim::Endpoint::Player(step.player.clone().unwrap());
        match step.kind {
            sim::Kind::Tell => {
                let flow = step.flow.as_ref().expect("tell has a flow");
                assert_eq!((&flow.from, &flow.to), (&sim::Endpoint::Storyteller, &me()));
            }
            sim::Kind::Ask => {
                let flow = step.flow.as_ref().expect("ask has a flow");
                assert_eq!((&flow.from, &flow.to), (&me(), &sim::Endpoint::Storyteller));
            }
            sim::Kind::Wake | sim::Kind::Sleep | sim::Kind::Decision => {
                assert!(step.flow.is_none())
            }
            _ => {}
        }
    }
    let nomination = trace
        .steps
        .iter()
        .find(|s| s.text == "Ada nominates Bram")
        .expect("nomination");
    let flow = nomination.flow.as_ref().expect("nomination flow");
    assert_eq!(flow.from, sim::Endpoint::Player("Ada".into()));
    assert_eq!(flow.to, sim::Endpoint::Player("Bram".into()));
}

#[test]
fn decisions_carry_the_players_and_characters_they_chose() {
    let trace = simulate(36, 8);
    let step = trace
        .steps
        .iter()
        .find(|s| s.kind == sim::Kind::Decision && s.text.contains("(Investigator)"))
        .expect("investigator decision");
    // "A and B: one is the X": both players and the character are carried.
    assert_eq!(step.players.len(), 2, "{}", step.text);
    assert_eq!(step.characters.len(), 1, "{}", step.text);
    for name in &step.players {
        assert!(step.text.contains(name.as_str()), "{}", step.text);
    }
    assert!(step.text.contains(step.characters[0]), "{}", step.text);
    let bluffs = trace
        .steps
        .iter()
        .find(|s| s.kind == sim::Kind::Decision && s.text.starts_with("demon bluffs"))
        .expect("bluffs decision");
    assert_eq!(bluffs.characters.len(), 3);
}

/// Trouble Brewing's evil team with every Outsider the engine has.
fn outsiders_script() -> script::Script {
    let tb = common::trouble_brewing();
    let mut chars: Vec<character::Ref> = tb
        .characters
        .iter()
        .copied()
        .filter(|c| c.kind() != character::Kind::Outsider)
        .collect();
    chars.extend(
        botc_narrator::characters::all()
            .iter()
            .copied()
            .filter(|c| c.kind() == character::Kind::Outsider),
    );
    script::Script::new("outsiders", "Outsiders", chars)
}

#[test]
fn games_with_every_outsider_terminate_and_are_well_formed() {
    let script = outsiders_script();
    let mut seen: Vec<&'static str> = Vec::new();
    let mut finished = 0;
    for seed in 0..120 {
        let players = sim::MIN_PLAYERS + (seed as usize % 11);
        let trace = sim::simulate(seed, players, &script);
        for step in &trace.steps {
            assert!(step.snapshot < trace.snapshots.len());
        }
        if let Some(bluffs) = &trace.bluffs {
            let demon = trace.players.iter().find(|s| s.name == bluffs.demon);
            assert_eq!(demon.map(|s| s.kind), Some(character::Kind::Demon));
        }
        for seat in &trace.players {
            if !seen.contains(&seat.character) {
                seen.push(seat.character);
            }
        }
        finished += usize::from(trace.winner.is_some());
    }
    for c in script.characters.iter() {
        if c.kind() == character::Kind::Outsider {
            assert!(seen.contains(&c.name()), "{} never set up", c.name());
        }
    }
    assert!(finished >= 100, "only {finished} of 120 games finished");
}
