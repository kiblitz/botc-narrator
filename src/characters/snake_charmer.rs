use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "snake_charmer"
    }

    fn name(&self) -> &'static str {
        "Snake Charmer"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(20),
            other: Some(11),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

/// Chooses a living player: a chosen Demon swaps characters and alignments
/// with the Snake Charmer and is then poisoned (for good). Both are told who
/// they are now; otherwise nothing is learned.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Whom do you charm?", &candidates);
    ctx.sleep(me);
    let Some(demon) = target else {
        return;
    };
    let state = ctx.state();
    if &demon == me
        || state.kind(&demon) != character::Kind::Demon
        || state.is_malfunctioning(me)
    {
        return;
    }
    let (mine, my_alignment) = (state.character(me), state.alignment(me));
    let (theirs, their_alignment) = (state.character(&demon), state.alignment(&demon));
    ctx.transform(me, theirs);
    ctx.set_alignment(me, their_alignment);
    ctx.transform(&demon, mine);
    ctx.set_alignment(&demon, my_alignment);
    let poison = game::Reminder::new(&Def, me, game::Effect::Poisoned, game::Expiry::Never);
    ctx.place_reminder(&demon, poison);
    for player in [me, &demon] {
        let state = ctx.state();
        let character = info::Message::Character(state.character(player).name());
        let alignment = info::Message::IsAligned {
            player: player.clone(),
            alignment: state.alignment(player),
        };
        ctx.wake(player);
        ctx.tell(player, &character);
        ctx.tell(player, &alignment);
        ctx.sleep(player);
    }
}
