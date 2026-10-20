//! Characters added with building blocks only (no engine edits): Noble,
//! Fang Gu, Cerenovus, Godfather.

mod common;

use expect_test::expect;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;
use botc_narrator::setup;

use common::p;

fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend([
        &characters::noble::Def as character::Ref,
        &characters::fang_gu::Def,
        &characters::cerenovus::Def,
        &characters::godfather::Def,
    ]);
    script::Script::new("test", "Test", chars)
}

fn night(ctx: &mut ctx::Write<'_>) {
    ctx.advance_phase();
    narrator::run_night(ctx, &script());
}

fn poison(ctx: &mut ctx::Write<'_>, target: &str) {
    let reminder = game::Reminder::new(
        &characters::poisoner::Def,
        &p("Nobody"),
        game::Effect::Poisoned,
        game::Expiry::Dusk,
    );
    ctx.place_reminder(&p(target), reminder);
}

#[test]
fn noble_learns_three_players_one_evil() {
    let seats: [(&str, character::Ref); 5] = [
        ("Imp", &characters::imp::Def),
        ("Noble", &characters::noble::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ];
    let mut g = common::Game::new(&seats);
    g.run(|ctx| narrator::run_order(ctx, &[&characters::noble::Def], 1));
    expect![[r#"
        wake Noble
          storyteller [1 of 3 options] information for Noble (Noble): "Imp, Chef, Monk: 1 evil"
          -> Noble: Imp, Chef, Monk: 1 evil
        sleep Noble
        --- grimoire ---
        Imp: Imp
        Noble: Noble
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Noble may see three good players.
    let mut g = common::Game::new(&seats);
    g.run(|ctx| poison(ctx, "Noble"))
        .quiet()
        .picks(&[&[3]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::noble::Def], 1));
    assert!(
        g.transcript()
            .contains("-> Noble: Chef, Monk, Mayor: 1 evil"),
        "{}",
        g.transcript()
    );
}

fn fang_gu_game() -> common::Game {
    common::Game::new(&[
        ("Fang", &characters::fang_gu::Def),
        ("Baron", &characters::baron::Def),
        ("Saint", &characters::saint::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn fang_gu_jumps_into_the_first_outsider_it_kills() {
    let mut g = fang_gu_game();
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet()
    .answers(&[&["Chef"], &["Saint"]])
    .run(night);
    expect![[r#"
        * === Night 2 ===
        wake Monk
          Monk? Who do you want to protect? => Chef
        * Monk (monk) marks Chef: Protected
        sleep Monk
        wake Fang
          Fang? Who do you want to kill? => Saint
        sleep Fang
        * Fang (fang_gu) marks Fang: AbilityUsed
        * Saint becomes the Fang Gu
        * Saint is now Evil
        * Saint (fang_gu) marks Saint: AbilityUsed
        * Fang dies (Ability { by: Id("Fang") })
        --- grimoire ---
        Fang: Fang Gu [dead] [AbilityUsed by fang_gu]
        Baron: Baron
        Saint: Fang Gu [AbilityUsed by fang_gu]
        Chef: Chef [Protected by monk]
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
    assert_eq!(g.state.winner(), None);
    assert_eq!(
        setup::Counts::base(7).shift_outsiders(1),
        character::Def::setup_counts(&characters::fang_gu::Def, setup::Counts::base(7))[0]
    );
}

#[test]
fn poisoned_fang_gu_neither_kills_nor_jumps() {
    let mut g = fang_gu_game();
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        poison(ctx, "Fang");
    })
    .quiet()
    .answers(&[&["Chef"], &["Saint"]])
    .run(|ctx| narrator::run_order(ctx, &[&characters::fang_gu::Def], 2));
    assert!(g.state.is_alive(&p("Saint")));
    assert!(g.state.is_alive(&p("Fang")));
}

fn cerenovus_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Ceren", &characters::cerenovus::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.run(|ctx| narrator::setup(ctx, &script())).quiet();
    g
}

#[test]
fn cerenovus_makes_a_player_mad_and_may_execute_them() {
    let mut g = cerenovus_game();
    g.answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::cerenovus::Def], 1));
    expect![[r#"
        wake Ceren
          Ceren? Who do you want to make mad? => Chef
        sleep Ceren
        * Ceren (cerenovus) marks Chef: Mad (Washerwoman)
        wake Chef
          -> Chef: Washerwoman
        sleep Chef
        --- grimoire ---
        Imp: Imp
        Ceren: Cerenovus
        Chef: Chef [Mad (Washerwoman) by cerenovus]
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // The Chef breaks madness; the storyteller has them executed ("yes").
    g.table.breaks_madness.push(p("Chef"));
    g.picks(&[&[1]]).run(narrator::check_madness);
    expect![[r#"
          Chef mad about Washerwoman: broke = true
        * Chef broke madness about the Washerwoman
          storyteller [1 of 2 options] Cerenovus Ceren: execute the mad player?: Yes
        * Chef is executed
        * Chef dies (Execution)
        --- grimoire ---
        Imp: Imp
        Ceren: Cerenovus
        Chef: Chef [dead] [Mad (Washerwoman) by cerenovus]
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_cerenovus_makes_nobody_mad() {
    let mut g = cerenovus_game();
    g.run(|ctx| poison(ctx, "Ceren"))
        .quiet()
        .answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::cerenovus::Def], 1));
    let log = g.transcript();
    assert!(!log.contains("Mad"), "{log}");
}

fn godfather_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("God", &characters::godfather::Def),
        ("Saint", &characters::saint::Def),
        ("Recluse", &characters::recluse::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
    })
    .quiet();
    g
}

#[test]
fn godfather_learns_outsiders_and_kills_after_an_outsider_dies() {
    let mut g = godfather_game();
    g.run(|ctx| narrator::run_order(ctx, &[&characters::godfather::Def], 1));
    assert!(
        g.transcript().contains("-> God: Saint, Recluse"),
        "{}",
        g.transcript()
    );

    // Night 2 after a quiet day: the Godfather sleeps.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet()
    .run(|ctx| narrator::run_order(ctx, &[&characters::godfather::Def], 2));
    assert!(!g.transcript().contains("wake God"));

    // An Outsider is executed on day 2: that night the Godfather kills.
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Recluse"));
        ctx.advance_phase();
    });
    g.quiet()
        .run(|ctx| narrator::run_order(ctx, &[&characters::godfather::Def], 3));
    expect![[r#"
        wake God
          God? Who do you want to kill? => Chef
        sleep God
        * Chef dies (Ability { by: Id("God") })
        --- grimoire ---
        Imp: Imp
        God: Godfather
        Saint: Saint
        Recluse: Recluse [dead]
        Chef: Chef [dead]
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
    let counts = character::Def::setup_counts(&characters::godfather::Def, setup::Counts::base(9));
    assert_eq!(counts.len(), 2);
}

#[test]
fn poisoned_godfather_may_learn_false_outsiders_and_kills_nobody() {
    let mut g = godfather_game();
    g.run(|ctx| poison(ctx, "God"))
        .quiet()
        .run(|ctx| narrator::run_order(ctx, &[&characters::godfather::Def], 1));
    assert!(g.transcript().contains("who is poisoned or drunk"));
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Recluse"));
        ctx.advance_phase();
        poison(ctx, "God");
        narrator::run_order(ctx, &[&characters::godfather::Def], 2);
    });
    assert!(g.state.is_alive(&p("Chef")));
}
