//! Random but plausible games, recorded step by step for the web simulator.
//!
//! Setup follows the Trouble Brewing character counts. Players act on simple
//! heuristics (evil targets good players, everyone else chooses at random), the
//! storyteller is [`storyteller::Random`], and days run nominations and a
//! majority vote. Every interaction is recorded as a [`Step`] with the
//! grimoire as it stood at that moment.

use rand::Rng;
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use rand::seq::index;
use serde::Serialize;

use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::narrator;
use crate::player;
use crate::storyteller;
use crate::table;
use crate::trouble_brewing;

pub const MIN_PLAYERS: usize = 5;
pub const MAX_PLAYERS: usize = 15;
const MAX_DAYS: u32 = 12;

const NAMES: [&str; MAX_PLAYERS] = [
    "Ada", "Bram", "Cleo", "Dev", "Esme", "Finn", "Gus", "Hana", "Ivo", "Juno", "Kit", "Lux",
    "Mira", "Nico", "Oona",
];

/// (townsfolk, outsiders, minions) for a player count; there is always one demon.
fn distribution(players: usize) -> (usize, usize, usize) {
    match players {
        5 => (3, 0, 1),
        6 => (3, 1, 1),
        n => {
            let extra = n - 7;
            (5 + 2 * (extra / 3), extra % 3, 1 + extra / 3)
        }
    }
}

/// A legal Trouble Brewing setup in random seat order. The Drunk is left out
/// because it is not modelled yet.
pub fn setup(rng: &mut impl Rng, players: usize) -> Vec<(player::Id, character::Ref)> {
    let players = players.clamp(MIN_PLAYERS, MAX_PLAYERS);
    let script = trouble_brewing::script();
    let of_kind = |kind: character::Kind| -> Vec<character::Ref> {
        script
            .characters
            .iter()
            .copied()
            .filter(|c| c.kind() == kind && c.id() != "drunk")
            .collect()
    };
    let pick = |rng: &mut dyn rand::RngCore, pool: &[character::Ref], n: usize| {
        index::sample(rng, pool.len(), n.min(pool.len()))
            .into_iter()
            .map(|i| pool[i])
            .collect::<Vec<_>>()
    };
    let (mut townsfolk, mut outsiders, minions) = distribution(players);
    let minions = pick(rng, &of_kind(character::Kind::Minion), minions);
    let outsider_pool = of_kind(character::Kind::Outsider);
    if minions.iter().any(|m| m.id() == "baron") {
        let added = 2.min(outsider_pool.len() - outsiders);
        outsiders += added;
        townsfolk -= added;
    }
    let mut characters = minions;
    characters.extend(pick(rng, &of_kind(character::Kind::Demon), 1));
    characters.extend(pick(rng, &outsider_pool, outsiders));
    characters.extend(pick(rng, &of_kind(character::Kind::Townsfolk), townsfolk));
    characters.shuffle(rng);
    NAMES
        .iter()
        .zip(characters)
        .map(|(name, c)| (player::Id::new(*name), c))
        .collect()
}

#[derive(Clone, Debug, Serialize)]
pub struct Trace {
    pub seed: u64,
    pub players: Vec<Seat>,
    /// Grimoire snapshots; each step refers to one by index.
    pub snapshots: Vec<Vec<Seat>>,
    pub steps: Vec<Step>,
    pub winner: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Seat {
    pub name: String,
    pub character: &'static str,
    pub kind: String,
    pub alignment: String,
    pub alive: bool,
    pub reminders: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    /// Everyone at the table sees it.
    Public,
    /// Only `player` (and the storyteller) sees it.
    Player,
    /// Only the storyteller knows.
    Storyteller,
}

#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub phase: String,
    pub kind: &'static str,
    pub player: Option<String>,
    pub text: String,
    pub audience: Audience,
    pub snapshot: usize,
}

