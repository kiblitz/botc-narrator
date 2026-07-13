//! Vertical-slice tests: prove the event pipeline resolves cross-role
//! interactions (Soldier/Monk immunity, poison, starpass, Scarlet Woman
//! promotion) without any role referencing another by name.

use botc_narrator::characters::{
    self, Empath, FortuneTeller, Imp, Monk, Poisoner, ScarletWoman, Soldier,
};
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::{Grimoire, PlayerSpec};
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::ScriptedStoryteller;
use botc_narrator::registry::Registry;
use botc_narrator::role::{CharacterInfo, Kind};
use botc_narrator::script::Script;

fn slice_script() -> Script {
    Script {
        id: "slice",
        name: "Slice",
        roster: vec![
            Poisoner::ID,
            ScarletWoman::ID,
            Imp::ID,
            Soldier::ID,
            Monk::ID,
            Empath::ID,
            FortuneTeller::ID,
        ],
        first_night: vec![Poisoner::ID, Empath::ID, FortuneTeller::ID],
        other_nights: vec![
            Poisoner::ID,
            Monk::ID,
            Imp::ID,
            Empath::ID,
            FortuneTeller::ID,
        ],
    }
}

fn game(specs: Vec<(&str, CharacterInfo)>) -> (Registry, Script, Grimoire) {
    let reg = characters::registry();
    let script = slice_script();
    let players = specs
        .into_iter()
        .map(|(name, info)| PlayerSpec::new(name, info))
        .collect();
    (reg, script, Grimoire::new(players))
}

/// Advance a fresh grimoire straight to a given night.
fn engine_at_night<'a>(engine: &mut Engine<'a>, night: u32) {
    engine.grim.advance_phase(); // Setup -> Night(1)
    for _ in 1..night {
        engine.grim.advance_phase(); // Night(k) -> Day(k)
        engine.grim.advance_phase(); // Day(k)   -> Night(k+1)
    }
    assert_eq!(
        engine.grim.phase,
        botc_narrator::grimoire::Phase::Night(night)
    );
}

#[test]
fn soldier_is_immune_to_the_demon() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Sarge", Soldier::INFO),
        ("Troi", Empath::INFO),
        ("Ivy", Poisoner::INFO),
        ("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 2);

    // The Imp targets the Soldier.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(1)]);
    engine.act(&mut st, Imp::ID, 2);

    assert!(engine.grim.is_alive(PlayerId(1)), "Soldier should survive");
    assert!(
        st.transcript_text().contains("immune"),
        "transcript should note immunity:\n{}",
        st.transcript_text()
    );
}

#[test]
fn poisoned_soldier_can_die() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Sarge", Soldier::INFO),
        ("Ivy", Poisoner::INFO),
        ("Troi", Empath::INFO),
        ("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 2);

    // Poisoner poisons the Soldier, then the Imp attacks the Soldier.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(1), PlayerId(1)]);
    engine.act(&mut st, Poisoner::ID, 2);
    engine.act(&mut st, Imp::ID, 2);

    assert!(
        !engine.grim.is_alive(PlayerId(1)),
        "poisoned Soldier should die"
    );
}

#[test]
fn monk_protects_against_the_demon() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Friar", Monk::INFO),
        ("Troi", Empath::INFO),
        ("Ivy", Poisoner::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 2);

    // Monk protects the Empath; the Imp then attacks the Empath.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(2), PlayerId(2)]);
    engine.act(&mut st, Monk::ID, 2);
    engine.act(&mut st, Imp::ID, 2);

    assert!(
        engine.grim.is_alive(PlayerId(2)),
        "protected Empath should survive"
    );
    assert!(st.transcript_text().contains("protected"));
}

#[test]
fn poisoned_monk_protects_no_one() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Friar", Monk::INFO),
        ("Troi", Empath::INFO),
        ("Ivy", Poisoner::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 2);

    // Poisoner poisons the Monk; the Monk's protection is now inert.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(1), PlayerId(2), PlayerId(2)]);
    engine.act(&mut st, Poisoner::ID, 2); // poison Monk
    engine.act(&mut st, Monk::ID, 2); // "protect" Empath (fails)
    engine.act(&mut st, Imp::ID, 2); // kill Empath

    assert!(
        !engine.grim.is_alive(PlayerId(2)),
        "a poisoned Monk cannot protect"
    );
}

