//! Ravenkeeper (Townsfolk): if you die at night, you are woken to choose a
//! player and learn their character.

use crate::ability::Ability;
use crate::ctx::Ctx;
use crate::event::DeathSource;
use crate::ids::PlayerId;
use crate::role::{Alignment, CharacterInfo, Kind};

character!(
    Ravenkeeper,
    "ravenkeeper",
    "Ravenkeeper",
    Kind::Townsfolk,
    Alignment::Good
);

impl Ability for Ravenkeeper {
    fn info(&self) -> CharacterInfo {
        Self::INFO
    }

    fn on_any_death(&self, ctx: &mut Ctx, me: PlayerId, dead: PlayerId, source: DeathSource) {
        // Trigger only on my own death, and only if it happened at night.
        if me != dead || source.is_execution() {
            return;
        }
        ctx.wake(me);
        let candidates: Vec<PlayerId> = ctx.grim.seats().collect();
        let choice = ctx.ask(me, "Choose a player", &candidates);
        // Impairment at the moment of death corrupts the reading.
        let shown = if ctx.is_impaired(me) {
            let names = ctx.roster_character_names();
            names[ctx.choose("ravenkeeper false info", &names)].clone()
        } else {
            let cid = ctx.registers_as_character(choice, "ravenkeeper");
            ctx.registry().info(cid).name.to_string()
        };
        ctx.reveal(me, &format!("{} is the {}", ctx.name(choice), shown));
        ctx.sleep(me);
    }
}
