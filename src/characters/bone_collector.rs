use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "bone_collector"
    }

    fn name(&self) -> &'static str {
        "Bone Collector"
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

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && !ability::used(state, me, &Def)
    }
}

/// Once per game, at night, may choose a dead player: their ability works
/// until dusk (they wake for it tonight). Malfunctioning, the use is spent
/// but nothing returns.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let dead: Vec<player::Id> = ctx
        .state()
        .players()
        .iter()
        .filter(|p| !p.alive)
        .map(|p| p.id.clone())
        .collect();
    ctx.wake(me);
    let uses = !dead.is_empty() && ctx.ask_yes_no(me, "Do you use your ability tonight?");
    let target = if uses {
        ctx.ask_one(me, "Whose ability returns until dusk?", &dead)
    } else {
        None
    };
    ctx.sleep(me);
    if let Some(target) = target
        && ability::use_once(ctx, me, &Def)
    {
        let effect = game::Effect::AbilityRestored;
        ability::mark(ctx, me, &Def, &target, effect, game::Expiry::Dusk);
    }
}
