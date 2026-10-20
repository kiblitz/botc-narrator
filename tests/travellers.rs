//! Travellers, Fabled and Loric: Apprentice, Barista, Beggar, Bishop, Bone
//! Collector, Bureaucrat, Butcher, Harlot, Matron, Sentinel, Spirit of Ivory,
//! Storm Catcher, Knaves.

mod common;

use expect_test::expect;
use rand::SeedableRng;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;
use botc_narrator::setup;
use botc_narrator::sim;

use common::p;

const GOOD: character::Alignment = character::Alignment::Good;
const EVIL: character::Alignment = character::Alignment::Evil;

/// Trouble Brewing plus every Traveller, Fabled and Loric of this file.
fn script_with(extra: &[character::Ref]) -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend(extra.iter().copied());
    script::Script::new("test", "Test", chars)
}

fn travellers() -> Vec<character::Ref> {
    vec![
        &characters::apprentice::Def,
        &characters::barista::Def,
        &characters::beggar::Def,
        &characters::bishop::Def,
        &characters::bone_collector::Def,
        &characters::bureaucrat::Def,
        &characters::butcher::Def,
        &characters::harlot::Def,
        &characters::matron::Def,
    ]
}

/// Residents (alignment from their character) followed by Travellers.
fn game(
    residents: &[(&str, character::Ref)],
    travellers: &[(&str, character::Ref, character::Alignment)],
) -> common::Game {
    let mut seats: Vec<game::Seat> = residents
        .iter()
        .map(|(name, c)| game::Seat::new(p(name), *c))
        .collect();
    seats.extend(
        travellers
            .iter()
            .map(|(name, c, a)| game::Seat::traveller(p(name), *c, *a)),
    );
    common::Game::seated(seats)
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

fn first_night(ctx: &mut ctx::Write<'_>, script: &script::Script) {
    narrator::setup(ctx, script);
    ctx.advance_phase();
    narrator::run_night(ctx, script);
}

/// Moves on to night `n` without running anything.
fn skip_to_night(ctx: &mut ctx::Write<'_>, n: u32) {
    while ctx.state().phase() != game::Phase::Night(n) {
        ctx.advance_phase();
    }
}

// --- Apprentice -------------------------------------------------------------------

#[test]
fn good_apprentice_gains_a_townsfolk_ability_and_uses_it_that_night() {
    let script = script_with(&[&characters::apprentice::Def]);
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Appr", &characters::apprentice::Def, GOOD)],
    );
    g.run(|ctx| first_night(ctx, &script));
    expect![[r#"
        * === Night 1 ===
          storyteller [1 of 10 options] Apprentice Appr: which ability the Apprentice gains: Washerwoman
        * Appr gains the Washerwoman ability
        wake Appr
          -> Appr: Washerwoman
        sleep Appr
        === parallel (2) ===
        wake Appr
          storyteller [1 of 24 options] information for Appr (Apprentice): "Chef and Imp: one is the Chef"
          -> Appr: Chef and Imp: one is the Chef
        sleep Appr
        wake Chef
          storyteller [1 of 1 options] information for Chef (Chef): "0 evil pairs"
          -> Chef: 0 evil pairs
        sleep Chef
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Chef: Chef
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
        Appr: Apprentice (+Washerwoman)
    "#]].assert_eq(&g.transcript());
}

#[test]
fn evil_apprentice_gains_a_minion_ability() {
    let script = script_with(&[&characters::apprentice::Def]);
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Appr", &characters::apprentice::Def, EVIL)],
    );
    g.run(|ctx| first_night(ctx, &script));
    expect![[r#"
        * === Night 1 ===
          storyteller [1 of 4 options] Apprentice Appr: which ability the Apprentice gains: Poisoner
        * Appr gains the Poisoner ability
        wake Appr
          -> Appr: Poisoner
        sleep Appr
        wake Appr
          Appr? Who do you want to poison? => Imp
        * Appr (poisoner) marks Imp: Poisoned
        sleep Appr
        wake Chef
          storyteller [1 of 1 options] information for Chef (Chef): "1 evil pairs"
          -> Chef: 1 evil pairs
        sleep Chef
        --- grimoire ---
        Imp: Imp [Poisoned by poisoner]
        Chef: Chef
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
        Appr: Apprentice (Evil) (+Poisoner)
    "#]]
    .assert_eq(&g.transcript());
}

// --- Barista ----------------------------------------------------------------------

fn barista_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Empath", &characters::empath::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Bari", &characters::barista::Def, GOOD)],
    )
}

