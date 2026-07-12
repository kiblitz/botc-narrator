//! Imp (Demon): each night after the first, kill a player. Killing yourself
//! "starpasses" the Imp to a Minion of the storyteller's choice.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Imp, "imp", "Imp", Kind::Demon, Alignment::Evil);

impl Ability for Imp {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night <= 1 || !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let candidates: Vec<PlayerId> = ctx.grim.alive().collect();
        let target = ctx.ask(me, "Who do you want to kill?", &candidates);

        if ctx.is_impaired(me) {
            // A poisoned/drunk Demon's kill simply fails.
        } else if target == me {
            self.starpass(ctx, me);
        } else {
            // Fire the intent; the pipeline handles Soldier/Monk/Mayor. The Imp
            // knows nothing about who might survive.
            ctx.demon_kill(target);
        }
        ctx.sleep(me);
    }
}

impl Imp {
    /// The Imp kills itself; a Minion (storyteller's choice) becomes the new
    /// Imp. With no Minion available, the Imp just dies.
    fn starpass(&self, ctx: &mut Ctx, me: PlayerId) {
        let minions: Vec<PlayerId> = ctx
            .grim
            .alive()
            .filter(|&id| id != me && ctx.grim.get(id).kind == Kind::Minion)
            .collect();
        if minions.is_empty() {
            ctx.ability_kill(me, "starpass");
            return;
        }
        let labels: Vec<String> = minions.iter().map(|&id| ctx.label(id)).collect();
        let i = ctx.choose("starpass: which minion becomes the Imp?", &labels);
        let new_imp = minions[i];
        // Create the new demon *before* the old one dies, so the resolved death
        // sees a living demon and the Scarlet Woman does not also promote.
        ctx.transform(new_imp, Self::INFO);
        ctx.ability_kill(me, "starpass");
    }
}
