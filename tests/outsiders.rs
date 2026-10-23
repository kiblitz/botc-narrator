//! The Outsiders beyond Trouble Brewing: Barber, Damsel, Golem, Goon, Klutz,
//! Lunatic, Moonchild, Mutant, Plague Doctor, Politician, Puzzlemaster,
//! Sweetheart and Tinker.

mod common;

use expect_test::expect;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;

use common::p;

fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend([
        &characters::barber::Def as character::Ref,
        &characters::damsel::Def,
        &characters::golem::Def,
        &characters::goon::Def,
        &characters::klutz::Def,
        &characters::lunatic::Def,
        &characters::moonchild::Def,
        &characters::mutant::Def,
        &characters::plague_doctor::Def,
        &characters::politician::Def,
        &characters::puzzlemaster::Def,
        &characters::sweetheart::Def,
        &characters::tinker::Def,
    ]);
    script::Script::new("test", "Test", chars)
}

/// A game with `outsider` (seated as "Out") among Trouble Brewing players,
/// set up and in its first day.
fn game(outsider: character::Ref) -> common::Game {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Out", outsider),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet();
    g
}

fn night(ctx: &mut ctx::Write<'_>) {
    ctx.advance_phase();
    narrator::run_night(ctx, &script());
}

fn only(c: character::Ref) -> impl FnOnce(&mut ctx::Write<'_>) {
    move |ctx| {
        let night = match ctx.state().phase() {
            game::Phase::Night(n) => n,
            _ => 1,
        };
        narrator::run_order(ctx, &[c], night);
    }
}

fn poison(ctx: &mut ctx::Write<'_>, target: &str) {
    let reminder = game::Reminder::new(
        &characters::poisoner::Def,
        &p("Poisoner"),
        game::Effect::Poisoned,
        game::Expiry::Dusk,
    );
    ctx.place_reminder(&p(target), reminder);
}

fn execute(name: &str) -> impl FnOnce(&mut ctx::Write<'_>) {
    let name = p(name);
    move |ctx| {
        ctx.execute(&name);
    }
}

// --- Sweetheart -----------------------------------------------------------------

