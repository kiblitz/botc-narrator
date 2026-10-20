mod common;

use expect_test::expect;

use botc_narrator::character;
use botc_narrator::characters::{
    baron, butler, chef, empath, fortune_teller, imp, investigator, librarian, mayor, monk,
    poisoner, ravenkeeper, saint, scarlet_woman, slayer, soldier, spy, undertaker, virgin,
    washerwoman,
};
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::trouble_brewing;

use common::p;

fn night(ctx: &mut botc_narrator::ctx::Write<'_>) {
    ctx.advance_phase();
    narrator::run_night(ctx, &trouble_brewing::script());
}

fn seven() -> common::Game {
    common::Game::new(&[
        ("Lucius", &imp::Def),
        ("Ivy", &poisoner::Def),
        ("Troi", &empath::Def),
        ("Gordon", &chef::Def),
        ("Zoltar", &fortune_teller::Def),
        ("Friar", &monk::Def),
        ("Buffy", &slayer::Def),
    ])
}

#[test]
fn poisoned_chef_and_three_phases() {
    let mut g = seven();
    // Bluffs take the defaults; the poisoned Chef is shown the false "0" (option 1).
    g.answers(&[&["Gordon"], &["Ivy", "Lucius"]])
        .picks(&[&[0, 1, 2], &[1]])
        .run(night);
    expect![[r#"
        * === Night 1 ===
        wake Ivy
          -> Ivy: Evil: Lucius, Ivy
        sleep Ivy
        wake Lucius
          -> Lucius: Evil: Lucius, Ivy
          storyteller [3 of 12 options] demon bluffs (not-in-play good characters): Washerwoman, Librarian, Investigator
          -> Lucius: Bluffs: Washerwoman, Librarian, Investigator
        sleep Lucius
        wake Ivy
          Ivy? Who do you want to poison? => Gordon
        sleep Ivy
        === parallel (3) ===
        wake Gordon
          storyteller [1 of 3 options] information for Gordon (Chef), who is poisoned or drunk: false info is allowed: "0 evil pairs" (false)
          -> Gordon: 0 evil pairs
        sleep Gordon
        wake Troi
          storyteller [1 of 1 options] information for Troi (Empath): "1 evil neighbours"
          -> Troi: 1 evil neighbours
        sleep Troi
        wake Zoltar
          Zoltar? Choose two players => Ivy, Lucius
          storyteller [1 of 1 options] information for Zoltar (Fortune Teller): "Yes"
          -> Zoltar: Yes
        sleep Zoltar
        === end parallel ===
        --- grimoire ---
        Lucius: Imp
        Ivy: Poisoner
        Troi: Empath
        Gordon: Chef [Poisoned by poisoner]
        Zoltar: Fortune Teller
        Friar: Monk
        Buffy: Slayer
    "#]]
    .assert_eq(&g.transcript());

    g.answers(&[&["Ivy"]]).run(|ctx| {
        ctx.advance_phase();
        narrator::day_action(ctx, &p("Buffy"));
        // A second shot is ignored: the ability is used up.
        narrator::day_action(ctx, &p("Buffy"));
    });
    expect![[r#"
        * === Day 1 ===
          Buffy? Who do you want to slay? => Ivy
        --- grimoire ---
        Lucius: Imp
        Ivy: Poisoner
        Troi: Empath
        Gordon: Chef [Poisoned by poisoner]
        Zoltar: Fortune Teller
        Friar: Monk
        Buffy: Slayer [AbilityUsed by slayer]
    "#]]
    .assert_eq(&g.transcript());

    // Poison moves to Zoltar, who is then lied to ("No", option 1).
    g.answers(&[&["Zoltar"], &["Troi"], &["Gordon"], &["Lucius", "Friar"]])
        .picks(&[&[1]])
        .run(night);
    expect![[r#"
        * === Night 2 ===
        wake Ivy
          Ivy? Who do you want to poison? => Zoltar
        sleep Ivy
        wake Friar
          Friar? Who do you want to protect? => Troi
        sleep Friar
        wake Lucius
          Lucius? Who do you want to kill? => Gordon
        sleep Lucius
        * Gordon dies (Demon)
        === parallel (2) ===
        wake Troi
          storyteller [1 of 1 options] information for Troi (Empath): "1 evil neighbours"
          -> Troi: 1 evil neighbours
        sleep Troi
        wake Zoltar
          Zoltar? Choose two players => Lucius, Friar
          storyteller [1 of 2 options] information for Zoltar (Fortune Teller), who is poisoned or drunk: false info is allowed: "No" (false)
          -> Zoltar: No
        sleep Zoltar
        === end parallel ===
        --- grimoire ---
        Lucius: Imp
        Ivy: Poisoner
        Troi: Empath [Protected by monk]
        Gordon: Chef [dead]
        Zoltar: Fortune Teller [Poisoned by poisoner]
        Friar: Monk
        Buffy: Slayer [AbilityUsed by slayer]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn monk_protection_stops_the_demon() {
    let mut g = seven();
    g.run(night).quiet();
    g.run(|ctx| {
        ctx.advance_phase();
    });
    g.answers(&[&["Buffy"], &["Troi"], &["Troi"], &["Ivy", "Troi"]])
        .quiet()
        .run(night);
    expect![[r#"
        * === Night 2 ===
        wake Ivy
          Ivy? Who do you want to poison? => Buffy
        sleep Ivy
        wake Friar
          Friar? Who do you want to protect? => Troi
        sleep Friar
        wake Lucius
          Lucius? Who do you want to kill? => Troi
        sleep Lucius
        * Troi survives (monk)
        === parallel (2) ===
        wake Troi
          storyteller [1 of 1 options] information for Troi (Empath): "1 evil neighbours"
          -> Troi: 1 evil neighbours
        sleep Troi
        wake Zoltar
          Zoltar? Choose two players => Ivy, Troi
          storyteller [1 of 1 options] information for Zoltar (Fortune Teller): "No"
          -> Zoltar: No
        sleep Zoltar
        === end parallel ===
        --- grimoire ---
        Lucius: Imp
        Ivy: Poisoner
        Troi: Empath [Protected by monk]
        Gordon: Chef
        Zoltar: Fortune Teller
        Friar: Monk
        Buffy: Slayer [Poisoned by poisoner]
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn invalid_player_answers_fall_back_to_first_candidate() {
    let mut g = seven();
    // The poisoner cannot poison themself; the answer is replaced.
    g.answers(&[&["Ivy"]]).run(night);
    assert!(g.state.has_effect(&p("Lucius"), game::Effect::Poisoned));
}

#[test]
fn soldier_survives_unless_poisoned() {
    let seats: &[(&str, character::Ref)] = &[
        ("Imp", &imp::Def),
        ("Poisoner", &poisoner::Def),
        ("Soldier", &soldier::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ];
    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.answers(&[&["Chef"], &["Soldier"]]).run(night);
    assert!(g.state.is_alive(&p("Soldier")));

    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.answers(&[&["Soldier"], &["Soldier"]]).quiet().run(night);
    expect![[r#"
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Soldier
        sleep Poisoner
        wake Imp
          Imp? Who do you want to kill? => Soldier
        sleep Imp
        * Soldier dies (Demon)
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Poisoner: Poisoner
        Soldier: Soldier [dead] [Poisoned by poisoner]
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn imp_starpass_passes_to_chosen_minion() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Poisoner", &poisoner::Def),
        ("Baron", &baron::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
        ("Saint", &saint::Def),
    ]);
    g.run(|ctx| {
        narrator::setup(ctx);
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.quiet()
        .answers(&[&["Chef"], &["Imp"]])
        .picks(&[&[1]])
        .run(night);
    expect![[r#"
        * === Night 2 ===
        wake Poisoner
          Poisoner? Who do you want to poison? => Chef
        sleep Poisoner
        wake Imp
          Imp? Who do you want to kill? => Imp
        sleep Imp
        * Imp dies (Starpass)
          storyteller [1 of 2 options] which minion becomes the new demon: Baron
        * Baron becomes the Imp
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "0 evil neighbours"
          -> Empath: 0 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp [dead]
        Poisoner: Poisoner
        Baron: Imp
        Chef: Chef [Poisoned by poisoner]
        Empath: Empath
        Saint: Saint
    "#]]
    .assert_eq(&g.transcript());
    assert_eq!(g.state.winner(), None);

    // The new Imp (seated after the dead one) kills on the following night.
    g.run(|ctx| {
        ctx.advance_phase();
    });
    g.answers(&[&["Chef"], &["Empath"]]).quiet().run(night);
    assert!(!g.state.is_alive(&p("Empath")), "{}", g.transcript());
}

#[test]
fn starpass_without_minions_ends_the_game() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.answers(&[&["Imp"]]).run(night);
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
}

#[test]
fn scarlet_woman_catches_the_demon_with_five_alive() {
    let seats: &[(&str, character::Ref)] = &[
        ("Imp", &imp::Def),
        ("Scarlet", &scarlet_woman::Def),
        ("Slayer", &slayer::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ];
    let mut g = common::Game::new(seats);
    g.answers(&[&["Imp"]]).run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::day_action(ctx, &p("Slayer"));
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
          Slayer? Who do you want to slay? => Imp
        * Imp dies (Ability { by: Id("Slayer") })
        * Scarlet becomes the Imp
        --- grimoire ---
        Imp: Imp [dead]
        Scarlet: Imp
        Slayer: Slayer [AbilityUsed by slayer]
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // With only four alive, the demon's death ends the game.
    let mut g = common::Game::new(&seats[..4]);
    g.answers(&[&["Imp"]]).run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::day_action(ctx, &p("Slayer"));
    });
    assert_eq!(g.state.winner(), Some(character::Alignment::Good));
    assert_eq!(g.state.character(&p("Scarlet")).id(), "scarlet_woman");
}

#[test]
fn ravenkeeper_learns_a_character_when_killed() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Spy", &spy::Def),
        ("Raven", &ravenkeeper::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.answers(&[&["Raven"], &["Spy"]]).quiet().run(night);
    expect![[r#"
        * === Night 2 ===
        wake Imp
          Imp? Who do you want to kill? => Raven
        sleep Imp
        * Raven dies (Demon)
        === parallel (3) ===
        wake Raven
          Raven? Choose a player to learn about => Spy
          storyteller [1 of 1 options] information for Raven (Ravenkeeper): "Spy"
          -> Raven: Spy
        sleep Raven
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        wake Spy
          storyteller [1 of 1 options] information for Spy (Spy): "Imp=Imp, Spy=Spy, Raven=Ravenkeeper, Chef=Chef, Empath=Empath"
          -> Spy: Imp=Imp, Spy=Spy, Raven=Ravenkeeper, Chef=Chef, Empath=Empath
        sleep Spy
        === end parallel ===
        --- grimoire ---
        Imp: Imp
        Spy: Spy
        Raven: Ravenkeeper [dead]
        Chef: Chef
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // Next night the Ravenkeeper is dead but did not die tonight: no wake.
    g.run(|ctx| {
        ctx.advance_phase();
    });
    g.answers(&[&["Chef"]]).quiet().run(night);
    assert!(!g.transcript().contains("wake Raven"));
}

#[test]
fn virgin_executes_a_townsfolk_nominator_once() {
    let seats: &[(&str, character::Ref)] = &[
        ("Imp", &imp::Def),
        ("Virgin", &virgin::Def),
        ("Chef", &chef::Def),
        ("Butler", &butler::Def),
        ("Empath", &empath::Def),
    ];
    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::nominate(ctx, &p("Chef"), &p("Virgin"));
        narrator::nominate(ctx, &p("Empath"), &p("Virgin"));
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * Chef nominates Virgin
        * Chef is executed
        * Chef dies (Execution)
        * Empath nominates Virgin
        --- grimoire ---
        Imp: Imp
        Virgin: Virgin [AbilityUsed by virgin]
        Chef: Chef [dead]
        Butler: Butler
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());

    // An outsider nominator uses up the ability without dying.
    let mut g = common::Game::new(seats);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        narrator::nominate(ctx, &p("Butler"), &p("Virgin"));
        narrator::nominate(ctx, &p("Chef"), &p("Virgin"));
    });
    assert!(g.state.is_alive(&p("Butler")));
    assert!(g.state.is_alive(&p("Chef")));
}

#[test]
fn executing_the_saint_wins_for_evil() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Saint", &saint::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.execute(&p("Saint"));
    });
    expect![[r#"
        * === Night 1 ===
        * === Day 1 ===
        * Saint is executed
        * Evil wins!
        * Saint dies (Execution)
        --- grimoire ---
        Imp: Imp
        Saint: Saint [dead]
        Chef: Chef
        Empath: Empath
        winner: Evil
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn mayor_kill_can_bounce() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Mayor", &mayor::Def),
        ("Chef", &chef::Def),
        ("Soldier", &soldier::Def),
        ("Empath", &empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    // Options: Mayor, then the other living non-demons. Bounce to the Soldier,
    // who is safe from the demon.
    g.answers(&[&["Mayor"]]).picks(&[&[2]]).quiet().run(night);
    expect![[r#"
        * === Night 2 ===
        wake Imp
          Imp? Who do you want to kill? => Mayor
        sleep Imp
          storyteller [1 of 4 options] the demon attacked Mayor Mayor: who dies instead: Soldier
        * kill on Mayor redirected to Soldier
        * Soldier survives (soldier)
        wake Empath
          storyteller [1 of 1 options] information for Empath (Empath): "1 evil neighbours"
          -> Empath: 1 evil neighbours
        sleep Empath
        --- grimoire ---
        Imp: Imp
        Mayor: Mayor
        Chef: Chef
        Soldier: Soldier
        Empath: Empath
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn undertaker_learns_todays_execution() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Baron", &baron::Def),
        ("Under", &undertaker::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
        ("Saint", &saint::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
        ctx.execute(&p("Baron"));
    });
    g.answers(&[&["Saint"]]).quiet().run(night);
    let transcript = g.transcript();
    assert!(transcript.contains("  -> Under: Baron\n"), "{transcript}");
}

#[test]
fn first_night_info_characters_get_truthful_options() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Spy", &spy::Def),
        ("Wash", &washerwoman::Def),
        ("Lib", &librarian::Def),
        ("Inv", &investigator::Def),
        ("Butler", &butler::Def),
    ]);
    // Six players: no evil info. The Washerwoman sees option 1, the others
    // the first option.
    g.picks(&[&[1]]).answers(&[&["Imp"]]).run(night);
    expect![[r#"
        * === Night 1 ===
        === parallel (3) ===
        wake Wash
          storyteller [1 of 16 options] information for Wash (Washerwoman): "Imp and Lib: one is the Librarian"
          -> Wash: Imp and Lib: one is the Librarian
        sleep Wash
        wake Lib
          storyteller [1 of 8 options] information for Lib (Librarian): "Butler and Imp: one is the Butler"
          -> Lib: Butler and Imp: one is the Butler
        sleep Lib
        wake Inv
          storyteller [1 of 8 options] information for Inv (Investigator): "Spy and Imp: one is the Spy"
          -> Inv: Spy and Imp: one is the Spy
        sleep Inv
        === end parallel ===
        wake Butler
          Butler? Choose your master => Imp
        sleep Butler
        wake Spy
          storyteller [1 of 1 options] information for Spy (Spy): "Imp=Imp, Spy=Spy, Wash=Washerwoman, Lib=Librarian, Inv=Investigator, Butler=Butler"
          -> Spy: Imp=Imp, Spy=Spy, Wash=Washerwoman, Lib=Librarian, Inv=Investigator, Butler=Butler
        sleep Spy
        --- grimoire ---
        Imp: Imp [Custom("master") by butler]
        Spy: Spy
        Wash: Washerwoman
        Lib: Librarian
        Inv: Investigator
        Butler: Butler
    "#]]
    .assert_eq(&g.transcript());
}

#[test]
fn invalid_storyteller_pick_falls_back_to_first_option() {
    let mut g = seven();
    // Out of range and duplicate picks are both rejected.
    g.picks(&[&[0, 0, 99]]).run(night);
    assert!(
        g.transcript()
            .contains("Bluffs: Washerwoman, Librarian, Investigator")
    );
}

#[test]
fn poison_ends_when_the_poisoner_dies() {
    let mut g = seven();
    g.answers(&[&["Gordon"]]).run(night);
    assert!(g.state.has_effect(&p("Gordon"), game::Effect::Poisoned));
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.execute(&p("Ivy"));
    });
    assert!(!g.state.has_effect(&p("Gordon"), game::Effect::Poisoned));
}

#[test]
fn poisoned_slayer_and_virgin_use_up_their_abilities_for_nothing() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Poisoner", &poisoner::Def),
        ("Slayer", &slayer::Def),
        ("Virgin", &virgin::Def),
        ("Chef", &chef::Def),
    ]);
    g.answers(&[&["Slayer"]]).run(night);
    g.answers(&[&["Imp"]]).run(|ctx| {
        ctx.advance_phase();
        narrator::day_action(ctx, &p("Slayer"));
    });
    assert!(g.state.is_alive(&p("Imp")));
    assert!(g.state.has_effect(&p("Slayer"), game::Effect::AbilityUsed));

    g.answers(&[&["Virgin"], &["Chef"]]).run(night);
    g.run(|ctx| {
        ctx.advance_phase();
        narrator::nominate(ctx, &p("Slayer"), &p("Virgin"));
    });
    assert!(g.state.is_alive(&p("Slayer")));
    assert!(g.state.has_effect(&p("Virgin"), game::Effect::AbilityUsed));
}

#[test]
fn evil_wins_when_two_players_remain() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
    ]);
    g.run(|ctx| {
        ctx.advance_phase();
        ctx.advance_phase();
    });
    g.answers(&[&["Chef"]]).run(night);
    assert_eq!(g.state.winner(), Some(character::Alignment::Evil));
}

#[test]
fn small_games_skip_minion_and_demon_info() {
    let mut g = common::Game::new(&[
        ("Imp", &imp::Def),
        ("Poisoner", &poisoner::Def),
        ("Chef", &chef::Def),
        ("Empath", &empath::Def),
        ("Monk", &monk::Def),
        ("Slayer", &slayer::Def),
    ]);
    g.answers(&[&["Chef"]]).run(night);
    let transcript = g.transcript();
    assert!(!transcript.contains("Evil:"), "{transcript}");
    assert!(!transcript.contains("Bluffs:"), "{transcript}");
}
