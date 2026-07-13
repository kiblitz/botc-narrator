//! Empath (Townsfolk): each night, learn how many of your two living
//! neighbours register as evil.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Empath, "empath", "Empath", Kind::Townsfolk, Alignment::Good);

impl Ability for Empath {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, _night: u32) {
        if !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let truth = match ctx.grim.alive_neighbours(me) {
            None => 0,
            Some((left, right)) => {
                // Registration (not raw alignment) so a Recluse can miscount and
                // a Spy can hide — resolved by storyteller discretion.
                let mut n = i32::from(ctx.registers_evil(left, "empath"));
                if right != left {
                    n += i32::from(ctx.registers_evil(right, "empath"));
                }
                n
            }
        };
        let shown = ctx.deliver(me, truth, &[0, 1, 2]);
        ctx.reveal(me, &format!("{shown} of your neighbours are evil"));
        ctx.sleep(me);
    }
}
