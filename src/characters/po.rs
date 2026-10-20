use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "po"
    }

    fn name(&self) -> &'static str {
        "Po"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(44),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// The "3 attacks" reminder: the Po's last choice was no-one.
const CHARGED: game::Effect = game::Effect::Custom("3 attacks");

/// Each night*, may choose a player to kill; after choosing no-one, the next
/// time it must choose 3. Choosing no-one charges it even while it
/// malfunctions (only the kills fail).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    let charged = ability::has_mark(ctx.state(), me, &Def, me, CHARGED);
    ctx.wake(me);
    let targets = if charged {
        ctx.ask(me, "Choose 3 players to kill", &candidates, 3)
    } else {
        ctx.ask_up_to(me, "Who do you want to kill, if anyone?", &candidates, 1)
    };
    ctx.sleep(me);
    let charge = game::Reminder::new(&Def, me, CHARGED, game::Expiry::Never);
    if charged {
        ctx.remove_reminder(me, &charge);
    } else if targets.is_empty() {
        ctx.place_reminder(me, charge);
    }
    for target in &targets {
        ability::demon_kill(ctx, me, target);
    }
}
