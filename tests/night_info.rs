//! The night-information Townsfolk: Balloonist, Chambermaid, Clockmaker,
//! Dreamer, Grandmother, Mathematician, Oracle, Seamstress, Shugenja, Knight,
//! General (the Noble is in `new_characters.rs`).

mod common;

use expect_test::expect;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::event;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;
use botc_narrator::setup;
use botc_narrator::sim;

use common::p;

fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend([
        &characters::balloonist::Def as character::Ref,
        &characters::chambermaid::Def,
        &characters::clockmaker::Def,
        &characters::dreamer::Def,
        &characters::grandmother::Def,
        &characters::mathematician::Def,
        &characters::oracle::Def,
        &characters::seamstress::Def,
        &characters::shugenja::Def,
        &characters::knight::Def,
        &characters::general::Def,
    ]);
    script::Script::new("night_info", "Night info", chars)
}

/// A seated game with the script recorded and its setup abilities run.
fn game(seats: &[(&str, character::Ref)]) -> common::Game {
    let mut g = common::Game::new(seats);
    g.run(|ctx| narrator::setup(ctx, &script())).quiet();
    g
}

/// Move to night `n` (from setup or an earlier phase).
fn to_night(ctx: &mut ctx::Write<'_>, n: u32) {
    while ctx.state().phase() != game::Phase::Night(n) {
        ctx.advance_phase();
    }
}

/// `c` acts on night `night`.
fn act(c: character::Ref, night: u32) -> impl FnOnce(&mut ctx::Write<'_>) {
    move |ctx| {
        to_night(ctx, night);
        narrator::run_order(ctx, &[c], night);
    }
}

/// Poison `target` until the game ends.
fn poison(ctx: &mut ctx::Write<'_>, target: &str) {
    poison_until(ctx, target, game::Expiry::Never);
}

fn poison_until(ctx: &mut ctx::Write<'_>, target: &str, expiry: game::Expiry) {
    let reminder = game::Reminder::new(
        &characters::poisoner::Def,
        &p("Nobody"),
        game::Effect::Poisoned,
        expiry,
    );
    ctx.place_reminder(&p(target), reminder);
}

// --- Balloonist ---------------------------------------------------------------

fn balloonist_game() -> common::Game {
    game(&[
        ("Alice", &characters::balloonist::Def),
        ("Bob", &characters::chef::Def),
        ("Cara", &characters::butler::Def),
        ("Dan", &characters::poisoner::Def),
        ("Eve", &characters::imp::Def),
    ])
}

