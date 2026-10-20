//! Protection and rule-changing Townsfolk: Alchemist, Amnesiac, Atheist,
//! Cannibal, Courtier, Engineer, Exorcist, Farmer, Fool, Innkeeper, Magician,
//! Minstrel, Pacifist, Philosopher, Pixie, Poppy Grower, Sailor, Tea Lady.

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

/// Trouble Brewing plus the characters under test (and the Fang Gu, for the
/// Engineer).
fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend([
        &characters::alchemist::Def as character::Ref,
        &characters::amnesiac::Def,
        &characters::atheist::Def,
        &characters::cannibal::Def,
        &characters::courtier::Def,
        &characters::engineer::Def,
        &characters::exorcist::Def,
        &characters::farmer::Def,
        &characters::fool::Def,
        &characters::innkeeper::Def,
        &characters::magician::Def,
        &characters::minstrel::Def,
        &characters::pacifist::Def,
        &characters::philosopher::Def,
        &characters::pixie::Def,
        &characters::poppy_grower::Def,
        &characters::sailor::Def,
        &characters::tea_lady::Def,
        &characters::fang_gu::Def,
    ]);
    script::Script::new("test", "Test", chars)
}

/// A game on [`script`], set up and at the start of the first night.
fn game(seats: &[(&str, character::Ref)]) -> common::Game {
    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
    })
    .quiet();
    g
}

/// Advances to the start of night `n` (from an earlier phase), quietly.
fn to_night(g: &mut common::Game, n: u32) {
    g.run(|ctx| {
        while ctx.state().phase() != game::Phase::Night(n) {
            ctx.advance_phase();
        }
    })
    .quiet();
}

/// Advances to the start of day `n`, quietly.
fn to_day(g: &mut common::Game, n: u32) {
    g.run(|ctx| {
        while ctx.state().phase() != game::Phase::Day(n) {
            ctx.advance_phase();
        }
    })
    .quiet();
}

/// Poisons `target` until dusk, by an unseated poisoner (one per target, so
/// several players can be poisoned at once).
fn poison(ctx: &mut ctx::Write<'_>, target: &str) {
    let reminder = game::Reminder::new(
        &characters::poisoner::Def,
        &p(&format!("Poisoner of {target}")),
        game::Effect::Poisoned,
        game::Expiry::Dusk,
    );
    ctx.place_reminder(&p(target), reminder);
}

fn alive(g: &common::Game, name: &str) -> bool {
    g.state.is_alive(&p(name))
}

// --- Sailor -----------------------------------------------------------------

