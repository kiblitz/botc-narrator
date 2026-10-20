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
use crate::script;
use crate::storyteller;
use crate::table;

use crate::characters;

pub const MIN_PLAYERS: usize = 5;
pub const MAX_PLAYERS: usize = 15;
const MAX_DAYS: u32 = 12;
/// Chance the Slayer shoots on a given day.
const SLAYER_SHOOTS: f64 = 0.35;
/// Chance the demon picks any player (possibly themself: a starpass) instead of
/// a good player.
const DEMON_WILD_PICK: f64 = 0.05;
/// Vote chances: evil for a good nominee, evil for an evil nominee, good for anyone.
const EVIL_VOTES_GOOD: f64 = 0.8;
const EVIL_VOTES_EVIL: f64 = 0.1;
const GOOD_VOTES: f64 = 0.45;

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

/// A legal setup from `script` in random seat order, using the standard
/// character counts. The Drunk is left out because it is not modelled yet, so
/// with only three Trouble Brewing Outsiders available the Baron adds fewer
/// than two when the base count already has two.
pub fn setup(
    rng: &mut impl Rng,
    players: usize,
    script: &script::Script,
) -> Vec<(player::Id, character::Ref)> {
    let players = players.clamp(MIN_PLAYERS, MAX_PLAYERS);
    let of_kind = |kind: character::Kind| -> Vec<character::Ref> {
        script
            .characters
            .iter()
            .copied()
            .filter(|c| c.kind() == kind && c.id() != character::Def::id(&characters::drunk::Def))
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
    if minions
        .iter()
        .any(|m| m.id() == character::Def::id(&characters::baron::Def))
    {
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
    /// The script's id.
    pub script: &'static str,
    pub players: Vec<Seat>,
    /// Grimoire snapshots; each step refers to one by index.
    pub snapshots: Vec<Vec<Seat>>,
    pub steps: Vec<Step>,
    pub winner: Option<character::Alignment>,
    /// The bluffs shown to the demon, if any.
    pub bluffs: Option<Bluffs>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Bluffs {
    pub demon: String,
    pub characters: Vec<&'static str>,
    /// The step at which the demon learned them.
    pub step: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Seat {
    pub name: String,
    pub character: &'static str,
    pub kind: character::Kind,
    pub alignment: character::Alignment,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    PhaseBegan,
    Wake,
    Sleep,
    Tell,
    Ask,
    /// A player died; public deaths are announced (at dawn for night deaths)
    /// without their cause.
    Death,
    /// A vote on a nomination: `player` is the nominee, `voters` raised hands.
    Vote,
    Event,
    Decision,
}

/// One end of a communication.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum Endpoint {
    Storyteller,
    Player(String),
}

/// Who a step's communication goes from and to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Flow {
    pub from: Endpoint,
    pub to: Endpoint,
}

impl Flow {
    fn to_player(player: &player::Id) -> Self {
        Self {
            from: Endpoint::Storyteller,
            to: Endpoint::Player(player.to_string()),
        }
    }

    fn from_player(player: &player::Id) -> Self {
        Self {
            from: Endpoint::Player(player.to_string()),
            to: Endpoint::Storyteller,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Step {
    pub phase: game::Phase,
    pub kind: Kind,
    pub player: Option<String>,
    pub text: String,
    pub audience: Audience,
    pub snapshot: usize,
    /// Who raised a hand, on vote steps.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub voters: Vec<String>,
    /// Players named by the step's information or chosen in its answer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub players: Vec<String>,
    /// Characters named by the step's information.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub characters: Vec<&'static str>,
    /// Which way the communication goes, for steps that are communication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<Flow>,
}

fn seats(view: &game::State) -> Vec<Seat> {
    view.players()
        .iter()
        .map(|p| Seat {
            name: p.id.to_string(),
            character: p.character.name(),
            kind: p.character.kind(),
            alignment: p.alignment,
            alive: p.alive,
            reminders: p.reminders.iter().map(|r| r.effect.to_string()).collect(),
        })
        .collect()
}

/// The table during a simulation: records every step and answers questions
/// for the players with plausible heuristics.
///
/// `view` and the snapshots are refreshed on every event. That is complete
/// because every grimoire change goes through `ctx::Write::record`.
struct Recorder {
    rng: rand::rngs::StdRng,
    view: game::State,
    phase: game::Phase,
    /// Night deaths, announced publicly when the day begins.
    unannounced: Vec<player::Id>,
    bluffs: Option<Bluffs>,
    snapshots: Vec<Vec<Seat>>,
    steps: Vec<Step>,
}

impl Recorder {
    fn push(
        &mut self,
        kind: Kind,
        player: Option<&player::Id>,
        text: String,
        audience: Audience,
    ) -> &mut Step {
        self.steps.push(Step {
            phase: self.phase,
            kind,
            player: player.map(ToString::to_string),
            text,
            audience,
            snapshot: self.snapshots.len() - 1,
            voters: Vec::new(),
            players: Vec::new(),
            characters: Vec::new(),
            flow: None,
        });
        self.steps.last_mut().expect("just pushed")
    }

    fn announce_night_deaths(&mut self) {
        for dead in std::mem::take(&mut self.unannounced) {
            let text = format!("{dead} died in the night");
            self.push(Kind::Death, Some(&dead), text, Audience::Public);
        }
    }

    fn sync(&mut self, view: &game::State) {
        self.view = view.clone();
        self.phase = view.phase();
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
        let targets_good = if me.kind() == character::Kind::Demon {
            !self.rng.random_bool(DEMON_WILD_PICK)
        } else {
            view.is_evil(player)
        };
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
            Kind::Wake,
            Some(player),
            format!("{player} wakes"),
            Audience::Player,
        );
    }

    fn sleep(&mut self, player: &player::Id) {
        self.push(
            Kind::Sleep,
            Some(player),
            format!("{player} sleeps"),
            Audience::Player,
        );
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        if let info::Message::Bluffs(characters) = message {
            debug_assert!(self.bluffs.is_none(), "the demon is told bluffs once");
            self.bluffs = Some(Bluffs {
                demon: player.to_string(),
                characters: characters.clone(),
                step: self.steps.len(),
            });
        }
        let step = self.push(
            Kind::Tell,
            Some(player),
            message.to_string(),
            Audience::Player,
        );
        step.players = message.players().iter().map(ToString::to_string).collect();
        step.characters = message.characters();
        step.flow = Some(Flow::to_player(player));
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
        // Day abilities (the Slayer's shot) are public.
        let audience = match self.phase {
            game::Phase::Day(_) => Audience::Public,
            _ => Audience::Player,
        };
        let step = self.push(Kind::Ask, Some(player), text, audience);
        step.players = answer.iter().map(ToString::to_string).collect();
        step.flow = Some(Flow::from_player(player));
        answer
    }

    fn event(&mut self, event: &event::Event, view: &game::State) {
        self.sync(view);
        let text = event.to_string();
        match event {
            event::Event::PhaseBegan(phase) => {
                self.push(Kind::PhaseBegan, None, text, Audience::Public);
                if let game::Phase::Day(_) = phase {
                    self.announce_night_deaths();
                }
            }
            event::Event::Died { player, .. } => {
                // The cause is the storyteller's; the table only learns who died.
                self.push(Kind::Event, Some(player), text, Audience::Storyteller);
                if let game::Phase::Night(_) = self.phase {
                    self.unannounced.push(player.clone());
                } else {
                    let text = format!("{player} dies");
                    self.push(Kind::Death, Some(player), text, Audience::Public);
                }
            }
            event::Event::Nominated { nominee, nominator } => {
                let step = self.push(Kind::Event, Some(nominator), text, Audience::Public);
                step.flow = Some(Flow {
                    from: Endpoint::Player(nominator.to_string()),
                    to: Endpoint::Player(nominee.to_string()),
                });
            }
            event::Event::Executed(_) => {
                self.push(Kind::Event, None, text, Audience::Public);
            }
            event::Event::Voted {
                nominee, voters, ..
            } => {
                self.push(Kind::Vote, Some(nominee), text, Audience::Public)
                    .voters = voters.iter().map(ToString::to_string).collect();
            }
            event::Event::Won(_) => {
                // A night kill can end the game before dawn.
                self.announce_night_deaths();
                self.push(Kind::Event, None, text, Audience::Public);
            }
            event::Event::KillPrevented { player, .. }
            | event::Event::Transformed { player, .. }
            | event::Event::ReminderPlaced { player, .. } => {
                self.push(Kind::Event, Some(player), text, Audience::Storyteller);
            }
            event::Event::KillRedirected { to, .. } => {
                self.push(Kind::Event, Some(to), text, Audience::Storyteller);
            }
            event::Event::Note(_) => {
                self.push(Kind::Event, None, text, Audience::Storyteller);
            }
        }
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
        let step = self.push(Kind::Decision, None, text, Audience::Storyteller);
        for &i in picks {
            let choice = &decision.options[i];
            step.players
                .extend(choice.players().iter().map(ToString::to_string));
            step.characters.extend(choice.characters());
        }
    }
}

/// Plays a whole random game with `players` seats.
pub fn simulate(seed: u64, players: usize, script: &script::Script) -> Trace {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let mut state = game::State::new(setup(&mut rng, players, script));
    let mut storyteller = storyteller::Random::seeded(rng.random());
    let mut table = Recorder {
        rng: rand::rngs::StdRng::seed_from_u64(rng.random()),
        view: state.clone(),
        phase: game::Phase::Setup,
        unannounced: Vec::new(),
        bluffs: None,
        snapshots: Vec::new(),
        steps: Vec::new(),
    };
    table.sync(&state);
    let initial = seats(&state);

    let mut ctx = ctx::Write::new(&mut state, &mut table, &mut storyteller);
    narrator::setup(&mut ctx);
    while ctx.state().winner().is_none() {
        match ctx.advance_phase() {
            game::Phase::Night(_) => narrator::run_night(&mut ctx, script),
            game::Phase::Day(day) if day > MAX_DAYS => break,
            game::Phase::Day(_) => day(&mut ctx, &mut rng),
            game::Phase::Setup => {}
        }
    }
    Trace {
        seed,
        script: script.id,
        players: initial,
        snapshots: table.snapshots,
        steps: table.steps,
        winner: state.winner(),
        bluffs: table.bluffs,
    }
}

/// A day: maybe a Slayer shot, then nominations and a majority vote.
fn day(ctx: &mut ctx::Write<'_>, rng: &mut impl Rng) {
    if let Some(slayer) = ctx
        .state()
        .find(character::Def::id(&characters::slayer::Def))
        .cloned()
        && ctx.state().is_alive(&slayer)
        && !ctx.state().has_effect(&slayer, game::Effect::AbilityUsed)
        && rng.random_bool(SLAYER_SHOOTS)
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
            narrator::end_day(ctx);
            return; // the Virgin executed the nominator
        }
        let voters = voters(ctx.state(), &nominee, rng);
        let votes = voters.len();
        let needed = ctx.state().alive_ids().len().div_ceil(2);
        ctx.record(event::Event::Voted {
            nominee: nominee.clone(),
            voters,
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
    if ctx.state().winner().is_none() {
        narrator::end_day(ctx);
    }
}

/// Living players vote (ghost votes are not modelled): evil players back good
/// nominees, good players are split.
fn voters(state: &game::State, nominee: &player::Id, rng: &mut impl Rng) -> Vec<player::Id> {
    state
        .alive_ids()
        .into_iter()
        .filter(|voter| {
            let p = match (state.is_evil(voter), state.is_evil(nominee)) {
                (true, false) => EVIL_VOTES_GOOD,
                (true, true) => EVIL_VOTES_EVIL,
                (false, _) => GOOD_VOTES,
            };
            rng.random_bool(p)
        })
        .collect()
}

/// [`simulate`] with the script named by `script_id`, serialized as JSON for
/// the web viewer.
pub fn simulate_json(seed: u64, players: usize, script_id: &str) -> Result<String, String> {
    let script = script::by_id(script_id).ok_or_else(|| format!("unknown script {script_id:?}"))?;
    Ok(serde_json::to_string(&simulate(seed, players, &script)).expect("traces serialize"))
}

#[derive(Serialize)]
struct ScriptInfo {
    id: &'static str,
    name: &'static str,
}

/// The available scripts as JSON: `[{ "id", "name" }]`.
pub fn scripts_json() -> String {
    let scripts: Vec<ScriptInfo> = script::all()
        .iter()
        .map(|s| ScriptInfo {
            id: s.id,
            name: s.name,
        })
        .collect();
    serde_json::to_string(&scripts).expect("scripts serialize")
}
