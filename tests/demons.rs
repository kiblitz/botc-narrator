//! The demons beyond Trouble Brewing: Al-Hadikhia, Legion, Leviathan,
//! Lil' Monsta, Lleech, No Dashii, Po, Pukka, Shabaloth, Vigormortis,
//! Vortox and Zombuul (the Fang Gu is in `new_characters.rs`).

mod common;

use expect_test::expect;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::script;
use botc_narrator::setup;
use botc_narrator::sim;

use common::p;

const DEMONS: [character::Ref; 12] = [
    &characters::al_hadikhia::Def,
    &characters::legion::Def,
    &characters::leviathan::Def,
    &characters::lil_monsta::Def,
    &characters::lleech::Def,
    &characters::no_dashii::Def,
    &characters::po::Def,
    &characters::pukka::Def,
    &characters::shabaloth::Def,
    &characters::vigormortis::Def,
    &characters::vortox::Def,
    &characters::zombuul::Def,
];

/// Trouble Brewing plus every demon here.
fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend(DEMONS);
    script::Script::new("demons", "Demons", chars)
}

/// Advances to the start of night `n` (night-start abilities fire).
fn to_night(ctx: &mut ctx::Write<'_>, n: u32) {
    while ctx.state().phase() != game::Phase::Night(n) {
        ctx.advance_phase();
    }
}

fn poison(ctx: &mut ctx::Write<'_>, target: &str) {
    let reminder = game::Reminder::new(
        &characters::poisoner::Def,
        &p(target),
        game::Effect::Poisoned,
        game::Expiry::Dusk,
    );
    ctx.place_reminder(&p(target), reminder);
}

fn run(ctx: &mut ctx::Write<'_>, c: character::Ref, night: u32) {
    narrator::run_order(ctx, &[c], night);
}

// --- Pukka ----------------------------------------------------------------------

fn pukka_game() -> common::Game {
    common::Game::new(&[
        ("Pukka", &characters::pukka::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn the_pukka_poisons_and_the_previous_victim_dies_next_night() {
    let mut g = pukka_game();
    g.answers(&[&["Chef"], &["Monk"]]).run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::pukka::Def, 1);
        to_night(ctx, 2);
        run(ctx, &characters::pukka::Def, 2);
    });
    expect![[r#"
        * === Night 1 ===
        wake Pukka
          Pukka? Who do you want to poison? => Chef
        sleep Pukka
        * Pukka (pukka) marks Chef: Poisoned
        * === Day 1 ===
        * === Night 2 ===
        wake Pukka
          Pukka? Who do you want to poison? => Monk
        sleep Pukka
        * Chef dies (Demon)
        * Pukka (pukka) removes Chef's Poisoned token
        * Pukka (pukka) marks Monk: Poisoned
        --- grimoire ---
        Pukka: Pukka
        Baron: Baron
        Chef: Chef [dead]
        Monk: Monk [Poisoned by pukka]
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_pukka_neither_kills_nor_poisons_until_healthy() {
    let mut g = pukka_game();
    g.answers(&[&["Chef"], &["Monk"], &["Mayor"]]).run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::pukka::Def, 1);
        to_night(ctx, 2);
        poison(ctx, "Pukka");
        run(ctx, &characters::pukka::Def, 2);
    });
    assert!(g.state.is_alive(&p("Chef")));
    assert!(g.state.has_effect(&p("Chef"), game::Effect::Poisoned));
    assert!(!g.state.has_effect(&p("Monk"), game::Effect::Poisoned));
    // Healthy again on night 3: the Chef, still the previous victim, dies.
    g.run(|ctx| {
        to_night(ctx, 3);
        run(ctx, &characters::pukka::Def, 3);
    });
    assert!(!g.state.is_alive(&p("Chef")));
    assert!(g.state.has_effect(&p("Mayor"), game::Effect::Poisoned));
}

// --- No Dashii ------------------------------------------------------------------

fn no_dashii_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Dashii", &characters::no_dashii::Def),
        ("Saint", &characters::saint::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| narrator::setup(ctx, &script()));
    g
}