#[test]
fn barista_makes_a_poisoned_player_sober_with_true_info() {
    let order: [character::Ref; 2] = [&characters::barista::Def, &characters::empath::Def];
    let mut g = barista_game();
    g.run(|ctx| skip_to_night(ctx, 2))
        .run(|ctx| poison(ctx, "Empath"))
        .quiet()
        // The storyteller picks the Empath and "sober & healthy".
        .picks(&[&[1], &[0]])
        .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
          storyteller [1 of 6 options] Barista Bari: who is affected: Empath
          storyteller [1 of 2 options] Barista Bari: does their ability work twice (instead of sober & healthy): No
        * Bari (barista) marks Empath: Sober
        wake Empath
          -> Empath: The Barista selected you: you are sober, healthy & get true info
        sleep Empath
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Empath: Empath [Poisoned by poisoner] [Sober by barista]
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bari: Barista
    "#]].assert_eq(&g.transcript());
    assert!(!g.state.is_malfunctioning(&p("Empath")));
}

#[test]
fn barista_makes_an_ability_work_twice() {
    let order: [character::Ref; 2] = [&characters::barista::Def, &characters::empath::Def];
    let mut g = barista_game();
    g.run(|ctx| skip_to_night(ctx, 2))
        .quiet()
        .picks(&[&[1], &[1]])
        .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
          storyteller [1 of 6 options] Barista Bari: who is affected: Empath
          storyteller [1 of 2 options] Barista Bari: does their ability work twice (instead of sober & healthy): Yes
        * Bari (barista) marks Empath: ActsTwice
        wake Empath
          -> Empath: The Barista selected you: your ability works twice
        sleep Empath
        === parallel (2) ===
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Empath: Empath [ActsTwice by barista]
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bari: Barista
    "#]].assert_eq(&g.transcript());
}

#[test]
fn poisoned_barista_has_no_effect() {
    let order: [character::Ref; 1] = [&characters::barista::Def];
    let mut g = barista_game();
    g.run(|ctx| skip_to_night(ctx, 2))
        .run(|ctx| poison(ctx, "Bari"))
        .quiet()
        .picks(&[&[1], &[1]])
        .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
          storyteller [1 of 6 options] Barista Bari: who is affected: Empath
          storyteller [1 of 2 options] Barista Bari: does their ability work twice (instead of sober & healthy): Yes
        wake Empath
          -> Empath: The Barista selected you: your ability works twice
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Empath: Empath
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bari: Barista [Poisoned by poisoner]
    "#]].assert_eq(&g.transcript());
}

// --- Beggar -----------------------------------------------------------------------

fn beggar_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Monk", &characters::monk::Def),
            ("Recluse", &characters::recluse::Def),
            ("Chef", &characters::chef::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Beg", &characters::beggar::Def, GOOD)],
    )
}

