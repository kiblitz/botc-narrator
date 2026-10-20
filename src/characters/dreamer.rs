use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "dreamer"
    }

    fn name(&self) -> &'static str {
        "Dreamer"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(61),
            other: Some(79),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

const GOOD: [character::Kind; 2] = [character::Kind::Townsfolk, character::Kind::Outsider];
const EVIL: [character::Kind; 2] = [character::Kind::Minion, character::Kind::Demon];

/// Each night, chooses a player (not themself or a Traveller): learns 1 good
/// and 1 evil character, 1 of which is the player's (as they register).
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let candidates: Vec<player::Id> = ability::others(ctx.state(), me)
        .into_iter()
        .filter(|p| ctx.state().kind(p) != character::Kind::Traveller)
        .collect();
    ctx.wake(me);
    let Some(target) = ctx.ask_one(me, "Whose character do you want to dream?", &candidates)
    else {
        ctx.sleep(me);
        return;
    };
    let registered = ctx.registered_character(me, &target);
    let mut good = ability::nameable(ctx.state(), &GOOD);
    let mut evil = ability::nameable(ctx.state(), &EVIL);
    // The registered character is always shown, even if it is off the script.
    let pool = if GOOD.contains(&registered.kind()) {
        &mut good
    } else {
        &mut evil
    };
    if !pool.iter().any(|c| character::same(*c, registered)) {
        pool.push(registered);
    }
    let pair = |g: character::Ref, e: character::Ref| info::Message::Characters(vec![g.name(), e.name()]);
    let (mut truthful, mut misinformation) = (Vec::new(), Vec::new());
    for g in &good {
        for e in &evil {
            let message = pair(*g, *e);
            if character::same(*g, registered) || character::same(*e, registered) {
                truthful.push(message);
            } else {
                misinformation.push(message);
            }
        }
    }
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}
