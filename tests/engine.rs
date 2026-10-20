//! Engine foundations: kinds, believed characters, registration, setup
//! adjustments, hooks, data night orders and script loading. Characters
//! that only exist to exercise a hook are defined here.

mod common;

use expect_test::expect;
use rand::SeedableRng;

use botc_narrator::character;
use botc_narrator::characters;
use botc_narrator::ctx;
use botc_narrator::event;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::player;
use botc_narrator::script;
use botc_narrator::setup;

use common::p;

fn night(ctx: &mut ctx::Write<'_>) {
    ctx.advance_phase();
    narrator::run_night(ctx, &common::trouble_brewing());
}

// --- Kinds ---------------------------------------------------------------

struct Traveller;

impl character::Def for Traveller {
    fn id(&self) -> &'static str {
        "test_traveller"
    }
    fn name(&self) -> &'static str {
        "Test Traveller"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }
}

struct Fabled;

impl character::Def for Fabled {
    fn id(&self) -> &'static str {
        "test_fabled"
    }
    fn name(&self) -> &'static str {
        "Test Fabled"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Fabled
    }
}

#[test]
fn travellers_choose_their_alignment_and_stay_out_of_evil_info() {
    let mut seats: Vec<game::Seat> = [
        ("Imp", &characters::imp::Def as character::Ref),
        ("Poisoner", &characters::poisoner::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Slayer", &characters::slayer::Def),
        ("Mayor", &characters::mayor::Def),
    ]
    .into_iter()
    .map(|(name, c)| game::Seat::new(p(name), c))
    .collect();
    seats.push(game::Seat::traveller(
        p("Guest"),
        &Traveller,
        character::Alignment::Evil,
    ));
    let mut g = common::Game::new(&[]);
    g.state = game::State::seated(seats);
    assert_eq!(g.state.resident_count(), 7);
    assert!(g.state.is_evil(&p("Guest")));
    g.answers(&[&["Chef"]]).run(night);
    let log = g.transcript();
    assert!(log.contains("-> Poisoner: Evil: Imp, Poisoner\n"), "{log}");
    assert!(!log.contains("Evil: Imp, Poisoner, Guest"), "{log}");
}

#[test]
#[should_panic(expected = "has no starting alignment")]
fn travellers_need_an_alignment_to_be_seated() {
    game::Seat::new(p("Guest"), &Traveller);
}

#[test]
fn fabled_are_in_play_but_never_seated() {
    let mut g = common::Game::new(&[("Imp", &characters::imp::Def)]);
    g.run(|ctx| ctx.add_fabled(&Fabled));
    assert_eq!(g.state.fabled().len(), 1);
    assert_eq!(g.state.players().len(), 1);
    assert_eq!(character::Kind::Fabled.alignment(), None);
    assert!(!character::Kind::Fabled.in_setup());
    assert!(!character::Kind::Traveller.in_setup());
    expect![[r#"
        * the Test Fabled is in play
        --- grimoire ---
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
}

// --- Believed characters (the Drunk) -------------------------------------

#[test]
fn the_drunk_believes_a_townsfolk_and_acts_as_it_with_false_info() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Drunk", &characters::drunk::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Slayer", &characters::slayer::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    // Setup: the Drunk believes they are the Chef (option 3 of the
    // not-in-play Townsfolk). Night 1: bluffs, then the drunk "Chef" is told
    // the false 2 (option 2).
    g.picks(&[&[3], &[0, 1, 2], &[2]])
        .answers(&[&["Empath"]])
        .run(|ctx| narrator::setup(ctx, &common::trouble_brewing()))
        .run(night);
    expect![[r#"
          storyteller [1 of 9 options] Drunk Drunk: which Townsfolk they believe they are: Chef
        * Drunk believes they are the Chef
        * === Night 1 ===
        wake Poisoner
          -> Poisoner: Evil: Imp, Poisoner
        sleep Poisoner
        wake Imp
          -> Imp: Evil: Imp, Poisoner
          storyteller [3 of 11 options] demon bluffs (not-in-play good characters): Washerwoman, Librarian, Investigator
          -> Imp: Bluffs: Washerwoman, Librarian, Investigator
        sleep Imp
        wake Poisoner
          Poisoner? Who do you want to poison? => Empath
        * Poisoner (poisoner) marks Empath: Poisoned
        sleep Poisoner
        === parallel (2) ===
        wake Drunk
          storyteller [1 of 3 options] information for Drunk (Chef), who is poisoned or drunk: false info is allowed: "2 evil pairs" (false)
          -> Drunk: 2 evil pairs
        sleep Drunk
        wake Empath
          storyteller [1 of 3 options] information for Empath (Empath), who is poisoned or drunk: false info is allowed: "0 evil neighbours"
          -> Empath: 0 evil neighbours
        sleep Empath
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Drunk: Drunk (believes Chef)
        Empath: Empath [Poisoned by poisoner]
        Monk: Monk
        Slayer: Slayer
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
    // The believed Chef is never offered as a demon bluff.
    assert!(botc_narrator::ability::in_play(
        &g.state,
        &characters::chef::Def
    ));
    assert!(g.state.is_malfunctioning(&p("Drunk")));
    assert_eq!(g.state.character(&p("Drunk")).id(), "drunk");
}

#[test]
fn a_drunk_soldier_dies_to_the_demon_and_a_drunk_slayer_shoots_blanks() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Baron", &characters::baron::Def),
        ("Drunk", &characters::drunk::Def),
        ("Slayer", &characters::slayer::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| ctx.set_believed(&p("Drunk"), Some(&characters::soldier::Def)));
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .answers(&[&["Drunk"]])
    .quiet()
    .run(|ctx| narrator::run_night(ctx, &common::trouble_brewing()));
    assert!(!g.state.is_alive(&p("Drunk")), "{}", g.transcript());

    // A Drunk who believes they are the Slayer uses the shot for nothing.
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Drunk", &characters::drunk::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| ctx.set_believed(&p("Drunk"), Some(&characters::slayer::Def)))
        .answers(&[&["Imp"]])
        .quiet()
        .run(|ctx| narrator::day_action(ctx, &p("Drunk"), &characters::slayer::Def));
    expect![[r#"
        * Drunk (slayer) marks Drunk: AbilityUsed
          Drunk? Who do you want to slay? => Imp
        --- grimoire ---
        Imp: Imp
        Drunk: Drunk (believes Slayer) [AbilityUsed by slayer]
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

// --- Registration ----------------------------------------------------------

#[test]
fn the_recluse_may_register_as_evil_unless_poisoned() {
    let seats: [(&str, character::Ref); 5] = [
        ("Imp", &characters::imp::Def),
        ("Recluse", &characters::recluse::Def),
        ("Empath", &characters::empath::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ];
    let mut g = common::Game::new(&seats);
    // The storyteller has the Recluse register as evil (option 1).
    g.picks(&[&[1]])
        .run(|ctx| narrator::run_order(ctx, &[&characters::empath::Def], 1));
    expect![[r#"
          storyteller [1 of 2 options] how Recluse registers to Empath (first option is true): Evil
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Recluse: Recluse
        Empath: Empath
        Chef: Chef
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Recluse registers truly: no decision is asked.
    let mut g = common::Game::new(&seats);
    g.run(|ctx| {
        let poison = game::Reminder::new(
            &characters::poisoner::Def,
            &p("Imp"),
            game::Effect::Poisoned,
            game::Expiry::Dusk,
        );
        ctx.place_reminder(&p("Recluse"), poison);
    })
    .quiet()
    .run(|ctx| narrator::run_order(ctx, &[&characters::empath::Def], 1));
    let log = g.transcript();
    assert!(!log.contains("registers"), "{log}");
    assert!(log.contains("-> Empath: 0 evil neighbours"), "{log}");
}

#[test]
fn the_slayer_kills_a_recluse_registering_as_the_demon() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Recluse", &characters::recluse::Def),
        ("Slayer", &characters::slayer::Def),
    ]);
    // Options: not a Demon (true), then the in-play Imp.
    g.picks(&[&[1]])
        .answers(&[&["Recluse"]])
        .run(|ctx| narrator::day_action(ctx, &p("Slayer"), &characters::slayer::Def));
    expect![[r#"
        * Slayer (slayer) marks Slayer: AbilityUsed
          Slayer? Who do you want to slay? => Recluse
          storyteller [1 of 2 options] how Recluse registers to Slayer (first option is true): Imp
        * Recluse dies (Ability { by: Id("Slayer") })
        * Evil wins!
        --- grimoire ---
        Imp: Imp
        Recluse: Recluse [dead]
        Slayer: Slayer [AbilityUsed by slayer]
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn the_spy_may_register_as_a_townsfolk_to_the_virgin() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Spy", &characters::spy::Def),
        ("Virgin", &characters::virgin::Def),
        ("Chef", &characters::chef::Def),
    ]);
    // Options: not a Townsfolk (true), Virgin, Chef.
    g.picks(&[&[2]])
        .run(|ctx| narrator::nominate(ctx, &p("Spy"), &p("Virgin")));
    expect![[r#"
        * Spy nominates Virgin
        * Virgin (virgin) marks Virgin: AbilityUsed
          storyteller [1 of 3 options] how Spy registers to Virgin (first option is true): Chef
        * Spy is executed
        * Spy dies (Execution)
        --- grimoire ---
        Imp: Imp
        Spy: Spy [dead]
        Virgin: Virgin [AbilityUsed by virgin]
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn the_undertaker_and_ravenkeeper_learn_registered_characters() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Spy", &characters::spy::Def),
        ("Undertaker", &characters::undertaker::Def),
        ("Chef", &characters::chef::Def),
        ("Raven", &characters::ravenkeeper::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.execute(&p("Spy"));
    })
    .quiet()
    // The Spy's options: Spy (true), then the in-play Townsfolk in seat order.
    .picks(&[&[2], &[1]])
    .answers(&[&["Chef"], &["Raven"], &["Spy"]])
    .run(night);
    expect![[r#"
        * === Night 2 ===
        wake Monk
          Monk? Who do you want to protect? => Chef
        * Monk (monk) marks Chef: Protected
        sleep Monk
        wake Imp
          Imp? Who do you want to kill? => Raven
        sleep Imp
        * Raven dies (Demon)
        === parallel (2) ===
        wake Raven
          Raven? Choose a player to learn about => Spy
          storyteller [1 of 5 options] how Spy registers to Raven (first option is true): Chef
          storyteller [1 of 1 options] information for Raven (Ravenkeeper): "Chef"
          -> Raven: Chef
        sleep Raven
        wake Undertaker
          storyteller [1 of 5 options] how Spy registers to Undertaker (first option is true): Undertaker
          storyteller [1 of 1 options] information for Undertaker (Undertaker): "Undertaker"
          -> Undertaker: Undertaker
        sleep Undertaker
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Spy: Spy [dead]
        Undertaker: Undertaker
        Chef: Chef [Protected by monk]
        Raven: Ravenkeeper [dead]
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn the_fortune_tellers_red_herring_registers_as_the_demon() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Baron", &characters::baron::Def),
        ("Teller", &characters::fortune_teller::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ]);
    // Good players: Teller, Chef, Monk; the red herring is the Chef.
    g.picks(&[&[1]])
        .answers(&[&["Chef", "Monk"]])
        .run(|ctx| narrator::setup(ctx, &common::trouble_brewing()))
        .run(night);
    expect![[r#"
          storyteller [1 of 3 options] Fortune Teller Teller: which good player is the red herring: Chef
        * Teller (fortune_teller) marks Chef: red herring
        * === Night 1 ===
        === parallel (2) ===
        wake Chef
          storyteller [1 of 1 options] information for Chef (Chef): "1 evil pairs"
          -> Chef: 1 evil pairs
        sleep Chef
        wake Teller
          Teller? Choose two players => Chef, Monk
          storyteller [1 of 1 options] information for Teller (Fortune Teller): "Yes"
          -> Teller: Yes
        sleep Teller
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Baron: Baron
        Teller: Fortune Teller
        Chef: Chef [red herring by fortune_teller]
        Monk: Monk
    "#]]
    .assert_eq(&g.transcript());
}

// --- Setup counts ------------------------------------------------------------

/// A Godfather-like Minion: one more or one fewer Outsider.
struct PlusMinusOne;

impl character::Def for PlusMinusOne {
    fn id(&self) -> &'static str {
        "test_plus_minus_one"
    }
    fn name(&self) -> &'static str {
        "Plus Minus One"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts.shift_outsiders(1), counts.shift_outsiders(-1)]
    }
}

/// A Riot-like Demon: every Minion slot is another copy of it.
struct Horde;

impl character::Def for Horde {
    fn id(&self) -> &'static str {
        "test_horde"
    }
    fn name(&self) -> &'static str {
        "Horde"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![
            counts
                .with(character::Kind::Demon, counts.demons + counts.minions)
                .with(character::Kind::Minion, 0),
        ]
    }
    fn duplicates_in_setup(&self) -> bool {
        true
    }
}

fn kinds(characters: &[character::Ref]) -> [usize; 4] {
    let mut counts = [0; 4];
    for c in characters {
        counts[c.kind() as usize] += 1;
    }
    counts
}

fn tb_with(extra: &[character::Ref], without_kind: Option<character::Kind>) -> script::Script {
    let base = common::trouble_brewing();
    let mut characters: Vec<character::Ref> = base
        .characters
        .into_iter()
        .filter(|c| Some(c.kind()) != without_kind)
        .collect();
    characters.extend_from_slice(extra);
    script::Script::new("test", "Test", characters)
}

#[test]
fn setup_counts_follow_each_chosen_characters_adjustment() {
    assert_eq!(
        setup::Counts::base(10),
        setup::Counts {
            townsfolk: 7,
            outsiders: 0,
            minions: 2,
            demons: 1
        }
    );
    // Only the +-1 Minion: 9 players start at 5/2/1/1.
    let script = tb_with(&[&PlusMinusOne], Some(character::Kind::Minion));
    let mut seen = Vec::new();
    for seed in 0..30 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let chosen = setup::choose(&mut rng, 9, &script);
        assert_eq!(chosen.len(), 9);
        let counts = kinds(&chosen);
        assert!(
            counts == [4, 3, 1, 1] || counts == [6, 1, 1, 1],
            "seed {seed}: {counts:?}"
        );
        seen.push(counts);
    }
    assert!(seen.contains(&[4, 3, 1, 1]) && seen.contains(&[6, 1, 1, 1]));
}

#[test]
fn a_duplicating_demon_fills_the_minion_slots_with_copies() {
    let script = tb_with(&[&Horde], Some(character::Kind::Demon));
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    let chosen = setup::choose(&mut rng, 13, &script);
    assert_eq!(kinds(&chosen), [9, 0, 0, 4]);
    assert!(
        chosen
            .iter()
            .all(|c| c.kind() != character::Kind::Demon || c.id() == "test_horde")
    );
}

#[test]
fn the_baron_adds_two_outsiders() {
    let script = tb_with(&[], None);
    for seed in 0..60 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let chosen = setup::choose(&mut rng, 7, &script);
        let expected = if chosen.iter().any(|c| c.id() == "baron") {
            [3, 2, 1, 1]
        } else {
            [5, 0, 1, 1]
        };
        assert_eq!(kinds(&chosen), expected, "seed {seed}");
    }
}

// --- Hooks -----------------------------------------------------------------

/// Logs every hook it gets as a note.
struct Watcher;

impl character::Def for Watcher {
    fn id(&self) -> &'static str {
        "test_watcher"
    }
    fn name(&self) -> &'static str {
        "Watcher"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }
    fn on_night_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        ctx.note(format!("{me}: night starts"));
    }
    fn on_day_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        ctx.note(format!("{me}: day starts"));
    }
    fn on_day_end(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        ctx.note(format!("{me}: day ends"));
    }
    fn on_nominates(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, nominee: &player::Id) {
        ctx.note(format!("{me} nominated {nominee}"));
    }
    fn on_vote(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        nominee: &player::Id,
        voters: &[player::Id],
    ) {
        ctx.note(format!("{me} saw {} vote on {nominee}", voters.len()));
    }
}

