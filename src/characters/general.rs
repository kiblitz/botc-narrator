use crate::character;
use crate::ctx;
use crate::info;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "general"
    }

    fn name(&self) -> &'static str {
        "General"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(75),
            other: Some(94),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

/// Each night, learns which alignment the storyteller believes is winning.
fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) {
    let answers = [
        None,
        Some(character::Alignment::Good),
        Some(character::Alignment::Evil),
    ]
    .map(info::Message::Winning);
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character::Def::name(&Def),
        question: "which team do you believe is winning?",
    };
    let believed = ctx
        .choose(reason, answers.to_vec(), 1, |message| {
            storyteller::Choice::Info {
                message: message.clone(),
                truthful: true,
            }
        })
        .into_iter()
        .next()
        .unwrap_or(info::Message::Winning(None));
    let misinformation = answers.into_iter().filter(|m| *m != believed).collect();
    ctx.wake(me);
    ctx.inform(me, vec![believed], misinformation);
    ctx.sleep(me);
}
