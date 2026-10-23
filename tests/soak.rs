//! The soak runner and the trace invariants it checks.

use botc_narrator::script;
use botc_narrator::sim;
use botc_narrator::soak;

fn trouble_brewing() -> script::Script {
    script::by_id("trouble_brewing").expect("registered")
}

/// A Trouble Brewing game that breaks no invariant.
fn clean_trace() -> sim::Trace {
    (0..50)
        .map(|seed| sim::simulate(seed, 8, &trouble_brewing()))
        .find(|t| t.winner.is_some() && sim::problems(t).is_empty())
        .expect("most games are clean")
}

#[test]
fn clean_games_have_no_problems() {
    let clean = (0..40)
        .filter(|&seed| sim::problems(&sim::simulate(seed, 9, &trouble_brewing())).is_empty())
        .count();
    assert!(clean >= 35, "only {clean} of 40 games were clean");
}

#[test]
fn broken_traces_are_reported() {
    let mut trace = clean_trace();
    let decision = trace
        .steps
        .iter()
        .position(|s| s.kind == sim::Kind::Decision)
        .expect("the storyteller decided something");
    trace.steps[decision].audience = sim::Audience::Public;
    trace.steps[0].players.push("Zed".to_string());
    let last = trace.snapshots.len() - 1;
    let alive = trace.snapshots[last]
        .iter()
        .position(|s| s.alive)
        .expect("someone lives");
    trace.snapshots[last][alive].alive = false;
    let name = trace.snapshots[last][alive].name.clone();
    let problems = sim::problems(&trace);
    let has = |needle: &str| problems.iter().any(|p| p.contains(needle));
    assert!(has("seen by Public"), "{problems:?}");
    assert!(has("names unseated player \"Zed\""), "{problems:?}");
    assert!(
        has(&format!(
            "{name} is dead but their death was never announced"
        )),
        "{problems:?}"
    );
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(matches!(
        soak::judge(&trace),
        soak::Outcome::Failed(soak::Kind::Invariant, _)
    ));
}

#[test]
fn signatures_blank_out_names_and_numbers() {
    assert_eq!(
        soak::signature(soak::Kind::Invariant, "Ivo is dead (step 12), Ivory too"),
        "invariant: <player> is dead (step #), Ivory too"
    );
    assert_eq!(
        soak::signature(soak::Kind::Invariant, "Ada is dead (step 3), Ivory too"),
        soak::signature(soak::Kind::Invariant, "Bram is dead (step 40), Ivory too"),
    );
}

#[test]
fn panics_are_caught_with_their_location() {
    soak::install_panic_hook();
    match soak::outcome(|| panic!("engine bug {}", 7)) {
        soak::Outcome::Failed(soak::Kind::Panic, message) => {
            assert!(message.starts_with("engine bug 7 at tests"), "{message}");
        }
        _ => panic!("the panic should be caught"),
    }
}

#[test]
fn the_soak_loop_plays_games_and_writes_its_output() {
    let out = std::env::temp_dir().join(format!("botc-soak-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    let config = soak::Config {
        out: out.clone(),
        games: Some(40),
        duration: None,
        threads: 2,
        summary_every: std::time::Duration::from_secs(30),
        commit: "test".to_string(),
    };
    let summary = soak::run(&config).expect("runs");
    assert_eq!(summary.games, 40);
    let per_script: u64 = summary.scripts.values().map(|s| s.games).sum();
    assert_eq!(per_script, 40);
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("summary.json")).unwrap()).unwrap();
    assert_eq!(json["games"], 40);
    let md = std::fs::read_to_string(out.join("summary.md")).unwrap();
    assert!(md.contains("# Soak summary"), "{md}");
    let failures = std::fs::read_to_string(out.join("failures.jsonl")).unwrap();
    let failed: u64 = summary.scripts.values().map(|s| s.failures).sum();
    assert!(failures.lines().count() as u64 <= failed);
    for line in failures.lines() {
        let failure: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(failure["commit"], "test");
        assert!(failure["seed"].is_u64(), "{line}");
    }
    std::fs::remove_dir_all(&out).unwrap();
}

/// Regression (soak): one ability killing several players at once (a
/// Shabaloth or Po choosing itself, a Harlot's visit) ended the game at the
/// first death, so the others died after the win and were never announced;
/// likewise a win declared mid-action (the Legion) before a death it caused
/// (the Grandmother dying with her grandchild).
#[test]
fn simultaneous_deaths_are_all_announced_before_the_win() {
    for (id, players, seed) in [
        ("bad_moon_rising", 10, 15554947400389634276),
        ("bad_moon_rising", 10, 11161736968609814600),
        ("catfishing", 8, 18026017878132569668),
        ("the_minotaur_s_labyrinth", 10, 8988868737507419238),
        ("the_minotaur_s_labyrinth", 7, 1144812118948214340),
        ("separation_of_church_and_state", 6, 6278229522625069947),
        ("separation_of_church_and_state", 5, 11355379199763691427),
    ] {
        let trace = sim::simulate(seed, players, &script::by_id(id).expect("registered"));
        assert_eq!(
            sim::problems(&trace),
            Vec::<String>::new(),
            "{id} {players} {seed}"
        );
    }
}
