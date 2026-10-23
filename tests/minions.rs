//! The Minions: Assassin, Boomdandy, Devil's Advocate, Fearmonger, Goblin,
//! Harpy, Marionette, Mastermind, Mezepheles, Pit-Hag, Psychopath, Widow and
//! Witch (the Cerenovus and Godfather are in `new_characters.rs`).

mod common;

use expect_test::expect;
use rand::SeedableRng;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;
use botc_narrator::sim;

use common::p;

const MINIONS: [character::Ref; 15] = [
    &characters::assassin::Def,
    &characters::boomdandy::Def,
    &characters::cerenovus::Def,
    &characters::devils_advocate::Def,
    &characters::fearmonger::Def,
    &characters::goblin::Def,
    &characters::godfather::Def,
    &characters::harpy::Def,
    &characters::marionette::Def,
    &characters::mastermind::Def,
    &characters::mezepheles::Def,
    &characters::pit_hag::Def,
    &characters::psychopath::Def,
    &characters::widow::Def,
    &characters::witch::Def,
];

/// Trouble Brewing with every Minion.
fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend(MINIONS);
    script::Script::new("minions", "Minions", chars)
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

/// Advance to `phase`.
fn to(ctx: &mut ctx::Write<'_>, phase: game::Phase) {
    while ctx.state().phase() != phase {
        ctx.advance_phase();
    }
}

/// A game with the script set up, at `phase`, with an empty log.
fn game(seats: &[(&str, character::Ref)], phase: game::Phase) -> common::Game {
    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        to(ctx, phase);
    })
    .quiet();
    g
}

fn alive(g: &common::Game, name: &str) -> bool {
    g.state.is_alive(&p(name))
}

// --- Assassin -------------------------------------------------------------------

