//! Scenario tests against the full Trouble Brewing script: triggered day
//! abilities, death-triggered info, misregistration, the Drunk, and win
//! conditions.

use botc_narrator::characters::{
    self, Drunk, Empath, Imp, Investigator, Monk, Poisoner, Ravenkeeper, Recluse, Saint, Slayer,
    Soldier, Spy, Undertaker, Virgin, Washerwoman,
};
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::{Grimoire, Phase, PlayerSpec};
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::ScriptedStoryteller;
use botc_narrator::registry::Registry;
use botc_narrator::role::{Alignment, CharacterInfo};
use botc_narrator::script::Script;
use botc_narrator::scripts::trouble_brewing;

fn tb() -> (Registry, Script) {
    (characters::registry(), trouble_brewing::script())
}

fn build(specs: Vec<PlayerSpec>) -> Grimoire {
    Grimoire::new(specs)
}

fn spec(name: &str, info: CharacterInfo) -> PlayerSpec {
    PlayerSpec::new(name, info)
}

fn at_night<'a>(engine: &mut Engine<'a>, night: u32) {
    engine.grim.advance_phase();
    for _ in 1..night {
        engine.grim.advance_phase();
        engine.grim.advance_phase();
    }
    assert_eq!(engine.grim.phase, Phase::Night(night));
}

fn at_day<'a>(engine: &mut Engine<'a>, day: u32) {
    at_night(engine, day);
    engine.grim.advance_phase();
    assert_eq!(engine.grim.phase, Phase::Day(day));
}

#[test]
fn virgin_executes_a_townsfolk_nominator() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Vera", Virgin::INFO),
        spec("Wanda", Washerwoman::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_day(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.nominate(&mut st, PlayerId(1), PlayerId(0)); // Washerwoman nominates Virgin

    assert!(!engine.grim.is_alive(PlayerId(1)), "the nominator dies");

    // A second nomination does nothing — the ability is spent.
    engine.nominate(&mut st, PlayerId(3), PlayerId(0));
    assert!(engine.grim.is_alive(PlayerId(3)));
}

#[test]
fn slayer_kills_the_demon_but_not_a_townsfolk() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Sam", Slayer::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_day(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    // Shoot a non-demon first: nothing happens, but the shot is spent.
    engine.day_ability(&mut st, PlayerId(0), PlayerId(2));
    assert!(engine.grim.is_alive(PlayerId(2)));
    // The (already used) Slayer cannot now kill the demon.
    engine.day_ability(&mut st, PlayerId(0), PlayerId(1));
    assert!(engine.grim.is_alive(PlayerId(1)));
}

#[test]
fn slayer_slays_the_demon() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Sam", Slayer::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_day(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.day_ability(&mut st, PlayerId(0), PlayerId(1));
    assert!(!engine.grim.is_alive(PlayerId(1)), "the demon dies");
    assert_eq!(engine.grim.winner, Some(Alignment::Good));
}

#[test]
fn ravenkeeper_learns_a_role_on_dying_at_night() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Diablo", Imp::INFO),
        spec("Ren", Ravenkeeper::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 2);

    // Imp kills the Ravenkeeper; the Ravenkeeper then looks at the Imp.
    let mut st = ScriptedStoryteller::new().with_responses([PlayerId(1), PlayerId(0)]);
    engine.act(&mut st, Imp::ID, 2);

    assert!(!engine.grim.is_alive(PlayerId(1)));
    assert_eq!(
        st.last_reveal_to("Ren(Ravenkeeper)"),
        Some("Diablo is the Imp")
    );
}

#[test]
fn undertaker_learns_the_executed_role() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Uma", Undertaker::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
        spec("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_day(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.execute(&mut st, PlayerId(2)); // execute the Poisoner
    engine.grim.advance_phase(); // -> Night(2)
    assert_eq!(engine.grim.phase, Phase::Night(2));
    engine.act(&mut st, Undertaker::ID, 2);

    assert_eq!(
        st.last_reveal_to("Uma(Undertaker)"),
        Some("The Poisoner was executed today")
    );
}