#[test]
fn balloonist_learns_a_different_type_each_night() {
    let mut g = balloonist_game();
    g.run(act(&characters::balloonist::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Alice
          storyteller [1 of 4 options] information for Alice (Balloonist): "Bob"
          -> Alice: Bob
        sleep Alice
        * Alice (balloonist) marks Bob: Know: Townsfolk
        --- grimoire ---
        Alice: Balloonist
        Bob: Chef [Know: Townsfolk by balloonist]
        Cara: Butler
        Dan: Poisoner
        Eve: Imp
    "#]]
    .assert_eq(&g.transcript());

    // Night 2: no Townsfolk; the storyteller shows the Butler.
    g.run(act(&characters::balloonist::Def, 2));
    expect![[r#"
        * === Day 1 ===
        * === Night 2 ===
        wake Alice
          storyteller [1 of 3 options] information for Alice (Balloonist): "Cara"
          -> Alice: Cara
        sleep Alice
        * Alice (balloonist) removes Bob's Know: Townsfolk token
        * Alice (balloonist) marks Cara: Know: Outsider
        --- grimoire ---
        Alice: Balloonist
        Bob: Chef
        Cara: Butler [Know: Outsider by balloonist]
        Dan: Poisoner
        Eve: Imp
    "#]]
    .assert_eq(&g.transcript());

    assert_eq!(
        character::Def::setup_counts(&characters::balloonist::Def, setup::Counts::base(7)),
        vec![
            setup::Counts::base(7),
            setup::Counts::base(7).shift_outsiders(1)
        ]
    );
}

#[test]
fn poisoned_balloonist_may_see_the_same_type_and_then_must_differ_from_it() {
    let mut g = balloonist_game();
    g.run(act(&characters::balloonist::Def, 1)).quiet();
    // Night 2, poisoned: the Chef again is legal (the fourth option).
    g.picks(&[&[3]])
        .run(|ctx| {
            to_night(ctx, 2);
            poison_until(ctx, "Alice", game::Expiry::Dusk);
        })
        .quiet()
        .run(act(&characters::balloonist::Def, 2));
    let log = g.transcript();
    assert!(log.contains("[1 of 4 options]"), "{log}");
    assert!(log.contains("-> Alice: Bob"), "{log}");
    // Night 3, healthy: not a Townsfolk, since a Townsfolk was shown.
    g.run(act(&characters::balloonist::Def, 3));
    let log = g.transcript();
    assert!(log.contains("[1 of 3 options]"), "{log}");
    assert!(log.contains("-> Alice: Cara"), "{log}");
}

// --- Chambermaid --------------------------------------------------------------

fn chambermaid_game() -> common::Game {
    game(&[
        ("Imp", &characters::imp::Def),
        ("Maid", &characters::chambermaid::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Seam", &characters::seamstress::Def),
    ])
}

#[test]
fn chambermaid_counts_who_woke_for_their_ability() {
    // Night 2: the Imp wakes to kill; the Chef only woke on night 1.
    let mut g = chambermaid_game();
    g.answers(&[&["Imp", "Chef"]])
        .run(act(&characters::chambermaid::Def, 2));
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Maid
          Maid? Choose 2 players: => Imp, Chef
          storyteller [1 of 1 options] information for Maid (Chambermaid): "1 woke"
          -> Maid: 1 woke
        sleep Maid
        --- grimoire ---
        Imp: Imp
        Maid: Chambermaid
        Chef: Chef
        Monk: Monk
        Seam: Seamstress
    "#]]
    .assert_eq(&g.transcript());

    // A Seamstress who used her ability tonight woke; one who used it on an
    // earlier night does not wake again.
    let mut g = chambermaid_game();
    g.answers(&[&["Imp", "Monk"]])
        .run(act(&characters::seamstress::Def, 1))
        .answers(&[&["Seam", "Monk"]])
        .run(act(&characters::chambermaid::Def, 1));
    assert!(g.transcript().contains("-> Maid: 1 woke"));
    g.answers(&[&["Seam", "Chef"]])
        .run(act(&characters::chambermaid::Def, 2));
    assert!(g.transcript().contains("-> Maid: 0 woke"));
}

#[test]
fn poisoned_chambermaid_may_get_any_number() {
    let mut g = chambermaid_game();
    g.run(|ctx| poison(ctx, "Maid"))
        .answers(&[&["Imp", "Chef"]])
        .picks(&[&[2]])
        .run(act(&characters::chambermaid::Def, 2));
    let log = g.transcript();
    assert!(log.contains("[1 of 3 options]"), "{log}");
    assert!(log.contains("-> Maid: 2 woke"), "{log}");
}

// --- Clockmaker ---------------------------------------------------------------

#[test]
fn clockmaker_counts_steps_from_the_demon_to_the_nearest_minion() {
    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Clock", &characters::clockmaker::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.run(act(&characters::clockmaker::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Clock
          storyteller [1 of 1 options] information for Clock (Clockmaker): "3 steps"
          -> Clock: 3 steps
        sleep Clock
        --- grimoire ---
        Imp: Imp
        Clock: Clockmaker
        Chef: Chef
        Baron: Baron
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn clockmaker_sees_a_recluse_registering_as_a_minion() {
    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Clock", &characters::clockmaker::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Recluse", &characters::recluse::Def),
    ]);
    // The Recluse (next to the Imp) registers as the Poisoner to the Minion
    // question: 1 step. Registration asks: not the Demon, then a Minion.
    g.picks(&[&[0], &[1]])
        .run(act(&characters::clockmaker::Def, 1));
    let log = g.transcript();
    assert!(log.contains("-> Clock: 1 steps"), "{log}");

    // Registering as both the Demon and a Minion, the Recluse is not 0 steps
    // from itself.
    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Clock", &characters::clockmaker::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Recluse", &characters::recluse::Def),
    ]);
    g.picks(&[&[1], &[1]])
        .run(act(&characters::clockmaker::Def, 1));
    let log = g.transcript();
    assert!(log.contains("-> Clock: 1 steps"), "{log}");

    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Clock", &characters::clockmaker::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.run(|ctx| poison(ctx, "Clock"))
        .picks(&[&[1]])
        .run(act(&characters::clockmaker::Def, 1));
    let log = g.transcript();
    assert!(log.contains("-> Clock: 0 steps"), "{log}");
}

