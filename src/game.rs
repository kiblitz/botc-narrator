use std::fmt;

use crate::character;
use crate::event;
use crate::player;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Setup,
    Night(u32),
    Day(u32),
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Setup => f.write_str("Setup"),
            Phase::Night(n) => write!(f, "Night {n}"),
            Phase::Day(n) => write!(f, "Day {n}"),
        }
    }
}

/// What a reminder token does to the player it sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Poisoned,
    Drunk,
    Protected,
    AbilityUsed,
    /// A token with no built-in rules meaning, interpreted by the character
    /// that placed it (e.g. the Butler's master).
    Custom(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    /// Removed when the next night begins.
    Dusk,
    Never,
}

/// A grimoire reminder token. Each `(source, effect)` token exists once:
/// placing it again moves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reminder {
    /// Id of the character whose ability placed the token.
    pub source: &'static str,
    pub effect: Effect,
    pub expiry: Expiry,
}

impl Reminder {
    pub fn new(source: character::Ref, effect: Effect, expiry: Expiry) -> Self {
        Self {
            source: source.id(),
            effect,
            expiry,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: player::Id,
    pub character: character::Ref,
    pub alignment: character::Alignment,
    pub alive: bool,
    pub ghost_vote: bool,
    pub reminders: Vec<Reminder>,
}

/// The grimoire: everything the storyteller knows.
#[derive(Clone, Debug)]
pub struct State {
    seats: Vec<Player>,
    phase: Phase,
    deaths_tonight: Vec<player::Id>,
    executed_today: Option<player::Id>,
    history: Vec<event::Event>,
    winner: Option<character::Alignment>,
}

impl State {
    /// Seats players in the given (clockwise) order.
    pub fn new(seats: Vec<(player::Id, character::Ref)>) -> Self {
        let seats = seats
            .into_iter()
            .map(|(id, character)| Player {
                id,
                character,
                alignment: character.kind().alignment(),
                alive: true,
                ghost_vote: false,
                reminders: Vec::new(),
            })
            .collect();
        Self {
            seats,
            phase: Phase::Setup,
            deaths_tonight: Vec::new(),
            executed_today: None,
            history: Vec::new(),
            winner: None,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn players(&self) -> &[Player] {
        &self.seats
    }

    pub fn player(&self, id: &player::Id) -> &Player {
        self.seats
            .iter()
            .find(|p| &p.id == id)
            .unwrap_or_else(|| panic!("unknown player {id}"))
    }

    fn player_mut(&mut self, id: &player::Id) -> &mut Player {
        self.seats
            .iter_mut()
            .find(|p| &p.id == id)
            .unwrap_or_else(|| panic!("unknown player {id}"))
    }

    pub fn seat_order(&self) -> Vec<player::Id> {
        self.seats.iter().map(|p| p.id.clone()).collect()
    }

    pub fn alive_ids(&self) -> Vec<player::Id> {
        self.seats
            .iter()
            .filter(|p| p.alive)
            .map(|p| p.id.clone())
            .collect()
    }

    pub fn alive_except(&self, me: &player::Id) -> Vec<player::Id> {
        self.alive_ids().into_iter().filter(|id| id != me).collect()
    }

    pub fn is_alive(&self, id: &player::Id) -> bool {
        self.player(id).alive
    }

    pub fn character(&self, id: &player::Id) -> character::Ref {
        self.player(id).character
    }

    pub fn kind(&self, id: &player::Id) -> character::Kind {
        self.character(id).kind()
    }

    pub fn alignment(&self, id: &player::Id) -> character::Alignment {
        self.player(id).alignment
    }

    pub fn is_evil(&self, id: &player::Id) -> bool {
        self.alignment(id) == character::Alignment::Evil
    }

    pub fn has_effect(&self, id: &player::Id, effect: Effect) -> bool {
        self.effect_source(id, effect).is_some()
    }

    /// The character that placed `effect` on this player, if any.
    pub fn effect_source(&self, id: &player::Id, effect: Effect) -> Option<&'static str> {
        self.player(id)
            .reminders
            .iter()
            .find(|r| r.effect == effect)
            .map(|r| r.source)
    }

    /// Poisoned or drunk: the player's ability has no effect and any
    /// information they receive may be false.
    pub fn is_malfunctioning(&self, id: &player::Id) -> bool {
        self.has_effect(id, Effect::Poisoned) || self.has_effect(id, Effect::Drunk)
    }

    /// The first seated player with this character.
    pub fn find(&self, character_id: &str) -> Option<&player::Id> {
        self.seats
            .iter()
            .find(|p| p.character.id() == character_id)
            .map(|p| &p.id)
    }

    pub fn alive_of_kind(&self, kind: character::Kind) -> Vec<player::Id> {
        self.seats
            .iter()
            .filter(|p| p.alive && p.character.kind() == kind)
            .map(|p| p.id.clone())
            .collect()
    }

    pub fn evil_ids(&self) -> Vec<player::Id> {
        self.seats
            .iter()
            .filter(|p| p.alignment == character::Alignment::Evil)
            .map(|p| p.id.clone())
            .collect()
    }

    /// Names of the characters currently in play, in seat order.
    pub fn character_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for p in &self.seats {
            if !names.contains(&p.character.name()) {
                names.push(p.character.name());
            }
        }
        names
    }

    pub fn deaths_tonight(&self) -> &[player::Id] {
        &self.deaths_tonight
    }

    pub fn executed_today(&self) -> Option<&player::Id> {
        self.executed_today.as_ref()
    }

    pub fn history(&self) -> &[event::Event] {
        &self.history
    }

    pub fn winner(&self) -> Option<character::Alignment> {
        self.winner
    }

    // Mutations are crate-private: abilities change the game through
    // `ctx::Write`, which records events and resolves rule interactions.

    pub(crate) fn advance_phase(&mut self) -> Phase {
        self.phase = match self.phase {
            Phase::Setup => Phase::Night(1),
            Phase::Night(n) => Phase::Day(n),
            Phase::Day(n) => Phase::Night(n + 1),
        };
        match self.phase {
            Phase::Night(_) => {
                for p in &mut self.seats {
                    p.reminders.retain(|r| r.expiry != Expiry::Dusk);
                }
                self.deaths_tonight.clear();
            }
            Phase::Day(_) => self.executed_today = None,
            Phase::Setup => {}
        }
        self.phase
    }

    pub(crate) fn place_reminder(&mut self, target: &player::Id, reminder: Reminder) {
        for p in &mut self.seats {
            p.reminders
                .retain(|r| (r.source, r.effect) != (reminder.source, reminder.effect));
        }
        self.player_mut(target).reminders.push(reminder);
    }

    pub(crate) fn remove_reminders(&mut self, source: &str, expiry: Expiry) {
        for p in &mut self.seats {
            p.reminders
                .retain(|r| r.source != source || r.expiry != expiry);
        }
    }

    pub(crate) fn mark_dead(&mut self, id: &player::Id) {
        let at_night = matches!(self.phase, Phase::Night(_));
        let p = self.player_mut(id);
        p.alive = false;
        p.ghost_vote = true;
        if at_night {
            self.deaths_tonight.push(id.clone());
        }
    }

    pub(crate) fn mark_executed(&mut self, id: &player::Id) {
        self.executed_today = Some(id.clone());
    }

    pub(crate) fn set_character(&mut self, id: &player::Id, character: character::Ref) {
        self.player_mut(id).character = character;
    }

    pub(crate) fn set_winner(&mut self, alignment: character::Alignment) {
        self.winner = Some(alignment);
    }

    pub(crate) fn record(&mut self, event: event::Event) {
        self.history.push(event);
    }
}