#[test]
fn recluse_can_register_as_evil_to_the_empath() {
    let (reg, script) = tb();
    // Seat order: Empath | Recluse (right) ... Soldier (left).
    let grim = build(vec![
        spec("Troi", Empath::INFO),
        spec("Rex", Recluse::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 1);

    // Storyteller decides the Recluse reads evil this time (choice index 0).
    let mut st = ScriptedStoryteller::new().with_choices([0]);
    engine.act(&mut st, Empath::ID, 1);
    assert_eq!(
        st.last_reveal_to("Troi(Empath)"),
        Some("1 of your neighbours are evil")
    );

    // A second game where the storyteller hides the Recluse (choice index 1).
    let grim = build(vec![
        spec("Troi", Empath::INFO),
        spec("Rex", Recluse::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 1);
    let mut st = ScriptedStoryteller::new().with_choices([1]);
    engine.act(&mut st, Empath::ID, 1);
    assert_eq!(
        st.last_reveal_to("Troi(Empath)"),
        Some("0 of your neighbours are evil")
    );
}

#[test]
fn drunk_runs_a_townsfolk_ability_but_gets_false_info() {
    let (reg, script) = tb();
    // Bob is the Drunk who believes he is the Empath, seated between two evils.
    let grim = build(vec![
        spec("Ivy", Poisoner::INFO),
        PlayerSpec::new("Bob", Drunk::INFO).believing(Empath::INFO),
        spec("Diablo", Imp::INFO),
        spec("Sarge", Soldier::INFO),
        spec("Friar", Monk::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 1);

    // The engine wakes Bob in the Empath slot; his true count is 2, but being
    // the Drunk he is impaired and the storyteller shows a false 0.
    let mut st = ScriptedStoryteller::new().with_choices([0]);
    engine.act(&mut st, Empath::ID, 1);

    // The storyteller sees Bob's true identity; the false info still lands.
    assert_eq!(
        st.last_reveal_to("Bob(Drunk)"),
        Some("0 of your neighbours are evil")
    );
}

#[test]
fn saint_executed_loses_the_game_for_good() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Sol", Saint::INFO),
        spec("Diablo", Imp::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_day(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.execute(&mut st, PlayerId(0));
    assert_eq!(engine.grim.winner, Some(Alignment::Evil));
}

#[test]
fn spy_sees_the_whole_grimoire() {
    let (reg, script) = tb();
    let grim = build(vec![
        spec("Esp", Spy::INFO),
        spec("Diablo", Imp::INFO),
        spec("Troi", Empath::INFO),
        spec("Sarge", Soldier::INFO),
        spec("Ivy", Poisoner::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);
    at_night(&mut engine, 1);

    let mut st = ScriptedStoryteller::new();
    engine.act(&mut st, Spy::ID, 1);
    let seen = st.last_reveal_to("Esp(Spy)").unwrap();
    assert!(seen.contains("Diablo(Imp)"));
    assert!(seen.contains("Troi(Empath)"));
}

#[test]
fn full_first_night_runs_with_default_choices() {
    let (reg, script) = tb();
    // A legal 12-player game touching most first-night roles.
    let grim = build(vec![
        spec("Wanda", Washerwoman::INFO),
        spec("Lore", characters::Librarian::INFO),
        spec("Ivan", Investigator::INFO),
        spec("Cora", characters::Chef::INFO),
        spec("Troi", Empath::INFO),
        spec("Zoe", characters::FortuneTeller::INFO),
        spec("Uma", Undertaker::INFO),
        spec("Bea", characters::Butler::INFO),
        spec("Friar", Monk::INFO),
        spec("Ivy", Poisoner::INFO),
        spec("Esp", Spy::INFO),
        spec("Diablo", Imp::INFO),
    ]);
    let mut engine = Engine::new(grim, &reg, &script);

    let mut st = ScriptedStoryteller::new();
    engine.run_setup(&mut st);
    engine.run_night(&mut st);

    let text = st.transcript_text();
    assert!(text.contains("Bluffs:"), "demon should get bluffs:\n{text}");
    assert!(text.contains("Your evil team:"), "minions get their team");
    // Nobody died on the first night.
    assert_eq!(engine.grim.alive_count(), 12);
}
