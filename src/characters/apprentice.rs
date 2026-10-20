use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "apprentice"
    }

    fn name(&self) -> &'static str {
        "Apprentice"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(1),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && state.phase() == game::Phase::Night(1)
    }
}

/// On their first night, gains a Townsfolk ability (if good) or a Minion
/// ability (if evil) from the script, usually one not in play. The gained
/// ability acts at its own place in tonight's order and does its setup now
/// (a gained Fortune Teller gets a red herring).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let kind = if ctx.state().is_evil(me) {
        character::Kind::Minion
    } else {
        character::Kind::Townsfolk
    };
    let mut options = ability::not_in_play(ctx.state(), kind);
    if options.is_empty() {
        options = ctx
            .state()
            .script()
            .iter()
            .copied()
            .filter(|c| c.kind() == kind)
            .collect();
    }
    let question = "which ability the Apprentice gains";
    let Some(gained) =
        ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
    else {
        return;
    };
    ctx.gain_ability(me, gained);
    gained.on_setup(ctx, me);
    ctx.wake(me);
    ctx.tell(me, &info::Message::Character(gained.name()));
    ctx.sleep(me);
}
