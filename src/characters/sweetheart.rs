use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "sweetheart"
    }

    fn name(&self) -> &'static str {
        "Sweetheart"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// The Drunk token is placed when the Sweetheart dies; it has no action.
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(41),
        }
    }

    /// When the Sweetheart dies, a player of the storyteller's choice is drunk
    /// from now on.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if dead != me || ctx.state().is_malfunctioning(me) {
            return;
        }
        let options = ctx.state().seat_order();
        let question = "who is drunk from now on";
        if let Some(target) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, options)
        {
            let reminder = game::Reminder::new(&Def, me, game::Effect::Drunk, game::Expiry::Never);
            ctx.place_reminder(&target, reminder);
        }
    }
}