#[test]
fn phase_nomination_and_vote_hooks_fire() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Watcher", &Watcher),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::nominate(ctx, &p("Watcher"), &p("Imp"));
        narrator::vote(ctx, &p("Imp"), vec![p("Watcher"), p("Chef")], 2);
        narrator::end_day(ctx);
    });
    expect![[r#"
        * === Night 1 ===
        * Watcher: night starts
        * === Day 1 ===
        * Watcher: day starts
        * Watcher nominates Imp
        * Watcher nominated Imp
        * 2 votes for Imp (2 needed)
        * Watcher saw 2 vote on Imp
        * Watcher: day ends
        --- grimoire ---
        Imp: Imp
        Watcher: Watcher
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

/// Pacifist-like: executed good players do not die.
struct Pacifist;

impl character::Def for Pacifist {
    fn id(&self) -> &'static str {
        "test_pacifist"
    }
    fn name(&self) -> &'static str {
        "Pacifist"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        if *cause == event::Cause::Execution && !ctx.state().is_evil(target) {
            character::Interception::Prevent
        } else {
            character::Interception::Pass
        }
    }
}

#[test]
fn abilities_can_prevent_deaths_other_than_demon_kills() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Pacifist", &Pacifist),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| {
        assert!(!ctx.execute(&p("Chef")));
    });
    expect![[r#"
        * Chef is executed
        * Chef survives (test_pacifist)
        --- grimoire ---
        Imp: Imp
        Pacifist: Pacifist
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Pacifist protects nobody.
    g.run(|ctx| {
        let poison = game::Reminder::new(
            &characters::poisoner::Def,
            &p("Imp"),
            game::Effect::Poisoned,
            game::Expiry::Dusk,
        );
        ctx.place_reminder(&p("Pacifist"), poison);
        assert!(ctx.execute(&p("Empath")));
    });
    assert!(!g.state.is_alive(&p("Empath")));
}

/// Kills its chosen neighbour at the start of each day: an extra death
/// outside the night order.
struct Grudge;

impl character::Def for Grudge {
    fn id(&self) -> &'static str {
        "test_grudge"
    }
    fn name(&self) -> &'static str {
        "Grudge"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }
    fn on_day_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let victims = ctx.state().alive_except(me);
        if let Some(victim) = victims.first() {
            ctx.kill(victim, event::Cause::Ability { by: me.clone() });
        }
    }
}

