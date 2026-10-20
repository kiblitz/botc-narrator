//! Random but plausible games, recorded step by step for the web simulator.
//!
//! Setups follow the standard character counts, adjusted by the characters
//! chosen (`setup::choose`). Players act on simple
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

use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::narrator;
use crate::player;
use crate::script;
use crate::setup;
use crate::storyteller;
use crate::table;

pub const MIN_PLAYERS: usize = setup::MIN_PLAYERS;
pub const MAX_PLAYERS: usize = setup::MAX_PLAYERS;
const MAX_DAYS: u32 = 12;
/// Chance a player uses an unused day ability (the Slayer's shot) on a given day.
const DAY_ACTION: f64 = 0.35;
/// Chance the demon picks any player (possibly themself: a starpass) instead of
/// a good player.
const DEMON_WILD_PICK: f64 = 0.05;
/// Vote chances: evil for a good nominee, evil for an evil nominee, good for anyone.
const EVIL_VOTES_GOOD: f64 = 0.8;
const EVIL_VOTES_EVIL: f64 = 0.1;
const GOOD_VOTES: f64 = 0.45;
/// Chance a mad player breaks their madness on a given day.
const BREAKS_MADNESS: f64 = 0.3;
/// Chance a player who "may choose" chooses no-one (the Po charging up, a
/// player chosen by the Al-Hadikhia choosing death).
const CHOOSES_NO_ONE: f64 = 0.2;
/// Chance a player agrees when asked (the Harlot's visit, another Matron swap).
const AGREES: f64 = 0.5;
/// At most this many Travellers join a game whose script has them.
const MAX_TRAVELLERS: usize = 2;
/// Chance a Traveller is evil.
const EVIL_TRAVELLER: f64 = 0.4;
/// Chance a player tries, on a given day, to convince the group they are
/// something they must not be mad about (the Mutant, an Outsider).
const MAD_ABOUT: f64 = 0.1;

const NAMES: [&str; MAX_PLAYERS + MAX_TRAVELLERS] = [
    "Ada", "Bram", "Cleo", "Dev", "Esme", "Finn", "Gus", "Hana", "Ivo", "Juno", "Kit", "Lux",
    "Mira", "Nico", "Oona", "Pip", "Quin",
];

/// A legal setup from `script` (see `setup::choose`) in random seat order.
pub fn setup(
    rng: &mut impl Rng,
    players: usize,
    script: &script::Script,
) -> Vec<(player::Id, character::Ref)> {
    let mut characters = setup::choose(rng, players, script);
    characters.shuffle(rng);
    NAMES
        .iter()
        .zip(characters)
        .map(|(name, c)| (player::Id::new(*name), c))
        .collect()
}

