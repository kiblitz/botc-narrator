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
    /// A storyteller-side rule, in play like a Fabled (Knaves, Storm Catcher).
    Loric,
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
            Kind::Traveller | Kind::Fabled | Kind::Loric => None,
        }
    }

    /// Townsfolk, Outsider, Minion or Demon: counted by the setup.
    pub fn in_setup(self) -> bool {
        self.alignment().is_some()
    }

    /// Fabled or Loric: in play on the storyteller's side, never seated.
    pub fn storyteller_side(self) -> bool {
        matches!(self, Kind::Fabled | Kind::Loric)
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

    /// Who may use this ability's `day_action` while `me` has it: `me` by
    /// default; the Damsel lets every Minion guess it.
    fn day_action_users(&self, _state: &game::State, me: &player::Id) -> Vec<player::Id> {
        vec![me.clone()]
    }

    /// Whether this ability lets `me` nominate (the Golem nominates once).
    fn can_nominate(&self, _state: &game::State, _me: &player::Id) -> bool {
        true
    }

    /// Legal setup counts with this character in play, given the counts so
    /// far (Baron: two more Outsiders). Several options leave the choice to
    /// whoever builds the setup.
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts]
    }

    /// Ids of characters that must be in play with this one ("[+the
    /// Damsel]"): if one is on the script but was not picked, it replaces a
    /// Townsfolk at setup.
    fn requires(&self) -> &'static [&'static str] {
        &[]
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

    /// Called for every player (dead ones too) after any nomination, after
    /// the nominee's and nominator's hooks (the Witch's curse).
    fn on_nomination(
        &self,
        _ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        _nominator: &player::Id,
        _nominee: &player::Id,
    ) {
    }

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

    /// Called for every player (dead ones too) when anyone is executed, after
    /// the executed player's `on_executed` and before they die.
    fn on_execution(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _executed: &player::Id) {}

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

    /// When `me` learns they died: at once for a day death, at dawn for a
    /// night death (the Klutz, the Moonchild). Called for the dead player's
    /// abilities.
    fn on_learns_death(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    /// At night, `chooser` chose `me` with their ability (the Goon). Called
    /// for each chosen player's abilities, after the chooser's `on_choose`.
    fn on_chosen(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _chooser: &player::Id) {}

    /// At night, `me` chose `chosen` with one of their abilities (the
    /// Lunatic's Demon learns whom the Lunatic chose).
    fn on_choose(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _chosen: &[player::Id]) {}

    /// The game ended with `winner` winning; called for every player's
    /// abilities, dead ones too (the Politician may change sides).
    fn on_win(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _winner: Alignment) {}

    /// The ability that made `mad` mad (a `game::Effect::Mad` token placed by
    /// `me`) learns that they broke madness.
    fn on_madness_broken(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _mad: &player::Id) {}

    /// Whether a player with this character can never be drunk or poisoned
    /// (the Beggar: "you are sober & healthy").
    fn always_sober(&self) -> bool {
        false
    }

    /// How much `voter`'s vote counts while `me` (functioning) has this
    /// ability, given the `weight` so far (1 for an ordinary vote): the
    /// Bureaucrat's chosen player counts 3, a Beggar without a token 0.
    fn vote_weight(
        &self,
        _state: &game::State,
        _me: &player::Id,
        _voter: &player::Id,
        weight: i32,
    ) -> i32 {
        weight
    }

    /// Whom the storyteller nominates today, if this ability (held by a
    /// living, functioning `me`) says only the storyteller nominates (the
    /// Bishop). `None` leaves nominations to the players.
    fn storyteller_nominations(
        &self,
        _ctx: &mut ctx::Read<'_>,
        _me: &player::Id,
    ) -> Option<Vec<player::Id>> {
        None
    }

    /// Whether `me` may nominate again after the day's first execution (the
    /// Butcher).
    fn nominates_again(&self) -> bool {
        false
    }

    // Storyteller-side hooks, for Fabled and Loric in play
    // (`game::State::fabled`): they belong to no player.

    /// Before the first night, after the players' setup (Storm Catcher).
    fn fabled_setup(&self, _ctx: &mut ctx::Write<'_>) {}

    /// When the day ends (Knaves might switch).
    fn fabled_dusk(&self, _ctx: &mut ctx::Write<'_>) {}

    /// Whether `target` may become `to` (the Spirit of Ivory caps extra evil
    /// players). A refused change does not happen.
    fn permits_alignment_change(
        &self,
        _state: &game::State,
        _target: &player::Id,
        _to: Alignment,
    ) -> bool {
        true
    }

    /// Whether `receiver`'s next ability information comes from a lying
    /// storyteller (Knaves). Asked once per piece of information.
    fn lies_to(&self, _ctx: &mut ctx::Read<'_>, _receiver: &player::Id) -> bool {
        false
    }

    fn misregistration(&self) -> Option<Misregistration> {
        None
    }

    /// Whether a living holder of this ability keeps evil in the game: good
    /// wins when no living player has one. Demons by default; a character
    /// that carries the demon (Lil' Monsta) gains such an ability.
    fn counts_as_demon(&self) -> bool {
        self.kind() == Kind::Demon
    }

    /// Whether a functioning holder appears in the evil team the Minions and
    /// Demon learn (the Magician: the Demon thinks they are a Minion, the
    /// Minions think they are a Demon).
    fn joins_evil_info(&self) -> bool {
        false
    }

    /// Whether, while a living holder's ability works, the Minions and Demon
    /// do not learn each other (the Poppy Grower; see `narrator::evil_info`).
    fn hides_evil_info(&self) -> bool {
        false
    }

    /// Whether this character in play (as a real character, not a believed
    /// one) means the game has no Demon, so good does not win by the Demon
    /// dying (the Atheist).
    fn demonless(&self) -> bool {
        false
    }

    /// Whether this ability (held by `me`, alive or dead) stops `winner`
    /// from winning now (the Mastermind's extra day). Asked by
    /// `ctx::Write::check_win` before the rules' win conditions apply.
    fn vetoes_win(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _winner: Alignment) -> bool {
        false
    }

    /// Whether this character must be seated next to a Demon (the
    /// Marionette); whoever seats the players arranges it.
    fn seated_next_to_demon(&self) -> bool {
        false
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