#[test]
fn hooks_can_add_extra_deaths() {
    let mut g = common::Game::new(&[
        ("Chef", &characters::chef::Def),
        ("Grudge", &Grudge),
        ("Imp", &characters::imp::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    assert!(!g.state.is_alive(&p("Chef")), "{}", g.transcript());
}

/// Wakes every night, but only on even nights.
struct EvenNights;

impl character::Def for EvenNights {
    fn id(&self) -> &'static str {
        "test_even_nights"
    }
    fn name(&self) -> &'static str {
        "Even Nights"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(2),
            other: Some(2),
        }
    }
    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(|ctx, me, night| {
            ctx.wake(me);
            ctx.tell(
                me,
                &botc_narrator::info::Message::Count {
                    count: night as usize,
                    noun: "nights",
                },
            );
            ctx.sleep(me);
        }))
    }
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && matches!(state.phase(), game::Phase::Night(n) if n % 2 == 0)
    }
}

#[test]
fn wakes_tonight_decides_who_wakes_when_their_turn_comes() {
    let script = tb_with(&[&EvenNights], None);
    let mut g = common::Game::new(&[("Imp", &characters::imp::Def), ("Even", &EvenNights)]);
    g.answers(&[&["Even"]]);
    g.run(|ctx| {
        ctx.advance_phase();
        narrator::run_night(ctx, &script);
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::run_night(ctx, &script);
    });
    let log = g.transcript();
    assert!(!log.contains("1 nights"), "{log}");
    assert!(log.contains("-> Even: 2 nights"), "{log}");
}