fn seats(view: &game::State) -> Vec<Seat> {
    view.players()
        .iter()
        .map(|p| Seat {
            name: p.id.to_string(),
            character: p.character.name(),
            kind: p.character.kind().to_string(),
            alignment: p.alignment.to_string(),
            alive: p.alive,
            reminders: p
                .reminders
                .iter()
                .map(|r| match r.effect {
                    game::Effect::Custom(name) => name.to_string(),
                    effect => format!("{effect:?}"),
                })
                .collect(),
        })
        .collect()
}

/// The table during a simulation: records every step and answers questions
/// for the players with plausible heuristics.
struct Recorder {
    rng: rand::rngs::StdRng,
    view: game::State,
    phase: String,
    snapshots: Vec<Vec<Seat>>,
    steps: Vec<Step>,
}

impl Recorder {
    fn push(
        &mut self,
        kind: &'static str,
        player: Option<&player::Id>,
        text: String,
        audience: Audience,
    ) {
        self.steps.push(Step {
            phase: self.phase.clone(),
            kind,
            player: player.map(ToString::to_string),
            text,
            audience,
            snapshot: self.snapshots.len() - 1,
        });
    }

    fn sync(&mut self, view: &game::State) {
        self.view = view.clone();
        self.phase = view.phase().to_string();
        let seats = seats(view);
        if self.snapshots.last() != Some(&seats) {
            self.snapshots.push(seats);
        }
    }

    /// Plausible answers: evil players aim at good players, everyone else
    /// chooses at random.
    fn answer(
        &mut self,
        player: &player::Id,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let view = &self.view;
        let me = view.character(player);
        let targets_good = view.is_evil(player) && me.kind() != character::Kind::Demon
            || me.kind() == character::Kind::Demon && !self.rng.random_bool(0.05);
        let good: Vec<player::Id> = candidates
            .iter()
            .filter(|c| !view.is_evil(c) && *c != player)
            .cloned()
            .collect();
        let pool = if targets_good && good.len() >= count {
            good
        } else {
            candidates.to_vec()
        };
        index::sample(&mut self.rng, pool.len(), count)
            .into_iter()
            .map(|i| pool[i].clone())
            .collect()
    }
}

impl table::Io for Recorder {
    fn wake(&mut self, player: &player::Id) {
        self.push(
            "wake",
            Some(player),
            format!("{player} wakes"),
            Audience::Player,
        );
    }

    fn sleep(&mut self, player: &player::Id) {
        self.push(
            "sleep",
            Some(player),
            format!("{player} sleeps"),
            Audience::Player,
        );
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.push("tell", Some(player), message.to_string(), Audience::Player);
    }

    fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let answer = self.answer(player, candidates, count);
        let names: Vec<&str> = answer.iter().map(player::Id::as_str).collect();
        let text = format!("{prompt} {}", names.join(", "));
        self.push("ask", Some(player), text, Audience::Player);
        answer
    }

    fn event(&mut self, event: &event::Event, view: &game::State) {
        self.sync(view);
        let (player, audience) = match event {
            event::Event::PhaseBegan(_)
            | event::Event::Nominated { .. }
            | event::Event::Executed(_)
            | event::Event::Voted { .. }
            | event::Event::Won(_) => (None, Audience::Public),
            event::Event::Died { player, .. } => (Some(player), Audience::Public),
            event::Event::KillPrevented { player, .. }
            | event::Event::Transformed { player, .. }
            | event::Event::ReminderPlaced { player, .. } => (Some(player), Audience::Storyteller),
            event::Event::KillRedirected { to, .. } => (Some(to), Audience::Storyteller),
            event::Event::Note(_) => (None, Audience::Storyteller),
        };
        self.push("event", player, event.to_string(), audience);
    }

    fn decided(&mut self, decision: &storyteller::Decision, picks: &[usize]) {
        let picked: Vec<String> = picks
            .iter()
            .map(|&i| decision.options[i].to_string())
            .collect();
        let text = format!(
            "{} ({} options): {}",
            decision.reason,
            decision.options.len(),
            picked.join(", ")
        );
        self.push("decision", None, text, Audience::Storyteller);
    }
}

