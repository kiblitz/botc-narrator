//! Townsfolk with day and active abilities: Town Crier, High Priestess,
//! Gossip, Juggler, Artist, Savant, Fisherman, Bounty Hunter, Snake Charmer,
//! Gambler, Professor, Huntsman.

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

/// A stand-in for the Damsel (an Outsider the Huntsman hunts) until it is
/// registered.
struct Damsel;

impl character::Def for Damsel {
    fn id(&self) -> &'static str {
        "damsel"
    }

    fn name(&self) -> &'static str {
        "Damsel"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }
}

fn script() -> script::Script {
    let mut chars = common::trouble_brewing().characters;
    chars.extend([
        &characters::town_crier::Def as character::Ref,
        &characters::high_priestess::Def,
        &characters::gossip::Def,
        &characters::juggler::Def,
        &characters::artist::Def,
        &characters::savant::Def,
        &characters::fisherman::Def,
        &characters::bounty_hunter::Def,
        &characters::snake_charmer::Def,
        &characters::gambler::Def,
        &characters::professor::Def,
        &characters::huntsman::Def,
        &Damsel,
    ]);
    script::Script::new("test", "Test", chars)
}

/// Advance to night `n` (from setup), quietly.
fn to_night(g: &mut common::Game, n: u32) {
    g.run(|ctx| {
        ctx.set_script(script().characters);
        while ctx.state().phase() != game::Phase::Night(n) {
            ctx.advance_phase();
        }
    })
    .quiet();
}