#[test]
fn gained_abilities_act_in_the_night_order_until_lost() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Monk", &characters::monk::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| ctx.gain_ability(&p("Monk"), &characters::empath::Def))
        .run(|ctx| narrator::run_order(ctx, &[&characters::empath::Def], 1));
    expect![[r#"
        * Monk gains the Empath ability
        wake Monk
          storyteller [1 of 1 options] information for Monk (Monk): "1 evil neighbours"
          -> Monk: 1 evil neighbours
        sleep Monk
        --- grimoire ---
        Imp: Imp
        Monk: Monk (+Empath)
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    g.run(|ctx| {
        ctx.lose_ability(&p("Monk"), &characters::empath::Def);
        ctx.lose_ability(&p("Chef"), &characters::chef::Def);
    })
    .run(|ctx| narrator::run_order(ctx, &[&characters::empath::Def, &characters::chef::Def], 1));
    expect![[r#"
        * Monk loses the Empath ability
        * Chef loses the Chef ability
        --- grimoire ---
        Imp: Imp
        Monk: Monk
        Chef: Chef (lost ability)
    "#]]
    .assert_eq(&g.transcript());
}

/// Cerenovus-like: makes a player mad about being the Chef, and has them
/// executed if they break madness.
struct Maddener;

impl character::Def for Maddener {
    fn id(&self) -> &'static str {
        "test_maddener"
    }
    fn name(&self) -> &'static str {
        "Maddener"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }
    fn on_day_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let reminder =
            game::Reminder::new(&Maddener, me, game::Effect::Mad("Chef"), game::Expiry::Dusk);
        ctx.place_reminder(&p("Monk"), reminder);
    }
    fn on_madness_broken(&self, ctx: &mut ctx::Write<'_>, _me: &player::Id, mad: &player::Id) {
        ctx.execute(mad);
    }
}