/// Plays a whole random game with `players` seats.
pub fn simulate(seed: u64, players: usize) -> Trace {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut state = game::State::new(setup(&mut rng, players));
    let mut storyteller = storyteller::Random::seeded(rng.random());
    let mut table = Recorder {
        rng: rand::rngs::StdRng::seed_from_u64(rng.random()),
        view: state.clone(),
        phase: String::new(),
        snapshots: Vec::new(),
        steps: Vec::new(),
    };
    table.sync(&state);
    let initial = seats(&state);
    let script = trouble_brewing::script();

    let mut ctx = ctx::Write::new(&mut state, &mut table, &mut storyteller);
    narrator::setup(&mut ctx);
    while ctx.state().winner().is_none() {
        match ctx.advance_phase() {
            game::Phase::Night(_) => narrator::run_night(&mut ctx, &script),
            game::Phase::Day(day) if day > MAX_DAYS => break,
            game::Phase::Day(_) => day(&mut ctx, &mut rng),
            game::Phase::Setup => {}
        }
    }
    Trace {
        seed,
        players: initial,
        snapshots: table.snapshots,
        steps: table.steps,
        winner: state.winner().map(|w| w.to_string()),
    }
}

/// A day: maybe a Slayer shot, then nominations and a majority vote.
fn day(ctx: &mut ctx::Write<'_>, rng: &mut impl Rng) {
    if let Some(slayer) = ctx.state().find("slayer").cloned()
        && ctx.state().is_alive(&slayer)
        && !ctx.state().has_effect(&slayer, game::Effect::AbilityUsed)
        && rng.random_bool(0.35)
    {
        narrator::day_action(ctx, &slayer);
    }
    let mut best: Option<(player::Id, usize)> = None;
    let mut tied = false;
    let mut nominators: Vec<player::Id> = ctx.state().alive_ids();
    nominators.shuffle(rng);
    let mut nominated: Vec<player::Id> = Vec::new();
    for nominator in nominators.into_iter().take(rng.random_range(1..=3)) {
        if ctx.state().winner().is_some() || !ctx.state().is_alive(&nominator) {
            continue;
        }
        let state = ctx.state();
        let choices: Vec<player::Id> = state
            .alive_except(&nominator)
            .into_iter()
            .filter(|p| !nominated.contains(p) && !(state.is_evil(&nominator) && state.is_evil(p)))
            .collect();
        let Some(nominee) = choices.choose(rng).cloned() else {
            continue;
        };
        nominated.push(nominee.clone());
        narrator::nominate(ctx, &nominator, &nominee);
        if !ctx.state().is_alive(&nominator) || ctx.state().executed_today().is_some() {
            return; // the Virgin executed the nominator
        }
        let votes = votes(ctx.state(), &nominee, rng);
        let needed = ctx.state().alive_ids().len().div_ceil(2);
        ctx.record(event::Event::Voted {
            nominee: nominee.clone(),
            votes,
            needed,
        });
        if votes >= needed {
            match &best {
                Some((_, top)) if votes < *top => {}
                Some((_, top)) if votes == *top => tied = true,
                _ => {
                    best = Some((nominee, votes));
                    tied = false;
                }
            }
        }
    }
    if let Some((nominee, _)) = best
        && !tied
        && ctx.state().winner().is_none()
    {
        ctx.execute(&nominee);
    }
}

/// Living players vote: evil players back good nominees, good players are split.
fn votes(state: &game::State, nominee: &player::Id, rng: &mut impl Rng) -> usize {
    state
        .alive_ids()
        .iter()
        .filter(|voter| {
            let p = match (state.is_evil(voter), state.is_evil(nominee)) {
                (true, false) => 0.8,
                (true, true) => 0.1,
                (false, _) => 0.45,
            };
            rng.random_bool(p)
        })
        .count()
}

/// [`simulate`], serialized as JSON for the web viewer.
pub fn simulate_json(seed: u64, players: usize) -> String {
    serde_json::to_string(&simulate(seed, players)).expect("traces serialize")
}
