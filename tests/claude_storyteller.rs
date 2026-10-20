mod common;

use expect_test::expect;

use botc_narrator::characters::{chef, empath, imp, poisoner};
use botc_narrator::game;
use botc_narrator::info;
use botc_narrator::storyteller;
use botc_narrator::storyteller::claude;

use common::p;

fn view() -> game::State {
    common::state(&[
        ("Lucius", &imp::Def),
        ("Ivy", &poisoner::Def),
        ("Gordon", &chef::Def),
        ("Troi", &empath::Def),
    ])
}

fn chef_decision() -> storyteller::Decision {
    let noun = "evil pairs";
    storyteller::Decision {
        reason: storyteller::Reason::Information {
            player: p("Gordon"),
            character: "Chef",
            malfunctioning: true,
        },
        options: vec![
            storyteller::Choice::Info {
                message: info::Message::Count { count: 1, noun },
                truthful: true,
            },
            storyteller::Choice::Info {
                message: info::Message::Count { count: 0, noun },
                truthful: false,
            },
            storyteller::Choice::Info {
                message: info::Message::Count { count: 2, noun },
                truthful: false,
            },
        ],
        count: 1,
    }
}

#[test]
fn prompt_describes_grimoire_reason_and_options() {
    expect![[r#"
        You are the Storyteller of a Blood on the Clocktower game. Be fair.

        Phase: Setup
        Grimoire (seat order):
        - Lucius: Imp (Demon, Evil)
        - Ivy: Poisoner (Minion, Evil)
        - Gordon: Chef (Townsfolk, Good)
        - Troi: Empath (Townsfolk, Good)

        Decision: information for Gordon (Chef), who is poisoned or drunk: false info is allowed
        Pick exactly 1 of these options:
        0: "1 evil pairs"
        1: "0 evil pairs" (false)
        2: "2 evil pairs" (false)

        Reply with only the chosen option number(s), separated by spaces."#]]
    .assert_eq(&claude::prompt("Be fair.", &chef_decision(), &view()));
}

#[test]
fn parse_picks_reads_the_first_valid_distinct_numbers() {
    let decision = chef_decision();
    assert_eq!(claude::parse_picks("2", &decision), Ok(vec![2]));
    assert_eq!(
        claude::parse_picks("I pick option 1.\n", &decision),
        Ok(vec![1])
    );
    assert_eq!(claude::parse_picks("7 then 0", &decision), Ok(vec![0]));
    assert!(claude::parse_picks("none of them", &decision).is_err());

    let mut two = chef_decision();
    two.count = 2;
    assert_eq!(claude::parse_picks("2, 2, 0", &two), Ok(vec![2, 0]));
    assert!(claude::parse_picks("1", &two).is_err());
}

#[test]
fn unavailable_command_falls_back_to_a_random_legal_pick() {
    let mut st = claude::Storyteller::with_command("botc-no-such-program", Vec::new());
    let decision = chef_decision();
    let picks = storyteller::Strategy::decide(&mut st, &decision, &view());
    assert!(decision.is_valid(&picks), "{picks:?}");
}

/// Calls the real Claude Code CLI. Run with `cargo test -- --ignored`.
#[test]
#[ignore]
fn live_claude_code_makes_a_legal_pick() {
    let mut st = claude::Storyteller::new();
    let decision = chef_decision();
    let picks = storyteller::Strategy::decide(&mut st, &decision, &view());
    assert!(decision.is_valid(&picks), "{picks:?}");
}
