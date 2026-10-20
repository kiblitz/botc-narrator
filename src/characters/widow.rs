use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "widow"
    }

    fn name(&self) -> &'static str {
        "Widow"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(18),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// On their 1st night, looks at the grimoire and chooses a player: they are
/// poisoned until the Widow dies. 1 good player (the storyteller's choice)
/// learns a Widow is in play.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let grimoire = ability::grimoire(ctx.state());
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    ctx.inform(me, vec![grimoire], Vec::new());
    let target = ctx.ask_one(me, "Who do you want to poison?", &candidates);
    ctx.sleep(me);
    if let Some(target) = target {
        let (effect, expiry) = (game::Effect::Poisoned, game::Expiry::Death);
        ability::mark(ctx, me, &Def, &target, effect, expiry);
    }
    let good: Vec<player::Id> = ctx
        .state()
        .alive_ids()
        .into_iter()
        .filter(|p| !ctx.state().is_evil(p))
        .collect();
    let question = "which good player knows a Widow is in play";
    if let Some(knows) = ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, good) {
        ctx.wake(&knows);
        ctx.tell(&knows, &info::Message::Character(character::Def::name(&Def)));
        ctx.sleep(&knows);
    }
}