/// Advance to day `n` (from setup), quietly.
fn to_day(g: &mut common::Game, n: u32) {
    g.run(|ctx| {
        ctx.set_script(script().characters);
        while ctx.state().phase() != game::Phase::Day(n) {
            ctx.advance_phase();
        }
    })
    .quiet();
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

fn act(c: character::Ref, night: u32) -> impl FnOnce(&mut ctx::Write<'_>) {
    move |ctx| narrator::run_order(ctx, &[c], night)
}

fn day_action(player: &'static str, c: character::Ref) -> impl FnOnce(&mut ctx::Write<'_>) {
    move |ctx| narrator::day_action(ctx, &p(player), c)
}

fn next_phase(ctx: &mut ctx::Write<'_>) {
    ctx.advance_phase();
}

// --- Town Crier -------------------------------------------------------------------

fn town_crier_game() -> common::Game {
    common::Game::new(&[
        ("Crier", &characters::town_crier::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
    ])
}

#[test]
fn town_crier_learns_whether_a_minion_nominated_today() {
    let crier: character::Ref = &characters::town_crier::Def;
    assert!(
        !script()
            .first_night
            .iter()
            .any(|c| character::same(*c, crier))
    );

    let mut g = town_crier_game();
    to_day(&mut g, 1);
    g.run(|ctx| narrator::nominate(ctx, &p("Poisoner"), &p("Chef")))
        .run(next_phase)
        .run(act(crier, 2));
    expect![[r#"
        * Poisoner nominates Chef
        * === Night 2 ===
        wake Crier
          storyteller [1 of 1 options] information for Crier (Town Crier): "Yes"
          -> Crier: Yes
        sleep Crier
        --- grimoire ---
        Crier: Town Crier
        Poisoner: Poisoner
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Only the demon nominated: the demon is not a Minion.
    let mut g = town_crier_game();
    to_day(&mut g, 1);
    g.run(|ctx| narrator::nominate(ctx, &p("Imp"), &p("Chef")))
        .run(next_phase)
        .quiet()
        .run(act(crier, 2));
    expect![[r#"
        wake Crier
          storyteller [1 of 1 options] information for Crier (Town Crier): "No"
          -> Crier: No
        sleep Crier
        --- grimoire ---
        Crier: Town Crier
        Poisoner: Poisoner
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the Town Crier may hear "no".
    let mut g = town_crier_game();
    to_day(&mut g, 1);
    g.run(|ctx| narrator::nominate(ctx, &p("Poisoner"), &p("Chef")))
        .run(next_phase)
        .run(|ctx| poison(ctx, "Crier"))
        .quiet()
        .picks(&[&[1]])
        .run(act(crier, 2));
    expect![[r#"
        wake Crier
          storyteller [1 of 2 options] information for Crier (Town Crier), who is poisoned or drunk: false info is allowed: "No" (false)
          -> Crier: No
        sleep Crier
        --- grimoire ---
        Crier: Town Crier [Poisoned by poisoner]
        Poisoner: Poisoner
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- High Priestess ---------------------------------------------------------------

#[test]
fn high_priestess_is_shown_any_player() {
    let priestess: character::Ref = &characters::high_priestess::Def;
    let seats: [(&str, character::Ref); 4] = [
        ("Priestess", priestess),
        ("Chef", &characters::chef::Def),
        ("Imp", &characters::imp::Def),
        ("Saint", &characters::saint::Def),
    ];
    let mut g = common::Game::new(&seats);
    to_night(&mut g, 1);
    g.picks(&[&[0]]).run(act(priestess, 1));
    expect![[r#"
        wake Priestess
          storyteller [1 of 3 options] information for Priestess (High Priestess): "Chef"
          -> Priestess: Chef
        sleep Priestess
        --- grimoire ---
        Priestess: High Priestess
        Chef: Chef
        Imp: Imp
        Saint: Saint
    "#]]
    .assert_eq(&g.transcript());

    // Dead and evil players are legal, poisoned or not.
    let mut g = common::Game::new(&seats);
    to_night(&mut g, 3);
    g.run(|ctx| {
        ctx.kill(&p("Saint"), botc_narrator::event::Cause::Demon);
        poison(ctx, "Priestess");
    })
    .quiet()
    .picks(&[&[2]])
    .run(act(priestess, 3));
    expect![[r#"
        wake Priestess
          storyteller [1 of 3 options] information for Priestess (High Priestess), who is poisoned or drunk: false info is allowed: "Saint"
          -> Priestess: Saint
        sleep Priestess
        --- grimoire ---
        Priestess: High Priestess [Poisoned by poisoner]
        Chef: Chef
        Imp: Imp
        Saint: Saint [dead]
    "#]]
    .assert_eq(&g.transcript());
}

// --- Gossip -----------------------------------------------------------------------

fn gossip_game() -> common::Game {
    common::Game::new(&[
        ("Gossip", &characters::gossip::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
    ])
}

#[test]
fn gossip_true_statement_kills_a_player_tonight() {
    let gossip: character::Ref = &characters::gossip::Def;
    let mut g = gossip_game();
    to_day(&mut g, 1);
    g.answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", gossip))
        .run(next_phase)
        .picks(&[&[3]])
        .run(act(gossip, 2));
    expect![[r#"
        * Gossip (gossip) marks Gossip: claimed
        * Announcement: Gossip gossips
          Gossip? Who is your statement about? => Imp
          Gossip? Which character do you say they are? => Imp
        * Gossip claims: Imp is the Imp
        * === Night 2 ===
          storyteller [1 of 4 options] Gossip Gossip: who dies because the Gossip's statement was true: Empath
        * Empath dies (Ability { by: Id("Gossip") })
        --- grimoire ---
        Gossip: Gossip
        Imp: Imp
        Chef: Chef
        Empath: Empath [dead]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn a_gossip_bluffing_the_juggler_makes_no_statement() {
    let gossip: character::Ref = &characters::gossip::Def;
    let mut g = gossip_game();
    to_day(&mut g, 1);
    // The Gossip's guesses as a (bluffed) Juggler are true, but they are
    // Juggler claims: the Gossip's ability does not judge them.
    g.answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", &characters::juggler::Def))
        .run(next_phase)
        .run(act(gossip, 2));
    expect![[r#"
        * Gossip (juggler) marks Gossip: claimed
        * Announcement: Gossip juggles
          Gossip? Whose characters do you guess? => Imp
          Gossip? Which character is Imp? => Imp
          Gossip? Which character is Chef? => Washerwoman
          Gossip? Which character is Empath? => Washerwoman
        * Gossip claims: Imp is the Imp, Chef is the Washerwoman, Empath is the Washerwoman
        * === Night 2 ===
        --- grimoire ---
        Gossip: Gossip [claimed by juggler]
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn gossip_false_statement_kills_nobody() {
    let gossip: character::Ref = &characters::gossip::Def;
    let mut g = gossip_game();
    to_day(&mut g, 1);
    g.answers(&[&["Chef"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", gossip))
        .run(next_phase)
        .run(act(gossip, 2));
    expect![[r#"
        * Gossip (gossip) marks Gossip: claimed
        * Announcement: Gossip gossips
          Gossip? Who is your statement about? => Chef
          Gossip? Which character do you say they are? => Imp
        * Gossip claims: Chef is the Imp
        * === Night 2 ===
        --- grimoire ---
        Gossip: Gossip
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn gossip_poisoned_tonight_or_executed_kills_nobody() {
    let gossip: character::Ref = &characters::gossip::Def;
    // Poisoned when the statement was made, healthy tonight: someone dies.
    let mut g = gossip_game();
    to_day(&mut g, 1);
    g.run(|ctx| poison(ctx, "Gossip"))
        .answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", gossip))
        .run(next_phase)
        .quiet()
        .run(act(gossip, 2));
    assert!(
        g.transcript().contains("dies (Ability"),
        "the poison ended at dusk"
    );

    // Poisoned tonight: nobody dies.
    let mut g = gossip_game();
    to_day(&mut g, 1);
    g.answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", gossip))
        .run(next_phase)
        .run(|ctx| poison(ctx, "Gossip"))
        .quiet()
        .run(act(gossip, 2));
    expect![[r#"
        --- grimoire ---
        Gossip: Gossip [Poisoned by poisoner]
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Executed after the statement: nobody dies.
    let mut g = gossip_game();
    to_day(&mut g, 1);
    g.answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Gossip", gossip))
        .run(|ctx| {
            ctx.execute(&p("Gossip"));
        })
        .run(next_phase)
        .quiet()
        .run(act(gossip, 2));
    expect![[r#"
        --- grimoire ---
        Gossip: Gossip [dead]
        Imp: Imp
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- Juggler ----------------------------------------------------------------------

fn juggler_game() -> common::Game {
    common::Game::new(&[
        ("Juggler", &characters::juggler::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Baron", &characters::baron::Def),
    ])
}

#[test]
fn juggler_learns_how_many_first_day_guesses_were_right() {
    let juggler: character::Ref = &characters::juggler::Def;
    let mut g = juggler_game();
    to_day(&mut g, 1);
    g.answers(&[&["Imp", "Chef", "Baron"]])
        .characters(&["Imp", "Empath", "Baron"])
        .run(day_action("Juggler", juggler))
        .run(next_phase)
        .run(act(juggler, 2));
    expect![[r#"
        * Juggler (juggler) marks Juggler: claimed
        * Announcement: Juggler juggles
        * Juggler (juggler) marks Juggler: AbilityUsed
          Juggler? Whose characters do you guess? => Imp, Chef, Baron
          Juggler? Which character is Imp? => Imp
          Juggler? Which character is Chef? => Empath
          Juggler? Which character is Baron? => Baron
        * Juggler claims: Imp is the Imp, Chef is the Empath, Baron is the Baron
        * === Night 2 ===
        wake Juggler
          storyteller [1 of 1 options] information for Juggler (Juggler): "2 correct"
          -> Juggler: 2 correct
        sleep Juggler
        --- grimoire ---
        Juggler: Juggler [claimed by juggler] [AbilityUsed by juggler]
        Imp: Imp
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());

    // No guesses on a later day; no wake the night after.
    g.run(next_phase)
        .run(day_action("Juggler", juggler))
        .run(next_phase)
        .quiet()
        .run(act(juggler, 3));
    expect![[r#"
        --- grimoire ---
        Juggler: Juggler [claimed by juggler] [AbilityUsed by juggler]
        Imp: Imp
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn juggler_poisoned_tonight_may_learn_a_wrong_count() {
    let juggler: character::Ref = &characters::juggler::Def;
    let mut g = juggler_game();
    to_day(&mut g, 1);
    g.answers(&[&["Imp", "Chef", "Baron"]])
        .characters(&["Imp", "Chef", "Baron"])
        .run(day_action("Juggler", juggler))
        .run(next_phase)
        .run(|ctx| poison(ctx, "Juggler"))
        .quiet()
        .picks(&[&[1]])
        .run(act(juggler, 2));
    expect![[r#"
        wake Juggler
          storyteller [1 of 4 options] information for Juggler (Juggler), who is poisoned or drunk: false info is allowed: "0 correct" (false)
          -> Juggler: 0 correct
        sleep Juggler
        --- grimoire ---
        Juggler: Juggler [claimed by juggler] [AbilityUsed by juggler] [Poisoned by poisoner]
        Imp: Imp
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn juggler_who_dies_learns_nothing() {
    let juggler: character::Ref = &characters::juggler::Def;
    let mut g = juggler_game();
    to_day(&mut g, 1);
    g.run(day_action("Juggler", juggler))
        .run(|ctx| {
            ctx.execute(&p("Juggler"));
        })
        .run(next_phase)
        .quiet()
        .run(act(juggler, 2));
    expect![[r#"
        --- grimoire ---
        Juggler: Juggler [dead] [AbilityUsed by juggler]
        Imp: Imp
        Chef: Chef
        Baron: Baron
    "#]]
    .assert_eq(&g.transcript());
}

// --- Artist -----------------------------------------------------------------------

#[test]
fn artist_asks_one_question_per_game() {
    let artist: character::Ref = &characters::artist::Def;
    let seats: [(&str, character::Ref); 3] = [
        ("Artist", artist),
        ("Imp", &characters::imp::Def),
        ("Empath", &characters::empath::Def),
    ];
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.answers(&[&["Imp"]])
        .characters(&["Imp"])
        .run(day_action("Artist", artist))
        .run(day_action("Artist", artist));
    expect![[r#"
        * Artist (artist) marks Artist: AbilityUsed
        wake Artist
          Artist? Whom is your question about? => Imp
          Artist? Are they which character? => Imp
          storyteller [1 of 1 options] information for Artist (Artist): "Yes"
          -> Artist: Yes
        sleep Artist
        --- grimoire ---
        Artist: Artist [AbilityUsed by artist]
        Imp: Imp
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned, the answer may be false; the ability is still spent.
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.run(|ctx| poison(ctx, "Artist"))
        .quiet()
        .answers(&[&["Imp"]])
        .characters(&["Imp"])
        .picks(&[&[1]])
        .run(day_action("Artist", artist));
    expect![[r#"
        * Artist (artist) marks Artist: AbilityUsed
        wake Artist
          Artist? Whom is your question about? => Imp
          Artist? Are they which character? => Imp
          storyteller [1 of 2 options] information for Artist (Artist), who is poisoned or drunk: false info is allowed: "No" (false)
          -> Artist: No
        sleep Artist
        --- grimoire ---
        Artist: Artist [Poisoned by poisoner] [AbilityUsed by artist]
        Imp: Imp
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- Savant -----------------------------------------------------------------------

#[test]
fn savant_learns_one_true_and_one_false_statement() {
    let savant: character::Ref = &characters::savant::Def;
    let seats: [(&str, character::Ref); 4] = [
        ("Savant", savant),
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Chef", &characters::chef::Def),
    ];
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.picks(&[&[3], &[4], &[1]])
        .run(day_action("Savant", savant));
    expect![[r#"
        wake Savant
          storyteller [1 of 6 options] Savant Savant: the Savant's true statement: "Poisoner is the Poisoner"
          storyteller [1 of 12 options] Savant Savant: the Savant's false statement: "Poisoner is Good" (false)
          storyteller [1 of 2 options] information for Savant (Savant): "Poisoner is Good; Poisoner is the Poisoner"
          -> Savant: Poisoner is Good; Poisoner is the Poisoner
        sleep Savant
        --- grimoire ---
        Savant: Savant
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned: both may be true (or both false).
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.run(|ctx| poison(ctx, "Savant"))
        .quiet()
        .picks(&[&[3], &[4], &[2]])
        .run(day_action("Savant", savant));
    expect![[r#"
        wake Savant
          storyteller [1 of 6 options] Savant Savant: the Savant's true statement: "Poisoner is the Poisoner"
          storyteller [1 of 12 options] Savant Savant: the Savant's false statement: "Poisoner is Good" (false)
          storyteller [1 of 4 options] information for Savant (Savant), who is poisoned or drunk: false info is allowed: "Poisoner is the Poisoner; Poisoner is Evil" (false)
          -> Savant: Poisoner is the Poisoner; Poisoner is Evil
        sleep Savant
        --- grimoire ---
        Savant: Savant [Poisoned by poisoner]
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

// --- Fisherman --------------------------------------------------------------------

#[test]
fn fisherman_gets_advice_once() {
    let fisherman: character::Ref = &characters::fisherman::Def;
    let seats: [(&str, character::Ref); 3] = [
        ("Fisher", fisherman),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
    ];
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.run(day_action("Fisher", fisherman))
        .run(day_action("Fisher", fisherman));
    expect![[r#"
        * Fisher (fisherman) marks Fisher: AbilityUsed
        wake Fisher
          storyteller [1 of 2 options] information for Fisher (Fisherman): "Do not trust Imp"
          -> Fisher: Do not trust Imp
        sleep Fisher
        --- grimoire ---
        Fisher: Fisherman [AbilityUsed by fisherman]
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // Drunk, the advice may be bad.
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.run(|ctx| poison(ctx, "Fisher"))
        .quiet()
        .picks(&[&[2]])
        .run(day_action("Fisher", fisherman));
    expect![[r#"
        * Fisher (fisherman) marks Fisher: AbilityUsed
        wake Fisher
          storyteller [1 of 4 options] information for Fisher (Fisherman), who is poisoned or drunk: false info is allowed: "Trust Imp" (false)
          -> Fisher: Trust Imp
        sleep Fisher
        --- grimoire ---
        Fisher: Fisherman [Poisoned by poisoner] [AbilityUsed by fisherman]
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

// --- Bounty Hunter ----------------------------------------------------------------

fn bounty_hunter_game() -> common::Game {
    common::Game::new(&[
        ("Hunter", &characters::bounty_hunter::Def),
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
        ("Monk", &characters::monk::Def),
        ("Mayor", &characters::mayor::Def),
    ])
}

#[test]
fn bounty_hunter_turns_a_townsfolk_evil_and_learns_evil_players() {
    let hunter: character::Ref = &characters::bounty_hunter::Def;
    let mut g = bounty_hunter_game();
    g.picks(&[&[0], &[0, 1, 2], &[1]])
        .answers(&[&["Monk"]])
        .run(|ctx| narrator::setup(ctx, &script()))
        .run(|ctx| {
            ctx.advance_phase();
            narrator::run_night(ctx, &script());
        });
    expect![[r#"
          storyteller [1 of 4 options] Bounty Hunter Hunter: which Townsfolk is evil: Chef
        * Chef is now Evil
        wake Chef
          -> Chef: Chef is Evil
        sleep Chef
        * === Night 1 ===
        wake Poisoner
          -> Poisoner: Evil: Imp, Poisoner
        sleep Poisoner
        wake Imp
          -> Imp: Evil: Imp, Poisoner
          storyteller [3 of 25 options] demon bluffs (not-in-play good characters): Washerwoman, Librarian, Investigator
          -> Imp: Bluffs: Washerwoman, Librarian, Investigator
        sleep Imp
        wake Poisoner
          Poisoner? Who do you want to poison? => Monk
        * Poisoner (poisoner) marks Monk: Poisoned
        sleep Poisoner
        === parallel (2) ===
        wake Chef
          storyteller [1 of 1 options] information for Chef (Chef): "2 evil pairs"
          -> Chef: 2 evil pairs
        sleep Chef
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        === end parallel ===
        wake Hunter
          storyteller [1 of 3 options] information for Hunter (Bounty Hunter): "Poisoner"
          -> Hunter: Poisoner
        sleep Hunter
        * Hunter (bounty_hunter) marks Poisoner: Known
        --- grimoire ---
        Hunter: Bounty Hunter
        Imp: Imp
        Poisoner: Poisoner [Known by bounty_hunter]
        Chef: Chef (Evil)
        Empath: Empath
        Monk: Monk [Poisoned by poisoner]
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());

    // The player they know dies: the next night they learn another, never
    // the same one.
    g.run(next_phase)
        .run(|ctx| {
            ctx.execute(&p("Poisoner"));
        })
        .run(next_phase)
        .quiet()
        .run(act(hunter, 2))
        .run(next_phase)
        .run(next_phase)
        .run(act(hunter, 3));
    expect![[r#"
        wake Hunter
          storyteller [1 of 2 options] information for Hunter (Bounty Hunter): "Imp"
          -> Hunter: Imp
        sleep Hunter
        * Hunter (bounty_hunter) marks Imp: Known
        * === Day 2 ===
        * === Night 3 ===
        --- grimoire ---
        Hunter: Bounty Hunter
        Imp: Imp [Known by bounty_hunter]
        Poisoner: Poisoner [dead]
        Chef: Chef (Evil)
        Empath: Empath
        Monk: Monk
        Mayor: Mayor
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn drunk_bounty_hunter_makes_nobody_evil() {
    let mut g = common::Game::new(&[
        ("Drunk", &characters::drunk::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
        ("Empath", &characters::empath::Def),
    ]);
    g.run(|ctx| {
        ctx.set_believed(&p("Drunk"), Some(&characters::bounty_hunter::Def));
        ctx.set_script(script().characters);
        character::Def::on_setup(&characters::bounty_hunter::Def, ctx, &p("Drunk"));
        ctx.advance_phase();
    })
    .quiet()
    .picks(&[&[1]])
    .run(act(&characters::bounty_hunter::Def, 1));
    expect![[r#"
        wake Drunk
          storyteller [1 of 3 options] information for Drunk (Bounty Hunter), who is poisoned or drunk: false info is allowed: "Chef" (false)
          -> Drunk: Chef
        sleep Drunk
        * Drunk (bounty_hunter) marks Chef: Known
        --- grimoire ---
        Drunk: Drunk (believes Bounty Hunter)
        Imp: Imp
        Chef: Chef [Known by bounty_hunter]
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

// --- Snake Charmer ----------------------------------------------------------------

fn snake_charmer_game() -> common::Game {
    common::Game::new(&[
        ("Charmer", &characters::snake_charmer::Def),
        ("Imp", &characters::imp::Def),
        ("Empath", &characters::empath::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Chef", &characters::chef::Def),
    ])
}

#[test]
fn snake_charmer_swaps_with_the_demon() {
    let mut g = snake_charmer_game();
    to_night(&mut g, 2);
    g.answers(&[&["Imp"], &["Empath"]]).run(|ctx| {
        narrator::run_order(
            ctx,
            &[&characters::snake_charmer::Def, &characters::imp::Def],
            2,
        )
    });
    expect![[r#"
        wake Charmer
          Charmer? Whom do you charm? => Imp
        sleep Charmer
        * Charmer becomes the Imp
        * Charmer is now Evil
        * Imp becomes the Snake Charmer
        * Imp is now Good
        * Charmer (snake_charmer) marks Imp: Poisoned
        wake Charmer
          -> Charmer: Imp
          -> Charmer: Charmer is Evil
        sleep Charmer
        wake Imp
          -> Imp: Snake Charmer
          -> Imp: Imp is Good
        sleep Imp
        wake Charmer
          Charmer? Who do you want to kill? => Empath
        sleep Charmer
        * Empath dies (Demon)
        --- grimoire ---
        Charmer: Imp
        Imp: Snake Charmer [Poisoned by snake_charmer]
        Empath: Empath [dead]
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // The old demon, now a poisoned Snake Charmer, cannot swap back.
    g.run(next_phase)
        .run(next_phase)
        .quiet()
        .answers(&[&["Charmer"]])
        .run(act(&characters::snake_charmer::Def, 3));
    expect![[r#"
        wake Imp
          Imp? Whom do you charm? => Charmer
        sleep Imp
        --- grimoire ---
        Charmer: Imp
        Imp: Snake Charmer [Poisoned by snake_charmer]
        Empath: Empath [dead]
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn snake_charmer_miss_or_poisoned_does_nothing() {
    let mut g = snake_charmer_game();
    to_night(&mut g, 1);
    g.answers(&[&["Empath"]])
        .run(act(&characters::snake_charmer::Def, 1));
    expect![[r#"
        wake Charmer
          Charmer? Whom do you charm? => Empath
        sleep Charmer
        --- grimoire ---
        Charmer: Snake Charmer
        Imp: Imp
        Empath: Empath
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    let mut g = snake_charmer_game();
    to_night(&mut g, 2);
    g.run(|ctx| poison(ctx, "Charmer"))
        .quiet()
        .answers(&[&["Imp"]])
        .run(act(&characters::snake_charmer::Def, 2));
    expect![[r#"
        wake Charmer
          Charmer? Whom do you charm? => Imp
        sleep Charmer
        --- grimoire ---
        Charmer: Snake Charmer [Poisoned by poisoner]
        Imp: Imp
        Empath: Empath
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

// --- Gambler ----------------------------------------------------------------------

#[test]
fn gambler_dies_on_a_wrong_guess() {
    let gambler: character::Ref = &characters::gambler::Def;
    let seats: [(&str, character::Ref); 4] = [
        ("Gambler", gambler),
        ("Poisoner", &characters::poisoner::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
    ];
    assert!(
        !script()
            .first_night
            .iter()
            .any(|c| character::same(*c, gambler))
    );

    let mut g = common::Game::new(&seats);
    to_night(&mut g, 2);
    g.answers(&[&["Chef"]])
        .characters(&["Chef"])
        .run(act(gambler, 2));
    expect![[r#"
        wake Gambler
          Gambler? Whose character do you guess? => Chef
          Gambler? Which character are they? => Chef
        sleep Gambler
        --- grimoire ---
        Gambler: Gambler
        Poisoner: Poisoner
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    let mut g = common::Game::new(&seats);
    to_night(&mut g, 2);
    g.answers(&[&["Chef"]])
        .characters(&["Empath"])
        .run(act(gambler, 2));
    expect![[r#"
        wake Gambler
          Gambler? Whose character do you guess? => Chef
          Gambler? Which character are they? => Empath
        sleep Gambler
        * Gambler dies (Ability { by: Id("Gambler") })
        --- grimoire ---
        Gambler: Gambler [dead]
        Poisoner: Poisoner
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // Poisoned: a wrong guess does not kill.
    let mut g = common::Game::new(&seats);
    to_night(&mut g, 2);
    g.run(|ctx| poison(ctx, "Gambler"))
        .quiet()
        .answers(&[&["Chef"]])
        .characters(&["Empath"])
        .run(act(gambler, 2));
    assert!(g.state.is_alive(&p("Gambler")));
}

// --- Professor --------------------------------------------------------------------

fn professor_game() -> common::Game {
    common::Game::new(&[
        ("Professor", &characters::professor::Def),
        ("Slayer", &characters::slayer::Def),
        ("Saint", &characters::saint::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
    ])
}

#[test]
fn professor_resurrects_a_dead_townsfolk_with_their_ability() {
    let professor: character::Ref = &characters::professor::Def;
    let mut g = professor_game();
    to_day(&mut g, 1);
    g.answers(&[&["Chef"]])
        .run(day_action("Slayer", &characters::slayer::Def))
        .run(|ctx| {
            ctx.execute(&p("Slayer"));
        })
        .run(next_phase)
        .quiet()
        .answers(&[&["Slayer"]])
        .run(act(professor, 2));
    expect![[r#"
        wake Professor
          Professor? Whom do you resurrect? => Slayer
        sleep Professor
        * Professor (professor) marks Professor: AbilityUsed
        * Slayer is alive again
        * Slayer (slayer) removes Slayer's AbilityUsed token
        --- grimoire ---
        Professor: Professor [AbilityUsed by professor]
        Slayer: Slayer
        Saint: Saint
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // Used: the Professor no longer wakes.
    g.run(next_phase)
        .run(next_phase)
        .quiet()
        .run(act(professor, 3));
    expect![[r#"
        --- grimoire ---
        Professor: Professor [AbilityUsed by professor]
        Slayer: Slayer
        Saint: Saint
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn professor_wastes_the_ability_on_a_non_townsfolk_or_when_drunk() {
    let professor: character::Ref = &characters::professor::Def;
    let mut g = professor_game();
    to_night(&mut g, 2);
    g.run(|ctx| {
        ctx.kill(&p("Saint"), botc_narrator::event::Cause::Demon);
    })
    .quiet()
    .answers(&[&["Saint"]])
    .run(act(professor, 2));
    expect![[r#"
        wake Professor
          Professor? Whom do you resurrect? => Saint
        sleep Professor
        * Professor (professor) marks Professor: AbilityUsed
        --- grimoire ---
        Professor: Professor [AbilityUsed by professor]
        Slayer: Slayer
        Saint: Saint [dead]
        Imp: Imp
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    let mut g = professor_game();
    to_night(&mut g, 2);
    g.run(|ctx| {
        ctx.kill(&p("Chef"), botc_narrator::event::Cause::Demon);
        poison(ctx, "Professor");
    })
    .quiet()
    .answers(&[&["Chef"]])
    .run(act(professor, 2));
    expect![[r#"
        wake Professor
          Professor? Whom do you resurrect? => Chef
        sleep Professor
        * Professor (professor) marks Professor: AbilityUsed
        --- grimoire ---
        Professor: Professor [Poisoned by poisoner] [AbilityUsed by professor]
        Slayer: Slayer
        Saint: Saint
        Imp: Imp
        Chef: Chef [dead]
    "#]]
    .assert_eq(&g.transcript());
}

// --- Huntsman ---------------------------------------------------------------------

fn huntsman_game() -> common::Game {
    common::Game::new(&[
        ("Huntsman", &characters::huntsman::Def),
        ("Damsel", &Damsel),
        ("Imp", &characters::imp::Def),
        ("Poisoner", &characters::poisoner::Def),
        ("Chef", &characters::chef::Def),
    ])
}

#[test]
fn huntsman_turns_the_damsel_into_a_townsfolk() {
    let huntsman: character::Ref = &characters::huntsman::Def;
    let mut g = huntsman_game();
    to_night(&mut g, 2);
    g.answers(&[&["Damsel"]])
        .picks(&[&[0]])
        .run(act(huntsman, 2));
    expect![[r#"
        wake Huntsman
          Huntsman? Whom do you choose? => Damsel
        sleep Huntsman
        * Huntsman (huntsman) marks Huntsman: AbilityUsed
          storyteller [1 of 23 options] Huntsman Huntsman: which Townsfolk the Damsel becomes: Washerwoman
        * Damsel becomes the Washerwoman
        wake Damsel
          -> Damsel: Washerwoman
        sleep Damsel
        --- grimoire ---
        Huntsman: Huntsman [AbilityUsed by huntsman]
        Damsel: Washerwoman
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // Used: no more waking.
    g.run(next_phase)
        .run(next_phase)
        .quiet()
        .run(act(huntsman, 3));
    expect![[r#"
        --- grimoire ---
        Huntsman: Huntsman [AbilityUsed by huntsman]
        Damsel: Washerwoman
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn huntsman_wrong_pick_or_poisoned_spends_the_ability() {
    let huntsman: character::Ref = &characters::huntsman::Def;
    let mut g = huntsman_game();
    to_night(&mut g, 1);
    g.answers(&[&["Chef"]]).run(act(huntsman, 1));
    expect![[r#"
        wake Huntsman
          Huntsman? Whom do you choose? => Chef
        sleep Huntsman
        * Huntsman (huntsman) marks Huntsman: AbilityUsed
        --- grimoire ---
        Huntsman: Huntsman [AbilityUsed by huntsman]
        Damsel: Damsel
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    let mut g = huntsman_game();
    to_night(&mut g, 1);
    g.run(|ctx| poison(ctx, "Huntsman"))
        .quiet()
        .answers(&[&["Damsel"]])
        .run(act(huntsman, 1));
    expect![[r#"
        wake Huntsman
          Huntsman? Whom do you choose? => Damsel
        sleep Huntsman
        * Huntsman (huntsman) marks Huntsman: AbilityUsed
        --- grimoire ---
        Huntsman: Huntsman [Poisoned by poisoner] [AbilityUsed by huntsman]
        Damsel: Damsel
        Imp: Imp
        Poisoner: Poisoner
        Chef: Chef
    "#]]
    .assert_eq(&g.transcript());

    // A poisoned Damsel still changes.
    let mut g = huntsman_game();
    to_night(&mut g, 1);
    g.run(|ctx| poison(ctx, "Damsel"))
        .quiet()
        .answers(&[&["Damsel"]])
        .run(act(huntsman, 1));
    assert_eq!(
        g.state.character(&p("Damsel")).kind(),
        character::Kind::Townsfolk
    );
}

#[test]
fn setup_adds_the_damsel_with_the_huntsman() {
    let script = script();
    let mut seen = 0;
    for seed in 0..200 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let picks = setup::choose(&mut rng, 8, &script);
        let has = |id: &str| picks.iter().any(|c| c.id() == id);
        assert_eq!(picks.len(), 8);
        if has("huntsman") {
            seen += 1;
            assert!(has("damsel"), "seed {seed}: {picks:?}");
        }
    }
    assert!(seen > 0, "the Huntsman is picked");
}

// --- Simulation -------------------------------------------------------------------

#[test]
fn simulated_games_with_these_characters_terminate() {
    let script = script();
    for seed in 0..60 {
        for players in [5, 8, 12, 15] {
            let trace = sim::simulate(seed, players, &script);
            assert!(!trace.steps.is_empty());
        }
    }
}

// --- Registration and regained abilities ------------------------------------------

#[test]
fn recluse_may_register_as_a_minion_to_the_town_crier_and_juggler() {
    let seats: [(&str, character::Ref); 5] = [
        ("Crier", &characters::town_crier::Def),
        ("Juggler", &characters::juggler::Def),
        ("Recluse", &characters::recluse::Def),
        ("Imp", &characters::imp::Def),
        ("Chef", &characters::chef::Def),
    ];
    let mut g = common::Game::new(&seats);
    to_day(&mut g, 1);
    g.answers(&[&["Recluse"]])
        .characters(&["Poisoner"])
        .run(|ctx| {
            ctx.record(botc_narrator::event::Event::Claimed {
                player: p("Juggler"),
                ability: "juggler",
                claims: vec![(p("Recluse"), "Poisoner")],
            });
            narrator::nominate(ctx, &p("Recluse"), &p("Chef"));
        })
        .run(next_phase)
        .quiet()
        .picks(&[&[1], &[1]])
        .run(act(&characters::town_crier::Def, 2))
        .run(act(&characters::juggler::Def, 2));
    let log = g.transcript();
    assert!(log.contains("-> Crier: Yes"), "{log}");
    assert!(log.contains("-> Juggler: 1 correct"), "{log}");
}

#[test]
fn resurrected_slayer_can_shoot_again() {
    let mut g = professor_game();
    to_day(&mut g, 1);
    g.answers(&[&["Chef"]])
        .run(day_action("Slayer", &characters::slayer::Def))
        .run(|ctx| {
            ctx.execute(&p("Slayer"));
        })
        .run(next_phase)
        .answers(&[&["Slayer"]])
        .run(act(&characters::professor::Def, 2))
        .run(next_phase)
        .quiet()
        .answers(&[&["Imp"]])
        .run(day_action("Slayer", &characters::slayer::Def));
    assert!(!g.state.is_alive(&p("Imp")), "{}", g.transcript());
}
