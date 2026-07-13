//! Chef (Townsfolk): on the first night, learn how many pairs of evil players
//! are sitting next to each other.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(Chef, "chef", "Chef", Kind::Townsfolk, Alignment::Good);

impl Ability for Chef {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_night(&self, ctx: &mut Ctx, me: PlayerId, night: u32) {
        if night != 1 || !ctx.grim.is_alive(me) {
            return;
        }
        ctx.wake(me);
        let seats: Vec<PlayerId> = ctx.grim.seats().collect();
        // Resolve each player's evil reading once (so a Recluse/Spy is judged
        // consistently), then count adjacent evil pairs around the ring.
        let evil: Vec<bool> = seats
            .iter()
            .map(|&id| ctx.registers_evil(id, "chef"))
            .collect();
        let n = seats.len();
        let mut pairs = 0i32;
        for i in 0..n {
            if evil[i] && evil[(i + 1) % n] {
                pairs += 1;
            }
        }
        let plausible: Vec<i32> = (0..=n as i32).collect();
        let shown = ctx.deliver(me, pairs, &plausible);
        ctx.reveal(me, &format!("{shown} pairs of evil players sit together"));
        ctx.sleep(me);
    }
}