/// Up to `MAX_TRAVELLERS` of the script's Travellers, each with a random
/// alignment, seated after the `residents` players.
pub fn travellers(
    rng: &mut impl Rng,
    residents: usize,
    script: &script::Script,
) -> Vec<game::Seat> {
    let available: Vec<character::Ref> = script
        .characters
        .iter()
        .copied()
        .filter(|c| c.kind() == character::Kind::Traveller)
        .collect();
    if available.is_empty() {
        return Vec::new();
    }
    let count = rng.random_range(0..=MAX_TRAVELLERS.min(available.len()));
    available
        .choose_multiple(rng, count)
        .zip(&NAMES[residents.min(MAX_PLAYERS)..])
        .map(|(c, name)| {
            let alignment = if rng.random_bool(EVIL_TRAVELLER) {
                character::Alignment::Evil
            } else {
                character::Alignment::Good
            };
            game::Seat::traveller(player::Id::new(*name), *c, alignment)
        })
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
    /// The character the player believes they are, if not their own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub believes: Option<&'static str>,
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
            believes: p.believes.map(|c| c.name()),
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
    /// Night deaths and revivals, announced publicly when the day begins.
    unannounced: Vec<(Kind, player::Id, String)>,
    /// Players awake now; by day, those privately visiting the storyteller.
    awake: Vec<player::Id>,
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
        for (kind, player, text) in std::mem::take(&mut self.unannounced) {
            self.push(kind, Some(&player), text, Audience::Public);
        }
    }

    /// A player's answer is public by day (the Slayer's shot, the Juggler's
    /// guesses) unless they are privately visiting the storyteller.
    fn answer_audience(&self, player: &player::Id) -> Audience {
        match self.phase {
            game::Phase::Day(_) if !self.awake.contains(player) => Audience::Public,
            _ => Audience::Player,
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
        let me = view.believed(player);
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
        // By day, waking is a private visit to the storyteller (the Savant).
        let text = match self.phase {
            game::Phase::Day(_) => format!("{player} visits the storyteller"),
            _ => format!("{player} wakes"),
        };
        self.awake.push(player.clone());
        self.push(Kind::Wake, Some(player), text, Audience::Player);
    }

    fn sleep(&mut self, player: &player::Id) {
        let text = match self.phase {
            game::Phase::Day(_) => format!("{player} returns from the storyteller"),
            _ => format!("{player} sleeps"),
        };
        self.awake.retain(|p| p != player);
        self.push(Kind::Sleep, Some(player), text, Audience::Player);
    }

    fn tell(&mut self, player: &player::Id, message: &info::Message) {
        // Several demons (Legion) all learn the same bluffs: keep the first.
        // The Lunatic is shown bluffs too; the trace keeps the real Demon's.
        if let info::Message::Bluffs(characters) = message
            && self.bluffs.is_none()
            && self.view.demons().contains(player)
        {
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
        let audience = self.answer_audience(player);
        let step = self.push(Kind::Ask, Some(player), text, audience);
        step.players = answer.iter().map(ToString::to_string).collect();
        step.flow = Some(Flow::from_player(player));
        answer
    }

    fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        if self.rng.random_bool(CHOOSES_NO_ONE) {
            let text = format!("{player} chooses no-one");
            let step = self.push(Kind::Ask, Some(player), text, Audience::Player);
            step.flow = Some(Flow::from_player(player));
            return Vec::new();
        }
        table::Io::ask(self, player, prompt, candidates, max)
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
                    let text = format!("{player} died in the night");
                    self.unannounced.push((Kind::Death, player.clone(), text));
                } else {
                    let text = format!("{player} dies");
                    self.push(Kind::Death, Some(player), text, Audience::Public);
                }
            }
            event::Event::Nominated { nominee, nominator } => {
                let step = self.push(Kind::Event, nominator.as_ref(), text, Audience::Public);
                step.flow = Some(Flow {
                    from: match nominator {
                        Some(p) => Endpoint::Player(p.to_string()),
                        None => Endpoint::Storyteller,
                    },
                    to: Endpoint::Player(nominee.to_string()),
                });
            }
            event::Event::Revived(player) => {
                self.push(Kind::Event, Some(player), text, Audience::Storyteller);
                let public = format!("{player} is alive again");
                if let game::Phase::Night(_) = self.phase {
                    // Died and came back tonight: the table hears of neither.
                    let before = self.unannounced.len();
                    self.unannounced
                        .retain(|(kind, p, _)| *kind != Kind::Death || p != player);
                    if self.unannounced.len() == before {
                        self.unannounced.push((Kind::Event, player.clone(), public));
                    }
                } else {
                    self.push(Kind::Event, Some(player), public, Audience::Public);
                }
            }
            event::Event::Announced(_) => {
                self.push(Kind::Event, None, text, Audience::Public);
            }
            event::Event::SeatsSwapped(a, _) | event::Event::GhostVoteSpent(a) => {
                self.push(Kind::Event, Some(a), text, Audience::Public);
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
            | event::Event::Believes { player, .. }
            | event::Event::AbilityGained { player, .. }
            | event::Event::AbilityLost { player, .. }
            | event::Event::MadnessBroken { player, .. }
            | event::Event::AlignmentChanged { player, .. }
            | event::Event::ReminderPlaced { player, .. }
            | event::Event::ReminderRemoved { player, .. } => {
                self.push(Kind::Event, Some(player), text, Audience::Storyteller);
            }
            event::Event::Claimed { player, claims } => {
                let step = self.push(Kind::Event, Some(player), text, Audience::Public);
                step.players = claims.iter().map(|(p, _)| p.to_string()).collect();
                step.characters = claims.iter().map(|(_, c)| *c).collect();
            }
            event::Event::KillRedirected { to, .. } => {
                self.push(Kind::Event, Some(to), text, Audience::Storyteller);
            }
            event::Event::Note(_)
            | event::Event::FabledAdded(_)
            | event::Event::FabledMarked { .. } => {
                self.push(Kind::Event, None, text, Audience::Storyteller);
            }
        }
    }

    fn ask_character(
        &mut self,
        player: &player::Id,
        prompt: &str,
        options: &[character::Ref],
    ) -> usize {
        let answer = self.rng.random_range(0..options.len());
        let name = options[answer].name();
        let text = format!("{prompt} {name}");
        let step = self.push(Kind::Ask, Some(player), text, self.answer_audience(player));
        step.characters = vec![name];
        step.flow = Some(Flow::from_player(player));
        answer
    }

    fn broke_madness(&mut self, _player: &player::Id, _about: &'static str) -> bool {
        self.rng.random_bool(BREAKS_MADNESS)
    }

    fn ask_yes_no(&mut self, player: &player::Id, prompt: &str) -> bool {
        let yes = self.rng.random_bool(AGREES);
        let answer = if yes { "yes" } else { "no" };
        let audience = match self.phase {
            game::Phase::Day(_) => Audience::Public,
            _ => Audience::Player,
        };
        let step = self.push(
            Kind::Ask,
            Some(player),
            format!("{prompt} {answer}"),
            audience,
        );
        step.flow = Some(Flow::from_player(player));
        yes
    }

    fn is_mad_about(&mut self, _player: &player::Id, _about: &'static str) -> bool {
        self.rng.random_bool(MAD_ABOUT)
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
    let mut seated: Vec<game::Seat> = setup(&mut rng, players, script)
        .into_iter()
        .map(|(id, c)| game::Seat::new(id, c))
        .collect();
    seated.extend(travellers(&mut rng, players, script));
    let mut state = game::State::seated(seated);
    let mut storyteller = storyteller::Random::seeded(rng.random());
    let mut table = Recorder {
        rng: rand::rngs::StdRng::seed_from_u64(rng.random()),
        view: state.clone(),
        phase: game::Phase::Setup,
        unannounced: Vec::new(),
        awake: Vec::new(),
        bluffs: None,
        snapshots: Vec::new(),
        steps: Vec::new(),
    };
    table.sync(&state);

    let mut ctx = ctx::Write::new(&mut state, &mut table, &mut storyteller);
    narrator::setup(&mut ctx, script);
    // After setup, so the Drunk's believed character is known.
    let initial = seats(ctx.state());
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

/// A day: maybe day abilities (the Slayer's shot, also by a Drunk who
/// believes they are the Slayer; a Minion's guess of the Damsel), then
/// nominations and a majority vote.
fn day(ctx: &mut ctx::Write<'_>, rng: &mut impl Rng) {
    let mut users: Vec<(player::Id, character::Ref)> = Vec::new();
    for (_, ability) in ctx.active_abilities() {
        for user in narrator::day_action_users(ctx.state(), ability) {
            if !users
                .iter()
                .any(|(p, a)| *p == user && character::same(*a, ability))
            {
                users.push((user, ability));
            }
        }
    }
    for (player, ability) in users {
        if ctx.state().winner().is_none()
            && ctx.state().is_alive(&player)
            && !ability::used(ctx.state(), &player, ability)
            && rng.random_bool(DAY_ACTION)
        {
            narrator::day_action(ctx, &player, ability);
        }
    }
    let mut best: Option<(player::Id, usize)> = None;
    let mut tied = false;
    let mut nominated: Vec<player::Id> = Vec::new();
    let storyteller_nominates = narrator::storyteller_nominations(ctx);
    let players_nominate = storyteller_nominates.is_none();
    let nominations: Vec<(Option<player::Id>, Option<player::Id>)> = match storyteller_nominates {
        // Only the storyteller nominates (the Bishop).
        Some(nominees) => nominees.into_iter().map(|n| (None, Some(n))).collect(),
        None => {
            let mut nominators: Vec<player::Id> = ctx
                .state()
                .alive_ids()
                .into_iter()
                .filter(|p| narrator::can_nominate(ctx.state(), p))
                .collect();
            nominators.shuffle(rng);
            let count = rng.random_range(1..=3);
            nominators
                .into_iter()
                .take(count)
                .map(|n| (Some(n), None))
                .collect()
        }
    };
    for (nominator, nominee) in nominations {
        if ctx.state().winner().is_some() {
            break;
        }
        let nominee = match (&nominator, nominee) {
            (_, Some(nominee)) => nominee,
            (Some(nominator), None) => {
                match pick_nominee(ctx.state(), nominator, &nominated, rng) {
                    Some(nominee) => nominee,
                    None => continue,
                }
            }
            (None, None) => continue,
        };
        nominated.push(nominee.clone());
        let Some(votes) = nominate_and_vote(ctx, nominator.as_ref(), &nominee, rng) else {
            narrator::end_day(ctx);
            return; // the Virgin executed the nominator
        };
        let needed = needed(ctx.state());
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
        // After the first execution, the Butcher may nominate again (once:
        // a second execution does not allow a third), unless only the
        // storyteller nominates.
        if players_nominate
            && let Some(nominator) = narrator::nominate_again(ctx.state()).first()
            && ctx.state().winner().is_none()
            && let Some(nominee) = pick_nominee(ctx.state(), nominator, &nominated, rng)
            && let Some(votes) = nominate_and_vote(ctx, Some(nominator), &nominee, rng)
            && votes >= needed(ctx.state())
            && ctx.state().winner().is_none()
        {
            ctx.execute(&nominee);
        }
    }
    if ctx.state().winner().is_none() {
        narrator::check_madness(ctx);
    }
    if ctx.state().winner().is_none() {
        narrator::end_day(ctx);
    }
}

/// Votes needed to execute: half the living players, rounded up.
fn needed(state: &game::State) -> usize {
    state.alive_ids().len().div_ceil(2)
}

/// A living player `nominator` may nominate who was not nominated today
/// (evil players do not nominate evil players).
fn pick_nominee(
    state: &game::State,
    nominator: &player::Id,
    nominated: &[player::Id],
    rng: &mut impl Rng,
) -> Option<player::Id> {
    if !state.is_alive(nominator) {
        return None;
    }
    let choices: Vec<player::Id> = state
        .alive_except(nominator)
        .into_iter()
        .filter(|p| !nominated.contains(p) && !(state.is_evil(nominator) && state.is_evil(p)))
        .collect();
    choices.choose(rng).cloned()
}

/// Nominates `nominee` (by the storyteller when `nominator` is `None`) and
/// holds the vote; returns the votes counted, or `None` if the nomination
/// itself ended the day (the Virgin).
fn nominate_and_vote(
    ctx: &mut ctx::Write<'_>,
    nominator: Option<&player::Id>,
    nominee: &player::Id,
    rng: &mut impl Rng,
) -> Option<usize> {
    let executed_before = ctx.state().executed_today().is_some();
    match nominator {
        Some(nominator) => narrator::nominate(ctx, nominator, nominee),
        None => narrator::storyteller_nominates(ctx, nominee),
    }
    let ended = nominator.is_some_and(|n| !ctx.state().is_alive(n))
        || (!executed_before && ctx.state().executed_today().is_some());
    if ended {
        return None;
    }
    if !ctx.state().is_alive(nominee) || ctx.state().winner().is_some() {
        return Some(0); // the Golem killed the nominee: no vote
    }
    let voters = voters(ctx.state(), nominee, rng);
    let needed = needed(ctx.state());
    Some(narrator::vote(ctx, nominee, voters, needed))
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
    name: String,
}

/// The available scripts as JSON: `[{ "id", "name" }]`.
pub fn scripts_json() -> String {
    let scripts: Vec<ScriptInfo> = script::all()
        .iter()
        .map(|s| ScriptInfo {
            id: s.id,
            name: s.name.clone(),
        })
        .collect();
    serde_json::to_string(&scripts).expect("scripts serialize")
}
