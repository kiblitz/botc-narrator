use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "cerenovus"
    }

    fn name(&self) -> &'static str {
        "Cerenovus"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(25),
            other: Some(15),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// The storyteller may execute a player who broke madness (if nobody has
    /// been executed today).
    fn on_madness_broken(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, mad: &player::Id) {
        if ctx.state().executed_today().is_none()
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "execute the mad player?")
        {
            ctx.execute(mad);
        }
    }
}

/// Chooses a player and a good character: they are mad about being it
/// tomorrow. The chosen player is woken and told.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let good: Vec<character::Ref> = ctx
        .state()
        .script()
        .iter()
        .copied()
        .filter(|c| c.kind().alignment() == Some(character::Alignment::Good))
        .collect();
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to make mad?", &candidates);
    let about = ctx.ask_character(me, "Mad about being which character?", &good);
    ctx.sleep(me);
    let (Some(target), Some(about)) = (target, about) else {
        return;
    };
    let mad = game::Effect::Mad(about.name());
    ability::mark(ctx, me, &Def, &target, mad, game::Expiry::Dusk);
    ctx.wake(&target);
    ctx.tell(&target, &info::Message::Character(about.name()));
    ctx.sleep(&target);
}