#[test]
fn beggar_votes_only_with_a_donated_token_and_learns_the_donors_alignment() {
    let mut g = beggar_game();
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.kill(&p("Monk"), botc_narrator::event::Cause::Demon);
        ctx.advance_phase();
    })
    .quiet()
    .run(|ctx| {
        // No token: the Beggar's hand does not count.
        narrator::vote(ctx, &p("Imp"), vec![p("Beg"), p("Chef")], 3);
        narrator::day_action(ctx, &p("Beg"), &characters::beggar::Def);
        narrator::vote(ctx, &p("Imp"), vec![p("Beg"), p("Chef")], 3);
        // The token is spent.
        narrator::vote(ctx, &p("Imp"), vec![p("Beg"), p("Chef")], 3);
    });
    expect![[r#"
        * 1 votes for Imp (3 needed)
          Beg? Which dead player gives you their vote token? => Monk
        * Monk gives away their vote token
        * Monk (beggar) marks Beg: vote token
          storyteller [1 of 1 options] information for Beg (Beggar): "Good"
          -> Beg: Good
        * 2 votes for Imp (3 needed)
        * Monk (beggar) removes Beg's vote token token
        * 1 votes for Imp (3 needed)
        --- grimoire ---
        Imp: Imp
        Monk: Monk [dead]
        Recluse: Recluse
        Chef: Chef
        Mayor: Mayor
        Beg: Beggar
    "#]]
    .assert_eq(&g.transcript());
    assert!(!g.state.player(&p("Monk")).ghost_vote);
}

#[test]
fn recluse_donor_may_register_evil_to_the_beggar() {
    let mut g = beggar_game();
    g.run(|ctx| {
        ctx.kill(&p("Recluse"), botc_narrator::event::Cause::Demon);
    })
    .quiet()
    .picks(&[&[1]])
    .run(|ctx| narrator::day_action(ctx, &p("Beg"), &characters::beggar::Def));
    expect![[r#"
          Beg? Which dead player gives you their vote token? => Recluse
        * Recluse gives away their vote token
        * Recluse (beggar) marks Beg: vote token
          storyteller [1 of 2 options] how Recluse registers to Beg (first option is true): Evil
          storyteller [1 of 1 options] information for Beg (Beggar): "Evil"
          -> Beg: Evil
        --- grimoire ---
        Imp: Imp
        Monk: Monk
        Recluse: Recluse [dead]
        Chef: Chef
        Mayor: Mayor
        Beg: Beggar [vote token by beggar]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn beggar_is_never_poisoned_and_loses_tokens_on_death() {
    let mut g = beggar_game();
    g.run(|ctx| {
        poison(ctx, "Beg");
        ctx.kill(&p("Monk"), botc_narrator::event::Cause::Demon);
        narrator::day_action(ctx, &p("Beg"), &characters::beggar::Def);
    });
    assert!(!g.state.is_malfunctioning(&p("Beg")));
    g.quiet().run(|ctx| {
        ctx.kill(&p("Beg"), botc_narrator::event::Cause::Demon);
    });
    expect![[r#"
        * Beg dies (Demon)
        * Monk (beggar) removes Beg's vote token token
        --- grimoire ---
        Imp: Imp
        Monk: Monk [dead]
        Recluse: Recluse
        Chef: Chef
        Mayor: Mayor
        Beg: Beggar [dead] [Poisoned by poisoner]
    "#]]
    .assert_eq(&g.transcript());
    // Dead, the Beggar votes with their own ghost vote.
    assert_eq!(narrator::tally(&g.state, &[p("Beg")]), 1);
}

// --- Bishop -----------------------------------------------------------------------

#[test]
fn bishop_has_the_storyteller_nominate_an_opposing_player() {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Poisoner", &characters::poisoner::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Bish", &characters::bishop::Def, GOOD)],
    );
    let mut nominees = None;
    g.run(|ctx| nominees = narrator::storyteller_nominations(ctx));
    assert_eq!(nominees, Some(vec![p("Imp")]));
    expect![[r#"
          storyteller [1 of 2 options] Bishop Bish: which opposing player to nominate: Imp
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bish: Bishop
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the players nominate normally.
    let mut nominees = None;
    g.run(|ctx| {
        poison(ctx, "Bish");
        nominees = narrator::storyteller_nominations(ctx);
    });
    assert_eq!(nominees, None);
}

// --- Bone Collector ---------------------------------------------------------------

fn bone_collector_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Empath", &characters::empath::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Bones", &characters::bone_collector::Def, GOOD)],
    )
}

#[test]
fn bone_collector_returns_a_dead_players_ability_until_dusk() {
    let order: [character::Ref; 2] = [&characters::bone_collector::Def, &characters::empath::Def];
    let mut g = bone_collector_game();
    g.run(|ctx| {
        ctx.kill(&p("Empath"), botc_narrator::event::Cause::Demon);
        skip_to_night(ctx, 2);
    })
    .quiet()
    .yes_no(&[true])
    .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
        wake Bones
          Bones? Do you use your ability tonight? => yes
          Bones? Whose ability returns until dusk? => Empath
        sleep Bones
        * Bones (bone_collector) marks Bones: AbilityUsed
        * Bones (bone_collector) marks Empath: AbilityRestored
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Empath: Empath [dead] [AbilityRestored by bone_collector]
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bones: Bone Collector [AbilityUsed by bone_collector]
    "#]]
    .assert_eq(&g.transcript());
    // Used once per game; the ability goes at the next dusk.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::run_order(ctx, &order, 3);
    });
    expect![[r#"
        * === Day 2 ===
        * === Night 3 ===
        --- grimoire ---
        Imp: Imp
        Empath: Empath [dead]
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bones: Bone Collector [AbilityUsed by bone_collector]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_bone_collector_spends_the_use_for_nothing() {
    let order: [character::Ref; 2] = [&characters::bone_collector::Def, &characters::empath::Def];
    let mut g = bone_collector_game();
    g.run(|ctx| {
        ctx.kill(&p("Empath"), botc_narrator::event::Cause::Demon);
        skip_to_night(ctx, 2);
        poison(ctx, "Bones");
    })
    .quiet()
    .yes_no(&[true])
    .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
        wake Bones
          Bones? Do you use your ability tonight? => yes
          Bones? Whose ability returns until dusk? => Empath
        sleep Bones
        * Bones (bone_collector) marks Bones: AbilityUsed
        --- grimoire ---
        Imp: Imp
        Empath: Empath [dead]
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bones: Bone Collector [Poisoned by poisoner] [AbilityUsed by bone_collector]
    "#]]
    .assert_eq(&g.transcript());
}

// --- Bureaucrat -------------------------------------------------------------------

fn bureaucrat_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Empath", &characters::empath::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Bur", &characters::bureaucrat::Def, EVIL)],
    )
}