fn sailor_game() -> common::Game {
    let mut g = game(&[
        ("Sailor", &characters::sailor::Def),
        ("Empath", &characters::empath::Def),
        ("Chef", &characters::chef::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Imp", &characters::imp::Def),
    ]);
    to_night(&mut g, 2);
    g
}

fn sailor_night(ctx: &mut ctx::Write<'_>) {
    narrator::run_order(ctx, &[&characters::sailor::Def, &characters::imp::Def], 2);
}

#[test]
fn sailor_drinks_with_a_player_and_cannot_die() {
    let mut g = sailor_game();
    g.answers(&[&["Empath"], &["Sailor"]])
        .picks(&[&[0]])
        .run(sailor_night);
    expect![[r#"
        wake Sailor
          Sailor? Who do you want to drink with? => Empath
        sleep Sailor
          storyteller [1 of 2 options] Sailor Sailor: who is drunk: Empath
        * Sailor (sailor) marks Empath: Drunk
        wake Imp
          Imp? Who do you want to kill? => Sailor
        sleep Imp
        * Sailor survives (sailor)
        --- grimoire ---
        Sailor: Sailor
        Empath: Empath [Drunk by sailor]
        Chef: Chef
        Poisoner: Poisoner
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
    assert!(g.state.is_malfunctioning(&p("Empath")));

    // Executed while sober, the Sailor survives too.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Sailor"));
    });
    assert!(alive(&g, "Sailor"));
}

#[test]
fn drunk_sailor_dies() {
    // The storyteller makes the Sailor drunk instead of the Imp they chose.
    let mut g = sailor_game();
    g.answers(&[&["Imp"], &["Sailor"]])
        .picks(&[&[1]])
        .run(sailor_night);
    assert!(!alive(&g, "Sailor"), "{}", g.transcript());

    // Poisoned: nobody is made drunk, and the Sailor dies.
    let mut g = sailor_game();
    g.run(|ctx| poison(ctx, "Sailor"))
        .quiet()
        .answers(&[&["Empath"], &["Sailor"]])
        .run(sailor_night);
    let log = g.transcript();
    assert!(!log.contains("Drunk"), "{log}");
    assert!(!alive(&g, "Sailor"));
}

// --- Fool and Tea Lady ----------------------------------------------------------

#[test]
fn fool_survives_their_first_death_only() {
    let mut g = game(&[
        ("Fool", &characters::fool::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    g.run(|ctx| {
        ctx.kill(&p("Fool"), botc_narrator::event::Cause::Demon);
        ctx.execute(&p("Fool"));
    });
    expect![[r#"
        * Fool (fool) marks Fool: AbilityUsed
        * Fool survives (fool)
        * Fool is executed
        * Fool dies (Execution)
        --- grimoire ---
        Fool: Fool [dead] [AbilityUsed by fool]
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn poisoned_fool_dies_and_tea_lady_protects_first() {
    let seats: [(&str, character::Ref); 5] = [
        ("Chef", &characters::chef::Def),
        ("Fool", &characters::fool::Def),
        ("Tea", &characters::tea_lady::Def),
        ("Monk", &characters::monk::Def),
        ("Imp", &characters::imp::Def),
    ];
    let mut g = game(&seats);
    g.run(|ctx| {
        poison(ctx, "Fool");
        ctx.kill(&p("Fool"), botc_narrator::event::Cause::Demon);
    });
    // The Tea Lady's neighbours (Fool, Monk) are good: the Fool is safe
    // without spending their own ability, even while poisoned.
    let log = g.transcript();
    assert!(log.contains("Fool survives (tea_lady)"), "{log}");
    assert!(!log.contains("AbilityUsed"), "{log}");

    // With the Tea Lady poisoned too, the poisoned Fool dies.
    let mut g = game(&seats);
    g.run(|ctx| {
        poison(ctx, "Fool");
        poison(ctx, "Tea");
        ctx.kill(&p("Fool"), botc_narrator::event::Cause::Demon);
    });
    assert!(!alive(&g, "Fool"), "{}", g.transcript());
}

#[test]
fn tea_lady_protects_good_neighbours_from_any_death() {
    let mut g = game(&[
        ("Chef", &characters::chef::Def),
        ("Tea", &characters::tea_lady::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    g.run(|ctx| {
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
        ctx.execute(&p("Monk"));
        ctx.kill(&p("Tea"), botc_narrator::event::Cause::Demon);
    });
    expect![[r#"
        * Chef survives (tea_lady)
        * Monk is executed
        * Monk survives (tea_lady)
        * Tea dies (Demon)
        --- grimoire ---
        Chef: Chef
        Tea: Tea Lady [dead]
        Monk: Monk
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());

    // An evil neighbour: no protection.
    let mut g = game(&[
        ("Chef", &characters::chef::Def),
        ("Tea", &characters::tea_lady::Def),
        ("Baron", &characters::baron::Def),
        ("Monk", &characters::monk::Def),
        ("Imp", &characters::imp::Def),
    ]);
    g.run(|ctx| {
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
    });
    assert!(!alive(&g, "Chef"));
}

// --- Pacifist -----------------------------------------------------------------

#[test]
fn pacifist_may_save_executed_good_players() {
    let seats: [(&str, character::Ref); 5] = [
        ("Pacifist", &characters::pacifist::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ];
    let mut g = game(&seats);
    g.picks(&[&[1]]).run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
        ctx.execute(&p("Baron"));
    });
    expect![[r#"
        * === Day 1 ===
        * Chef is executed
          storyteller [1 of 2 options] Pacifist Pacifist: does the executed good player survive?: Yes
        * Chef survives (pacifist)
        * Baron is executed
        * Baron dies (Execution)
        --- grimoire ---
        Pacifist: Pacifist
        Chef: Chef
        Monk: Monk
        Baron: Baron [dead]
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Pacifist is not consulted.
    let mut g = game(&seats);
    g.run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Pacifist");
        ctx.execute(&p("Chef"));
    });
    assert!(!alive(&g, "Chef"));
}

// --- Innkeeper -------------------------------------------------------------------

fn innkeeper_game() -> common::Game {
    let mut g = game(&[
        ("Inn", &characters::innkeeper::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    to_night(&mut g, 2);
    g
}

fn innkeeper_night(ctx: &mut ctx::Write<'_>) {
    narrator::run_order(
        ctx,
        &[&characters::innkeeper::Def, &characters::imp::Def],
        2,
    );
}

#[test]
fn innkeeper_protects_two_and_one_is_drunk() {
    let mut g = innkeeper_game();
    g.answers(&[&["Chef", "Monk"], &["Monk"]])
        .picks(&[&[0]])
        .run(innkeeper_night);
    expect![[r#"
        wake Inn
          Inn? Which 2 players do you want to protect? => Chef, Monk
        sleep Inn
        * Inn (innkeeper) marks Chef: Safe
        * Inn (innkeeper) marks Monk: Safe
          storyteller [1 of 2 options] Innkeeper Inn: who is drunk: Chef
        * Inn (innkeeper) marks Chef: Drunk
        wake Imp
          Imp? Who do you want to kill? => Monk
        sleep Imp
        * Monk survives (innkeeper)
        --- grimoire ---
        Inn: Innkeeper
        Chef: Chef [Safe by innkeeper] [Drunk by innkeeper]
        Monk: Monk [Safe by innkeeper]
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());

    // Not the next day's execution.
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Monk"));
    });
    assert!(!alive(&g, "Monk"));
}

#[test]
fn drunk_innkeeper_protects_nobody() {
    // The Innkeeper chose themself and is the one made drunk.
    let mut g = innkeeper_game();
    g.answers(&[&["Inn", "Monk"], &["Monk"]])
        .picks(&[&[0]])
        .run(innkeeper_night);
    assert!(!alive(&g, "Monk"), "{}", g.transcript());

    // Poisoned: no tokens at all.
    let mut g = innkeeper_game();
    g.run(|ctx| poison(ctx, "Inn"))
        .quiet()
        .answers(&[&["Chef", "Monk"], &["Monk"]])
        .run(innkeeper_night);
    let log = g.transcript();
    assert!(!log.contains("Safe"), "{log}");
    assert!(!alive(&g, "Monk"));
}

// --- Minstrel ---------------------------------------------------------------------

fn minstrel_game() -> common::Game {
    let mut g = game(&[
        ("Minstrel", &characters::minstrel::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    to_day(&mut g, 1);
    g
}

#[test]
fn minstrel_makes_everyone_drunk_until_dusk_tomorrow() {
    let mut g = minstrel_game();
    g.run(|ctx| {
        ctx.execute(&p("Baron"));
    });
    expect![[r#"
        * Baron is executed
        * Baron dies (Execution)
        * Minstrel (minstrel) marks Chef: Drunk
        * Minstrel (minstrel) marks Monk: Drunk
        * Minstrel (minstrel) marks Baron: Drunk
        * Minstrel (minstrel) marks Imp: Drunk
        --- grimoire ---
        Minstrel: Minstrel
        Chef: Chef [Drunk by minstrel]
        Monk: Monk [Drunk by minstrel]
        Baron: Baron [dead] [Drunk by minstrel]
        Imp: Imp [Drunk by minstrel]
    "#]]
    .assert_eq(&g.transcript());
    // Drunk through night 2 and day 2; sober when night 3 begins.
    to_day(&mut g, 2);
    assert!(g.state.is_malfunctioning(&p("Imp")));
    to_night(&mut g, 3);
    assert!(!g.state.is_malfunctioning(&p("Imp")));
}

#[test]
fn minstrel_needs_a_sober_minstrel_and_an_executed_minion() {
    let mut g = minstrel_game();
    g.run(|ctx| {
        ctx.execute(&p("Chef"));
        poison(ctx, "Minstrel");
        ctx.kill(&p("Baron"), botc_narrator::event::Cause::Demon);
    });
    assert!(!g.transcript().contains("Drunk"));
    let mut g = minstrel_game();
    g.run(|ctx| {
        poison(ctx, "Minstrel");
        ctx.execute(&p("Baron"));
    });
    assert!(!g.transcript().contains("Drunk by minstrel"));
}

// --- Courtier ---------------------------------------------------------------------

fn courtier_game() -> common::Game {
    game(&[
        ("Courtier", &characters::courtier::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ])
}

#[test]
fn courtier_makes_a_character_drunk_for_three_nights_and_days() {
    let mut g = courtier_game();
    g.characters(&["Imp"])
        .run(|ctx| narrator::run_order(ctx, &[&characters::courtier::Def], 1));
    expect![[r#"
        wake Courtier
          Courtier? Which character do you want to make drunk? => Imp
        sleep Courtier
        * Courtier (courtier) marks Courtier: AbilityUsed
        * Courtier (courtier) marks Imp: Drunk
        --- grimoire ---
        Courtier: Courtier [AbilityUsed by courtier]
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp [Drunk by courtier]
    "#]]
    .assert_eq(&g.transcript());
    to_night(&mut g, 2);
    // While the Courtier is poisoned, the Imp is sober.
    g.run(|ctx| poison(ctx, "Courtier"));
    assert!(!g.state.is_malfunctioning(&p("Imp")));
    to_day(&mut g, 3);
    assert!(g.state.is_malfunctioning(&p("Imp")));
    to_night(&mut g, 4);
    assert!(!g.state.is_malfunctioning(&p("Imp")));
    // Once per game: the Courtier no longer wakes.
    g.run(|ctx| narrator::run_order(ctx, &[&characters::courtier::Def], 4));
    assert!(!g.transcript().contains("wake Courtier"));
}

#[test]
fn poisoned_courtier_spends_the_ability_for_nothing() {
    let mut g = courtier_game();
    g.run(|ctx| poison(ctx, "Courtier"))
        .quiet()
        .characters(&["Imp"])
        .run(|ctx| narrator::run_order(ctx, &[&characters::courtier::Def], 1));
    let log = g.transcript();
    assert!(log.contains("marks Courtier: AbilityUsed"), "{log}");
    assert!(!log.contains("Drunk"), "{log}");
}

// --- Engineer ---------------------------------------------------------------------

fn engineer_game() -> common::Game {
    game(&[
        ("Engineer", &characters::engineer::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Imp", &characters::imp::Def),
    ])
}

#[test]
fn engineer_changes_the_demon_or_minions() {
    let mut g = engineer_game();
    g.characters(&["Fang Gu"])
        .run(|ctx| narrator::run_order(ctx, &[&characters::engineer::Def], 1));
    expect![[r#"
        wake Engineer
          Engineer? Which Demon or Minion is in play? => Fang Gu
        sleep Engineer
        * Engineer (engineer) marks Engineer: AbilityUsed
        * Imp becomes the Fang Gu
        wake Imp
          -> Imp: Fang Gu
        sleep Imp
        --- grimoire ---
        Engineer: Engineer [AbilityUsed by engineer]
        Chef: Chef
        Monk: Monk
        Poisoner: Poisoner
        Imp: Fang Gu
    "#]]
    .assert_eq(&g.transcript());

    let mut g = engineer_game();
    g.characters(&["Baron"])
        .run(|ctx| narrator::run_order(ctx, &[&characters::engineer::Def], 1));
    assert!(g.transcript().contains("Poisoner becomes the Baron"));
}

#[test]
fn poisoned_engineer_changes_nothing() {
    let mut g = engineer_game();
    g.run(|ctx| poison(ctx, "Engineer"))
        .quiet()
        .characters(&["Fang Gu"])
        .run(|ctx| narrator::run_order(ctx, &[&characters::engineer::Def], 1));
    let log = g.transcript();
    assert!(log.contains("AbilityUsed"), "{log}");
    assert!(!log.contains("becomes"), "{log}");
}

// --- Exorcist ---------------------------------------------------------------------

fn exorcist_game() -> common::Game {
    let mut g = game(&[
        ("Exorcist", &characters::exorcist::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    to_night(&mut g, 2);
    g
}

fn exorcist_night(ctx: &mut ctx::Write<'_>) {
    let night = match ctx.state().phase() {
        game::Phase::Night(n) => n,
        _ => unreachable!(),
    };
    narrator::run_order(
        ctx,
        &[&characters::exorcist::Def, &characters::imp::Def],
        night,
    );
}

#[test]
fn exorcised_demon_learns_the_exorcist_and_does_not_wake() {
    let mut g = exorcist_game();
    g.answers(&[&["Imp"]]).run(exorcist_night);
    expect![[r#"
        wake Exorcist
          Exorcist? Who do you want to exorcise? => Imp
        sleep Exorcist
        * Exorcist (exorcist) marks Imp: Chosen
        * Exorcist (exorcist) marks Imp: DoesNotWake
        wake Imp
          -> Imp: Exorcist=Exorcist
        sleep Imp
        --- grimoire ---
        Exorcist: Exorcist
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp [Chosen by exorcist] [DoesNotWake by exorcist]
    "#]]
    .assert_eq(&g.transcript());

    // Not the same player two nights running: the answer falls back to the
    // first candidate, and the Imp wakes and kills.
    to_night(&mut g, 3);
    g.answers(&[&["Imp"], &["Chef"]]).run(exorcist_night);
    let log = g.transcript();
    assert!(log.contains("marks Exorcist: Chosen"), "{log}");
    assert!(!alive(&g, "Chef"));
}

#[test]
fn poisoned_exorcist_does_not_stop_the_demon() {
    let mut g = exorcist_game();
    g.run(|ctx| poison(ctx, "Exorcist"))
        .quiet()
        .answers(&[&["Imp"], &["Chef"]])
        .run(exorcist_night);
    let log = g.transcript();
    assert!(!log.contains("DoesNotWake"), "{log}");
    assert!(!alive(&g, "Chef"));
}

// --- Farmer ---------------------------------------------------------------------

fn farmer_game() -> common::Game {
    let mut g = game(&[
        ("Farmer", &characters::farmer::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    to_night(&mut g, 2);
    g
}

fn farmer_night(ctx: &mut ctx::Write<'_>) {
    narrator::run_order(ctx, &[&characters::imp::Def, &characters::farmer::Def], 2);
}

#[test]
fn farmer_dying_at_night_makes_a_new_farmer() {
    let mut g = farmer_game();
    g.answers(&[&["Farmer"]]).picks(&[&[1]]).run(farmer_night);
    expect![[r#"
        wake Imp
          Imp? Who do you want to kill? => Farmer
        sleep Imp
        * Farmer dies (Demon)
          storyteller [1 of 2 options] Farmer Farmer: which alive good player becomes a Farmer: Monk
        * Monk becomes the Farmer
        wake Monk
          -> Monk: Farmer
        sleep Monk
        --- grimoire ---
        Farmer: Farmer [dead]
        Chef: Chef
        Monk: Farmer
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn farmer_executed_or_poisoned_makes_no_farmer() {
    let mut g = farmer_game();
    g.run(|ctx| poison(ctx, "Farmer"))
        .quiet()
        .answers(&[&["Farmer"]])
        .run(farmer_night);
    assert!(!g.transcript().contains("becomes"));

    let mut g = farmer_game();
    to_day(&mut g, 2);
    g.run(|ctx| {
        ctx.execute(&p("Farmer"));
        ctx.advance_phase();
    })
    .answers(&[&["Chef"]])
    .run(|ctx| narrator::run_order(ctx, &[&characters::farmer::Def], 3));
    assert!(!g.transcript().contains("becomes"));
}

// --- Magician and Poppy Grower ---------------------------------------------------

fn evil_info_game(townsfolk: character::Ref) -> common::Game {
    game(&[
        ("Imp", &characters::imp::Def),
        ("Baron", &characters::baron::Def),
        ("Special", townsfolk),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Saint", &characters::saint::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn magician_appears_in_the_evil_team() {
    let mut g = evil_info_game(&characters::magician::Def);
    g.run(|ctx| narrator::run_night(ctx, &script()));
    let log = g.transcript();
    assert!(log.contains("-> Baron: Evil: Imp, Baron, Special"), "{log}");
    assert!(log.contains("-> Imp: Evil: Imp, Baron, Special"), "{log}");

    // Poisoned, evil learn each other as normal.
    let mut g = evil_info_game(&characters::magician::Def);
    g.run(|ctx| poison(ctx, "Special"))
        .quiet()
        .run(|ctx| narrator::run_night(ctx, &script()));
    let log = g.transcript();
    assert!(log.contains("-> Baron: Evil: Imp, Baron\n"), "{log}");
}

#[test]
fn poppy_grower_hides_evil_until_they_die() {
    let mut g = evil_info_game(&characters::poppy_grower::Def);
    g.run(|ctx| narrator::run_night(ctx, &script()));
    let log = g.transcript();
    assert!(!log.contains("Evil:"), "{log}");
    assert!(log.contains("-> Imp: Bluffs:"), "{log}");

    to_day(&mut g, 1);
    g.run(|ctx| {
        ctx.execute(&p("Special"));
        ctx.advance_phase();
    })
    .quiet()
    .run(|ctx| narrator::run_order(ctx, &[&characters::poppy_grower::Def], 2));
    expect![[r#"
        * Special (poppy_grower) marks Special: AbilityUsed
        wake Baron
          -> Baron: Evil: Imp, Baron
        sleep Baron
        wake Imp
          -> Imp: Evil: Imp, Baron
        sleep Imp
        --- grimoire ---
        Imp: Imp
        Baron: Baron
        Special: Poppy Grower [dead] [Evil wakes by poppy_grower] [AbilityUsed by poppy_grower]
        Chef: Chef
        Monk: Monk
        Saint: Saint
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // Only once.
    to_night(&mut g, 3);
    g.run(|ctx| narrator::run_order(ctx, &[&characters::poppy_grower::Def], 3));
    assert!(!g.transcript().contains("Evil:"));
}

#[test]
fn poppy_grower_dying_poisoned_reveals_nothing() {
    let mut g = evil_info_game(&characters::poppy_grower::Def);
    g.run(|ctx| narrator::run_night(ctx, &script())).quiet();
    to_night(&mut g, 2);
    g.run(|ctx| {
        poison(ctx, "Special");
        ctx.kill(&p("Special"), botc_narrator::event::Cause::Demon);
        ctx.advance_phase();
        ctx.advance_phase();
    })
    .quiet()
    .run(|ctx| narrator::run_order(ctx, &[&characters::poppy_grower::Def], 3));
    assert!(!g.transcript().contains("Evil:"));
}

#[test]
fn poisoned_poppy_grower_hides_nothing() {
    let mut g = evil_info_game(&characters::poppy_grower::Def);
    g.run(|ctx| poison(ctx, "Special"))
        .quiet()
        .run(|ctx| narrator::run_night(ctx, &script()));
    assert!(g.transcript().contains("-> Baron: Evil: Imp, Baron"));
}

// --- Philosopher ------------------------------------------------------------------

fn philosopher_game() -> common::Game {
    game(&[
        ("Philo", &characters::philosopher::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ])
}

fn philosopher_night(ctx: &mut ctx::Write<'_>) {
    narrator::run_order(
        ctx,
        &[
            &characters::philosopher::Def,
            &characters::chef::Def,
            &characters::empath::Def,
        ],
        1,
    );
}

#[test]
fn philosopher_gains_an_ability_and_acts_with_it_tonight() {
    let mut g = philosopher_game();
    g.characters(&["Empath"]).run(philosopher_night);
    expect![[r#"
        wake Philo
          Philo? Which good character's ability do you want? => Empath
        sleep Philo
        * Philo (philosopher) marks Philo: AbilityUsed
        * Philo gains the Empath ability
        === parallel (2) ===
        wake Chef
          storyteller [1 of 1 options] information for Chef (Chef): "1 evil pairs"
          -> Chef: 1 evil pairs
        sleep Chef
        wake Philo
          storyteller [1 of 1 options] information for Philo (Philosopher): "1 evil neighbours"
          -> Philo: 1 evil neighbours
        sleep Philo
        === end parallel ===
        --- grimoire ---
        Philo: Philosopher (+Empath) [AbilityUsed by philosopher]
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn philosopher_choosing_an_in_play_character_makes_it_drunk() {
    let mut g = philosopher_game();
    g.characters(&["Chef"]).run(philosopher_night);
    let log = g.transcript();
    assert!(log.contains("marks Chef: Drunk"), "{log}");
    assert!(g.state.is_malfunctioning(&p("Chef")));
    // The Philosopher dies: the Chef sobers.
    g.run(|ctx| {
        ctx.kill(&p("Philo"), botc_narrator::event::Cause::Demon);
    });
    assert!(!g.state.is_malfunctioning(&p("Chef")));

    // Poisoned: the ability is spent and nothing else happens.
    let mut g = philosopher_game();
    g.run(|ctx| poison(ctx, "Philo"))
        .quiet()
        .characters(&["Chef"])
        .run(philosopher_night);
    let log = g.transcript();
    assert!(log.contains("AbilityUsed"), "{log}");
    assert!(!log.contains("gains") && !log.contains("Drunk"), "{log}");
}

// --- Pixie ------------------------------------------------------------------------

fn pixie_game() -> common::Game {
    game(&[
        ("Pixie", &characters::pixie::Def),
        ("Chef", &characters::chef::Def),
        ("Saint", &characters::saint::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ])
}

fn pixie_night(ctx: &mut ctx::Write<'_>) {
    narrator::run_order(ctx, &[&characters::pixie::Def], 1);
}

#[test]
fn pixie_gains_the_ability_of_the_townsfolk_they_stayed_mad_about() {
    let mut g = pixie_game();
    g.run(pixie_night);
    expect![[r#"
        wake Pixie
          storyteller [1 of 1 options] information for Pixie (Pixie): "Chef"
          -> Pixie: Chef
        sleep Pixie
        * Pixie (pixie) marks Pixie: Mad (Chef)
        * Pixie (pixie) marks Chef: Learnt
        --- grimoire ---
        Pixie: Pixie [Mad (Chef) by pixie]
        Chef: Chef [Learnt by pixie]
        Saint: Saint
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
    g.run(|ctx| {
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
    });
    assert!(g.transcript().contains("Pixie gains the Chef ability"));
}

#[test]
fn pixie_who_broke_madness_or_is_poisoned_gains_nothing() {
    let mut g = pixie_game();
    g.run(pixie_night).quiet();
    g.table.breaks_madness.push(p("Pixie"));
    g.run(|ctx| {
        ctx.advance_phase();
        narrator::check_madness(ctx);
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
    });
    assert!(!g.transcript().contains("gains"));

    // Poisoned, the Pixie may be told a Townsfolk not in play.
    let mut g = pixie_game();
    g.run(|ctx| poison(ctx, "Pixie"))
        .quiet()
        .picks(&[&[1]])
        .run(pixie_night);
    let log = g.transcript();
    assert!(log.contains("-> Pixie: Washerwoman"), "{log}");
    assert!(!log.contains("Learnt"), "{log}");
}

// --- Alchemist ----------------------------------------------------------------------

#[test]
fn alchemist_learns_and_uses_a_not_in_play_minion_ability() {
    let mut g = common::Game::new(&[
        ("Alch", &characters::alchemist::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    // The storyteller picks the Poisoner (not in play).
    g.picks(&[&[0]]).run(|ctx| {
        narrator::setup(ctx, &script());
        ctx.advance_phase();
    });
    g.quiet().answers(&[&["Imp"]]).run(|ctx| {
        narrator::run_order(
            ctx,
            &[&characters::alchemist::Def, &characters::poisoner::Def],
            1,
        )
    });
    expect![[r#"
        wake Alch
          -> Alch: Poisoner
        sleep Alch
        wake Alch
          Alch? Who do you want to poison? => Imp
        * Alch (poisoner) marks Imp: Poisoned
        sleep Alch
        --- grimoire ---
        Alch: Alchemist (+Poisoner)
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp [Poisoned by poisoner]
    "#]]
    .assert_eq(&g.transcript());
    // The Alchemist is good and not part of the evil team.
    assert!(!g.state.is_evil(&p("Alch")));
}

// --- Amnesiac -----------------------------------------------------------------------

#[test]
fn amnesiac_has_a_hidden_ability_and_guesses_it_daily() {
    let mut g = common::Game::new(&[
        ("Amnesiac", &characters::amnesiac::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    // The storyteller gives them the Empath's ability (not in play).
    g.picks(&[&[3]]).run(|ctx| narrator::setup(ctx, &script()));
    g.quiet()
        .run(|ctx| {
            ctx.advance_phase();
            narrator::run_order(ctx, &[&characters::empath::Def], 1);
        })
        .characters(&["Chef", "Empath"])
        .picks(&[&[1]])
        .run(|ctx| {
            ctx.advance_phase();
            ctx.advance_phase();
            ctx.advance_phase();
        });
    expect![[r#"
        * === Night 1 ===
        wake Amnesiac
          storyteller [1 of 1 options] information for Amnesiac (Amnesiac): "1 evil neighbours"
          -> Amnesiac: 1 evil neighbours
        sleep Amnesiac
        * === Day 1 ===
          Amnesiac? Which ability do you think you have? => Chef
          storyteller [1 of 3 options] information for Amnesiac (Amnesiac): "Warm"
          -> Amnesiac: Warm
        * === Night 2 ===
        * === Day 2 ===
          Amnesiac? Which ability do you think you have? => Empath
          storyteller [1 of 1 options] information for Amnesiac (Amnesiac): "Bingo"
          -> Amnesiac: Bingo
        --- grimoire ---
        Amnesiac: Amnesiac (+Empath) [Empath by amnesiac]
        Chef: Chef
        Monk: Monk
        Baron: Baron
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, even a right guess may be answered wrongly.
    g.run(|ctx| {
        ctx.advance_phase();
        poison(ctx, "Amnesiac");
    })
    .quiet()
    .characters(&["Empath"])
    .picks(&[&[1]])
    .run(|ctx| {
        ctx.advance_phase();
    });
    let log = g.transcript();
    assert!(log.contains("-> Amnesiac: Cold"), "{log}");
}

// --- Atheist ------------------------------------------------------------------------

#[test]
fn atheist_setups_have_no_evil_and_the_game_needs_no_demon() {
    let counts = character::Def::setup_counts(&characters::atheist::Def, setup::Counts::base(7));
    assert_eq!(
        counts,
        vec![setup::Counts {
            townsfolk: 7,
            outsiders: 0,
            minions: 0,
            demons: 0
        }]
    );
    let mut g = game(&[
        ("Atheist", &characters::atheist::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Saint", &characters::saint::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
    });
    assert_eq!(g.state.winner(), None, "no Demon is not a good win");
    g.run(|ctx| narrator::day_action(ctx, &p("Atheist"), &characters::atheist::Def));
    expect![[r#"
        * === Day 1 ===
        * Chef is executed
        * Chef dies (Execution)
        * Atheist (atheist) marks Atheist: AbilityUsed
        * Atheist has the Storyteller executed
        * Good wins!
        --- grimoire ---
        Atheist: Atheist [AbilityUsed by atheist]
        Chef: Chef [dead]
        Monk: Monk
        Saint: Saint
        Empath: Empath
        winner: Good
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn executing_the_storyteller_without_an_atheist_loses() {
    // A Drunk who believes they are the Atheist.
    let mut g = common::Game::new(&[
        ("Drunk", &characters::drunk::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Baron", &characters::baron::Def),
        ("Imp", &characters::imp::Def),
    ]);
    g.run(|ctx| {
        ctx.set_believed(&p("Drunk"), Some(&characters::atheist::Def));
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::day_action(ctx, &p("Drunk"), &characters::atheist::Def);
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Evil));
}

// --- Cannibal ------------------------------------------------------------------------

#[test]
fn cannibal_has_the_last_executees_ability() {
    let mut g = game(&[
        ("Cannibal", &characters::cannibal::Def),
        ("Chef", &characters::chef::Def),
        ("Monk", &characters::monk::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Slayer", &characters::slayer::Def),
        ("Imp", &characters::imp::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Chef"));
        ctx.execute(&p("Poisoner"));
    });
    expect![[r#"
        * === Day 1 ===
        * Chef is executed
        * Chef dies (Execution)
        * Cannibal gains the Chef ability
        * Cannibal (cannibal) marks Cannibal: Chef
        * Poisoner is executed
        * Poisoner dies (Execution)
        * Cannibal (cannibal) removes Cannibal's Chef token
        * Cannibal loses the Chef ability
        * Cannibal (cannibal) marks Cannibal: Poisoned
        * Cannibal gains the Poisoner ability
        * Cannibal (cannibal) marks Cannibal: Poisoner
        --- grimoire ---
        Cannibal: Cannibal (+Poisoner) [Poisoned by cannibal] [Poisoner by cannibal]
        Chef: Chef [dead]
        Monk: Monk
        Poisoner: Poisoner [dead]
        Slayer: Slayer
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
    assert!(g.state.is_malfunctioning(&p("Cannibal")));

    // A good executee: the poison ends and the ability is replaced.
    g.run(|ctx| {
        ctx.execute(&p("Monk"));
    });
    expect![[r#"
        * Monk is executed
        * Monk dies (Execution)
        * Cannibal (cannibal) removes Cannibal's Poisoner token
        * Cannibal loses the Poisoner ability
        * Cannibal (cannibal) removes Cannibal's Poisoned token
        * Cannibal gains the Monk ability
        * Cannibal (cannibal) marks Cannibal: Monk
        --- grimoire ---
        Cannibal: Cannibal (+Monk) [Monk by cannibal]
        Chef: Chef [dead]
        Monk: Monk [dead]
        Poisoner: Poisoner [dead]
        Slayer: Slayer
        Imp: Imp
    "#]]
    .assert_eq(&g.transcript());
    assert!(!g.state.is_malfunctioning(&p("Cannibal")));

    // An executed Demon ends the game; its ability is not taken.
    g.run(|ctx| {
        ctx.execute(&p("Imp"));
    });
    assert!(!g.state.has_ability(&p("Cannibal"), "imp"));
}

// --- Simulator ------------------------------------------------------------------------

#[test]
fn simulated_games_with_these_characters_finish() {
    let script = script();
    let mut finished = 0;
    for seed in 0..120 {
        let players = 5 + (seed as usize % 11);
        let trace = sim::simulate(seed, players, &script);
        finished += usize::from(trace.winner.is_some());
    }
    assert!(finished > 100, "{finished} of 120 games have a winner");
}
