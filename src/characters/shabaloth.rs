use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "shabaloth"
    }

    fn name(&self) -> &'static str {
        "Shabaloth"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(43),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Who the Shabaloth chose last night (one token each).
const CHOSEN: game::Effect = game::Effect::Custom("Chosen");

/// Before it wakes, the storyteller may regurgitate one dead player it chose
/// last night; then it chooses 2 players, who die.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let last_night = ability::marked(ctx.state(), me, &Def, CHOSEN);
    if !ctx.state().is_malfunctioning(me) {
        regurgitate(ctx, me, &last_night);
    }
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let targets = ctx.ask(me, "Choose 2 players to kill", &candidates, 2);
    ctx.sleep(me);
    ability::sync_marks(ctx, me, &Def, CHOSEN, game::Expiry::Never, &targets);
    for target in &targets {
        ability::demon_kill(ctx, me, target);
    }
}

/// Rarely (the storyteller's call) a dead player chosen last night returns.
fn regurgitate(ctx: &mut ctx::Write<'_>, me: &player::Id, chosen: &[player::Id]) {
    let dead: Vec<player::Id> = chosen
        .iter()
        .filter(|p| !ctx.state().is_alive(p))
        .cloned()
        .collect();
    if dead.is_empty() {
        return;
    }
    let mut read = ctx.read();
    if !ability::storyteller_agrees(&mut read, me, &Def, "regurgitate a player?") {
        return;
    }
    let question = "which player chosen last night is regurgitated?";
    if let Some(back) = ability::storyteller_picks_player(&mut read, me, &Def, question, dead) {
        ctx.revive(&back);
    }
}
