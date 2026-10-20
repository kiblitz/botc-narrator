use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "spy"
    }

    fn name(&self) -> &'static str {
        "Spy"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_action(&self, _night: u32) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Sees the grimoire.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    if !state.is_alive(me) {
        return;
    }
    let grimoire = state
        .players()
        .iter()
        .map(|p| (p.id.clone(), p.character.name()))
        .collect();
    ctx.wake(me);
    ctx.inform(me, vec![info::Message::Grimoire(grimoire)], Vec::new());
    ctx.sleep(me);
}