#[test]
fn bureaucrats_choice_votes_three_times_until_the_bureaucrat_dies() {
    let order: [character::Ref; 1] = [&characters::bureaucrat::Def];
    let mut g = bureaucrat_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        narrator::run_order(ctx, &order, 1);
        ctx.advance_phase();
        narrator::vote(ctx, &p("Imp"), vec![p("Chef"), p("Monk")], 3);
        ctx.kill(&p("Bur"), botc_narrator::event::Cause::Demon);
        narrator::vote(ctx, &p("Imp"), vec![p("Chef"), p("Monk")], 3);
    });
    expect![[r#"
        * === Night 1 ===
        wake Bur
          Bur? Whose vote counts as 3 tomorrow? => Chef
        sleep Bur
        * Bur (bureaucrat) marks Chef: 3 votes
        * === Day 1 ===
        * 4 votes for Imp (3 needed)
        * Bur dies (Demon)
        * 2 votes for Imp (3 needed)
        --- grimoire ---
        Imp: Imp
        Empath: Empath
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
        Bur: Bureaucrat (Evil) [dead]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_bureaucrat_changes_no_vote() {
    let order: [character::Ref; 1] = [&characters::bureaucrat::Def];
    let mut g = bureaucrat_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        narrator::run_order(ctx, &order, 1);
        poison(ctx, "Bur");
    });
    assert_eq!(narrator::tally(&g.state, &[p("Chef"), p("Monk")]), 5 - 3);
    let mut g = bureaucrat_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Bur");
        narrator::run_order(ctx, &order, 1);
    });
    assert_eq!(narrator::tally(&g.state, &[p("Chef"), p("Monk")]), 2);
}

// --- Butcher ----------------------------------------------------------------------

#[test]
fn butcher_may_nominate_again_unless_malfunctioning() {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Butch", &characters::butcher::Def, GOOD)],
    );
    assert_eq!(narrator::nominate_again(&g.state), vec![p("Butch")]);
    g.run(|ctx| poison(ctx, "Butch"));
    assert!(narrator::nominate_again(&g.state).is_empty());
}

// --- Harlot -----------------------------------------------------------------------

fn harlot_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Harl", &characters::harlot::Def, GOOD)],
    )
}

