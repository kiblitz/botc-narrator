use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "pukka"
    }

    fn name(&self) -> &'static str {
        "Pukka"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(47),
            other: Some(42),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// A dead Pukka's poison ends.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if me == dead {
            ability::sync_marks(ctx, me, &Def, game::Effect::Poisoned, POISON, &[]);
        }
    }
}

/// The Pukka's poison lasts until its next kill, across the night boundary.
const POISON: game::Expiry = game::Expiry::Never;

/// Each night, poisons a player; the previously poisoned player dies, then
/// becomes healthy. While the Pukka malfunctions the previous player neither
/// dies nor recovers (they die once it is healthy again), and the new choice
/// poisons nobody.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to poison?", &candidates);
    ctx.sleep(me);
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    for previous in ability::marked(ctx.state(), me, &Def, game::Effect::Poisoned) {
        ability::demon_kill(ctx, me, &previous);
    }
    let poisoned: Vec<player::Id> = target.into_iter().collect();
    ability::sync_marks(ctx, me, &Def, game::Effect::Poisoned, POISON, &poisoned);
}