#[test]
fn breaking_madness_lets_the_maddening_ability_react() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Maddener", &Maddener),
        ("Monk", &characters::monk::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::check_madness(ctx);
    });
    assert!(g.state.is_alive(&p("Monk")));
    g.table.breaks_madness.push(p("Monk"));
    g.quiet().run(narrator::check_madness);
    expect![[r#"
          Monk mad about Chef: broke = true
        * Monk broke madness about the Chef
        * Monk is executed
        * Monk dies (Execution)
        --- grimoire ---
        Imp: Imp
        Maddener: Maddener
        Monk: Monk [dead] [Mad (Chef) by test_maddener]
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- Night order as data ------------------------------------------------------

fn ids(order: &[character::Ref]) -> Vec<&'static str> {
    order
        .iter()
        .filter(|c| c.night_action().is_some())
        .map(|c| c.id())
        .collect()
}

#[test]
fn trouble_brewing_night_orders_come_from_character_data() {
    let tb = common::trouble_brewing();
    assert_eq!(
        ids(&tb.first_night),
        [
            "poisoner",
            "washerwoman",
            "librarian",
            "investigator",
            "chef",
            "empath",
            "fortune_teller",
            "butler",
            "spy"
        ]
    );
    assert_eq!(
        ids(&tb.other_nights),
        [
            "poisoner",
            "monk",
            "imp",
            "ravenkeeper",
            "empath",
            "fortune_teller",
            "undertaker",
            "butler",
            "spy"
        ]
    );
}

/// Wakes at position 2 of the first night (Philosopher-like).
struct Early;

impl character::Def for Early {
    fn id(&self) -> &'static str {
        "test_early"
    }
    fn name(&self) -> &'static str {
        "Early"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(2),
            other: None,
        }
    }
    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(|ctx, me, _night| {
            ctx.wake(me);
            ctx.sleep(me);
        }))
    }
}