// --- Dreamer ------------------------------------------------------------------

fn dreamer_game() -> common::Game {
    game(&[
        ("Imp", &characters::imp::Def),
        ("Dream", &characters::dreamer::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
    ])
}

#[test]
fn dreamer_learns_one_good_and_one_evil_character() {
    let mut g = dreamer_game();
    g.answers(&[&["Chef"]])
        .picks(&[&[2]])
        .run(act(&characters::dreamer::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Dream
          Dream? Whose character do you want to dream? => Chef
          storyteller [1 of 5 options] information for Dream (Dreamer): "Chef, Scarlet Woman"
          -> Dream: Chef, Scarlet Woman
        sleep Dream
        --- grimoire ---
        Imp: Imp
        Dream: Dreamer
        Chef: Chef
        Baron: Baron
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // An evil player: their character with any good one.
    g.answers(&[&["Imp"]])
        .run(act(&characters::dreamer::Def, 2));
    let log = g.transcript();
    assert!(log.contains("-> Dream: Washerwoman, Imp"), "{log}");
}

#[test]
fn poisoned_dreamer_may_see_two_wrong_characters() {
    let mut g = dreamer_game();
    g.run(|ctx| poison(ctx, "Dream"))
        .answers(&[&["Chef"]])
        .picks(&[&[5]])
        .run(act(&characters::dreamer::Def, 1));
    let log = g.transcript();
    assert!(log.contains("-> Dream: Washerwoman, Poisoner"), "{log}");
}

// --- Grandmother --------------------------------------------------------------

fn grandmother_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Gran", &characters::grandmother::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.picks(&[&[0]]).run(|ctx| narrator::setup(ctx, &script()));
    g
}

#[test]
fn grandmother_knows_her_grandchild_and_dies_when_the_demon_kills_them() {
    let mut g = grandmother_game();
    g.run(act(&characters::grandmother::Def, 1));
    expect![[r#"
          storyteller [1 of 2 options] Grandmother Gran: who is the Grandchild?: Chef
        * Gran (grandmother) marks Chef: Grandchild
        * === Night 1 ===
        wake Gran
          storyteller [1 of 1 options] information for Gran (Grandmother): "Chef is the Chef"
          -> Gran: Chef is the Chef
        sleep Gran
        --- grimoire ---
        Imp: Imp
        Gran: Grandmother
        Chef: Chef [Grandchild by grandmother]
        Baron: Baron
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    g.answers(&[&["Chef"]]).run(act(&characters::imp::Def, 2));
    expect![[r#"
        * === Day 1 ===
        * === Night 2 ===
        wake Imp
          Imp? Who do you want to kill? => Chef
        sleep Imp
        * Chef dies (Demon)
        * Gran dies (Ability { by: Id("Gran") })
        --- grimoire ---
        Imp: Imp
        Gran: Grandmother [dead]
        Chef: Chef [dead] [Grandchild by grandmother]
        Baron: Baron
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn grandmother_survives_other_deaths_and_when_poisoned() {
    let mut g = grandmother_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
    });
    assert!(g.state.is_alive(&p("Gran")));

    let mut g = grandmother_game();
    g.run(|ctx| {
        to_night(ctx, 2);
        poison(ctx, "Gran");
        ctx.kill(&p("Chef"), event::Cause::Demon);
    });
    assert!(g.state.is_alive(&p("Gran")));
}

// --- Mathematician ------------------------------------------------------------

#[test]
fn mathematician_learns_how_many_abilities_malfunctioned() {
    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Math", &characters::mathematician::Def),
        ("Chef", &characters::chef::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        to_night(ctx, 1);
        poison(ctx, "Chef");
    })
    .quiet()
    .picks(&[&[1]])
    .run(act(&characters::mathematician::Def, 1));
    expect![[r#"
        wake Math
          storyteller [1 of 2 options] information for Math (Mathematician): "1 abnormal"
          -> Math: 1 abnormal
        sleep Math
        --- grimoire ---
        Imp: Imp
        Math: Mathematician
        Chef: Chef [Poisoned by poisoner]
        Poisoner: Poisoner
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned itself, any number is legal.
    g.run(|ctx| poison(ctx, "Math"))
        .quiet()
        .picks(&[&[4]])
        .run(act(&characters::mathematician::Def, 2));
    assert!(g.transcript().contains("-> Math: 4 abnormal"));
}

// --- Oracle -------------------------------------------------------------------

#[test]
fn oracle_learns_how_many_dead_are_evil() {
    let mut g = game(&[
        ("Imp", &characters::imp::Def),
        ("Oracle", &characters::oracle::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    // The Oracle does not wake on the first night.
    assert!(!script().first_night.iter().any(|c| c.id() == "oracle"));
    g.run(|ctx| {
        to_night(ctx, 2);
        ctx.execute(&p("Baron"));
        ctx.kill(&p("Chef"), event::Cause::Demon);
    })
    .quiet()
    .run(act(&characters::oracle::Def, 2));
    expect![[r#"
        wake Oracle
          storyteller [1 of 1 options] information for Oracle (Oracle): "1 dead evil"
          -> Oracle: 1 dead evil
        sleep Oracle
        --- grimoire ---
        Imp: Imp
        Oracle: Oracle
        Chef: Chef [dead]
        Baron: Baron [dead]
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    g.run(|ctx| {
        to_night(ctx, 3);
        poison(ctx, "Oracle");
    })
    .quiet()
    .picks(&[&[2]])
    .run(act(&characters::oracle::Def, 3));
    assert!(g.transcript().contains("-> Oracle: 2 dead evil"));
}

// --- Seamstress ---------------------------------------------------------------

fn seamstress_game() -> common::Game {
    game(&[
        ("Imp", &characters::imp::Def),
        ("Seam", &characters::seamstress::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
    ])
}

#[test]
fn seamstress_may_pass_then_uses_her_ability_once() {
    let mut g = seamstress_game();
    g.answers(&[&["Seam", "Chef"]])
        .run(act(&characters::seamstress::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Seam
          Seam? Choose 2 players (or yourself, to pass): => Seam, Chef
        sleep Seam
        --- grimoire ---
        Imp: Imp
        Seam: Seamstress
        Chef: Chef
        Baron: Baron
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Two evil players are the same alignment.
    g.answers(&[&["Imp", "Baron"]])
        .run(act(&characters::seamstress::Def, 2));
    expect![[r#"
        * === Day 1 ===
        * === Night 2 ===
        wake Seam
          Seam? Choose 2 players (or yourself, to pass): => Imp, Baron
        * Seam (seamstress) marks Seam: AbilityUsed
          storyteller [1 of 1 options] information for Seam (Seamstress): "Yes"
          -> Seam: Yes
        sleep Seam
        --- grimoire ---
        Imp: Imp
        Seam: Seamstress [AbilityUsed by seamstress]
        Chef: Chef
        Baron: Baron
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    g.run(act(&characters::seamstress::Def, 3));
    assert!(!g.transcript().contains("wake Seam"));
}

#[test]
fn poisoned_seamstress_uses_her_ability_and_may_be_told_either_answer() {
    let mut g = seamstress_game();
    g.run(|ctx| poison(ctx, "Seam"))
        .answers(&[&["Imp", "Chef"]])
        .picks(&[&[1]])
        .run(act(&characters::seamstress::Def, 1));
    let log = g.transcript();
    assert!(log.contains("-> Seam: Yes"), "{log}");
    assert!(log.contains("[AbilityUsed by seamstress]"), "{log}");
}

// --- Shugenja -----------------------------------------------------------------

#[test]
fn shugenja_learns_the_direction_of_the_closest_evil_player() {
    // Clockwise: Chef, Imp (2 steps). Anticlockwise: Mayor, Monk, Baron (3).
    let mut g = game(&[
        ("Shu", &characters::shugenja::Def),
        ("Chef", &characters::chef::Def),
        ("Imp", &characters::imp::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.run(act(&characters::shugenja::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Shu
          storyteller [1 of 1 options] information for Shu (Shugenja): "Clockwise"
          -> Shu: Clockwise
        sleep Shu
        --- grimoire ---
        Shu: Shugenja
        Chef: Chef
        Imp: Imp
        Baron: Baron
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // Equidistant: the storyteller picks either.
    let mut g = game(&[
        ("Shu", &characters::shugenja::Def),
        ("Chef", &characters::chef::Def),
        ("Imp", &characters::imp::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.picks(&[&[1]]).run(act(&characters::shugenja::Def, 1));
    let log = g.transcript();
    assert!(log.contains("[1 of 2 options]"), "{log}");
    assert!(log.contains("-> Shu: Anticlockwise"), "{log}");
}

#[test]
fn poisoned_shugenja_may_point_the_wrong_way() {
    let mut g = game(&[
        ("Shu", &characters::shugenja::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| poison(ctx, "Shu"))
        .picks(&[&[1]])
        .run(act(&characters::shugenja::Def, 1));
    assert!(g.transcript().contains("-> Shu: Anticlockwise"));
}

// --- Knight -------------------------------------------------------------------

fn knight_game() -> common::Game {
    game(&[
        ("Imp", &characters::imp::Def),
        ("Knight", &characters::knight::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
    ])
}

#[test]
fn knight_learns_two_players_that_are_not_the_demon() {
    let mut g = knight_game();
    g.run(act(&characters::knight::Def, 1));
    expect![[r#"
        * === Night 1 ===
        wake Knight
          storyteller [1 of 1 options] information for Knight (Knight): "Chef, Baron: 0 demons"
          -> Knight: Chef, Baron: 0 demons
        sleep Knight
        --- grimoire ---
        Imp: Imp
        Knight: Knight
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());

    let mut g = knight_game();
    g.run(|ctx| poison(ctx, "Knight"))
        .picks(&[&[1]])
        .run(act(&characters::knight::Def, 1));
    assert!(g.transcript().contains("-> Knight: Imp, Chef: 0 demons"));
}

// --- General ------------------------------------------------------------------

#[test]
fn general_learns_who_the_storyteller_thinks_is_winning() {
    let seats: [(&str, character::Ref); 4] = [
        ("Imp", &characters::imp::Def),
        ("General", &characters::general::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
    ];
    let mut g = game(&seats);
    g.picks(&[&[2]]).run(act(&characters::general::Def, 1));
    expect![[r#"
        * === Night 1 ===
          storyteller [1 of 3 options] General General: which team do you believe is winning?: "Evil is winning"
        wake General
          storyteller [1 of 1 options] information for General (General): "Evil is winning"
          -> General: Evil is winning
        sleep General
        --- grimoire ---
        Imp: Imp
        General: General
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());

    let mut g = game(&seats);
    g.run(|ctx| poison(ctx, "General"))
        .picks(&[&[0], &[1]])
        .run(act(&characters::general::Def, 1));
    assert!(g.transcript().contains("-> General: Good is winning"));
}

// --- Simulation ---------------------------------------------------------------

#[test]
fn simulated_games_with_these_characters_end() {
    let script = script();
    for seed in 0..40 {
        for players in [5, 8, 12] {
            let trace = sim::simulate(seed, players, &script);
            assert!(
                trace.winner.is_some(),
                "seed {seed}, {players} players: no winner"
            );
        }
    }
}
