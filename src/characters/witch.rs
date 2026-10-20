use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

const CURSED: game::Effect = game::Effect::Custom("Cursed");

/// With this few players alive the Witch loses the ability.
const POWERLESS_AT: usize = 3;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "witch"
    }

    fn name(&self) -> &'static str {
        "Witch"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(24),
            other: Some(14),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && has_power(state)
    }

    /// The cursed player dies when they nominate (the nomination still counts).
    fn on_nomination(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        nominator: &player::Id,
        _nominee: &player::Id,
    ) {
        let state = ctx.state();
        if state.is_alive(me)
            && has_power(state)
            && ability::has_mark(state, me, &Def, nominator, CURSED)
        {
            ability::kill(ctx, me, nominator);
        }
    }
}

fn has_power(state: &game::State) -> bool {
    state.alive_ids().len() > POWERLESS_AT
}

/// Each night, chooses a player (alive or dead, even themself): if they
/// nominate tomorrow, they die.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().seat_order();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to curse?", &candidates);
    ctx.sleep(me);
    if let Some(target) = target {
        ability::mark(ctx, me, &Def, &target, CURSED, game::Expiry::Dusk);
    }
}