#[test]
fn early_first_night_characters_act_before_the_evil_info() {
    let script = tb_with(&[&Early], None);
    assert_eq!(script.first_night[0].id(), "test_early");
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Early", &Early),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.answers(&[&["Chef"]]).run(|ctx| {
        ctx.advance_phase();
        narrator::run_night(ctx, &script);
    });
    let log = g.transcript();
    assert!(
        log.starts_with(
            "* === Night 1 ===
wake Early
sleep Early
wake Poisoner
  -> Poisoner: Evil"
        ),
        "{log}"
    );
}

// --- Script loading ---------------------------------------------------------

#[test]
fn trouble_brewing_loads_from_its_json() {
    let tb = common::trouble_brewing();
    assert_eq!(tb.name, "Trouble Brewing");
    assert_eq!(tb.characters.len(), 22);
    assert_eq!(script::all().len(), 1);
}

#[test]
fn unknown_character_ids_are_listed() {
    let json = r#"[{"id": "_meta", "name": "Odd"}, "chef", "nonsense", {"id": "gibberish"}]"#;
    let error = script::parse("odd", json).err().expect("unknown ids");
    assert_eq!(
        error,
        script::Error::UnknownCharacters(vec!["nonsense".into(), "gibberish".into()])
    );
    assert_eq!(
        error.to_string(),
        "unknown character ids: nonsense, gibberish"
    );
}

#[test]
fn scripts_accept_object_entries_and_need_a_name() {
    let script = script::parse(
        "x",
        r#"[{"id": "_meta", "name": "X"}, {"id": "scarletwoman"}]"#,
    )
    .expect("valid");
    assert_eq!(script.characters[0].id(), "scarlet_woman");
    assert_eq!(
        script::parse("x", r#"["chef"]"#).err(),
        Some(script::Error::MissingName)
    );
    assert!(matches!(
        script::parse("x", "{"),
        Err(script::Error::Json(_))
    ));
}

#[test]
fn every_target_script_is_well_formed() {
    // Character modules arrive script by script: until then a target script
    // may only fail for its not-yet-registered characters.
    for entry in std::fs::read_dir("data/scripts").expect("data/scripts") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path).expect("readable");
        match script::parse("check", &json) {
            Ok(_) | Err(script::Error::UnknownCharacters(_)) => {}
            Err(e) => panic!("{}: {e}", path.display()),
        }
    }
}

