use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "storm_catcher"
    }

    fn name(&self) -> &'static str {
        "Storm Catcher"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Loric
    }

    /// The storyteller names a good character on the script. If a player is
    /// it, they can only die by execution; every evil player learns which
    /// player it is (or that it is not in play).
    fn fabled_setup(&self, ctx: &mut ctx::Write<'_>) {
        let good: Vec<character::Ref> = ctx
            .state()
            .script()
            .iter()
            .copied()
            .filter(|c| c.kind().alignment() == Some(character::Alignment::Good))
            .collect();
        let reason = storyteller::Reason::Fabled {
            character: character::Def::name(&Def),
            question: "which good character to name",
        };
        let Some(named) = ctx
            .choose(reason, good, 1, |c| storyteller::Choice::Character(c.name()))
            .into_iter()
            .next()
        else {
            return;
        };
        let holder: Option<player::Id> = ctx
            .state()
            .players()
            .iter()
            .find(|p| character::same(p.character, named))
            .map(|p| p.id.clone());
        let message = match &holder {
            Some(holder) => {
                let reminder = game::Reminder::new(
                    &Def,
                    holder,
                    game::Effect::ExecutionOnly,
                    game::Expiry::Never,
                );
                ctx.place_reminder(holder, reminder);
                info::Message::Is {
                    player: holder.clone(),
                    character: named.name(),
                }
            }
            None => info::Message::NotInPlay(named.name()),
        };
        for evil in ctx.state().evil_ids() {
            ctx.wake(&evil);
            ctx.tell(&evil, &message);
            ctx.sleep(&evil);
        }
    }
}
