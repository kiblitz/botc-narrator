use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "poisoner"
    }

    fn name(&self) -> &'static str {
        "Poisoner"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(17),
            other: Some(7),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Poisons a player until the next dusk.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    if let Some(target) = ctx.ask_one(me, "Who do you want to poison?", &candidates)
        && !ctx.state().is_malfunctioning(me)
    {
        let reminder = game::Reminder::new(&Def, me, game::Effect::Poisoned, game::Expiry::Dusk);
        ctx.place_reminder(&target, reminder);
    }
    ctx.sleep(me);
}