#[test]
fn registry_ids_are_unique() {
    let all = characters::all();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(
                a.id().replace('_', ""),
                b.id().replace('_', ""),
                "duplicate character id"
            );
        }
    }
}

/// Gives the Monk the Empath ability when it acts (a Philosopher-like gain
/// in the middle of the night).
struct Gifter;

impl character::Def for Gifter {
    fn id(&self) -> &'static str {
        "test_gifter"
    }
    fn name(&self) -> &'static str {
        "Gifter"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }
    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(|ctx, _me, _night| {
            ctx.gain_ability(&p("Monk"), &characters::empath::Def);
        }))
    }
}

#[test]
fn an_ability_gained_earlier_tonight_acts_at_its_turn() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Gifter", &Gifter),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| narrator::run_order(ctx, &[&Gifter, &characters::empath::Def], 1));
    assert!(g.transcript().contains("wake Monk"));
}

/// Vortox-like: Townsfolk information is always false.
struct Liar;

impl character::Def for Liar {
    fn id(&self) -> &'static str {
        "test_liar"
    }
    fn name(&self) -> &'static str {
        "Liar"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }
    fn forces_false_info(
        &self,
        state: &game::State,
        _me: &player::Id,
        receiver: &player::Id,
    ) -> bool {
        state.kind(receiver) == character::Kind::Townsfolk
    }
}

#[test]
fn an_ability_can_force_false_information() {
    let mut g = common::Game::new(&[
        ("Liar", &Liar),
        ("Empath", &characters::empath::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| narrator::run_order(ctx, &[&characters::empath::Def], 1));
    let log = g.transcript();
    assert!(log.contains("(false)"), "{log}");
    assert!(!log.contains("-> Empath: 1 evil"), "{log}");
}

/// Carries the demon without being one (Lil' Monsta-like).
struct Carrier;

impl character::Def for Carrier {
    fn id(&self) -> &'static str {
        "test_carrier"
    }
    fn name(&self) -> &'static str {
        "Carrier"
    }
    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }
    fn counts_as_demon(&self) -> bool {
        true
    }
}

#[test]
fn a_living_demon_carrier_keeps_the_game_going() {
    let mut g = common::Game::new(&[
        ("Baron", &characters::baron::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ]);
    g.run(|ctx| {
        ctx.gain_ability(&p("Baron"), &Carrier);
        ctx.execute(&p("Chef"));
    });
    assert_eq!(g.state.winner(), None);
    g.run(|ctx| {
        ctx.execute(&p("Baron"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn every_demon_learns_the_team_and_the_same_bluffs() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Imp2", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
        ("Empath", &characters::empath::Def),
        ("Slayer", &characters::slayer::Def),
    ]);
    g.run(|ctx| narrator::demon_info(ctx, &common::trouble_brewing()));
    let log = g.transcript();
    assert_eq!(
        log.matches("-> Imp2: Bluffs: Washerwoman, Librarian, Investigator")
            .count(),
        1,
        "{log}"
    );
    assert_eq!(log.matches("demon bluffs").count(), 1, "{log}");
}

#[test]
fn the_spy_may_misregister_as_any_script_character() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Spy", &characters::spy::Def),
        ("Wash", &characters::washerwoman::Def),
        ("Chef", &characters::chef::Def),
    ]);
    g.run(|ctx| ctx.set_script(common::trouble_brewing().characters))
        .run(|ctx| {
            let as_townsfolk = ctx.registers_as(&p("Wash"), &p("Spy"), character::Kind::Townsfolk);
            assert!(as_townsfolk.is_none());
        });
    // 13 Townsfolk on the script, plus "not a Townsfolk".
    assert!(
        g.transcript()
            .contains("[1 of 14 options] how Spy registers to Wash")
    );
}

#[test]
fn a_player_who_only_believes_they_are_the_demon_does_not_keep_evil_alive() {
    let mut g = common::Game::new(&[
        ("Imp", &characters::imp::Def),
        ("Lunatic", &characters::drunk::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
    ]);
    g.run(|ctx| {
        ctx.set_believed(&p("Lunatic"), Some(&characters::imp::Def));
        ctx.execute(&p("Imp"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}
