use crate::ability;
use crate::character;
use crate::ctx;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "fisherman"
    }

    fn name(&self) -> &'static str {
        "Fisherman"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn day_action(&self) -> Option<character::DayFn> {
        Some(visit)
    }
}

/// Once per game, by day, visits the storyteller for advice to help their
/// team win, modelled as whom to trust: good advice trusts players on the
/// Fisherman's team and distrusts the others; bad advice (when
/// malfunctioning) is the reverse.
fn visit(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    if !ctx.state().is_alive(me) || !ability::use_once(ctx, me, &Def) {
        return;
    }
    let state = ctx.state();
    let team = state.alignment(me);
    let advice = |good: bool| -> Vec<info::Message> {
        state
            .alive_except(me)
            .into_iter()
            .map(|player| {
                let ally = state.alignment(&player) == team;
                info::Message::Advice {
                    player,
                    trust: ally == good,
                }
            })
            .collect()
    };
    let (good, bad) = (advice(true), advice(false));
    ctx.wake(me);
    ctx.inform(me, good, bad);
    ctx.sleep(me);
}
