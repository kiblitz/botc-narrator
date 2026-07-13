//! Appendability proof: a brand-new character (Assassin) and a second script
//! (homebrew) work with zero changes to the engine or existing roles.

use botc_narrator::characters::{self, Assassin, Empath, Imp, Monk, Soldier, Washerwoman};
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::{Grimoire, Phase, PlayerSpec};
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::ScriptedStoryteller;
use botc_narrator::role::CharacterInfo;
use botc_narrator::scripts::homebrew;
use botc_narrator::token::Token;

fn game(specs: Vec<(&str, CharacterInfo)>) -> Grimoire {
    Grimoire::new(
        specs
            .into_iter()
            .map(|(n, i)| PlayerSpec::new(n, i))
            .collect(),
    )
}

fn at_night(engine: &mut Engine, night: u32) {
    engine.grim.advance_phase();
    for _ in 1..night {
        engine.grim.advance_phase();
        engine.grim.advance_phase();
    }
    assert_eq!(engine.grim.phase, Phase::Night(night));
}

#[test]
fn assassin_kills_through_soldier_immunity() {
    let reg = characters::registry();
    let script = homebrew::script();
    let grim = game(vec![
        ("Ada", Assassin::INFO),
        ("Sarge", Soldier::INFO),
        ("Diablo", Imp::INFO),
        ("Troi", Empath::INFO),
        ("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 2);

    // The Assassin targets the Soldier. The Soldier is immune to the *demon*,
    // but the Assassin's kill is unpreventable — it uses the same pipeline the
    // Soldier's immunity plugs into, and simply isn't a demon kill.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(1)]);
    engine.act(&mut st, Assassin::ID, 2);

    assert!(
        !engine.grim.is_alive(PlayerId(1)),
        "the Assassin kills through Soldier immunity"
    );
    assert!(engine.grim.has_token(PlayerId(0), Token::UsedAbility));
}

#[test]
fn assassin_is_once_per_game() {
    let reg = characters::registry();
    let script = homebrew::script();
    let grim = game(vec![
        ("Ada", Assassin::INFO),
        ("Sarge", Soldier::INFO),
        ("Diablo", Imp::INFO),
        ("Troi", Empath::INFO),
        ("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 2);

    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(3)]);
    engine.act(&mut st, Assassin::ID, 2);
    assert!(!engine.grim.is_alive(PlayerId(3)));

    // Next night: the ability is spent, so the Assassin never wakes.
    engine.grim.advance_phase(); // -> Day 2
    engine.grim.advance_phase(); // -> Night 3
    let mut st2 = ScriptedStoryteller::new().with_responses([PlayerId(4)]);
    engine.act(&mut st2, Assassin::ID, 3);
    assert!(
        engine.grim.is_alive(PlayerId(4)),
        "second kill must not happen"
    );
    assert!(
        st2.transcript().is_empty(),
        "the Assassin is not even woken"
    );
}

#[test]
fn homebrew_script_runs_a_first_night() {
    let reg = characters::registry();
    let script = homebrew::script();
    let grim = game(vec![
        ("Wanda", Washerwoman::INFO),
        ("Troi", Empath::INFO),
        ("Sam", characters::Slayer::INFO),
        ("Sarge", Soldier::INFO),
        ("Friar", Monk::INFO),
        ("Bea", characters::Butler::INFO),
        ("Rex", characters::Recluse::INFO),
        ("Ada", Assassin::INFO),
        ("Diablo", Imp::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);

    let mut st = ScriptedStoryteller::new();
    engine.run_setup(&mut st);
    engine.run_night(&mut st);

    let text = st.transcript_text();
    assert!(text.contains("Your evil team:"));
    assert!(text.contains("Bluffs:"));
    assert_eq!(engine.grim.alive_count(), 9);
}
