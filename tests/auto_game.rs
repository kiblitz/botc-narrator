//! The automated storyteller plays complete games to a winner, with no human
//! input at all — proving the engine + auto-ST + auto-setup form a self-running
//! game.

use botc_narrator::characters;
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::Grimoire;
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::{AutoStoryteller, Rng};
use botc_narrator::role::Alignment;
use botc_narrator::scripts::trouble_brewing;
use botc_narrator::setup;

/// Drive one fully-automated game to completion; return the winner (if reached
/// within the round cap) and the final grimoire.
fn play(seed: u64, player_count: usize) -> (Option<Alignment>, Grimoire) {
    let reg = characters::registry();
    let script = trouble_brewing::script();

    let names: Vec<String> = (0..player_count).map(|i| format!("P{i}")).collect();
    let mut setup_rng = Rng::new(seed);
    let specs = setup::random_trouble_brewing(&reg, &script, &names, &mut setup_rng);
    let grim = Grimoire::new(specs);

    let mut engine = Engine::new(grim, &reg, &script);
    let mut st = AutoStoryteller::random(seed);
    let mut day_rng = Rng::new(seed ^ 0xABCD);

    engine.run_setup(&mut st);

    for _round in 0..60 {
        engine.run_night(&mut st);
        if engine.grim.winner.is_some() {
            break;
        }
        engine.begin_day();

        // A crude but terminating day: each living player may nominate a random
        // other, and every living player votes yes with probability 1/2.
        let alive: Vec<PlayerId> = engine.grim.alive().collect();
        for &nominator in &alive {
            if engine.grim.winner.is_some() || !engine.grim.is_alive(nominator) {
                break;
            }
            if day_rng.below(2) == 0 {
                continue;
            }
            let others: Vec<PlayerId> = engine.grim.alive().filter(|&p| p != nominator).collect();
            if others.is_empty() {
                continue;
            }
            let nominee = others[day_rng.below(others.len())];
            let voters: Vec<PlayerId> = engine
                .grim
                .alive()
                .filter(|_| day_rng.below(2) == 0)
                .collect();
            engine.call_vote(&mut st, nominator, nominee, &voters);
        }
        engine.resolve_day(&mut st);
        if engine.grim.winner.is_some() {
            break;
        }
    }

    (engine.grim.winner, engine.grim)
}

#[test]
fn automated_games_always_terminate_with_a_winner() {
    let mut good = 0;
    let mut evil = 0;
    for seed in 0..40u64 {
        let players = 5 + (seed as usize % 11); // 5..=15
        let (winner, _grim) = play(seed, players);
        match winner {
            Some(Alignment::Good) => good += 1,
            Some(Alignment::Evil) => evil += 1,
            None => panic!("seed {seed} ({players} players) did not resolve to a winner"),
        }
    }
    // Every game terminates; random play should land some of each.
    assert!(
        good > 0 && evil > 0,
        "expected a mix of outcomes: {good} good / {evil} evil"
    );
}

#[test]
fn a_seeded_game_is_reproducible() {
    let (w1, g1) = play(7, 8);
    let (w2, g2) = play(7, 8);
    assert_eq!(w1, w2);
    assert_eq!(g1.alive_count(), g2.alive_count());
}
