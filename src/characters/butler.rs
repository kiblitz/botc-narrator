use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "butler"
    }

    fn name(&self) -> &'static str {
        "Butler"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_action(&self, _night: u32) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

pub const MASTER: game::Effect = game::Effect::Custom("master");

/// Chooses a master (whose vote they must follow tomorrow).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if !ctx.state().is_alive(me) {
        return;
    }
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    if let Some(master) = ctx.ask_one(me, "Choose your master", &candidates) {
        let reminder = game::Reminder::new(&Def, me, MASTER, game::Expiry::Dusk);
        ctx.place_reminder(&master, reminder);
    }
    ctx.sleep(me);
}
