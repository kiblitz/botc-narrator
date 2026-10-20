use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "grandmother"
    }

    fn name(&self) -> &'static str {
        "Grandmother"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(59),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }

    /// The storyteller picks a good player as the Grandchild.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let good: Vec<player::Id> = ability::others(ctx.state(), me)
            .into_iter()
            .filter(|p| !ctx.state().is_evil(p))
            .collect();
        let question = "who is the Grandchild?";
        if let Some(child) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, good)
        {
            let reminder = game::Reminder::new(&Def, me, GRANDCHILD, game::Expiry::Never);
            ctx.place_reminder(&child, reminder);
        }
    }

    /// If the Demon kills the Grandchild, the Grandmother dies too.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        cause: &event::Cause,
    ) {
        if *cause == event::Cause::Demon
            && ctx.state().is_alive(me)
            && ability::has_mark(ctx.state(), me, &Def, dead, GRANDCHILD)
        {
            ability::kill(ctx, me, me);
        }
    }
}

const GRANDCHILD: game::Effect = game::Effect::Custom("Grandchild");

/// Starts knowing a good player (the Grandchild) and their character.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let others = ability::others(ctx.state(), me);
    let Some(child) = others
        .iter()
        .find(|p| ability::has_mark(ctx.state(), me, &Def, p, GRANDCHILD))
        .cloned()
    else {
        return;
    };
    let character = ctx.registered_character(me, &child).name();
    let truthful = info::Message::Is {
        player: child,
        character,
    };
    let names = ctx.state().character_names();
    let misinformation = others
        .iter()
        .flat_map(|p| {
            names.iter().map(|c| info::Message::Is {
                player: p.clone(),
                character: c,
            })
        })
        .collect();
    ctx.wake(me);
    ctx.inform(me, vec![truthful], misinformation);
    ctx.sleep(me);
}
