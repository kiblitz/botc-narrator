//! A non-interactive console demo: a small Trouble Brewing game driven by the
//! [`ConsoleStoryteller`](botc_narrator::interp::ConsoleStoryteller), which
//! always takes the first available option. Run with `cargo run`.

use botc_narrator::characters::{
    self, Chef, Empath, FortuneTeller, Imp, Monk, Poisoner, Slayer,
};
use botc_narrator::engine::Engine;
use botc_narrator::grimoire::{Grimoire, PlayerSpec};
use botc_narrator::ids::PlayerId;
use botc_narrator::interp::ConsoleStoryteller;
use botc_narrator::scripts::trouble_brewing;

fn main() {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let grim = Grimoire::new(vec![
        PlayerSpec::new("Diablo", Imp::INFO),
        PlayerSpec::new("Ivy", Poisoner::INFO),
        PlayerSpec::new("Troi", Empath::INFO),
        PlayerSpec::new("Gordon", Chef::INFO),
        PlayerSpec::new("Zoltar", FortuneTeller::INFO),
        PlayerSpec::new("Friar", Monk::INFO),
        PlayerSpec::new("Buffy", Slayer::INFO),
    ]);

    let mut engine = Engine::new(grim, &reg, &script);
    let mut st = ConsoleStoryteller::new();

    println!("== Setup ==");
    engine.run_setup(&mut st);

    println!("\n== Night 1 ==");
    engine.run_night(&mut st);

    println!("\n== Day 1: the town executes the Poisoner ==");
    engine.grim.advance_phase(); // -> Day 1
    engine.execute(&mut st, PlayerId(1));

    println!("\n== Night 2 ==");
    engine.run_night(&mut st);

    println!("\n== Final grimoire ==");
    print_grimoire(&engine.grim, &reg);
}

fn print_grimoire(grim: &Grimoire, reg: &botc_narrator::registry::Registry) {
    for id in grim.seats() {
        let p = grim.get(id);
        let role = reg.info(p.role).name;
        let mut tags = Vec::new();
        if !p.alive {
            tags.push("dead".to_string());
        }
        if p.is_impaired() {
            tags.push("impaired".to_string());
        }
        for t in p.tokens() {
            tags.push(format!("{t:?}"));
        }
        let suffix = if tags.is_empty() {
            String::new()
        } else {
            format!("  [{}]", tags.join(", "))
        };
        println!("  seat {}: {} — {}{}", id.seat(), p.name, role, suffix);
    }
    match grim.winner {
        Some(team) => println!("\n  Winner: {team:?}"),
        None => println!("\n  Game continues."),
    }
}