#[test]
fn imp_starpasses_to_a_minion() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Ivy", Poisoner::INFO),
        ("Troi", Empath::INFO),
        ("Friar", Monk::INFO),
        ("Sarge", Soldier::INFO),
        ("Vex", ScarletWoman::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 2);

    // Imp kills itself; storyteller passes the Imp to the Poisoner (choice 0).
    let mut st = ScriptedStoryteller::new()
        .with_responses([PlayerId(0)])
        .with_choices([0]);
    engine.act(&mut st, Imp::ID, 2);

    assert!(!engine.grim.is_alive(PlayerId(0)), "old Imp should be dead");
    assert_eq!(engine.grim.get(PlayerId(1)).role, Imp::ID);
    assert_eq!(engine.grim.get(PlayerId(1)).kind, Kind::Demon);
    // The Scarlet Woman must NOT also have been promoted — a Demon already lives.
    assert_eq!(engine.grim.get(PlayerId(5)).role, ScarletWoman::ID);
}

#[test]
fn scarlet_woman_catches_the_executed_demon() {
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Vex", ScarletWoman::INFO),
        ("Troi", Empath::INFO),
        ("Friar", Monk::INFO),
        ("Sarge", Soldier::INFO),
        ("Zoe", FortuneTeller::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 1);
    engine.grim.advance_phase(); // -> Day(1)

    // Execute the Imp with 6 alive (5 remain) — Scarlet Woman promotes.
    let mut st = ScriptedStoryteller::new();
    engine.execute(&mut st, PlayerId(0));

    assert!(!engine.grim.is_alive(PlayerId(0)));
    assert_eq!(engine.grim.get(PlayerId(1)).role, Imp::ID);
    assert_eq!(engine.grim.get(PlayerId(1)).kind, Kind::Demon);
}

#[test]
fn scarlet_woman_does_not_promote_below_threshold() {
    // Only 4 players: after the Imp dies, 3 remain — no promotion.
    let (reg, script, grim) = game(vec![
        ("Diablo", Imp::INFO),
        ("Vex", ScarletWoman::INFO),
        ("Troi", Empath::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 1);
    engine.grim.advance_phase(); // -> Day(1)

    let mut st = ScriptedStoryteller::new();
    engine.execute(&mut st, PlayerId(0));

    assert_eq!(
        engine.grim.get(PlayerId(1)).role,
        ScarletWoman::ID,
        "should stay a Scarlet Woman below the threshold"
    );
}

#[test]
fn empath_counts_evil_neighbours() {
    // Seat order puts the Empath between the Poisoner and the Imp.
    let (reg, script, grim) = game(vec![
        ("Ivy", Poisoner::INFO),
        ("Troi", Empath::INFO),
        ("Diablo", Imp::INFO),
        ("Friar", Monk::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.act(&mut st, Empath::ID, 1);

    assert_eq!(
        st.last_reveal_to("Troi(Empath)"),
        Some("2 of your neighbours are evil")
    );
}

#[test]
fn poisoned_empath_gets_false_info() {
    let (reg, script, grim) = game(vec![
        ("Ivy", Poisoner::INFO),
        ("Troi", Empath::INFO),
        ("Diablo", Imp::INFO),
        ("Friar", Monk::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    engine_at_night(&mut engine, 1);

    // Poison the Empath, then the storyteller shows a false "0".
    let mut st = ScriptedStoryteller::new()
        .with_responses([PlayerId(1)]) // poison the Empath
        .with_choices([0]); // false-info index 0 -> value 0
    engine.act(&mut st, Poisoner::ID, 1);
    engine.act(&mut st, Empath::ID, 1);

    assert_eq!(
        st.last_reveal_to("Troi(Empath)"),
        Some("0 of your neighbours are evil")
    );
}

#[test]
fn fortune_teller_pings_the_red_herring() {
    let (reg, script, grim) = game(vec![
        ("Zoe", FortuneTeller::INFO),
        ("Troi", Empath::INFO),
        ("Diablo", Imp::INFO),
        ("Ivy", Poisoner::INFO),
        ("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);

    // Setup: storyteller marks the Empath (index 1 among the good players) as
    // the red herring.
    let mut st = ScriptedStoryteller::new().with_choices([1]);
    engine.run_setup(&mut st);
    engine_at_night(&mut engine, 1);

    // Fortune Teller checks the Empath (red herring) and the Soldier -> "Yes".
    st.push_responses([PlayerId(1), PlayerId(4)]);
    engine.act(&mut st, FortuneTeller::ID, 1);

    assert_eq!(st.last_reveal_to("Zoe(Fortune Teller)"), Some("Yes"));
}
