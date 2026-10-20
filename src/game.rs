use std::fmt;

use crate::character;
use crate::event;
use crate::player;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "number", rename_all = "snake_case")]
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
    /// Mad about being this character (by name): the player should try to
    /// convince the group of it, or the ability that placed the token may
    /// punish them (see `narrator::check_madness`).
    Mad(&'static str),
    /// A token with no built-in rules meaning, interpreted by the character
    /// that placed it (e.g. the Butler's master).
    Custom(&'static str),
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Custom(name) => f.write_str(name),
            Effect::Mad(about) => write!(f, "Mad ({about})"),
            effect => fmt::Debug::fmt(effect, f),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    /// Removed when the next night begins.
    Dusk,
    Never,
}

/// A grimoire reminder token. Each player's `effect` token exists once:
/// placing it again moves it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reminder {
    /// Id of the character whose ability placed the token.
    pub source: &'static str,
    /// The player whose ability placed the token.
    pub placed_by: player::Id,
    pub effect: Effect,
    pub expiry: Expiry,
}

impl Reminder {
    pub fn new(
        source: character::Ref,
        placed_by: &player::Id,
        effect: Effect,
        expiry: Expiry,
    ) -> Self {
        Self {
            source: source.id(),
            placed_by: placed_by.clone(),
            effect,
            expiry,
        }
    }
}

impl Effect {
    /// Whether `other` is the same kind of token: an ability has one Mad
    /// token, whatever character it is about.
    fn same_token(self, other: Effect) -> bool {
        match (self, other) {
            (Effect::Mad(_), Effect::Mad(_)) => true,
            _ => self == other,
        }
    }
}

/// A player to seat: who, their character and their starting alignment.
#[derive(Clone, Debug)]
pub struct Seat {
    pub id: player::Id,
    pub character: character::Ref,
    pub alignment: character::Alignment,
}

impl Seat {
    /// A player whose alignment is their character's. Travellers have none:
    /// seat them with [`Seat::traveller`]. Fabled are never seated.
    pub fn new(id: player::Id, character: character::Ref) -> Self {
        let alignment = character.kind().alignment().unwrap_or_else(|| {
            panic!(
                "{} is a {}: it has no starting alignment",
                character.name(),
                character.kind()
            )
        });
        Self {
            id,
            character,
            alignment,
        }
    }

