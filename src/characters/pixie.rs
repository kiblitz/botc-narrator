use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

/// On the Townsfolk the Pixie learnt.
const LEARNT: game::Effect = game::Effect::Custom("Learnt");
/// On a Pixie who broke madness: they gain nothing.
const BROKE: game::Effect = game::Effect::Custom("Not mad");

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "pixie"
    }

    fn name(&self) -> &'static str {
        "Pixie"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(29),
            other: None,
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    fn on_madness_broken(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, mad: &player::Id) {
        if mad == me {
            let reminder = game::Reminder::new(&Def, me, BROKE, game::Expiry::Never);
            ctx.place_reminder(me, reminder);
        }
    }

    /// The learnt Townsfolk died: a Pixie who stayed mad (and is sober) gains
    /// the ability of the character they learnt.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        let state = ctx.state();
        if dead == me
            || !state.is_alive(me)
            || state.is_malfunctioning(me)
            || !ability::has_mark(state, me, &Def, dead, LEARNT)
            || ability::has_mark(state, me, &Def, me, BROKE)
        {
            return;
        }
        let about = state.player(me).reminders.iter().find_map(|r| match r.effect {
            game::Effect::Mad(about) if r.source == character::Def::id(&Def) => Some(about),
            _ => None,
        });
        let gained = state
            .script()
            .iter()
            .copied()
            .find(|c| Some(c.name()) == about);
        if let Some(gained) = gained {
            ctx.gain_ability(me, gained);
        }
    }
}

/// Learns an in-play Townsfolk (any script Townsfolk if malfunctioning) and is
/// mad about being it; a player who is that character is marked.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let state = ctx.state();
    let in_play: Vec<info::Message> = ability::others(state, me)
        .into_iter()
        .map(|p| state.character(&p))
        .filter(|c| c.kind() == character::Kind::Townsfolk)
        .map(|c| info::Message::Character(c.name()))
        .collect();
    let misinformation: Vec<info::Message> = state
        .script()
        .iter()
        .filter(|c| c.kind() == character::Kind::Townsfolk && !character::same(**c, &Def))
        .map(|c| info::Message::Character(c.name()))
        .collect();
    ctx.wake(me);
    let told = ctx.inform(me, in_play, misinformation);
    ctx.sleep(me);
    let Some(info::Message::Character(about)) = told else {
        return;
    };
    let mad = game::Reminder::new(&Def, me, game::Effect::Mad(about), game::Expiry::Never);
    ctx.place_reminder(me, mad);
    let state = ctx.state();
    let learnt = ability::others(state, me)
        .into_iter()
        .find(|p| state.character(p).name() == about);
    if let Some(learnt) = learnt {
        let reminder = game::Reminder::new(&Def, me, LEARNT, game::Expiry::Never);
        ctx.place_reminder(&learnt, reminder);
    }
}