#[test]
fn the_no_dashii_poisons_its_nearest_townsfolk_and_kills() {
    let mut g = no_dashii_game();
    g.answers(&[&["Monk"]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::no_dashii::Def, 2);
    });
    // The Saint is skipped: the Chef and the Empath are the Townsfolk neighbours.
    expect![[r#"
        * Dashii (no_dashii) marks Chef: Poisoned
        * Dashii (no_dashii) marks Empath: Poisoned
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Dashii
          Dashii? Who do you want to kill? => Monk
        sleep Dashii
        * Monk dies (Demon)
        --- grimoire ---
        Dashii: No Dashii
        Saint: Saint
        Chef: Chef [Poisoned by no_dashii]
        Baron: Baron
        Monk: Monk [dead]
        Empath: Empath [Poisoned by no_dashii]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_or_dead_no_dashii_poisons_nobody() {
    let mut g = no_dashii_game();
    g.answers(&[&["Monk"]]).run(|ctx| {
        to_night(ctx, 2);
        poison(ctx, "Dashii");
        run(ctx, &characters::no_dashii::Def, 2);
        ctx.advance_phase();
    });
    assert!(g.state.is_alive(&p("Monk")));
    assert!(!g.state.has_effect(&p("Chef"), game::Effect::Poisoned));
    // Healthy again at night 3, then executed: the poison ends with it.
    g.run(|ctx| {
        ctx.advance_phase();
        assert!(ctx.state().has_effect(&p("Chef"), game::Effect::Poisoned));
        ctx.execute(&p("Dashii"));
    });
    assert!(!g.state.has_effect(&p("Chef"), game::Effect::Poisoned));
    assert!(!g.state.has_effect(&p("Empath"), game::Effect::Poisoned));
}

// --- Vortox ---------------------------------------------------------------------

fn vortox_game() -> common::Game {
    common::Game::new(&[
        ("Vortox", &characters::vortox::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn the_vortox_makes_townsfolk_information_false() {
    let mut g = vortox_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::chef::Def, 1);
    });
    expect![[r#"
        * === Night 1 ===
        wake Chef
          storyteller [1 of 2 options] information for Chef (Chef): "0 evil pairs" (false)
          -> Chef: 0 evil pairs
        sleep Chef
        --- grimoire ---
        Vortox: Vortox
        Baron: Baron
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Vortox no longer forces lies: the truth is an option.
    let mut g = vortox_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        poison(ctx, "Vortox");
        run(ctx, &characters::chef::Def, 1);
    });
    assert!(g.transcript().contains("\"1 evil pairs\""));
}

#[test]
fn evil_wins_when_nobody_is_executed_under_the_vortox() {
    let mut g = vortox_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        ctx.execute(&p("Monk"));
        narrator::end_day(ctx);
    });
    assert_eq!(g.state.winner(), None);
    g.run(|ctx| {
        to_night(ctx, 2);
        ctx.advance_phase();
        narrator::end_day(ctx);
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Evil));

    // A poisoned Vortox does not win that way.
    let mut g = vortox_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        poison(ctx, "Vortox");
        narrator::end_day(ctx);
    });
    assert_eq!(g.state.winner(), None);
}

// --- Vigormortis ----------------------------------------------------------------

