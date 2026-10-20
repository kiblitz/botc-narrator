use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "goon"
    }

    fn name(&self) -> &'static str {
        "Goon"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// Each night, the first player to choose the Goon with their ability is
    /// drunk until dusk, and the Goon becomes their alignment (and is told).
    fn on_chosen(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, chooser: &player::Id) {
        let state = ctx.state();
        if !state.is_alive(me)
            || state.is_malfunctioning(me)
            || ability::has_mark(state, me, &Def, me, CHOSEN)
        {
            return;
        }
        let chosen = game::Reminder::new(&Def, me, CHOSEN, game::Expiry::Dusk);
        ctx.place_reminder(me, chosen);
        let drunk = game::Reminder::new(&Def, me, game::Effect::Drunk, game::Expiry::Dusk);
        ctx.place_reminder(chooser, drunk);
        let alignment = ctx.state().alignment(chooser);
        if ctx.set_alignment(me, alignment) {
            ctx.wake(me);
            ctx.tell(me, &info::Message::Alignment(alignment));
            ctx.sleep(me);
        }
    }
}

/// The Goon has been chosen tonight.
const CHOSEN: game::Effect = game::Effect::Custom("chosen tonight");