    pub fn traveller(
        id: player::Id,
        character: character::Ref,
        alignment: character::Alignment,
    ) -> Self {
        assert_eq!(
            character.kind(),
            character::Kind::Traveller,
            "{} is not a Traveller",
            character.name()
        );
        Self {
            id,
            character,
            alignment,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub id: player::Id,
    /// The real character, as in the grimoire.
    pub character: character::Ref,
    /// The character the player was told they are, when it is not their real
    /// one (the Drunk, the Lunatic, the Marionette).
    pub believes: Option<character::Ref>,
    /// Abilities gained from other characters (the Philosopher's choice).
    pub gained: Vec<character::Ref>,
    /// Whether the player has lost their own character's ability.
    pub lost_ability: bool,
    pub alignment: character::Alignment,
    pub alive: bool,
    pub ghost_vote: bool,
    pub reminders: Vec<Reminder>,
}

impl Player {
    /// The character the player thinks they are.
    pub fn believed(&self) -> character::Ref {
        self.believes.unwrap_or(self.character)
    }
}

/// The grimoire: everything the storyteller knows.
#[derive(Clone, Debug)]
pub struct State {
    seats: Vec<Player>,
    fabled: Vec<character::Ref>,
    script: Vec<character::Ref>,
    phase: Phase,
    deaths_tonight: Vec<player::Id>,
    executed_today: Option<player::Id>,
    history: Vec<event::Event>,
    winner: Option<character::Alignment>,
}

impl State {
    /// Seats players in the given (clockwise) order, each with their
    /// character's alignment. Use [`State::seated`] to seat Travellers.
    pub fn new(seats: Vec<(player::Id, character::Ref)>) -> Self {
        Self::seated(
            seats
                .into_iter()
                .map(|(id, character)| Seat::new(id, character))
                .collect(),
        )
    }

    pub fn seated(seats: Vec<Seat>) -> Self {
        let seats = seats
            .into_iter()
            .map(|seat| Player {
                id: seat.id,
                character: seat.character,
                believes: None,
                gained: Vec::new(),
                lost_ability: false,
                alignment: seat.alignment,
                alive: true,
                ghost_vote: false,
                reminders: Vec::new(),
            })
            .collect();
        Self {
            seats,
            fabled: Vec::new(),
            script: Vec::new(),
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

    /// The character the player thinks they are (see [`Player::believes`]).
    pub fn believed(&self, id: &player::Id) -> character::Ref {
        self.player(id).believed()
    }

    /// Every ability the player acts with: their own character's (unless
    /// lost), the character they believe they are, and gained ones. A
    /// believed character's ability never works, since the player is
    /// malfunctioning (see [`State::is_malfunctioning`]).
    pub fn abilities(&self, id: &player::Id) -> Vec<character::Ref> {
        let p = self.player(id);
        let own = (!p.lost_ability).then_some(p.character);
        let mut abilities: Vec<character::Ref> = Vec::new();
        for c in own
            .into_iter()
            .chain(p.believes)
            .chain(p.gained.iter().copied())
        {
            if !abilities.iter().any(|a| character::same(*a, c)) {
                abilities.push(c);
            }
        }
        abilities
    }

    /// Whether the player acts with this character's ability.
    pub fn has_ability(&self, id: &player::Id, character_id: &str) -> bool {
        self.abilities(id).iter().any(|c| c.id() == character_id)
    }

    /// The characters on the script being played (set by `narrator::setup`):
    /// what abilities that name "any good character" choose from.
    pub fn script(&self) -> &[character::Ref] {
        &self.script
    }

    /// Storyteller-side Fabled characters in play.
    pub fn fabled(&self) -> &[character::Ref] {
        &self.fabled
    }

    /// Seated players who are not Travellers: the count setups and the
    /// first night's evil info use.
    pub fn resident_count(&self) -> usize {
        self.seats
            .iter()
            .filter(|p| p.character.kind() != character::Kind::Traveller)
            .count()
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
    /// information they receive may be false. A player who believes they are
    /// a character they are not (the Drunk) counts as drunk.
    pub fn is_malfunctioning(&self, id: &player::Id) -> bool {
        self.player(id).believes.is_some()
            || self.has_effect(id, Effect::Poisoned)
            || self.has_effect(id, Effect::Drunk)
    }

    /// Living players with an ability that counts as the demon
    /// (`character::Def::counts_as_demon`): their own (unless lost) or a
    /// gained one, never one they only believe they have.
    pub fn demons(&self) -> Vec<player::Id> {
        self.seats
            .iter()
            .filter(|p| {
                let own = (!p.lost_ability).then_some(p.character);
                p.alive
                    && own
                        .into_iter()
                        .chain(p.gained.iter().copied())
                        .any(|c| c.counts_as_demon())
            })
            .map(|p| p.id.clone())
            .collect()
    }

    /// Whether any demon lives: good wins when none does.
    pub fn demon_alive(&self) -> bool {
        !self.demons().is_empty()
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
            p.reminders.retain(|r| {
                r.placed_by != reminder.placed_by
                    || r.source != reminder.source
                    || !r.effect.same_token(reminder.effect)
            });
        }
        self.player_mut(target).reminders.push(reminder);
    }

    /// Returns whether the token was there.
    pub(crate) fn remove_reminder(&mut self, target: &player::Id, reminder: &Reminder) -> bool {
        let reminders = &mut self.player_mut(target).reminders;
        let before = reminders.len();
        reminders.retain(|r| r != reminder);
        reminders.len() != before
    }

    pub(crate) fn remove_reminders(&mut self, placed_by: &player::Id, expiry: Expiry) {
        for p in &mut self.seats {
            p.reminders
                .retain(|r| &r.placed_by != placed_by || r.expiry != expiry);
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

    pub(crate) fn set_believed(&mut self, id: &player::Id, believes: Option<character::Ref>) {
        self.player_mut(id).believes = believes;
    }

    pub(crate) fn gain_ability(&mut self, id: &player::Id, ability: character::Ref) {
        let p = self.player_mut(id);
        if character::same(p.character, ability) {
            p.lost_ability = false;
        } else if !p.gained.iter().any(|c| character::same(*c, ability)) {
            p.gained.push(ability);
        }
    }

    pub(crate) fn lose_ability(&mut self, id: &player::Id, ability: character::Ref) {
        let p = self.player_mut(id);
        if character::same(p.character, ability) {
            p.lost_ability = true;
        }
        p.gained.retain(|c| !character::same(*c, ability));
    }

    pub(crate) fn set_alignment(&mut self, id: &player::Id, alignment: character::Alignment) {
        self.player_mut(id).alignment = alignment;
    }

    pub(crate) fn set_script(&mut self, characters: Vec<character::Ref>) {
        self.script = characters;
    }

    pub(crate) fn add_fabled(&mut self, fabled: character::Ref) {
        self.fabled.push(fabled);
    }

    pub(crate) fn set_winner(&mut self, alignment: character::Alignment) {
        self.winner = Some(alignment);
    }

    pub(crate) fn record(&mut self, event: event::Event) {
        self.history.push(event);
    }
}
