//! Spy (Minion): each night you see the Grimoire. You might register as good,
//! and as a Townsfolk or Outsider.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::event::Registration;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Spy, "spy", "Spy", Kind::Minion, Alignment::Evil);

impl Ability for Spy {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, _night: u32) {
        if !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        // The Spy sees everything: reveal the whole grimoire, seat by seat.
        let lines: Vec<String> = ctx
            .grim
            .seats()
            .map(|id| {
                let alive = if ctx.grim.is_alive(id) { "" } else { " [dead]" };
                format!("{}{}", ctx.label(id), alive)
            })
            .collect();
        ctx.reveal(me, &format!("Grimoire: {}", lines.join(", ")));
        ctx.sleep(me);
    }

    fn misregistration(&self) -> Option<Registration> {
        Some(Registration::spy())
    }
}
