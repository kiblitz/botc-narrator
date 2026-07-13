//! Tests for the voting layer: majority thresholds, the Butler's master
//! constraint, ghost votes, and ties.

use botc_narrator::characters::{self, Butler, Empath, Imp, Poisoner, Slayer, Soldier};
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::{Grimoire, PlayerSpec};
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::ScriptedStoryteller;
use botc_narrator::role::CharacterInfo;
use botc_narrator::scripts::trouble_brewing;
use botc_narrator::token::Token;

fn game(specs: Vec<(&str, CharacterInfo)>) -> Grimoire {
    Grimoire::new(
        specs
            .into_iter()
            .map(|(n, i)| PlayerSpec::new(n, i))
            .collect(),
    )
}

#[test]
fn a_majority_puts_a_player_on_the_block_and_executes_them() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = game(vec![
        ("Ann", Slayer::INFO),
        ("Bob", Empath::INFO),
        ("Cara", Soldier::INFO),
        ("Dan", Imp::INFO),
        ("Eve", Poisoner::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine.grim.advance_phase(); // -> Night 1
    engine.begin_day(); // -> Day 1

    let mut st = ScriptedStoryteller::new();
    // 5 alive => threshold 3. Three players vote for the Imp.
    let r = engine.call_vote(
        &mut st,
        PlayerId(0),
        PlayerId(3),
        &[PlayerId(0), PlayerId(1), PlayerId(2)],
    );
    assert_eq!(r.threshold, 3);
    assert!(r.reached());
    assert_eq!(engine.on_the_block(), Some(PlayerId(3)));

    let executed = engine.resolve_day(&mut st);
    assert_eq!(executed, Some(PlayerId(3)));
    assert!(!engine.grim.is_alive(PlayerId(3)));
}

#[test]
fn below_threshold_no_execution() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = game(vec![
        ("Ann", Slayer::INFO),
        ("Bob", Empath::INFO),
        ("Cara", Soldier::INFO),
        ("Dan", Imp::INFO),
        ("Eve", Poisoner::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine.grim.advance_phase();
    engine.begin_day();

    let mut st = ScriptedStoryteller::new();
    let r = engine.call_vote(
        &mut st,
        PlayerId(0),
        PlayerId(3),
        &[PlayerId(0), PlayerId(1)],
    );
    assert!(!r.reached());
    assert_eq!(engine.on_the_block(), None);
    assert_eq!(engine.resolve_day(&mut st), None);
    assert!(engine.grim.is_alive(PlayerId(3)));
}

#[test]
fn butler_vote_only_counts_when_the_master_votes() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = game(vec![
        ("Ann", Slayer::INFO),
        ("Bea", Butler::INFO),
        ("Cara", Soldier::INFO),
        ("Dan", Imp::INFO),
        ("Eve", Poisoner::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine.grim.advance_phase();
    engine.begin_day();
    // Bea's master is Ann.
    engine
        .grim
        .add_token(PlayerId(1), Token::Master(PlayerId(0)));

    let mut st = ScriptedStoryteller::new();
    // Bea and Cara vote, but Ann (Bea's master) does not: Bea's vote is void,
    // so only Cara counts -> 1 of 3 needed.
    let r = engine.call_vote(
        &mut st,
        PlayerId(2),
        PlayerId(3),
        &[PlayerId(1), PlayerId(2)],
    );
    assert_eq!(r.valid_voters, vec![PlayerId(2)]);
    assert!(!r.reached());

    // Now Ann votes too: Bea's vote counts, reaching the threshold.
    let r = engine.call_vote(
        &mut st,
        PlayerId(2),
        PlayerId(3),
        &[PlayerId(0), PlayerId(1), PlayerId(2)],
    );
    assert_eq!(r.votes, 3);
    assert!(r.reached());
}

#[test]
fn a_ghost_vote_can_be_spent_once() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = game(vec![
        ("Ann", Slayer::INFO),
        ("Bob", Empath::INFO),
        ("Cara", Soldier::INFO),
        ("Dan", Imp::INFO),
        ("Eve", Poisoner::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine.grim.advance_phase();
    engine.begin_day();
    // Kill Ann; she keeps a single ghost vote.
    engine.grim.kill(PlayerId(0));
    assert!(engine.grim.has_ghost_vote(PlayerId(0)));

    let mut st = ScriptedStoryteller::new();
    // Ann's ghost vote counts once.
    let r = engine.call_vote(&mut st, PlayerId(1), PlayerId(3), &[PlayerId(0)]);
    assert_eq!(r.valid_voters, vec![PlayerId(0)]);
    assert!(!engine.grim.has_ghost_vote(PlayerId(0)), "ghost vote spent");

    // A later nomination: Ann can no longer vote.
    let r = engine.call_vote(&mut st, PlayerId(1), PlayerId(4), &[PlayerId(0)]);
    assert!(r.valid_voters.is_empty());
}

#[test]
fn a_tie_executes_no_one() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = game(vec![
        ("Ann", Slayer::INFO),
        ("Bob", Empath::INFO),
        ("Cara", Soldier::INFO),
        ("Dan", Imp::INFO),
        ("Eve", Poisoner::INFO),
        ("Fay", Butler::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine.grim.advance_phase();
    engine.begin_day();

    let mut st = ScriptedStoryteller::new();
    // 6 alive => threshold 3. Two separate nominees each get exactly 3 votes.
    engine.call_vote(
        &mut st,
        PlayerId(0),
        PlayerId(3),
        &[PlayerId(0), PlayerId(1), PlayerId(2)],
    );
    assert_eq!(engine.on_the_block(), Some(PlayerId(3)));
    engine.call_vote(
        &mut st,
        PlayerId(1),
        PlayerId(4),
        &[PlayerId(3), PlayerId(4), PlayerId(5)],
    );
    // Tied at 3 apiece: no one is on the block.
    assert_eq!(engine.on_the_block(), None);
    assert_eq!(engine.resolve_day(&mut st), None);
}
