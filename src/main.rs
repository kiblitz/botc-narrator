//! Demo: plays a 7-player Trouble Brewing game on the console.
//!
//! Usage: botc_narrator [random|claude|first] [seed]
//!   random  storyteller picks uniformly at random (default)
//!   claude  storyteller decisions are made by Claude Code (`claude -p`)
//!   first   storyteller always takes the first legal option

use rand::SeedableRng;
use rand::seq::IndexedRandom;

use botc_narrator::characters::{chef, empath, fortune_teller, imp, monk, poisoner, slayer};
use botc_narrator::ctx;
use botc_narrator::game;
use botc_narrator::narrator;
use botc_narrator::player;
use botc_narrator::storyteller;
use botc_narrator::table;
use botc_narrator::trouble_brewing;

const MAX_DAYS: u32 = 10;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut storyteller: Box<dyn storyteller::Strategy> = match args.first().map(String::as_str) {
        Some("claude") => Box::new(storyteller::claude::Storyteller::new()),
        Some("first") => Box::new(storyteller::Scripted::default()),
        Some("random") | None => Box::new(storyteller::Random::seeded(seed)),
        Some(other) => {
            eprintln!("unknown storyteller {other:?}: expected random, claude or first");
            std::process::exit(2);
        }
    };
    let script = trouble_brewing::script();
    let mut state = game::State::new(vec![
        (player::Id::new("Lucius"), &imp::Def),
        (player::Id::new("Ivy"), &poisoner::Def),
        (player::Id::new("Troi"), &empath::Def),
        (player::Id::new("Gordon"), &chef::Def),
        (player::Id::new("Zoltar"), &fortune_teller::Def),
        (player::Id::new("Friar"), &monk::Def),
        (player::Id::new("Buffy"), &slayer::Def),
    ]);
    let mut io = table::Console::new();
    let mut ctx = ctx::Write::new(&mut state, &mut io, storyteller.as_mut());
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    narrator::setup(&mut ctx);
    while ctx.state().winner().is_none() {
        ctx.advance_phase();
        match ctx.state().phase() {
            game::Phase::Night(_) => narrator::run_night(&mut ctx, &script),
            game::Phase::Day(day) if day > MAX_DAYS => break,
            game::Phase::Day(day) => {
                if day == 1
                    && let Some(slayer) = ctx.state().find("slayer").cloned()
                {
                    narrator::day_action(&mut ctx, &slayer);
                }
                let alive = ctx.state().alive_ids();
                if let (Some(nominator), Some(nominee)) =
                    (alive.choose(&mut rng), alive.choose(&mut rng))
                {
                    narrator::nominate(&mut ctx, nominator, nominee);
                    ctx.execute(nominee);
                }
            }
            game::Phase::Setup => {}
        }
    }
    if ctx.state().winner().is_none() {
        println!("No winner after {MAX_DAYS} days.");
    }
}
