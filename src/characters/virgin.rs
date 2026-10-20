use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "virgin"
    }

    fn name(&self) -> &'static str {
        "Virgin"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// The first time she is nominated, a Townsfolk nominator is executed.
    fn on_nominated(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, nominator: &player::Id) {
        if ctx.state().has_effect(me, game::Effect::AbilityUsed) {
            return;
        }
        let reminder = game::Reminder::new(&Def, game::Effect::AbilityUsed, game::Expiry::Never);
        ctx.place_reminder(me, reminder);
        if !ctx.state().is_malfunctioning(me)
            && ctx.state().kind(nominator) == character::Kind::Townsfolk
        {
            ctx.execute(nominator);
        }
    }
}
