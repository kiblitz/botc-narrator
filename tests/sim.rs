use rand::SeedableRng;

use botc_narrator::character;
use botc_narrator::sim;

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
    for seed in 0..40 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for (players, [t, o, m, d]) in expected {
            let seats = sim::setup(&mut rng, players);
            assert_eq!(seats.len(), players);
            let has_baron = seats.iter().any(|(_, c)| c.id() == "baron");
            let shift = if has_baron { 2.min(3 - o) } else { 0 };
            assert_eq!(
                counts(&seats),
                [t - shift, o + shift, m, d],
                "{players} players, seed {seed}"
            );
            let mut ids: Vec<&str> = seats.iter().map(|(_, c)| c.id()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), players, "duplicate character");
            assert!(!ids.contains(&"drunk"));
        }
    }
}

#[test]
fn simulated_games_are_well_formed_and_deterministic() {
    let mut finished = 0;
    for seed in 0..60 {
        let players = sim::MIN_PLAYERS + (seed as usize % 11);
        let trace = sim::simulate(seed, players);
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
        assert_eq!(
            sim::simulate_json(seed, players),
            sim::simulate_json(seed, players)
        );
    }
    assert!(
        finished >= 50,
        "only {finished} of 60 games reached a winner"
    );
}

#[test]
fn every_death_is_announced_publicly() {
    for seed in 0..60 {
        let trace = sim::simulate(seed, sim::MIN_PLAYERS + (seed as usize % 11));
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
    let trace = sim::simulate(7, 8);
    let bluffs = trace.bluffs.expect("bluffs");
    assert_eq!(bluffs.characters.len(), 3);
    let step = &trace.steps[bluffs.step];
    assert_eq!(step.kind, sim::Kind::Tell);
    assert_eq!(step.player.as_deref(), Some(bluffs.demon.as_str()));
    assert!(step.text.starts_with("Bluffs: "));
    // Too few players: no demon info, so no bluffs.
    assert!(sim::simulate(7, 5).bluffs.is_none());
}