#[test]
fn harlot_learns_a_character_and_both_may_die() {
    let order: [character::Ref; 1] = [&characters::harlot::Def];
    let mut g = harlot_game();
    g.run(|ctx| skip_to_night(ctx, 2))
        .quiet()
        .answers(&[&["Chef"]])
        .yes_no(&[true])
        .picks(&[&[1]])
        .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
        wake Harl
          Harl? Whose character do you want to learn? => Chef
        sleep Harl
        wake Chef
          -> Chef: The Harlot selected you: do you agree to show them your character?
          Chef? Do you agree? => yes
        sleep Chef
        wake Harl
          storyteller [1 of 1 options] information for Harl (Harlot): "Chef"
          -> Harl: Chef
        sleep Harl
          storyteller [1 of 2 options] Harlot Harl: do both players die?: Yes
        * Chef dies (Ability { by: Id("Harl") })
        * Harl dies (Ability { by: Id("Harl") })
        --- grimoire ---
        Imp: Imp
        Chef: Chef [dead]
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
        Harl: Harlot [dead]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn harlot_learns_nothing_when_declined_and_kills_nobody_when_poisoned() {
    let order: [character::Ref; 1] = [&characters::harlot::Def];
    let mut g = harlot_game();
    g.run(|ctx| skip_to_night(ctx, 2))
        .quiet()
        .answers(&[&["Chef"]])
        .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
        wake Harl
          Harl? Whose character do you want to learn? => Chef
        sleep Harl
        wake Chef
          -> Chef: The Harlot selected you: do you agree to show them your character?
          Chef? Do you agree? => no
        sleep Chef
        --- grimoire ---
        Imp: Imp
        Chef: Chef
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
        Harl: Harlot
    "#]]
    .assert_eq(&g.transcript());

    let mut g = harlot_game();
    g.run(|ctx| {
        skip_to_night(ctx, 2);
        poison(ctx, "Harl");
    })
    .quiet()
    .answers(&[&["Chef"]])
    .yes_no(&[true])
    // A false character.
    .picks(&[&[2]])
    .run(|ctx| narrator::run_order(ctx, &order, 2));
    expect![[r#"
        wake Harl
          Harl? Whose character do you want to learn? => Chef
        sleep Harl
        wake Chef
          -> Chef: The Harlot selected you: do you agree to show them your character?
          Chef? Do you agree? => yes
        sleep Chef
        wake Harl
          storyteller [1 of 6 options] information for Harl (Harlot), who is poisoned or drunk: false info is allowed: "Monk" (false)
          -> Harl: Monk
        sleep Harl
        --- grimoire ---
        Imp: Imp
        Chef: Chef
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
        Harl: Harlot [Poisoned by poisoner]
    "#]].assert_eq(&g.transcript());
}

// --- Matron -----------------------------------------------------------------------

#[test]
fn matron_swaps_seats_unless_malfunctioning() {
    let seats = [
        ("Imp", &characters::imp::Def as character::Ref),
        ("Empath", &characters::empath::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ];
    let matron = [("Mat", &characters::matron::Def as character::Ref, GOOD)];
    let mut g = game(&seats, &matron);
    g.answers(&[&["Imp", "Mayor"], &["Empath", "Mat"]])
        .yes_no(&[true, false])
        .run(|ctx| narrator::day_action(ctx, &p("Mat"), &characters::matron::Def));
    expect![[r#"
          Mat? Which two players swap seats? => Imp, Mayor
        * Imp and Mayor swap seats
          Mat? Swap another pair? => yes
          Mat? Which two players swap seats? => Empath, Mat
        * Empath and Mat swap seats
          Mat? Swap another pair? => no
        --- grimoire ---
        Mayor: Mayor
        Mat: Matron
        Chef: Chef
        Monk: Monk
        Imp: Imp
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    let mut g = game(&seats, &matron);
    g.run(|ctx| poison(ctx, "Mat"))
        .quiet()
        .answers(&[&["Imp", "Mayor"]])
        .run(|ctx| narrator::day_action(ctx, &p("Mat"), &characters::matron::Def));
    let order: Vec<String> = g
        .state
        .seat_order()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(order, ["Imp", "Empath", "Chef", "Monk", "Mayor", "Mat"]);
}

// --- Sentinel ---------------------------------------------------------------------

#[test]
fn sentinel_allows_one_more_or_one_fewer_outsider() {
    let script = script_with(&[&characters::sentinel::Def]);
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..60 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let picks = setup::choose(&mut rng, 9, &script);
        assert_eq!(picks.len(), 9);
        let outsiders = picks
            .iter()
            .filter(|c| c.kind() == character::Kind::Outsider)
            .count();
        let baron = picks.iter().any(|c| c.id() == "baron");
        seen.insert(outsiders - if baron { 2 } else { 0 });
    }
    // Nine players have 2 Outsiders: the Sentinel makes 1, 2 or 3 legal.
    assert_eq!(seen.into_iter().collect::<Vec<_>>(), [1, 2, 3]);
}

// --- Spirit of Ivory --------------------------------------------------------------

#[test]
fn spirit_of_ivory_allows_only_one_extra_evil_player() {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[],
    );
    g.run(|ctx| {
        ctx.add_fabled(&characters::spirit_of_ivory::Def);
        assert!(ctx.set_alignment(&p("Chef"), EVIL));
        assert!(!ctx.set_alignment(&p("Monk"), EVIL));
        // Once the extra evil player is good again, another may turn.
        assert!(ctx.set_alignment(&p("Chef"), GOOD));
        assert!(ctx.set_alignment(&p("Monk"), EVIL));
    });
    expect![[r#"
        * the Spirit of Ivory is in play
        * Chef is now Evil
        * Monk stays Good (Spirit of Ivory)
        * Chef is now Good
        * Monk is now Evil
        --- grimoire ---
        Imp: Imp
        Chef: Chef
        Monk: Monk (Evil)
        Saint: Saint
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

// --- Storm Catcher ----------------------------------------------------------------

fn storm_catcher_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Poisoner", &characters::poisoner::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Slayer", &characters::slayer::Def),
        ],
        &[],
    )
}

#[test]
fn storm_catcher_protects_the_named_character_except_from_execution() {
    let script = script_with(&[&characters::storm_catcher::Def]);
    let chef = script
        .characters
        .iter()
        .filter(|c| c.kind().alignment() == Some(GOOD))
        .position(|c| c.id() == "chef")
        .unwrap();
    let mut g = storm_catcher_game();
    g.picks(&[&[chef]]).run(|ctx| {
        narrator::setup(ctx, &script);
        ctx.advance_phase();
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
        ctx.kill(
            &p("Chef"),
            botc_narrator::event::Cause::Ability { by: p("Slayer") },
        );
        ctx.execute(&p("Chef"));
    });
    expect![[r#"
        * the Storm Catcher is in play
          storyteller [1 of 17 options] Storm Catcher: which good character to name: Chef
        * Chef (storm_catcher) marks Chef: ExecutionOnly
        wake Imp
          -> Imp: Chef is the Chef
        sleep Imp
        wake Poisoner
          -> Poisoner: Chef is the Chef
        sleep Poisoner
        * === Night 1 ===
        * Chef survives (storm_catcher)
        * Chef survives (storm_catcher)
        * Chef is executed
        * Chef dies (Execution)
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef [dead] [ExecutionOnly by storm_catcher]
        Monk: Monk
        Slayer: Slayer
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn storm_catcher_naming_a_character_not_in_play_tells_evil_so() {
    let script = script_with(&[&characters::storm_catcher::Def]);
    let mut g = storm_catcher_game();
    // The first good character on the script is the Washerwoman: not in play.
    g.run(|ctx| narrator::setup(ctx, &script));
    expect![[r#"
        * the Storm Catcher is in play
          storyteller [1 of 17 options] Storm Catcher: which good character to name: Washerwoman
        wake Imp
          -> Imp: The Washerwoman is not in play
        sleep Imp
        wake Poisoner
          -> Poisoner: The Washerwoman is not in play
        sleep Poisoner
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
        Monk: Monk
        Slayer: Slayer
    "#]]
    .assert_eq(&g.transcript());
}

// --- Knaves -----------------------------------------------------------------------

fn knaves_game() -> common::Game {
    game(
        &[
            ("Imp", &characters::imp::Def),
            ("Poisoner", &characters::poisoner::Def),
            ("Chef", &characters::chef::Def),
            ("Monk", &characters::monk::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[],
    )
}

#[test]
fn knaves_lying_storyteller_gives_false_info_and_the_other_true() {
    let script = script_with(&[&characters::knaves::Def]);
    let chef: [character::Ref; 1] = [&characters::chef::Def];
    // The first storyteller lies; the Chef asks them.
    let mut g = knaves_game();
    g.picks(&[&[1]]).yes_no(&[true]).run(|ctx| {
        narrator::setup(ctx, &script);
        ctx.advance_phase();
        narrator::run_order(ctx, &chef, 1);
    });
    expect![[r#"
        * the Knaves is in play
          storyteller [1 of 2 options] Knaves: does the first storyteller lie: Yes
        * the Knaves is marked: first storyteller lies
        * === Night 1 ===
        wake Chef
          Chef? Do you ask the first storyteller? => yes
          storyteller [1 of 2 options] information for Chef (Chef): "0 evil pairs" (false)
          -> Chef: 0 evil pairs
        sleep Chef
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
    // The Chef asks the truthful one.
    let mut g = knaves_game();
    g.picks(&[&[1]]).yes_no(&[false]).run(|ctx| {
        narrator::setup(ctx, &script);
        ctx.advance_phase();
    });
    g.quiet().run(|ctx| narrator::run_order(ctx, &chef, 1));
    expect![[r#"
        wake Chef
          Chef? Do you ask the first storyteller? => no
          storyteller [1 of 1 options] information for Chef (Chef): "1 evil pairs"
          -> Chef: 1 evil pairs
        sleep Chef
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn knaves_switch_once_at_dusk() {
    let script = script_with(&[&characters::knaves::Def]);
    let chef: [character::Ref; 1] = [&characters::chef::Def];
    let mut g = knaves_game();
    // The second storyteller lies, then at the first dusk they switch.
    g.picks(&[&[0], &[1]]).run(|ctx| {
        narrator::setup(ctx, &script);
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::end_day(ctx);
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::end_day(ctx);
    });
    g.quiet()
        .yes_no(&[true])
        .run(|ctx| narrator::run_order(ctx, &chef, 1));
    expect![[r#"
        wake Chef
          Chef? Do you ask the first storyteller? => yes
          storyteller [1 of 2 options] information for Chef (Chef): "0 evil pairs" (false)
          -> Chef: 0 evil pairs
        sleep Chef
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

// --- Simulator --------------------------------------------------------------------

#[test]
fn simulated_games_with_travellers_and_fabled_terminate() {
    let mut extra = travellers();
    extra.extend([
        &characters::sentinel::Def as character::Ref,
        &characters::spirit_of_ivory::Def,
        &characters::storm_catcher::Def,
        &characters::knaves::Def,
    ]);
    let script = script_with(&extra);
    let mut travellers_seen = std::collections::BTreeSet::new();
    for seed in 0..120 {
        let players = 5 + (seed as usize % 11);
        let trace = sim::simulate(seed, players, &script);
        assert!(trace.players.len() >= players);
        for seat in &trace.players {
            if seat.kind == character::Kind::Traveller {
                travellers_seen.insert(seat.character);
            }
        }
    }
    assert_eq!(
        travellers_seen.len(),
        travellers().len(),
        "{travellers_seen:?}"
    );
}

#[test]
fn barista_cannot_make_a_drunks_believed_ability_work() {
    let mut g = game(
        &[
            ("Imp", &characters::imp::Def),
            ("Drunk", &characters::drunk::Def),
            ("Chef", &characters::chef::Def),
            ("Saint", &characters::saint::Def),
            ("Mayor", &characters::mayor::Def),
        ],
        &[("Bari", &characters::barista::Def, GOOD)],
    );
    g.run(|ctx| {
        ctx.set_believed(&p("Drunk"), Some(&characters::monk::Def));
        let sober = game::Reminder::new(
            &characters::barista::Def,
            &p("Bari"),
            game::Effect::Sober,
            game::Expiry::Dusk,
        );
        ctx.place_reminder(&p("Drunk"), sober);
    });
    // Sober, the Drunk is still the Drunk: the Monk ability they believe in
    // does nothing.
    assert!(g.state.is_malfunctioning(&p("Drunk")));
}

#[test]
fn beggar_asking_the_lying_knave_learns_a_false_alignment() {
    let mut g = beggar_game();
    g.run(|ctx| {
        ctx.add_fabled(&characters::knaves::Def);
        ctx.mark_fabled(&characters::knaves::Def, characters::knaves::FIRST_LIES);
        ctx.kill(&p("Monk"), botc_narrator::event::Cause::Demon);
    })
    .quiet()
    .yes_no(&[true])
    .run(|ctx| narrator::day_action(ctx, &p("Beg"), &characters::beggar::Def));
    expect![[r#"
          Beg? Which dead player gives you their vote token? => Monk
        * Monk gives away their vote token
        * Monk (beggar) marks Beg: vote token
          Beg? Do you ask the first storyteller? => yes
          storyteller [1 of 1 options] information for Beg (Beggar): "Evil" (false)
          -> Beg: Evil
        --- grimoire ---
        Imp: Imp
        Monk: Monk [dead]
        Recluse: Recluse
        Chef: Chef
        Mayor: Mayor
        Beg: Beggar [vote token by beggar]
    "#]]
    .assert_eq(&g.transcript());
}
