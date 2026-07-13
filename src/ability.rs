//! The [`Ability`] trait: everything a character *does*, in one place.
//!
//! A character is a stateless behaviour object (all mutable state lives in the
//! grimoire as tokens), so a single shared instance per role serves the whole
//! game. Every hook has a do-nothing default, so a new role implements only the
//! handful that apply to it and is otherwise inert. Roles never reference each
//! other: they raise intents through [`Ctx`] and react to events about
//! themselves. That is the whole appendability story.

use crate::ctx::Ctx;
use crate::event::{DeathSource, Registration};
use crate::ids::PlayerId;
use crate::role::CharacterInfo;

pub trait Ability {
    /// Static metadata (id, name, starting kind/alignment).
    fn info(&self) -> CharacterInfo;

    // --- Scheduled lifecycle ------------------------------------------------

    /// Runs during setup, before the first night (e.g. Baron adds Outsiders,
    /// Fortune Teller places its red herring, Drunk is assigned).
    fn on_setup(&self, _ctx: &mut Ctx, _me: PlayerId) {}

    /// Runs when this character is called during the night order. `night` is
    /// 1-based. Roles that do nothing at night leave this defaulted.
    fn on_night(&self, _ctx: &mut Ctx, _me: PlayerId, _night: u32) {}

    // --- Triggered day interactions ----------------------------------------

    /// This player was nominated by `nominator` (Virgin).
    fn on_nominated(&self, _ctx: &mut Ctx, _me: PlayerId, _nominator: PlayerId) {}

    /// This player is about to be executed (Saint). Fires before the death is
    /// applied, so the hook can end the game or otherwise intervene.
    fn on_executed(&self, _ctx: &mut Ctx, _me: PlayerId) {}

    /// This player uses their public, once-per-day ability on `target` (Slayer
    /// shot). Driven by an explicit player declaration, not the night order.
    fn on_day_ability(&self, _ctx: &mut Ctx, _me: PlayerId, _target: PlayerId) {}

    /// A day ended with no execution. Fired for every living player's ability so
    /// the Mayor can win the "three players, no execution" endgame. Keeps that
    /// win condition out of the engine.
    fn on_no_execution(&self, _ctx: &mut Ctx, _me: PlayerId) {}

    // --- Death pipeline -----------------------------------------------------

    /// Can this player shrug off the Demon's kill tonight? `true` means they
    /// survive (Soldier). Only consulted for demon kills, and only when no
    /// token protection already applied. Should honour impairment.
    fn blocks_demon_kill(&self, _ctx: &Ctx, _me: PlayerId) -> bool {
        false
    }

    /// The demon's kill on this player may land on someone else instead (Mayor
    /// bounce). Returning `Some(other)` redirects the death.
    fn redirect_death(
        &self,
        _ctx: &mut Ctx,
        _me: PlayerId,
        _source: DeathSource,
    ) -> Option<PlayerId> {
        None
    }

    /// React to any death that has just been applied. Fired for every living
    /// player's ability, plus the newly dead player's own ability (so the
    /// Ravenkeeper can trigger on its own death). Scarlet Woman promotion,
    /// Undertaker bookkeeping, and Poisoner self-cleanup all live here.
    fn on_any_death(&self, _ctx: &mut Ctx, _me: PlayerId, _dead: PlayerId, _source: DeathSource) {}

    // --- Misregistration ----------------------------------------------------

    /// How this player may register to information abilities. `None` (default)
    /// means "exactly as my current kind/alignment". The Recluse and Spy
    /// override this.
    fn misregistration(&self) -> Option<Registration> {
        None
    }
}
