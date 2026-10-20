use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "harlot"
    }

    fn name(&self) -> &'static str {
        "Harlot"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(1),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Each night*, chooses a living player: if they agree, the Harlot learns
/// their character (as it registers), and the storyteller may kill both. The
/// storyteller is not offered the deaths when they would kill a demon (that
/// would end the game on a reveal). Malfunctioning, the character may be
/// false and nobody dies.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_except(me);
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whose character do you want to learn?", &candidates);
    ctx.sleep(me);
    let Some(target) = target else {
        return;
    };
    ctx.wake(&target);
    let message = info::Message::Selected {
        by: character::Def::name(&Def),
        effect: "do you agree to show them your character?",
    };
    ctx.tell(&target, &message);
    let agrees = ctx.ask_yes_no(&target, "Do you agree?");
    ctx.sleep(&target);
    if !agrees {
        return;
    }
    ctx.wake(me);
    ability::tell_character(&mut ctx.read(), me, &target);
    ctx.sleep(me);
    let state = ctx.state();
    let demon = state.demons().contains(&target);
    if !demon
        && !state.is_malfunctioning(me)
        && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "do both players die?")
    {
        ability::kill(ctx, me, &target);
        ability::kill(ctx, me, me);
    }
}
