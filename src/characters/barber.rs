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
        "barber"
    }

    fn name(&self) -> &'static str {
        "Barber"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(40),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Acts (dead) on the night of, or after, the day they died.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        ability::has_mark(state, me, &Def, me, HAIRCUTS)
    }

    /// A functioning Barber who dies gives the Demon haircuts tonight.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if dead == me && !ctx.state().is_malfunctioning(me) {
            ctx.place_reminder(me, haircuts(me));
        }
    }
}

const HAIRCUTS: game::Effect = game::Effect::Custom("haircuts tonight");

fn haircuts(me: &player::Id) -> game::Reminder {
    game::Reminder::new(&Def, me, HAIRCUTS, game::Expiry::Never)
}

/// The Demon (the storyteller's pick if there are several) chooses two
/// players, not another Demon, to swap characters; alignments stay, and each
/// learns their new character.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    ctx.remove_reminder(me, &haircuts(me));
    let demons = ctx.state().demons();
    let question = "which Demon swaps characters";
    let Some(demon) = ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, demons)
    else {
        return;
    };
    let others = ctx.state().demons();
    let candidates: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| *p == demon || !others.contains(p))
        .collect();
    ctx.wake(&demon);
    ctx.tell(&demon, &info::Message::Character(character::Def::name(&Def)));
    let chosen = ctx.ask(&demon, "Choose two players to swap characters", &candidates, 2);
    ctx.sleep(&demon);
    let [a, b] = chosen.as_slice() else {
        return;
    };
    let (into_a, into_b) = (ctx.state().character(b), ctx.state().character(a));
    for (player, into) in [(a, into_a), (b, into_b)] {
        ctx.transform(player, into);
        ctx.wake(player);
        ctx.tell(player, &info::Message::Character(into.name()));
        ctx.sleep(player);
    }
}