fn vigormortis_game() -> common::Game {
    common::Game::new(&[
        ("Vigor", &characters::vigormortis::Def),
        ("Chef", &characters::chef::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn a_minion_the_vigormortis_kills_keeps_their_ability_and_poisons_a_neighbour() {
    let mut g = vigormortis_game();
    // Storyteller: the Monk (option 1) is the poisoned neighbour.
    g.answers(&[&["Poisoner"]]).picks(&[&[1]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::vigormortis::Def, 2);
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Vigor
          Vigor? Who do you want to kill? => Poisoner
        sleep Vigor
        * Poisoner dies (Demon)
        * Vigor (vigormortis) marks Poisoner: KeepsAbility
          storyteller [1 of 2 options] Vigormortis Vigor: which Townsfolk neighbour of the killed Minion is poisoned?: Monk
        * Vigor (vigormortis) marks Monk: Poisoned
        --- grimoire ---
        Vigor: Vigormortis
        Chef: Chef
        Poisoner: Poisoner [dead] [KeepsAbility by vigormortis]
        Monk: Monk [Poisoned by vigormortis]
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // The dead Poisoner still wakes and poisons.
    g.answers(&[&["Chef"]]).run(|ctx| {
        to_night(ctx, 3);
        run(ctx, &characters::poisoner::Def, 3);
    });
    assert!(g.state.has_effect(&p("Chef"), game::Effect::Poisoned));

    // Once the Vigormortis dies, the Poisoner's ability and the poison end.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Vigor"));
    });
    assert!(!g.state.ability_active(&p("Poisoner")));
    assert!(!g.state.has_effect(&p("Monk"), game::Effect::Poisoned));
    assert_eq!(
        character::Def::setup_counts(&characters::vigormortis::Def, setup::Counts::base(8)),
        vec![setup::Counts::base(8).shift_outsiders(-1)]
    );
}

#[test]
fn a_poisoned_vigormortis_kills_no_one_and_minions_keep_nothing() {
    let mut g = vigormortis_game();
    g.answers(&[&["Poisoner"]]).run(|ctx| {
        to_night(ctx, 2);
        poison(ctx, "Vigor");
        run(ctx, &characters::vigormortis::Def, 2);
    });
    assert!(g.state.is_alive(&p("Poisoner")));
    assert!(
        !g.state
            .has_effect(&p("Poisoner"), game::Effect::KeepsAbility)
    );
}

// --- Zombuul --------------------------------------------------------------------

fn zombuul_game() -> common::Game {
    common::Game::new(&[
        ("Zombuul", &characters::zombuul::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn the_zombuul_survives_its_first_death_and_sleeps_that_night() {
    let mut g = zombuul_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        ctx.execute(&p("Zombuul"));
        to_night(ctx, 2);
        run(ctx, &characters::zombuul::Def, 2);
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * Zombuul is executed
        * Zombuul (zombuul) marks Zombuul: AbilityUsed
        * Zombuul (Zombuul) seems to die but lives
        * Zombuul survives (zombuul)
        * === Night 2 ===
        --- grimoire ---
        Zombuul: Zombuul [AbilityUsed by zombuul]
        Baron: Baron
        Chef: Chef
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // After a day without deaths it kills; its second death is real.
    g.answers(&[&["Chef"]]).run(|ctx| {
        to_night(ctx, 3);
        run(ctx, &characters::zombuul::Def, 3);
    });
    assert!(!g.state.is_alive(&p("Chef")));
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Zombuul"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn the_zombuul_sleeps_after_a_death_and_dies_for_real_when_poisoned() {
    let mut g = zombuul_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
        to_night(ctx, 2);
        run(ctx, &characters::zombuul::Def, 2);
    });
    assert!(!g.transcript().contains("wake Zombuul"));

    let mut g = zombuul_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        poison(ctx, "Zombuul");
        ctx.execute(&p("Zombuul"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

// --- Po -------------------------------------------------------------------------

fn po_game() -> common::Game {
    common::Game::new(&[
        ("Po", &characters::po::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
        ("Empath", &characters::empath::Def),
        ("Soldier", &characters::soldier::Def),
    ])
}

#[test]
fn the_po_charges_by_choosing_no_one_then_kills_three() {
    let mut g = po_game();
    g.answers(&[&[], &["Chef", "Monk", "Empath"]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::po::Def, 2);
        to_night(ctx, 3);
        run(ctx, &characters::po::Def, 3);
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Po
          Po? Who do you want to kill, if anyone? => 
        sleep Po
        * Po (po) marks Po: 3 attacks
        * === Day 2 ===
        * === Night 3 ===
        wake Po
          Po? Choose 3 players to kill => Chef, Monk, Empath
        sleep Po
        * Po (po) removes Po's 3 attacks token
        * Chef dies (Demon)
        * Monk dies (Demon)
        * Empath dies (Demon)
        --- grimoire ---
        Po: Po
        Baron: Baron
        Chef: Chef [dead]
        Monk: Monk [dead]
        Mayor: Mayor
        Empath: Empath [dead]
        Soldier: Soldier
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_po_still_charges_and_a_failed_kill_does_not() {
    let mut g = po_game();
    g.answers(&[&[], &["Chef", "Monk", "Empath"], &["Soldier"]])
        .run(|ctx| {
            to_night(ctx, 2);
            poison(ctx, "Po");
            run(ctx, &characters::po::Def, 2);
            to_night(ctx, 3);
            poison(ctx, "Po");
            run(ctx, &characters::po::Def, 3);
            to_night(ctx, 4);
            run(ctx, &characters::po::Def, 4);
        });
    let log = g.transcript();
    assert!(log.contains("Choose 3 players to kill"), "{log}");
    assert!(g.state.is_alive(&p("Chef")) && g.state.is_alive(&p("Soldier")));
    // The Soldier survived night 4: night 5 offers one choice again.
    g.answers(&[&["Chef"]]).run(|ctx| {
        to_night(ctx, 5);
        run(ctx, &characters::po::Def, 5);
    });
    assert!(g.transcript().contains("if anyone? => Chef"));
}

// --- Shabaloth ------------------------------------------------------------------

fn shabaloth_game() -> common::Game {
    common::Game::new(&[
        ("Shab", &characters::shabaloth::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
        ("Empath", &characters::empath::Def),
        ("Soldier", &characters::soldier::Def),
    ])
}

#[test]
fn the_shabaloth_kills_two_and_may_regurgitate_one() {
    let mut g = shabaloth_game();
    // Night 3: the storyteller regurgitates ("yes"), choosing the Chef.
    g.answers(&[&["Chef", "Monk"], &["Mayor", "Empath"]])
        .picks(&[&[1], &[0]])
        .run(|ctx| {
            to_night(ctx, 2);
            run(ctx, &characters::shabaloth::Def, 2);
            to_night(ctx, 3);
            ctx.note("--- night 3 ---");
            run(ctx, &characters::shabaloth::Def, 3);
        });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Shab
          Shab? Choose 2 players to kill => Chef, Monk
        sleep Shab
        * Shab (shabaloth) marks Chef: Chosen
        * Shab (shabaloth) marks Monk: Chosen
        * Chef dies (Demon)
        * Monk dies (Demon)
        * === Day 2 ===
        * === Night 3 ===
        * --- night 3 ---
          storyteller [1 of 2 options] Shabaloth Shab: regurgitate a player?: Yes
          storyteller [1 of 2 options] Shabaloth Shab: which player chosen last night is regurgitated?: Chef
        * Chef is alive again
        wake Shab
          Shab? Choose 2 players to kill => Mayor, Empath
        sleep Shab
        * Shab (shabaloth) removes Chef's Chosen token
        * Shab (shabaloth) removes Monk's Chosen token
        * Shab (shabaloth) marks Mayor: Chosen
        * Shab (shabaloth) marks Empath: Chosen
          storyteller [1 of 5 options] the demon attacked Mayor Mayor: who dies instead: Mayor
        * Mayor dies (Demon)
        * Empath dies (Demon)
        --- grimoire ---
        Shab: Shabaloth
        Baron: Baron
        Chef: Chef
        Monk: Monk [dead]
        Mayor: Mayor [dead] [Chosen by shabaloth]
        Empath: Empath [dead] [Chosen by shabaloth]
        Soldier: Soldier
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_shabaloth_kills_no_one() {
    let mut g = shabaloth_game();
    g.answers(&[&["Chef", "Monk"]]).run(|ctx| {
        to_night(ctx, 2);
        poison(ctx, "Shab");
        run(ctx, &characters::shabaloth::Def, 2);
    });
    assert!(g.state.is_alive(&p("Chef")) && g.state.is_alive(&p("Monk")));
}

// --- Lleech ---------------------------------------------------------------------

fn lleech_game() -> common::Game {
    common::Game::new(&[
        ("Lleech", &characters::lleech::Def),
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
        ("Empath", &characters::empath::Def),
    ])
}

#[test]
fn the_lleech_cannot_die_while_its_host_lives_and_dies_with_it() {
    let mut g = lleech_game();
    g.answers(&[&["Chef"], &["Monk"]]).run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::lleech::Def, 1);
        ctx.advance_phase();
        ctx.execute(&p("Lleech"));
        to_night(ctx, 2);
        run(ctx, &characters::lleech::Def, 2);
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
    });
    expect![[r#"
        * === Night 1 ===
        wake Lleech
          Lleech? Choose your host => Chef
        sleep Lleech
        * Lleech (lleech) marks Chef: Host
        * Lleech (lleech) marks Chef: Poisoned
        * === Day 1 ===
        * Lleech is executed
        * Lleech survives (lleech)
        * === Night 2 ===
        wake Lleech
          Lleech? Who do you want to kill? => Monk
        sleep Lleech
        * Monk dies (Demon)
        * === Day 2 ===
        * Chef is executed
        * Chef dies (Execution)
        * Lleech dies (Ability { by: Id("Lleech") })
        * Good wins!
        --- grimoire ---
        Lleech: Lleech [dead]
        Baron: Baron
        Chef: Chef [dead] [Host by lleech] [Poisoned by lleech]
        Monk: Monk [dead]
        Mayor: Mayor
        Empath: Empath
        winner: Good
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_poisoned_lleech_survives_its_hosts_death_and_can_then_die() {
    let mut g = lleech_game();
    g.answers(&[&["Chef"]]).run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::lleech::Def, 1);
        ctx.advance_phase();
        poison(ctx, "Lleech");
        ctx.execute(&p("Chef"));
    });
    assert!(g.state.is_alive(&p("Lleech")));
    g.run(|ctx| {
        to_night(ctx, 2);
        ctx.advance_phase();
        ctx.execute(&p("Lleech"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

// --- Legion ---------------------------------------------------------------------

fn legion_game() -> common::Game {
    common::Game::new(&[
        ("Alice", &characters::legion::Def),
        ("Bob", &characters::legion::Def),
        ("Carol", &characters::legion::Def),
        ("Dan", &characters::legion::Def),
        ("Eve", &characters::chef::Def),
        ("Frank", &characters::empath::Def),
        ("Gina", &characters::monk::Def),
    ])
}

#[test]
fn most_players_are_legion() {
    let only_legion = {
        let mut chars: Vec<character::Ref> = common::trouble_brewing()
            .characters
            .into_iter()
            .filter(|c| {
                matches!(
                    c.kind(),
                    character::Kind::Townsfolk | character::Kind::Outsider
                )
            })
            .collect();
        chars.push(&characters::legion::Def);
        script::Script::new("legion", "Legion", chars)
    };
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(1);
    for players in [5, 7, 10] {
        let chosen = setup::choose(&mut rng, players, &only_legion);
        let legion = chosen.iter().filter(|c| c.id() == "legion").count();
        assert_eq!(legion, players / 2 + 1, "{players} players: {chosen:?}");
        assert_eq!(chosen.len(), players);
    }
}

#[test]
fn an_execution_only_evil_voted_for_fails() {
    let evil = ["Alice", "Bob", "Carol", "Dan"].map(p).to_vec();
    let mut g = legion_game();
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        narrator::vote(ctx, &p("Eve"), evil.clone(), 4);
        ctx.execute(&p("Eve"));
    });
    assert!(g.state.is_alive(&p("Eve")), "{}", g.transcript());

    let mut g = legion_game();
    let mut with_good = evil.clone();
    with_good.push(p("Frank"));
    g.run(|ctx| {
        to_night(ctx, 1);
        ctx.advance_phase();
        narrator::vote(ctx, &p("Eve"), with_good, 4);
        ctx.execute(&p("Eve"));
    });
    assert!(!g.state.is_alive(&p("Eve")));
}

#[test]
fn legion_kills_through_the_storyteller_and_registers_as_a_minion() {
    let mut g = legion_game();
    // Yes, a player dies: Bob (option 1).
    g.picks(&[&[1], &[1]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::legion::Def, 2);
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
          storyteller [1 of 2 options] Legion Alice: does a player die tonight?: Yes
          storyteller [1 of 7 options] Legion Alice: who dies tonight?: Bob
        * Bob dies (Demon)
        --- grimoire ---
        Alice: Legion
        Bob: Legion [dead]
        Carol: Legion
        Dan: Legion
        Eve: Chef
        Frank: Empath
        Gina: Monk
    "#]]
    .assert_eq(&g.transcript());

    let mut g = legion_game();
    g.run(|ctx| ctx.set_script(script().characters));
    g.picks(&[&[1]]);
    let mut registered = None;
    g.run(|ctx| {
        registered = ctx.registers_as(&p("Eve"), &p("Alice"), character::Kind::Minion);
    });
    assert_eq!(registered.map(|c| c.kind()), Some(character::Kind::Minion));
}

// --- Leviathan ------------------------------------------------------------------

fn leviathan_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Levi", &characters::leviathan::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| narrator::setup(ctx, &script()));
    g
}

fn day_with_execution(ctx: &mut ctx::Write<'_>, executed: Option<&str>) {
    let night = match ctx.state().phase() {
        game::Phase::Day(n) | game::Phase::Night(n) => n + 1,
        game::Phase::Setup => 1,
    };
    to_night(ctx, night);
    ctx.advance_phase();
    if let Some(executed) = executed {
        ctx.execute(&p(executed));
    }
    narrator::end_day(ctx);
}

#[test]
fn evil_wins_when_a_second_good_player_is_executed() {
    let mut g = leviathan_game();
    g.run(|ctx| {
        day_with_execution(ctx, Some("Baron"));
        day_with_execution(ctx, Some("Chef"));
    });
    assert_eq!(g.state.winner(), None);
    g.run(|ctx| day_with_execution(ctx, Some("Empath")));
    expect![[r#"
        * Announcement: The Leviathan is in play
        * === Night 1 ===
        * === Day 1 ===
        * Baron is executed
        * Baron dies (Execution)
        * === Night 2 ===
        * === Day 2 ===
        * Chef is executed
        * Chef dies (Execution)
        * Levi (leviathan) marks Chef: Good player executed
        * === Night 3 ===
        * === Day 3 ===
        * Empath is executed
        * Empath dies (Execution)
        * Evil wins!
        --- grimoire ---
        Levi: Leviathan
        Chef: Chef [dead] [Good player executed by leviathan]
        Empath: Empath [dead]
        Baron: Baron [dead]
        Monk: Monk
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn evil_wins_after_day_five_unless_the_leviathan_is_poisoned() {
    let mut g = leviathan_game();
    g.run(|ctx| {
        for _ in 0..4 {
            day_with_execution(ctx, None);
        }
    });
    assert_eq!(g.state.winner(), None);
    g.run(|ctx| day_with_execution(ctx, None));
    assert_eq!(g.state.winner(), Some(character::Alignment::Evil));

    let mut g = leviathan_game();
    g.run(|ctx| {
        for _ in 0..4 {
            day_with_execution(ctx, None);
        }
        to_night(ctx, 5);
        ctx.advance_phase();
        poison(ctx, "Levi");
        narrator::end_day(ctx);
    });
    assert_eq!(g.state.winner(), None);
}

// --- Lil' Monsta ----------------------------------------------------------------

fn lil_monsta_game() -> common::Game {
    let mut g = common::Game::new(&[
        ("Monsta", &characters::lil_monsta::Def),
        ("Spy", &characters::spy::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Saint", &characters::saint::Def),
    ]);
    // The player dealt Lil' Monsta becomes the Poisoner.
    let poisoner = script()
        .characters
        .iter()
        .filter(|c| c.kind() == character::Kind::Minion && c.id() != "spy" && c.id() != "baron")
        .position(|c| c.id() == "poisoner")
        .expect("the Poisoner is on the script");
    g.picks(&[&[poisoner]])
        .run(|ctx| narrator::setup(ctx, &script()));
    g
}

#[test]
fn the_player_dealt_lil_monsta_becomes_a_minion_and_babysits() {
    let mut g = lil_monsta_game();
    // Both Minions point at the Spy.
    g.answers(&[&["Spy"], &["Spy"]]).run(|ctx| {
        to_night(ctx, 1);
        run(ctx, &characters::lil_monsta::Def, 1);
    });
    expect![[r#"
          storyteller [1 of 2 options] Lil' Monsta Monsta: which Minion does the player dealt Lil' Monsta become?: Poisoner
        * Monsta becomes the Poisoner
        * Monsta gains the Lil' Monsta ability
        * === Night 1 ===
        wake Monsta
          Monsta? Who babysits Lil' Monsta? => Spy
        sleep Monsta
        wake Spy
          Spy? Who babysits Lil' Monsta? => Spy
        sleep Spy
          storyteller [1 of 1 options] Lil' Monsta Monsta: the Minions disagree: who babysits?: Spy
        * Monsta loses the Lil' Monsta ability
        * Spy gains the Lil' Monsta ability
        --- grimoire ---
        Monsta: Poisoner
        Spy: Spy (+Lil' Monsta)
        Chef: Chef
        Empath: Empath
        Monk: Monk
        Saint: Saint
    "#]]
    .assert_eq(&g.transcript());

    // Executing the other Minion changes nothing; the babysitter's death wins for good.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Monsta"));
    });
    assert_eq!(g.state.winner(), None);
    g.run(|ctx| {
        ctx.execute(&p("Spy"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn a_player_might_die_but_never_the_babysitter() {
    let mut g = lil_monsta_game();
    // Night 2: the Minions agree on Monsta; yes, someone dies: the first option.
    g.answers(&[&["Monsta"], &["Monsta"]])
        .picks(&[&[1], &[0]])
        .run(|ctx| {
            to_night(ctx, 2);
            run(ctx, &characters::lil_monsta::Def, 2);
        });
    assert!(g.state.is_alive(&p("Monsta")));
    assert!(!g.state.is_alive(&p("Spy")), "{}", g.transcript());
}

// --- Al-Hadikhia ----------------------------------------------------------------

fn al_hadikhia_game() -> common::Game {
    common::Game::new(&[
        ("Al", &characters::al_hadikhia::Def),
        ("Bob", &characters::chef::Def),
        ("Carol", &characters::empath::Def),
        ("Dan", &characters::poisoner::Def),
        ("Eve", &characters::monk::Def),
    ])
}

#[test]
fn the_al_hadikhias_chosen_choose_to_live_or_die() {
    let mut g = al_hadikhia_game();
    // Bob lives, Carol dies (no-one), Dan lives.
    g.answers(&[&["Bob", "Carol", "Dan"], &["Bob"], &[], &["Dan"]])
        .run(|ctx| {
            to_night(ctx, 2);
            run(ctx, &characters::al_hadikhia::Def, 2);
        });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * === Night 2 ===
        wake Al
          Al? Choose 3 players, if any => Bob, Carol, Dan
        sleep Al
        * Announcement: The Al-Hadikhia chose Bob, Carol, Dan
        wake Bob
          Bob? You were chosen: point at yourself to live, or at no-one to die => Bob
        sleep Bob
        wake Carol
          Carol? You were chosen: point at yourself to live, or at no-one to die => 
        sleep Carol
        * Carol dies (Demon)
        wake Dan
          Dan? You were chosen: point at yourself to live, or at no-one to die => Dan
        sleep Dan
        --- grimoire ---
        Al: Al-Hadikhia
        Bob: Chef
        Carol: Empath [dead]
        Dan: Poisoner
        Eve: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Night 3: Carol chooses life and returns; all three live, so all die.
    g.answers(&[&["Bob", "Carol", "Dan"], &["Bob"], &["Carol"], &["Dan"]])
        .run(|ctx| {
            to_night(ctx, 3);
            run(ctx, &characters::al_hadikhia::Def, 3);
        });
    let log = g.transcript();
    assert!(log.contains("Carol is alive again"), "{log}");
    for name in ["Bob", "Carol", "Dan"] {
        assert!(!g.state.is_alive(&p(name)), "{log}");
    }
}

#[test]
fn a_poisoned_al_hadikhia_or_choosing_no_one_changes_nothing() {
    let mut g = al_hadikhia_game();
    g.answers(&[&[]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::al_hadikhia::Def, 2);
    });
    assert!(!g.transcript().contains("Announcement"));

    let mut g = al_hadikhia_game();
    g.answers(&[&["Bob", "Carol", "Eve"], &["Bob"], &["Carol"], &["Eve"]])
        .run(|ctx| {
            to_night(ctx, 2);
            poison(ctx, "Al");
            run(ctx, &characters::al_hadikhia::Def, 2);
        });
    assert_eq!(g.state.alive_ids().len(), 5);
}

// --- Simulation -----------------------------------------------------------------

#[test]
fn simulated_games_with_every_demon_terminate() {
    let script = script();
    let mut seen: Vec<&str> = Vec::new();
    for seed in 0..300 {
        let players = sim::MIN_PLAYERS + (seed as usize % 11);
        let trace = sim::simulate(seed, players, &script);
        assert!(!trace.steps.is_empty());
        for seat in &trace.snapshots[0] {
            if seat.kind == character::Kind::Demon && !seen.contains(&seat.character) {
                seen.push(seat.character);
            }
        }
    }
    // The first snapshot is from before setup, when Lil' Monsta is still dealt.
    for demon in DEMONS {
        assert!(
            seen.contains(&demon.name()),
            "{} never played",
            demon.name()
        );
    }
}

#[test]
fn a_chosen_player_who_chose_death_but_survived_is_not_living_by_choice() {
    let mut g = common::Game::new(&[
        ("Al", &characters::al_hadikhia::Def),
        ("Bob", &characters::chef::Def),
        ("Soldier", &characters::soldier::Def),
        ("Dan", &characters::poisoner::Def),
        ("Eve", &characters::monk::Def),
    ]);
    // The Soldier chooses death (no-one) and survives; the others choose life.
    g.answers(&[&["Bob", "Soldier", "Dan"], &["Bob"], &[], &["Dan"]])
        .run(|ctx| {
            to_night(ctx, 2);
            run(ctx, &characters::al_hadikhia::Def, 2);
        });
    assert_eq!(g.state.alive_ids().len(), 5, "{}", g.transcript());
}

#[test]
fn a_poisoned_babysitter_is_still_the_demon_and_a_player_still_dies() {
    // Lil' Monsta is not a player: poisoning its babysitter does not stop it.
    let mut g = lil_monsta_game();
    g.answers(&[&["Monsta"], &["Monsta"]])
        .picks(&[&[1], &[0]])
        .run(|ctx| {
            to_night(ctx, 2);
            poison(ctx, "Monsta");
            run(ctx, &characters::lil_monsta::Def, 2);
        });
    assert!(!g.state.is_alive(&p("Spy")), "{}", g.transcript());
    assert!(g.state.demon_alive());
}

#[test]
fn a_godfather_the_vigormortis_kills_still_wakes_after_an_outsider_dies() {
    let mut g = common::Game::new(&[
        ("Vigor", &characters::vigormortis::Def),
        ("Chef", &characters::chef::Def),
        ("God", &characters::godfather::Def),
        ("Monk", &characters::monk::Def),
        ("Recluse", &characters::recluse::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.answers(&[&["God"], &["Monk"]]).picks(&[&[0]]).run(|ctx| {
        to_night(ctx, 2);
        run(ctx, &characters::vigormortis::Def, 2);
        ctx.advance_phase();
        ctx.execute(&p("Recluse"));
        to_night(ctx, 3);
        run(ctx, &characters::godfather::Def, 3);
    });
    assert!(!g.state.is_alive(&p("Monk")), "{}", g.transcript());
}