#[test]
fn sweetheart_makes_a_player_drunk_when_she_dies() {
    let mut g = game(&characters::sweetheart::Def);
    g.picks(&[&[0]]).run(execute("Out"));
    expect![[r#"
        * Out is executed
        * Out dies (Execution)
          storyteller [1 of 6 options] Sweetheart Out: who is drunk from now on: Imp
        * Out (sweetheart) marks Imp: Drunk
        --- grimoire ---
        Imp: Imp [Drunk by sweetheart]
        Poisoner: Poisoner
        Out: Sweetheart [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // The drunk Imp's kill fails.
    g.quiet().answers(&[&["Monk"], &["Chef"]]).run(night);
    assert!(g.state.is_alive(&p("Chef")), "{}", g.transcript());
}

#[test]
fn poisoned_sweetheart_makes_nobody_drunk() {
    let mut g = game(&characters::sweetheart::Def);
    g.run(|ctx| poison(ctx, "Out")).quiet().run(execute("Out"));
    expect![[r#"
        * Out is executed
        * Out dies (Execution)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Sweetheart [dead] [Poisoned by poisoner]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- Tinker -----------------------------------------------------------------------

#[test]
fn tinker_might_die_at_night() {
    let mut g = game(&characters::tinker::Def);
    g.run(|ctx| {
        ctx.advance_phase();
    })
    .quiet()
    .picks(&[&[1]])
    .run(only(&characters::tinker::Def));
    expect![[r#"
          storyteller [1 of 2 options] Tinker Out: does the Tinker die tonight?: Yes
        * Out dies (Ability { by: Id("Out") })
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Tinker [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the storyteller is not even asked.
    let mut g = game(&characters::tinker::Def);
    g.run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Out");
    })
    .quiet()
    .picks(&[&[1]])
    .run(only(&characters::tinker::Def));
    assert!(g.state.is_alive(&p("Out")));
    assert!(!g.transcript().contains("storyteller"));
}

#[test]
fn tinker_does_not_die_when_it_would_end_the_game() {
    let mut g = game(&characters::tinker::Def);
    g.run(|ctx| {
        for name in ["Chef", "Monk", "Empath"] {
            ctx.execute(&p(name));
        }
        ctx.advance_phase();
    })
    .quiet()
    .picks(&[&[1]])
    .run(only(&characters::tinker::Def));
    assert!(g.state.is_alive(&p("Out")));
}

// --- Klutz -----------------------------------------------------------------------

#[test]
fn klutz_choosing_an_evil_player_loses_the_game() {
    let mut g = game(&characters::klutz::Def);
    g.answers(&[&["Imp"]]).run(execute("Out"));
    expect![[r#"
        * Out is executed
        * Out dies (Execution)
          Out? Choose a living player => Imp
        * Evil wins!
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Klutz [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn klutz_choosing_a_good_player_changes_nothing() {
    let mut g = game(&characters::klutz::Def);
    g.answers(&[&["Chef"]]).run(execute("Out"));
    assert_eq!(g.state.winner(), None);
}

#[test]
fn klutz_killed_at_night_chooses_at_dawn() {
    let mut g = game(&characters::klutz::Def);
    g.answers(&[&["Chef"], &["Chef"], &["Out"], &["Poisoner"]])
        .run(night)
        .run(|ctx| {
            ctx.advance_phase();
        });
    expect![[r#"
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Chef
        * Poisoner (poisoner) marks Chef: Poisoned
        sleep Poisoner
        wake Monk
          Monk? Who do you want to protect? => Chef
        * Monk (monk) marks Chef: Protected
        sleep Monk
        wake Imp
          Imp? Who do you want to kill? => Out
        sleep Imp
        * Out dies (Demon)
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        * === Day 2 ===
          Out? Choose a living player => Poisoner
        * Evil wins!
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Klutz [dead]
        Chef: Chef [Poisoned by poisoner] [Protected by monk]
        Monk: Monk
        Empath: Empath
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_klutz_and_evil_klutz() {
    let mut g = game(&characters::klutz::Def);
    g.run(|ctx| poison(ctx, "Out"))
        .answers(&[&["Imp"]])
        .run(execute("Out"));
    assert_eq!(g.state.winner(), None);

    // An evil Klutz choosing an evil player makes evil lose.
    let mut g = game(&characters::klutz::Def);
    g.run(|ctx| {
        ctx.set_alignment(&p("Out"), character::Alignment::Evil);
    })
    .answers(&[&["Imp"]])
    .run(execute("Out"));
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

// --- Moonchild -------------------------------------------------------------------

#[test]
fn moonchild_curses_a_good_player_who_dies_that_night() {
    let mut g = game(&characters::moonchild::Def);
    g.answers(&[&["Chef"]]).run(execute("Out"));
    g.answers(&[&["Monk"], &["Empath"], &["Monk"]]).run(night);
    expect![[r#"
        * Out is executed
        * Out dies (Execution)
          Out? Choose a living player => Chef
        * Out (moonchild) marks Chef: dies tonight
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Monk
        * Poisoner (poisoner) marks Monk: Poisoned
        sleep Poisoner
        wake Monk
          Monk? Who do you want to protect? => Empath
        sleep Monk
        wake Imp
          Imp? Who do you want to kill? => Monk
        sleep Imp
        * Monk dies (Demon)
        * Out (moonchild) removes Chef's dies tonight token
        * Chef dies (Ability { by: Id("Out") })
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "2 evil neighbours"
          -> Empath: 2 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Moonchild [dead]
        Chef: Chef [dead]
        Monk: Monk [dead] [Poisoned by poisoner]
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn moonchild_choosing_evil_or_poisoned_at_night_kills_nobody() {
    let mut g = game(&characters::moonchild::Def);
    g.answers(&[&["Poisoner"]]).run(execute("Out"));
    g.quiet().answers(&[&["Monk"], &["Empath"]]).run(night);
    assert!(g.state.is_alive(&p("Poisoner")));

    // Chosen while sober, but poisoned at night: the curse fails.
    let mut g = game(&characters::moonchild::Def);
    g.answers(&[&["Chef"]]).run(execute("Out"));
    g.run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Out");
    })
    .quiet()
    .run(only(&characters::moonchild::Def));
    assert!(g.state.is_alive(&p("Chef")), "{}", g.transcript());
}

// --- Mutant ------------------------------------------------------------------------

#[test]
fn mutant_claiming_outsider_may_be_executed() {
    let mut g = game(&characters::mutant::Def);
    g.table.mad_about.push(p("Out"));
    g.picks(&[&[1]]).run(narrator::end_day);
    expect![[r#"
          Out claims Outsider: true
          storyteller [1 of 2 options] Mutant Out: execute the Mutant?: Yes
        * Out is executed
        * Out dies (Execution)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Mutant [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

/// A Mutant who reports breaking madness (claiming to be an Outsider) may be
/// executed, though the table never said so.
#[test]
fn a_mutant_reporting_madness_may_be_executed() {
    let mut g = game(&characters::mutant::Def);
    g.picks(&[&[1]]).run(|ctx| {
        narrator::report_madness(ctx, &p("Out"));
        narrator::end_day(ctx);
    });
    expect![[r#"
        * Out reports breaking madness
          -> Out: The storyteller has noted it
          storyteller [1 of 2 options] Mutant Out: execute the Mutant?: Yes
        * Out is executed
        * Out dies (Execution)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Mutant [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_mutant_is_not_executed() {
    let mut g = game(&characters::mutant::Def);
    g.table.mad_about.push(p("Out"));
    g.run(|ctx| poison(ctx, "Out"))
        .quiet()
        .picks(&[&[1]])
        .run(narrator::end_day);
    assert!(g.state.is_alive(&p("Out")));
}

// --- Golem -------------------------------------------------------------------------

#[test]
fn golem_nominee_dies_unless_the_demon_and_only_once() {
    let mut g = game(&characters::golem::Def);
    g.run(|ctx| narrator::nominate(ctx, &p("Out"), &p("Chef")))
        .run(|ctx| narrator::nominate(ctx, &p("Out"), &p("Monk")));
    expect![[r#"
        * Out nominates Chef
        * Out (golem) marks Out: AbilityUsed
        * Chef dies (Ability { by: Id("Out") })
        * Out may not nominate
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Golem [AbilityUsed by golem]
        Chef: Chef [dead]
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    let mut g = game(&characters::golem::Def);
    g.run(|ctx| narrator::nominate(ctx, &p("Out"), &p("Imp")));
    assert!(g.state.is_alive(&p("Imp")));
    assert!(!narrator::can_nominate(&g.state, &p("Out")));
}

#[test]
fn poisoned_golem_uses_its_nomination_without_killing() {
    let mut g = game(&characters::golem::Def);
    g.run(|ctx| {
        poison(ctx, "Out");
        narrator::nominate(ctx, &p("Out"), &p("Chef"));
    });
    assert!(g.state.is_alive(&p("Chef")));
    assert!(!narrator::can_nominate(&g.state, &p("Out")));
}

// --- Goon ---------------------------------------------------------------------------

#[test]
fn goon_chosen_by_the_imp_makes_it_drunk_and_turns_evil() {
    let mut g = game(&characters::goon::Def);
    g.answers(&[&["Chef"], &["Chef"], &["Out"]]).run(night);
    expect![[r#"
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Chef
        * Poisoner (poisoner) marks Chef: Poisoned
        sleep Poisoner
        wake Monk
          Monk? Who do you want to protect? => Chef
        * Monk (monk) marks Chef: Protected
        sleep Monk
        wake Imp
          Imp? Who do you want to kill? => Out
        * Out (goon) marks Out: chosen tonight
        * Out (goon) marks Imp: Drunk
        * Out is now Evil
        wake Out
          -> Out: Evil
        sleep Out
        sleep Imp
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp [Drunk by goon]
        Poisoner: Poisoner
        Out: Goon (Evil) [chosen tonight by goon]
        Chef: Chef [Poisoned by poisoner] [Protected by monk]
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn only_the_goons_first_chooser_is_drunk() {
    let mut g = game(&characters::goon::Def);
    g.answers(&[&["Chef"], &["Out"], &["Out"]]).run(night);
    expect![[r#"
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Chef
        * Poisoner (poisoner) marks Chef: Poisoned
        sleep Poisoner
        wake Monk
          Monk? Who do you want to protect? => Out
        * Out (goon) marks Out: chosen tonight
        * Out (goon) marks Monk: Drunk
        sleep Monk
        wake Imp
          Imp? Who do you want to kill? => Out
        sleep Imp
        * Out dies (Demon)
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Goon [dead]
        Chef: Chef [Poisoned by poisoner]
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn goon_chosen_by_a_read_only_ability_makes_the_chooser_drunk_afterwards() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Teller", &characters::fortune_teller::Def),
        ("Out", &characters::goon::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
    })
    .quiet()
    .answers(&[&["Out", "Chef"]])
    .run(only(&characters::fortune_teller::Def));
    expect![[r#"
        wake Teller
          Teller? Choose two players => Out, Chef
          storyteller [1 of 1 options] information for Teller (Fortune Teller): "No"
          -> Teller: No
        sleep Teller
        * Out (goon) marks Out: chosen tonight
        * Out (goon) marks Teller: Drunk
        --- grimoire ---
        Imp: Imp
        Teller: Fortune Teller [red herring by fortune_teller] [Drunk by goon]
        Out: Goon [chosen tonight by goon]
        Chef: Chef
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_goon_does_nothing() {
    let mut g = game(&characters::goon::Def);
    g.run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Out");
    })
    .answers(&[&["Out"]])
    .run(only(&characters::imp::Def));
    assert!(!g.state.is_alive(&p("Out")));
    assert_eq!(g.state.alignment(&p("Out")), character::Alignment::Good);
}

// --- Politician ----------------------------------------------------------------------

#[test]
fn politician_most_responsible_for_losing_changes_sides() {
    let mut g = game(&characters::politician::Def);
    g.picks(&[&[1]])
        .run(|ctx| ctx.win(character::Alignment::Evil));
    expect![[r#"
        * Evil wins!
          storyteller [1 of 2 options] Politician Out: was the Politician most responsible for their team losing?: Yes
        * Out is now Evil
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Politician (Evil)
        Chef: Chef
        Monk: Monk
        Empath: Empath
        winner: Evil
    "#]].assert_eq(&g.transcript());

    // Poisoned, they lose with their team; on the winning team, nothing is asked.
    let mut g = game(&characters::politician::Def);
    g.run(|ctx| poison(ctx, "Out"))
        .picks(&[&[1]])
        .run(|ctx| ctx.win(character::Alignment::Evil));
    assert_eq!(g.state.alignment(&p("Out")), character::Alignment::Good);
    let mut g = game(&characters::politician::Def);
    g.run(|ctx| ctx.win(character::Alignment::Good));
    assert!(!g.transcript().contains("storyteller"));
}

// --- Plague Doctor ------------------------------------------------------------------

#[test]
fn plague_doctor_death_gives_a_minion_a_minion_ability() {
    let mut g = game(&characters::plague_doctor::Def);
    g.picks(&[&[1]]).run(execute("Out"));
    expect![[r#"
        * Out is executed
        * Out dies (Execution)
          storyteller [1 of 4 options] Plague Doctor Out: which Minion ability: Spy
          storyteller [1 of 1 options] Plague Doctor Out: which Minion gains the ability: Poisoner
        * Poisoner gains the Spy ability
          -> Poisoner: Spy
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner (+Spy)
        Out: Plague Doctor [dead]
        Chef: Chef
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_plague_doctor_gives_nothing() {
    let mut g = game(&characters::plague_doctor::Def);
    g.run(|ctx| poison(ctx, "Out")).run(execute("Out"));
    assert!(g.state.player(&p("Poisoner")).gained.is_empty());
}

// --- Puzzlemaster ---------------------------------------------------------------------

fn puzzlemaster_game() -> common::Game {
    // Setup marks the first Townsfolk (the Chef) drunk.
    game(&characters::puzzlemaster::Def)
}

#[test]
fn puzzlemaster_guessing_right_learns_the_demon() {
    let mut g = puzzlemaster_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Out"), &characters::puzzlemaster::Def);
    });
    expect![[r#"
        wake Out
          Out? Who is the drunk player? => Chef
        * Out (puzzlemaster) marks Out: claimed
        * Out (puzzlemaster) marks Out: AbilityUsed
          storyteller [1 of 1 options] information for Out (Puzzlemaster): "Imp"
          -> Out: Imp
        sleep Out
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Puzzlemaster [claimed by puzzlemaster] [AbilityUsed by puzzlemaster]
        Chef: Chef [Drunk by puzzlemaster]
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Only once.
    g.answers(&[&["Chef"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Out"), &characters::puzzlemaster::Def);
    });
    expect![[r#"
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Puzzlemaster [claimed by puzzlemaster] [AbilityUsed by puzzlemaster]
        Chef: Chef [Drunk by puzzlemaster]
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn puzzlemaster_guessing_wrong_learns_a_non_demon() {
    let mut g = puzzlemaster_game();
    g.answers(&[&["Monk"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Out"), &characters::puzzlemaster::Def);
    });
    expect![[r#"
        wake Out
          Out? Who is the drunk player? => Monk
        * Out (puzzlemaster) marks Out: claimed
        * Out (puzzlemaster) marks Out: AbilityUsed
          storyteller [1 of 4 options] Puzzlemaster Out: which non-Demon player to show (wrong guess): Poisoner
          -> Out: Poisoner
        sleep Out
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Puzzlemaster [claimed by puzzlemaster] [AbilityUsed by puzzlemaster]
        Chef: Chef [Drunk by puzzlemaster]
        Monk: Monk
        Empath: Empath
    "#]].assert_eq(&g.transcript());
}

#[test]
fn puzzlemasters_drunk_stays_drunk_after_they_die() {
    let mut g = puzzlemaster_game();
    g.run(execute("Out"));
    assert!(g.state.is_malfunctioning(&p("Chef")));
}

// --- Damsel ----------------------------------------------------------------------------

#[test]
fn minions_learn_the_damsel_and_a_right_guess_wins() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Out", &characters::damsel::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
    })
    .quiet()
    .run(only(&characters::damsel::Def));
    expect![[r#"
        wake Poisoner
          -> Poisoner: Damsel
        sleep Poisoner
        * Out (damsel) marks Poisoner: knows Damsel
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner [knows Damsel by damsel]
        Out: Damsel
        Chef: Chef
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // The Demon may only bluff a guess (nothing happens); a Minion's right
    // guess wins.
    g.run(|ctx| {
        ctx.advance_phase();
    })
    .quiet()
    .answers(&[&["Out"], &["Out"]])
    .run(|ctx| {
        narrator::day_action(ctx, &p("Imp"), &characters::damsel::Def);
        narrator::day_action(ctx, &p("Poisoner"), &characters::damsel::Def);
    });
    expect![[r#"
          Imp? Who is the Damsel? => Out
        * Imp (damsel) marks Imp: claimed
        * Announcement: Imp guesses that the Damsel is Out
        * Announcement: Nothing happens
          Poisoner? Who is the Damsel? => Out
        * Poisoner (damsel) marks Poisoner: claimed
        * Announcement: Poisoner guesses that the Damsel is Out
        * Out (damsel) marks Out: AbilityUsed
        * Evil wins!
        --- grimoire ---
        Imp: Imp [claimed by damsel]
        Poisoner: Poisoner [knows Damsel by damsel] [claimed by damsel]
        Out: Damsel [AbilityUsed by damsel]
        Chef: Chef
        Monk: Monk
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn damsel_wrong_guess_uses_it_up() {
    let mut g = game(&characters::damsel::Def);
    g.answers(&[&["Chef"], &["Out"]]).run(|ctx| {
        narrator::day_action(ctx, &p("Poisoner"), &characters::damsel::Def);
        narrator::day_action(ctx, &p("Poisoner"), &characters::damsel::Def);
    });
    assert_eq!(g.state.winner(), None);
    assert!(narrator::day_action_users(&g.state, &characters::damsel::Def).is_empty());
}

#[test]
fn poisoned_damsel_guessed_right_changes_nothing() {
    let mut g = game(&characters::damsel::Def);
    g.run(|ctx| poison(ctx, "Out"))
        .answers(&[&["Out"]])
        .run(|ctx| narrator::day_action(ctx, &p("Poisoner"), &characters::damsel::Def));
    assert_eq!(g.state.winner(), None);
}

// --- Lunatic -----------------------------------------------------------------------------

fn lunatic_game() -> common::Game {
    common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Out", &characters::lunatic::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Empath", &characters::empath::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn lunatic_believes_they_are_the_demon_and_the_demon_knows() {
    let mut g = lunatic_game();
    g.run(|ctx| narrator::setup(ctx, &script()))
        .answers(&[&["Chef"], &["Chef"]])
        .run(night);
    expect![[r#"
          storyteller [1 of 1 options] Lunatic Out: which Demon they believe they are: Imp
        * Out believes they are the Imp
        * === Night 1 ===
        wake Poisoner
          -> Poisoner: Evil: Imp, Poisoner
        sleep Poisoner
          storyteller [1 of 6 options] Lunatic Out: who the Lunatic is shown as a Minion: Imp
          storyteller [3 of 30 options] demon bluffs (not-in-play good characters): Washerwoman, Librarian, Investigator
        wake Out
          -> Out: Evil: Imp, Out
          -> Out: Bluffs: Washerwoman, Librarian, Investigator
        sleep Out
        wake Imp
          -> Imp: Out is the Lunatic
        sleep Imp
        wake Imp
          -> Imp: Evil: Imp, Poisoner
          storyteller [3 of 25 options] demon bluffs (not-in-play good characters): Washerwoman, Librarian, Investigator
          -> Imp: Bluffs: Washerwoman, Librarian, Investigator
        sleep Imp
        wake Poisoner
          Poisoner? Who do you want to poison? => Chef
        * Poisoner (poisoner) marks Chef: Poisoned
        sleep Poisoner
        === parallel (2) ===
        wake Chef
          storyteller [1 of 3 options] information for Chef (Chef), who is poisoned or drunk: false info is allowed: "1 evil pairs"
          -> Chef: 1 evil pairs
        sleep Chef
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "0 evil neighbours"
          -> Empath: 0 evil neighbours
        sleep Empath
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Lunatic (believes Imp)
        Chef: Chef [Poisoned by poisoner]
        Monk: Monk
        Empath: Empath
        Mayor: Mayor
    "#]].assert_eq(&g.transcript());
}

#[test]
fn lunatic_attacks_first_and_the_demon_learns_the_choice() {
    let mut g = lunatic_game();
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet()
    .answers(&[&["Chef"], &["Empath"]])
    .run(only(&characters::imp::Def));
    expect![[r#"
        wake Out
          Out? Who do you want to kill? => Chef
          storyteller [1 of 1 options] Lunatic Out: whom the Demon is shown the Lunatic chose: "Chef"
        wake Imp
          -> Imp: Out is the Lunatic
          -> Imp: Chef
        sleep Imp
        sleep Out
        wake Imp
          Imp? Who do you want to kill? => Empath
        sleep Imp
        * Empath dies (Demon)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Lunatic (believes Imp)
        Chef: Chef
        Monk: Monk
        Empath: Empath [dead]
        Mayor: Mayor
    "#]].assert_eq(&g.transcript());
}

#[test]
fn poisoned_lunatic_may_be_misreported_to_the_demon() {
    let mut g = lunatic_game();
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.advance_phase();
        poison(ctx, "Out");
    })
    .quiet()
    .answers(&[&["Chef"], &["Empath"]])
    .picks(&[&[3]])
    .run(only(&characters::imp::Def));
    expect![[r#"
        wake Out
          Out? Who do you want to kill? => Chef
          storyteller [1 of 7 options] Lunatic Out: whom the Demon is shown the Lunatic chose: "Out" (false)
        wake Imp
          -> Imp: Out is the Lunatic
          -> Imp: Out
        sleep Imp
        sleep Out
        wake Imp
          Imp? Who do you want to kill? => Empath
        sleep Imp
        * Empath dies (Demon)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Out: Lunatic (believes Imp) [Poisoned by poisoner]
        Chef: Chef
        Monk: Monk
        Empath: Empath [dead]
        Mayor: Mayor
    "#]].assert_eq(&g.transcript());
}

// --- Barber -------------------------------------------------------------------------------

#[test]
fn barber_death_lets_the_demon_swap_two_characters() {
    let mut g = game(&characters::barber::Def);
    g.run(execute("Out"))
        .quiet()
        .answers(&[&["Imp", "Chef"]])
        .run(|ctx| {
            ctx.advance_phase();
        })
        .run(only(&characters::barber::Def));
    expect![[r#"
        * === Night 2 ===
        * Out (barber) removes Out's haircuts tonight token
          storyteller [1 of 1 options] Barber Out: which Demon swaps characters: Imp
        wake Imp
          -> Imp: Barber
          Imp? Choose two players to swap characters => Imp, Chef
        sleep Imp
        * Imp becomes the Chef
        wake Imp
          -> Imp: Chef
        sleep Imp
        * Chef becomes the Imp
        wake Chef
          -> Chef: Imp
        sleep Chef
        --- grimoire ---
        Imp: Chef (Evil)
        Poisoner: Poisoner
        Out: Barber [dead]
        Chef: Imp (Good)
        Monk: Monk
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
    assert!(!character::Def::wakes_tonight(
        &characters::barber::Def,
        &g.state,
        &p("Out")
    ));
}

#[test]
fn poisoned_barber_gives_no_haircuts() {
    let mut g = game(&characters::barber::Def);
    g.run(|ctx| poison(ctx, "Out")).run(execute("Out"));
    assert!(!character::Def::wakes_tonight(
        &characters::barber::Def,
        &g.state,
        &p("Out")
    ));
}

// --- Registry --------------------------------------------------------------------------------

#[test]
fn outsiders_are_registered_with_their_night_order() {
    let rows: Vec<String> = script().characters[22..]
        .iter()
        .map(|c| {
            let registered = characters::find(c.id()).expect("registered");
            let order = c.night_order();
            format!(
                "{} {} {:?} {:?}/{:?}",
                registered.name(),
                c.kind(),
                c.id(),
                order.first,
                order.other
            )
        })
        .collect();
    expect![[r#"
        Barber Outsider "barber" None/Some(40)
        Damsel Outsider "damsel" Some(31)/Some(46)
        Golem Outsider "golem" None/None
        Goon Outsider "goon" None/None
        Klutz Outsider "klutz" None/None
        Lunatic Outsider "lunatic" Some(8)/Some(20)
        Moonchild Outsider "moonchild" None/Some(50)
        Mutant Outsider "mutant" None/None
        Plague Doctor Outsider "plague_doctor" None/None
        Politician Outsider "politician" None/None
        Puzzlemaster Outsider "puzzlemaster" None/None
        Sweetheart Outsider "sweetheart" None/Some(41)
        Tinker Outsider "tinker" None/Some(49)"#]]
    .assert_eq(&rows.join("\n"));
}

#[test]
fn no_day_end_ability_acts_after_the_game_is_won() {
    // Three alive, nobody executed: the Mayor (seated first) wins; the Mutant
    // must not then be executed.
    let mut g = common::Game::new(&[
        ("Mayor", &characters::mayor::Def),
        ("Out", &characters::mutant::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet();
    g.table.mad_about.push(p("Out"));
    g.picks(&[&[1]]).run(narrator::end_day);
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
    assert!(g.state.is_alive(&p("Out")), "{}", g.transcript());
}