fn assassin_game() -> common::Game {
    game(
        &[
            ("Assassin", &characters::assassin::Def),
            ("Imp", &characters::imp::Def),
            ("Soldier", &characters::soldier::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(2),
    )
}

#[test]
fn assassin_kills_through_protection_once() {
    let mut g = assassin_game();
    g.table.agrees.push(p("Assassin"));
    g.answers(&[&["Soldier"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::assassin::Def], 2));
    expect![[r#"
        wake Assassin
          Assassin? use the Assassin ability? => yes
          Assassin? Who do you want to assassinate? => Soldier
        sleep Assassin
        * Assassin (assassin) marks Assassin: AbilityUsed
        * Soldier dies (Ability { by: Id("Assassin") })
        --- grimoire ---
        Assassin: Assassin [AbilityUsed by assassin]
        Imp: Imp
        Soldier: Soldier [dead]
        Chef: Chef
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Monk protection does not stop it either; but the ability is spent.
    let mut g = assassin_game();
    g.table.agrees.push(p("Assassin"));
    g.answers(&[&["Chef"], &["Chef"]]).run(|ctx| {
        narrator::run_order(
            ctx,
            &[&characters::monk::Def, &characters::assassin::Def],
            2,
        )
    });
    assert!(!alive(&g, "Chef"), "{}", g.transcript());
    g.quiet().run(|ctx| {
        to(ctx, game::Phase::Night(3));
        narrator::run_order(ctx, &[&characters::assassin::Def], 3);
    });
    assert!(!g.transcript().contains("wake Assassin"));
}

#[test]
fn assassin_may_wait() {
    let mut g = assassin_game();
    g.run(|ctx| narrator::run_order(ctx, &[&characters::assassin::Def], 2));
    expect![[r#"
        wake Assassin
          Assassin? use the Assassin ability? => no
        sleep Assassin
        --- grimoire ---
        Assassin: Assassin
        Imp: Imp
        Soldier: Soldier
        Chef: Chef
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_assassin_spends_the_ability_for_nothing() {
    let mut g = assassin_game();
    g.table.agrees.push(p("Assassin"));
    g.run(|ctx| poison(ctx, "Assassin"))
        .answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::assassin::Def], 2));
    assert!(alive(&g, "Chef"));
    assert!(g.transcript().contains("[AbilityUsed by assassin]"));
}

// --- Boomdandy ------------------------------------------------------------------

fn boomdandy_game() -> common::Game {
    game(
        &[
            ("Boom", &characters::boomdandy::Def),
            ("Imp", &characters::imp::Def),
            ("Poisoner", &characters::poisoner::Def),
            ("Teller", &characters::fortune_teller::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    )
}

#[test]
fn executed_boomdandy_explodes_and_the_table_points() {
    let mut g = boomdandy_game();
    // The storyteller keeps the Poisoner and Fortune Teller with the Imp;
    // most fingers point at the Fortune Teller: two remain, evil wins.
    g.picks(&[&[0, 1]])
        .answers(&[
            &["Teller"],
            &["Teller"],
            &["Teller"],
            &["Imp"],
            &["Poisoner"],
            &["Teller"],
            &["Imp"],
        ])
        .run(|ctx| {
            ctx.execute(&p("Boom"));
        });
    expect![[r#"
        * Boom is executed
          storyteller [2 of 5 options] Boomdandy Boom: who survives the explosion?: Poisoner, Teller
        * Announcement: Boom explodes
        * Chef dies (Ability { by: Id("Boom") })
        * Empath dies (Ability { by: Id("Boom") })
        * Monk dies (Ability { by: Id("Boom") })
          Boom? Who do you point at? => Teller
          Imp? Who do you point at? => Teller
          Poisoner? Who do you point at? => Teller
          Teller? Who do you point at? => Imp
          Chef? Who do you point at? => Poisoner
          Empath? Who do you point at? => Teller
          Monk? Who do you point at? => Imp
        * Teller dies (Ability { by: Id("Boom") })
        * Boom dies (Execution)
        * Evil wins!
        --- grimoire ---
        Boom: Boomdandy [dead]
        Imp: Imp
        Poisoner: Poisoner
        Teller: Fortune Teller [dead] [red herring by fortune_teller]
        Chef: Chef [dead]
        Empath: Empath [dead]
        Monk: Monk [dead]
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn the_table_can_point_the_demon_to_death() {
    let mut g = boomdandy_game();
    g.answers(&[&["Imp"], &["Imp"], &["Imp"], &["Imp"]])
        .run(|ctx| {
            ctx.execute(&p("Boom"));
        });
    assert!(!alive(&g, "Imp"));
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn a_tie_kills_nobody_and_a_poisoned_boomdandy_just_dies() {
    let mut g = boomdandy_game();
    // Survivors: Imp, Poisoner, Teller; fingers split between Imp and Poisoner.
    g.answers(&[
        &["Imp"],
        &["Poisoner"],
        &["Imp"],
        &["Poisoner"],
        &["Imp"],
        &["Poisoner"],
        &["Teller"],
    ])
    .run(|ctx| {
        ctx.execute(&p("Boom"));
    });
    let living: Vec<String> = g
        .state
        .alive_ids()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(living, ["Imp", "Poisoner", "Teller"]);

    let mut g = boomdandy_game();
    g.run(|ctx| {
        poison(ctx, "Boom");
        ctx.execute(&p("Boom"));
    });
    assert_eq!(g.state.alive_ids().len(), 6);
    assert!(!g.transcript().contains("explodes"));
}

// --- Devil's Advocate -------------------------------------------------------------

fn advocate_game() -> common::Game {
    game(
        &[
            ("Advocate", &characters::devils_advocate::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(1),
    )
}

#[test]
fn devils_advocate_saves_the_chosen_player_from_execution() {
    let mut g = advocate_game();
    g.answers(&[&["Imp"]]).run(|ctx| {
        narrator::run_order(ctx, &[&characters::devils_advocate::Def], 1);
        to(ctx, game::Phase::Day(1));
        ctx.execute(&p("Imp"));
    });
    expect![[r#"
        wake Advocate
          Advocate? Who do you want to save from execution? => Imp
        sleep Advocate
        * Advocate (devils_advocate) marks Imp: Chosen
        * Advocate (devils_advocate) marks Imp: Survives execution
        * === Day 1 ===
        * Imp is executed
        * Imp survives (devils_advocate)
        --- grimoire ---
        Advocate: Devil's Advocate
        Imp: Imp [Chosen by devils_advocate] [Survives execution by devils_advocate]
        Chef: Chef
        Empath: Empath
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Night 2: the Imp is not a legal choice (the answer falls back to the
    // first legal one), and the protection has ended.
    g.answers(&[&["Imp"]]).run(|ctx| {
        to(ctx, game::Phase::Night(2));
        narrator::run_order(ctx, &[&characters::devils_advocate::Def], 2);
        to(ctx, game::Phase::Day(2));
        ctx.execute(&p("Imp"));
    });
    expect![[r#"
        * === Night 2 ===
        wake Advocate
          Advocate? Who do you want to save from execution? => Imp
        sleep Advocate
        * Advocate (devils_advocate) marks Advocate: Chosen
        * Advocate (devils_advocate) marks Advocate: Survives execution
        * === Day 2 ===
        * Imp is executed
        * Imp dies (Execution)
        * Good wins!
        --- grimoire ---
        Advocate: Devil's Advocate [Chosen by devils_advocate] [Survives execution by devils_advocate]
        Imp: Imp [dead]
        Chef: Chef
        Empath: Empath
        Monk: Monk
        winner: Good
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_devils_advocate_saves_nobody() {
    // Poisoned when choosing.
    let mut g = advocate_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Advocate");
        narrator::run_order(ctx, &[&characters::devils_advocate::Def], 1);
        to(ctx, game::Phase::Day(1));
        ctx.execute(&p("Chef"));
    });
    assert!(!alive(&g, "Chef"));

    // Poisoned at the execution.
    let mut g = advocate_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::run_order(ctx, &[&characters::devils_advocate::Def], 1);
        to(ctx, game::Phase::Day(1));
        poison(ctx, "Advocate");
        ctx.execute(&p("Chef"));
    });
    assert!(!alive(&g, "Chef"));
}

// --- Fearmonger -------------------------------------------------------------------

fn fearmonger_game() -> common::Game {
    game(
        &[
            ("Fear", &characters::fearmonger::Def),
            ("Imp", &characters::imp::Def),
            ("Poisoner", &characters::poisoner::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(1),
    )
}

fn fear_night(ctx: &mut ctx::Write<'_>, night: u32) {
    to(ctx, game::Phase::Night(night));
    narrator::run_order(ctx, &[&characters::fearmonger::Def], night);
}

#[test]
fn fearmonger_executing_their_target_makes_that_team_lose() {
    let mut g = fearmonger_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        fear_night(ctx, 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Fear"), &p("Chef"));
        ctx.execute(&p("Chef"));
    });
    expect![[r#"
        wake Fear
          Fear? Who do you want to frighten? => Chef
        sleep Fear
        * Fear (fearmonger) marks Chef: Fear
        * Announcement: The Fearmonger has chosen a new player
        * === Day 1 ===
        * Fear nominates Chef
        * Chef is executed
        * Evil wins!
        * Chef dies (Execution)
        --- grimoire ---
        Fear: Fearmonger
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef [dead] [Fear by fearmonger]
        Empath: Empath
        Monk: Monk
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());

    // An evil target: evil loses.
    let mut g = fearmonger_game();
    g.answers(&[&["Poisoner"]]).run(|ctx| {
        fear_night(ctx, 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Fear"), &p("Poisoner"));
        ctx.execute(&p("Poisoner"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn fearmonger_only_counts_their_own_nomination_of_the_current_target() {
    // Someone else nominates the target.
    let mut g = fearmonger_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        fear_night(ctx, 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Empath"), &p("Chef"));
        ctx.execute(&p("Chef"));
    });
    assert_eq!(g.state.winner(), None);

    // Only a new choice is announced; an earlier target no longer counts.
    let mut g = fearmonger_game();
    g.answers(&[&["Chef"], &["Empath"], &["Empath"]])
        .run(|ctx| {
            fear_night(ctx, 1);
            fear_night(ctx, 2);
            fear_night(ctx, 3);
        });
    let log = g.transcript();
    assert_eq!(log.matches("chosen a new player").count(), 2, "{log}");
    assert!(log.contains("Empath: Empath [Fear by fearmonger]"), "{log}");
    assert!(log.contains("Chef: Chef\n"), "{log}");
}

#[test]
fn a_poisoned_fearmonger_frightens_nobody() {
    let mut g = fearmonger_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Fear");
        fear_night(ctx, 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Fear"), &p("Chef"));
        ctx.execute(&p("Chef"));
    });
    assert_eq!(g.state.winner(), None);
}

// --- Goblin -----------------------------------------------------------------------

fn goblin_game() -> common::Game {
    game(
        &[
            ("Goblin", &characters::goblin::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    )
}

#[test]
fn a_goblin_who_claims_and_is_executed_wins_for_evil() {
    let mut g = goblin_game();
    g.table.agrees.push(p("Goblin"));
    g.run(|ctx| {
        narrator::nominate(ctx, &p("Chef"), &p("Goblin"));
        ctx.execute(&p("Goblin"));
    });
    expect![[r#"
        * Chef nominates Goblin
          Goblin? claim to be the Goblin? => yes
        * Announcement: Goblin claims to be the Goblin
        * Goblin (goblin) marks Goblin: Claimed
        * Goblin is executed
        * Evil wins!
        * Goblin dies (Execution)
        --- grimoire ---
        Goblin: Goblin [dead]
        Imp: Imp
        Chef: Chef
        Empath: Empath
        Monk: Monk
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_goblin_needs_a_claim_on_the_same_day_and_their_ability() {
    // No claim.
    let mut g = goblin_game();
    g.run(|ctx| {
        narrator::nominate(ctx, &p("Chef"), &p("Goblin"));
        ctx.execute(&p("Goblin"));
    });
    assert_eq!(g.state.winner(), None);

    // Claimed yesterday, executed today without claiming.
    let mut g = goblin_game();
    g.table.agrees.push(p("Goblin"));
    g.run(|ctx| narrator::nominate(ctx, &p("Chef"), &p("Goblin")));
    g.table.agrees.clear();
    g.run(|ctx| {
        to(ctx, game::Phase::Day(2));
        narrator::nominate(ctx, &p("Chef"), &p("Goblin"));
        ctx.execute(&p("Goblin"));
    });
    assert_eq!(g.state.winner(), None);

    // Poisoned.
    let mut g = goblin_game();
    g.table.agrees.push(p("Goblin"));
    g.run(|ctx| {
        poison(ctx, "Goblin");
        narrator::nominate(ctx, &p("Chef"), &p("Goblin"));
        ctx.execute(&p("Goblin"));
    });
    assert_eq!(g.state.winner(), None);
}

// --- Harpy ------------------------------------------------------------------------

fn harpy_game() -> common::Game {
    game(
        &[
            ("Harpy", &characters::harpy::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
        ],
        game::Phase::Night(1),
    )
}

#[test]
fn harpy_madness_broken_lets_the_storyteller_kill_both() {
    let mut g = harpy_game();
    g.answers(&[&["Chef"], &["Empath"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::harpy::Def], 1));
    expect![[r#"
        wake Harpy
          Harpy? Who do you want to make mad? => Chef
          Harpy? Mad that which player is evil? => Empath
        sleep Harpy
        * Harpy (harpy) marks Chef: Mad (2nd player being evil)
        * Harpy (harpy) marks Empath: 2nd
        wake Chef
          -> Chef: Evil: Empath
        sleep Chef
        --- grimoire ---
        Harpy: Harpy
        Imp: Imp
        Chef: Chef [Mad (2nd player being evil) by harpy]
        Empath: Empath [2nd by harpy]
        Monk: Monk
        Saint: Saint
    "#]]
    .assert_eq(&g.transcript());

    g.table.breaks_madness.push(p("Chef"));
    g.picks(&[&[1], &[1]]).run(|ctx| {
        to(ctx, game::Phase::Day(1));
        narrator::check_madness(ctx);
    });
    assert!(
        !alive(&g, "Chef") && !alive(&g, "Empath"),
        "{}",
        g.transcript()
    );
}

#[test]
fn harpy_madness_kept_or_harpy_poisoned_kills_nobody() {
    let mut g = harpy_game();
    g.answers(&[&["Chef"], &["Empath"]]).run(|ctx| {
        narrator::run_order(ctx, &[&characters::harpy::Def], 1);
        to(ctx, game::Phase::Day(1));
        narrator::check_madness(ctx);
    });
    assert_eq!(g.state.alive_ids().len(), 6);

    let mut g = harpy_game();
    g.table.breaks_madness.push(p("Chef"));
    g.picks(&[&[1], &[1]])
        .answers(&[&["Chef"], &["Empath"]])
        .run(|ctx| {
            poison(ctx, "Harpy");
            narrator::run_order(ctx, &[&characters::harpy::Def], 1);
            to(ctx, game::Phase::Day(1));
            narrator::check_madness(ctx);
        });
    assert_eq!(g.state.alive_ids().len(), 6);
    assert!(!g.transcript().contains("Mad ("));
}

// --- Marionette ---------------------------------------------------------------------

fn marionette_seats() -> [(&'static str, character::Ref); 7] {
    [
        ("Imp", &characters::imp::Def),
        ("Puppet", &characters::marionette::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Saint", &characters::saint::Def),
    ]
}

#[test]
fn the_marionette_believes_they_are_good_and_only_the_demon_knows() {
    let mut g = common::Game::new(&marionette_seats());
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::setup(ctx, &script());
        to(ctx, game::Phase::Night(1));
        narrator::run_night(ctx, &script());
    });
    let log = g.transcript();
    assert!(
        log.contains("* Puppet believes they are the Washerwoman"),
        "{log}"
    );
    // Minion info: the Poisoner learns the Imp only; the Marionette sleeps.
    assert!(log.contains("-> Poisoner: Evil: Imp, Poisoner\n"), "{log}");
    assert!(!log.contains("-> Puppet: Evil"), "{log}");
    // The Demon is shown the Marionette.
    assert!(log.contains("-> Imp: Puppet=Marionette"), "{log}");
    // As the Washerwoman, the Marionette is malfunctioning.
    assert!(
        log.contains("information for Puppet (Washerwoman), who is poisoned or drunk"),
        "{log}"
    );
    // The Marionette registers as evil: the Chef counts them.
    assert!(g.state.is_evil(&p("Puppet")));
}

#[test]
fn simulated_marionettes_sit_next_to_the_demon() {
    let script = script();
    let mut seen = 0;
    for seed in 0..300 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let seats = sim::setup(&mut rng, 5 + (seed as usize % 11), &script);
        let n = seats.len();
        for (i, (_, c)) in seats.iter().enumerate() {
            if c.id() != "marionette" {
                continue;
            }
            seen += 1;
            let beside = [&seats[(i + 1) % n].1, &seats[(i + n - 1) % n].1];
            assert!(
                beside.iter().any(|c| c.kind() == character::Kind::Demon),
                "seed {seed}: {seats:?}"
            );
        }
    }
    assert!(seen > 0);
}

// --- Mastermind ---------------------------------------------------------------------

fn mastermind_game() -> common::Game {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Master", &characters::mastermind::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    );
    g.run(|ctx| {
        ctx.execute(&p("Imp"));
    });
    g
}

#[test]
fn mastermind_plays_one_more_day_after_the_demon_is_executed() {
    let mut g = mastermind_game();
    g.run(|ctx| {
        to(ctx, game::Phase::Day(2));
        ctx.execute(&p("Chef"));
    });
    expect![[r#"
        * Imp is executed
        * Imp dies (Execution)
        * Master (mastermind) marks Master: Extra day
        * === Night 2 ===
        * === Day 2 ===
        * Chef is executed
        * Evil wins!
        * Chef dies (Execution)
        --- grimoire ---
        Imp: Imp [dead]
        Master: Mastermind [Extra day by mastermind]
        Chef: Chef [dead]
        Empath: Empath
        Monk: Monk
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());

    // Executing an evil player on the extra day: good wins.
    let mut g = mastermind_game();
    g.run(|ctx| {
        to(ctx, game::Phase::Day(2));
        ctx.execute(&p("Master"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));

    // No execution on the extra day: good wins at its end.
    let mut g = mastermind_game();
    g.run(|ctx| {
        to(ctx, game::Phase::Day(2));
        assert_eq!(ctx.state().winner(), None);
        narrator::end_day(ctx);
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));

    // The Mastermind dies on the extra night: good wins then.
    let mut g = mastermind_game();
    g.run(|ctx| {
        to(ctx, game::Phase::Night(2));
        ctx.kill(&p("Master"), botc_narrator::event::Cause::Demon);
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn the_pit_hag_turning_the_only_demon_into_a_townsfolk_ends_the_game() {
    let (mut g, _) = pit_hag_game(&characters::virgin::Def);
    g.answers(&[&["Imp"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::pit_hag::Def], 2));
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn the_assassin_kills_a_player_the_devils_advocate_saved() {
    let mut g = game(
        &[
            ("Assassin", &characters::assassin::Def),
            ("Imp", &characters::imp::Def),
            ("Advocate", &characters::devils_advocate::Def),
            ("Psycho", &characters::psychopath::Def),
            ("Chef", &characters::chef::Def),
        ],
        game::Phase::Night(2),
    );
    g.table.agrees.push(p("Assassin"));
    g.answers(&[&["Chef"], &["Chef"]]).run(|ctx| {
        narrator::run_order(
            ctx,
            &[
                &characters::devils_advocate::Def,
                &characters::assassin::Def,
            ],
            2,
        )
    });
    assert!(!alive(&g, "Chef"), "{}", g.transcript());
}

#[test]
fn a_poisoned_mastermind_lets_good_win() {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Master", &characters::mastermind::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    );
    g.run(|ctx| {
        poison(ctx, "Master");
        ctx.execute(&p("Imp"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));

    // A demon killed at night is no trigger either.
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Master", &characters::mastermind::Def),
            ("Chef", &characters::chef::Def),
            ("Slayer", &characters::slayer::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    );
    g.answers(&[&["Imp"]])
        .run(|ctx| narrator::day_action(ctx, &p("Slayer"), &characters::slayer::Def));
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

// --- Mezepheles ---------------------------------------------------------------------

fn mezepheles_game() -> common::Game {
    game(
        &[
            ("Mez", &characters::mezepheles::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(1),
    )
}

#[test]
fn the_first_good_player_to_say_the_word_turns_evil_that_night() {
    let mut g = mezepheles_game();
    g.table.agrees.push(p("Empath"));
    g.table.agrees.push(p("Monk"));
    g.run(|ctx| {
        narrator::run_order(ctx, &[&characters::mezepheles::Def], 1);
        to(ctx, game::Phase::Day(1));
        narrator::end_day(ctx);
        to(ctx, game::Phase::Night(2));
        narrator::run_order(ctx, &[&characters::mezepheles::Def], 2);
    });
    expect![[r#"
        wake Mez
          -> Mez: The word is "marmalade"
        sleep Mez
        * === Day 1 ===
          Chef? said "marmalade"? => no
          Empath? said "marmalade"? => yes
        * Mez (mezepheles) marks Empath: Turns evil
        * === Night 2 ===
        * Mez (mezepheles) marks Mez: AbilityUsed
        * Empath is now Evil
        wake Empath
          -> Empath: Evil
        sleep Empath
        --- grimoire ---
        Mez: Mezepheles [marmalade by mezepheles] [AbilityUsed by mezepheles]
        Imp: Imp
        Chef: Chef
        Empath: Empath (Evil) [Turns evil by mezepheles]
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_mezepheles_spends_the_ability_and_nobody_turns() {
    let mut g = mezepheles_game();
    g.table.agrees.push(p("Empath"));
    g.run(|ctx| {
        to(ctx, game::Phase::Day(1));
        narrator::end_day(ctx);
        to(ctx, game::Phase::Night(2));
        poison(ctx, "Mez");
        narrator::run_order(ctx, &[&characters::mezepheles::Def], 2);
        to(ctx, game::Phase::Day(2));
        narrator::end_day(ctx);
    });
    let log = g.transcript();
    assert!(!g.state.is_evil(&p("Empath")), "{log}");
    assert!(log.contains("[AbilityUsed by mezepheles]"), "{log}");
    // Spent: nobody is asked about the word again.
    assert_eq!(log.matches("said \"marmalade\"").count(), 2, "{log}");
}

// --- Pit-Hag ------------------------------------------------------------------------

fn pit_hag_game(first: character::Ref) -> (common::Game, script::Script) {
    // The test table always picks the first character offered.
    let mut chars = vec![first];
    chars.extend(script().characters);
    let script = script::Script::new("pit_hag", "Pit-Hag", chars);
    let mut g = common::Game::new(&[
        ("Hag", &characters::pit_hag::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script);
        to(ctx, game::Phase::Night(2));
    })
    .quiet();
    (g, script)
}

#[test]
fn pit_hag_turns_a_player_into_a_not_in_play_character() {
    let (mut g, _) = pit_hag_game(&characters::virgin::Def);
    g.answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::pit_hag::Def], 2));
    expect![[r#"
        wake Hag
          Hag? Who do you want to change? => Chef
          Hag? Into which character? => Virgin
        sleep Hag
        * Chef becomes the Virgin
        wake Chef
          -> Chef: Virgin
        sleep Chef
        --- grimoire ---
        Hag: Pit-Hag
        Imp: Imp
        Chef: Virgin
        Empath: Empath
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // An in-play character: nothing happens.
    let (mut g, _) = pit_hag_game(&characters::empath::Def);
    g.answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::pit_hag::Def], 2));
    assert_eq!(g.state.character(&p("Chef")).id(), "chef");

    // Poisoned: nothing happens.
    let (mut g, _) = pit_hag_game(&characters::virgin::Def);
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Hag");
        narrator::run_order(ctx, &[&characters::pit_hag::Def], 2);
    });
    assert_eq!(g.state.character(&p("Chef")).id(), "chef");
}

#[test]
fn a_demon_made_by_the_pit_hag_lets_the_storyteller_kill() {
    let (mut g, _) = pit_hag_game(&characters::fang_gu::Def);
    // Yes, kill; the storyteller picks the Monk (living: Hag, Imp, Chef, Empath, Monk).
    g.answers(&[&["Chef"]])
        .picks(&[&[1], &[4]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::pit_hag::Def], 2));
    let log = g.transcript();
    assert!(log.contains("Chef: Fang Gu\n"), "{log}");
    assert!(!g.state.is_evil(&p("Chef")), "alignment stays: {log}");
    assert!(!alive(&g, "Monk"), "{log}");
}

// --- Psychopath ---------------------------------------------------------------------

fn psychopath_game() -> common::Game {
    game(
        &[
            ("Psycho", &characters::psychopath::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Day(1),
    )
}

#[test]
fn a_bluffed_psychopath_kill_does_nothing_and_comes_back_each_day() {
    let mut g = psychopath_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Empath"), &characters::psychopath::Def);
    });
    expect![[r#"
        * Empath (psychopath) marks Empath: claimed
        * Announcement: Empath uses the Psychopath ability
          Empath? Who do you want to kill? => Chef
        * Announcement: Nothing happens
        --- grimoire ---
        Psycho: Psychopath
        Imp: Imp
        Chef: Chef
        Empath: Empath [claimed by psychopath]
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
    let offered = |g: &common::Game| {
        narrator::day_actions(&g.state)
            .iter()
            .any(|(p, a)| p.as_str() == "Empath" && a.id() == "psychopath")
    };
    assert!(!offered(&g), "once a day");
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    assert!(offered(&g), "again the next day");
}

#[test]
fn psychopath_kills_by_day_and_survives_execution_unless_losing_roshambo() {
    let mut g = psychopath_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Psycho"), &characters::psychopath::Def);
        narrator::nominate(ctx, &p("Empath"), &p("Psycho"));
        ctx.execute(&p("Psycho"));
    });
    expect![[r#"
        * Psycho (psychopath) marks Psycho: claimed
        * Announcement: Psycho uses the Psychopath ability
          Psycho? Who do you want to kill? => Chef
        * Chef dies (Ability { by: Id("Psycho") })
        * Empath nominates Psycho
        * Psycho is executed
          Psycho? lost roshambo against Empath? => no
        * Announcement: Psycho does not lose roshambo
        * Psycho survives (psychopath)
        --- grimoire ---
        Psycho: Psychopath [claimed by psychopath]
        Imp: Imp
        Chef: Chef [dead]
        Empath: Empath
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Self-nominated and losing to the storyteller.
    let mut g = psychopath_game();
    g.table.agrees.push(p("Psycho"));
    g.run(|ctx| {
        narrator::nominate(ctx, &p("Psycho"), &p("Psycho"));
        ctx.execute(&p("Psycho"));
    });
    let log = g.transcript();
    assert!(
        log.contains("lost roshambo against the storyteller? => yes"),
        "{log}"
    );
    assert!(!alive(&g, "Psycho"));
}

#[test]
fn a_poisoned_psychopath_kills_nobody_and_dies_without_roshambo() {
    let mut g = psychopath_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Psycho");
        narrator::day_action(ctx, &p("Psycho"), &characters::psychopath::Def);
        narrator::nominate(ctx, &p("Empath"), &p("Psycho"));
        ctx.execute(&p("Psycho"));
    });
    let log = g.transcript();
    assert!(alive(&g, "Chef"), "{log}");
    assert!(!alive(&g, "Psycho"), "{log}");
    assert!(!log.contains("roshambo"), "{log}");
}

// --- Widow --------------------------------------------------------------------------

fn widow_game() -> common::Game {
    game(
        &[
            ("Widow", &characters::widow::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(1),
    )
}

#[test]
fn widow_poisons_until_she_dies_and_a_good_player_knows() {
    let mut g = widow_game();
    g.answers(&[&["Chef"]])
        .picks(&[&[1]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::widow::Def], 1));
    expect![[r#"
        wake Widow
          storyteller [1 of 1 options] information for Widow (Widow): "Widow=Widow, Imp=Imp, Chef=Chef, Empath=Empath, Monk=Monk"
          -> Widow: Widow=Widow, Imp=Imp, Chef=Chef, Empath=Empath, Monk=Monk
          Widow? Who do you want to poison? => Chef
        sleep Widow
        * Widow (widow) marks Chef: Poisoned
          storyteller [1 of 3 options] Widow Widow: which good player knows a Widow is in play: Empath
        wake Empath
          -> Empath: Widow
        sleep Empath
        --- grimoire ---
        Widow: Widow
        Imp: Imp
        Chef: Chef [Poisoned by widow]
        Empath: Empath
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // The poison lasts through later nights, until the Widow dies.
    g.run(|ctx| to(ctx, game::Phase::Night(3)));
    assert!(g.state.has_effect(&p("Chef"), game::Effect::Poisoned));
    g.run(|ctx| {
        ctx.execute(&p("Widow"));
    });
    assert!(!g.state.has_effect(&p("Chef"), game::Effect::Poisoned));
}

#[test]
fn a_poisoned_widow_poisons_nobody_but_a_good_player_still_knows() {
    let mut g = widow_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Widow");
        narrator::run_order(ctx, &[&characters::widow::Def], 1);
    });
    let log = g.transcript();
    assert!(
        !g.state.has_effect(&p("Chef"), game::Effect::Poisoned),
        "{log}"
    );
    assert!(log.contains("-> Chef: Widow"), "{log}");
}

// --- Witch --------------------------------------------------------------------------

fn witch_game() -> common::Game {
    game(
        &[
            ("Witch", &characters::witch::Def),
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Empath", &characters::empath::Def),
            ("Monk", &characters::monk::Def),
        ],
        game::Phase::Night(1),
    )
}

#[test]
fn the_witch_curses_a_player_who_dies_if_they_nominate() {
    let mut g = witch_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::run_order(ctx, &[&characters::witch::Def], 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Chef"), &p("Imp"));
    });
    expect![[r#"
        wake Witch
          Witch? Who do you want to curse? => Chef
        sleep Witch
        * Witch (witch) marks Chef: Cursed
        * === Day 1 ===
        * Chef nominates Imp
        * Chef dies (Ability { by: Id("Witch") })
        --- grimoire ---
        Witch: Witch
        Imp: Imp
        Chef: Chef [dead] [Cursed by witch]
        Empath: Empath
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // The curse lasts one day.
    g.run(|ctx| {
        to(ctx, game::Phase::Day(2));
        narrator::nominate(ctx, &p("Empath"), &p("Imp"));
    });
    assert!(alive(&g, "Empath"));
}

#[test]
fn a_poisoned_witch_or_three_living_players_curse_nobody() {
    let mut g = witch_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        poison(ctx, "Witch");
        narrator::run_order(ctx, &[&characters::witch::Def], 1);
        to(ctx, game::Phase::Day(1));
        narrator::nominate(ctx, &p("Chef"), &p("Imp"));
    });
    assert!(alive(&g, "Chef"));

    let mut g = witch_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::run_order(ctx, &[&characters::witch::Def], 1);
        to(ctx, game::Phase::Day(1));
        ctx.execute(&p("Monk"));
        ctx.execute(&p("Empath"));
        narrator::nominate(ctx, &p("Chef"), &p("Imp"));
    });
    assert!(alive(&g, "Chef"), "{}", g.transcript());
}

// --- Simulation ---------------------------------------------------------------------

#[test]
fn simulated_games_with_every_minion_terminate() {
    let script = script();
    let mut seen: Vec<&str> = Vec::new();
    for seed in 0..200 {
        let players = 5 + (seed as usize % 11);
        let trace = sim::simulate(seed, players, &script);
        assert!(!trace.steps.is_empty());
        for seat in &trace.players {
            if !seen.contains(&seat.character) {
                seen.push(seat.character);
            }
        }
    }
    for minion in MINIONS {
        assert!(
            seen.contains(&minion.name()),
            "{} never played",
            minion.name()
        );
    }
}
