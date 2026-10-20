use std::fmt;

use crate::ctx;
use crate::event;
use crate::game;
use crate::player;
use crate::setup;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Kind {
    Townsfolk,
    Outsider,
    Minion,
    Demon,
    /// Sits at the table but is not part of the setup counts; their alignment
    /// is chosen when they are seated.
    Traveller,
    /// A storyteller-side character: never seated (see `game::State::fabled`).
    Fabled,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Townsfolk,
        Kind::Outsider,
        Kind::Minion,
        Kind::Demon,
        Kind::Traveller,
        Kind::Fabled,
    ];

    /// The alignment a character of this kind starts with. Travellers have
    /// their alignment chosen at seating, and Fabled have none.
    pub fn alignment(self) -> Option<Alignment> {
        match self {
            Kind::Townsfolk | Kind::Outsider => Some(Alignment::Good),
            Kind::Minion | Kind::Demon => Some(Alignment::Evil),
            Kind::Traveller | Kind::Fabled => None,
        }
    }

    /// Townsfolk, Outsider, Minion or Demon: counted by the setup.
    pub fn in_setup(self) -> bool {
        self.alignment().is_some()
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Alignment {
    Good,
    Evil,
}

impl Alignment {
    pub fn opposite(self) -> Self {
        match self {
            Alignment::Good => Alignment::Evil,
            Alignment::Evil => Alignment::Good,
        }
    }
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// How a character may register falsely to information abilities (the
/// Recluse as an evil Minion or Demon, the Spy as a good Townsfolk or
/// Outsider). The storyteller decides, per query, whether it applies; it never
/// applies while the character is malfunctioning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Misregistration {
    pub alignment: Alignment,
    /// Kinds whose in-play characters this character may register as.
    pub kinds: &'static [Kind],
}

/// What happens to a player who would die, as decided by an ability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interception {
    /// This ability does not interfere.
    Pass,
    /// The player does not die.
    Prevent,
    /// Someone else dies instead.
    Redirect(player::Id),
}

/// Positions in the official night order (the clocktower.online numbering);
/// a script's night orders are its characters sorted by these. `None` means
/// the character does not act that night.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NightOrder {
    pub first: Option<u32>,
    pub other: Option<u32>,
}

pub type ReadOnlyFn = fn(&mut ctx::Read<'_>, &player::Id, u32);
pub type ReadWriteFn = fn(&mut ctx::Write<'_>, &player::Id, u32);
pub type DayFn = fn(&mut ctx::Write<'_>, &player::Id);

/// A night ability, tagged with the access it needs. Consecutive read-only
/// actions in the night order are independent and announced as one batch;
/// read-write actions are sequential sync points.
#[derive(Clone, Copy)]
pub enum NightAction {
    ReadOnly(ReadOnlyFn),
    ReadWrite(ReadWriteFn),
}

/// A character definition. Every hook defaults to "no ability"; a character
/// overrides only the hooks its ability uses.
///
/// Player hooks run for each of a player's abilities (`game::State::abilities`:
/// their character, the character they believe they are, and gained ones).
/// Abilities check `game::State::is_malfunctioning` themselves where the rules
/// say a poisoned or drunk player's ability does nothing.
pub trait Def: Sync {
    /// Unique id. Script JSON ids are matched ignoring underscores, so
    /// `fortune_teller` loads from the official `fortuneteller`.
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn kind(&self) -> Kind;

    /// When this character wakes; script night orders are computed from it.
    fn night_order(&self) -> NightOrder {
        NightOrder::default()
    }

    /// What this character does when its turn in the night order comes.
    fn night_action(&self) -> Option<NightAction> {
        None
    }

    /// Whether `me` wakes for this ability tonight. By default players whose
    /// ability is active (`game::State::ability_active`: alive, or dead but
    /// keeping it) wake; checked when their turn comes, not when the night
    /// starts.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.ability_active(me)
    }

    fn day_action(&self) -> Option<DayFn> {
        None
    }

    /// Legal setup counts with this character in play, given the counts so
    /// far (Baron: two more Outsiders). Several options leave the choice to
    /// whoever builds the setup.
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts]
    }

    /// Whether extra slots of this character's kind are filled with copies of
    /// it (Legion, Riot) when the setup counts ask for more than are available.
    fn duplicates_in_setup(&self) -> bool {
        false
    }

    /// Before the first night, with every player seated: storyteller setup
    /// choices (the Drunk's believed Townsfolk, the Fortune Teller's red herring).
    fn on_setup(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    /// Called for each living player when a night begins.
    fn on_night_start(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    /// Called for each living player when a day begins.
    fn on_day_start(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    /// Called for each living player when the day ends.
    fn on_day_end(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    fn on_nominated(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _nominator: &player::Id) {}

    /// Called for the nominator, after the nominee's `on_nominated`.
    fn on_nominates(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _nominee: &player::Id) {}

    /// Called for every player (dead ones too) after a vote is tallied.
    fn on_vote(
        &self,
        _ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        _nominee: &player::Id,
        _voters: &[player::Id],
    ) {
    }

    fn on_executed(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    /// Called for every functioning living player's abilities when `target`
    /// would die of `cause`, in seat order; the first that does not `Pass`
    /// decides. (Monk protection is a reminder token, checked first.)
    fn intercept_death(
        &self,
        _ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        _target: &player::Id,
        _cause: &event::Cause,
    ) -> Interception {
        Interception::Pass
    }

    /// Called after any player dies: first for every living player, then for
    /// the player who died.
    fn on_death(
        &self,
        _ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        _dead: &player::Id,
        _cause: &event::Cause,
    ) {
    }

    /// The ability that made `mad` mad (a `game::Effect::Mad` token placed by
    /// `me`) learns that they broke madness.
    fn on_madness_broken(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _mad: &player::Id) {}

    fn misregistration(&self) -> Option<Misregistration> {
        None
    }

    /// Whether a living holder of this ability keeps evil in the game: good
    /// wins when no living player has one. Demons by default; a character
    /// that carries the demon (Lil' Monsta) gains such an ability.
    fn counts_as_demon(&self) -> bool {
        self.kind() == Kind::Demon
    }

    /// Whether `receiver`'s information must be false while `me` (living and
    /// functioning) has this ability (the Vortox: Townsfolk get false info).
    fn forces_false_info(
        &self,
        _state: &game::State,
        _me: &player::Id,
        _receiver: &player::Id,
    ) -> bool {
        false
    }
}

pub type Ref = &'static dyn Def;

impl fmt::Debug for dyn Def {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether two references are the same character.
pub fn same(a: Ref, b: Ref) -> bool {
    a.id() == b.id()
}
