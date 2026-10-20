use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "philosopher"
    }

    fn name(&self) -> &'static str {
        "Philosopher"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(2),
            other: Some(2),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, chooses a good character and gains that ability; whoever
/// is that character is drunk while the Philosopher lives and is sober. A
/// drunk or poisoned Philosopher spends the ability for nothing. The gained
/// ability acts at its own night-order position, tonight if still to come.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let options: Vec<character::Ref> = ctx
        .state()
        .script()
        .iter()
        .copied()
        .filter(|c| {
            c.kind().alignment() == Some(character::Alignment::Good) && !character::same(*c, &Def)
        })
        .collect();
    ctx.wake(me);
    let chosen = ctx.ask_character(me, "Which good character's ability do you want?", &options);
    ctx.sleep(me);
    let Some(chosen) = chosen else { return };
    ability::use_once(ctx, me, &Def);
    if ctx.state().is_malfunctioning(me) {
        return;
    }
    let state = ctx.state();
    let in_play: Vec<player::Id> = ability::others(state, me)
        .into_iter()
        .filter(|p| character::same(state.character(p), chosen))
        .collect();
    ctx.gain_ability(me, chosen);
    let expiry = game::Expiry::Death;
    ability::mark_all(ctx, me, &Def, &in_play, game::Effect::Drunk, expiry);
}
