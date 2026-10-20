use std::fmt;

use crate::ctx;
use crate::event;
use crate::player;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Townsfolk,
    Outsider,
    Minion,
    Demon,
}

impl Kind {
    pub fn alignment(self) -> Alignment {
        match self {
            Kind::Townsfolk | Kind::Outsider => Alignment::Good,
            Kind::Minion | Kind::Demon => Alignment::Evil,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alignment {
    Good,
    Evil,
}

impl fmt::Display for Alignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub type ReadOnlyFn = fn(&mut ctx::Read<'_>, &player::Id, u32);
pub type ReadWriteFn = fn(&mut ctx::Write<'_>, &player::Id, u32);
pub type DayFn = fn(&mut ctx::Write<'_>, &player::Id);

/// A night ability, tagged with the access it needs. Consecutive read-only
/// actions in the night order form a batch the engine may run concurrently;
/// read-write actions are sequential sync points.
#[derive(Clone, Copy)]
pub enum NightAction {
    ReadOnly(ReadOnlyFn),
    ReadWrite(ReadWriteFn),
}

/// A character definition. Every hook defaults to "no ability"; a character
/// overrides only the hooks its ability uses.
pub trait Def: Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn kind(&self) -> Kind;

    fn night_action(&self, _night: u32) -> Option<NightAction> {
        None
    }

    fn day_action(&self) -> Option<DayFn> {
        None
    }

    fn on_setup(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

    fn on_nominated(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id, _nominator: &player::Id) {}

    fn on_executed(&self, _ctx: &mut ctx::Write<'_>, _me: &player::Id) {}

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

    /// Whether a functioning player with this character survives demon kills.
    fn safe_from_demon(&self) -> bool {
        false
    }

    /// Where a demon kill aimed at this functioning player lands instead.
    /// `None` (or `Some(me)`) means the kill is not redirected.
    fn redirect_demon_kill(
        &self,
        _ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
    ) -> Option<player::Id> {
        None
    }
}

pub type Ref = &'static dyn Def;

impl fmt::Debug for dyn Def {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
