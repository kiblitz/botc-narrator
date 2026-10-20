use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "godfather"
    }

    fn name(&self) -> &'static str {
        "Godfather"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(21),
            other: Some(37),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// On the first night to learn; afterwards only if an Outsider died today.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me)
            && (state.phase() == game::Phase::Night(1)
                || ability::died_today(state)
                    .iter()
                    .any(|p| state.kind(p) == character::Kind::Outsider))
    }

    /// [-1 or +1 Outsider]
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        let mut options = vec![counts.shift_outsiders(1)];
        if counts.outsiders > 0 {
            options.push(counts.shift_outsiders(-1));
        }
        options
    }
}

/// Starts knowing which Outsiders are in play. If one died today, chooses a
/// player tonight: they die.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    if night == 1 {
        let others = ability::others(ctx.state(), me);
        let mut in_play: Vec<&'static str> = Vec::new();
        for p in &others {
            if let Some(c) = ctx.registers_as(me, p, character::Kind::Outsider)
                && !in_play.contains(&c.name())
            {
                in_play.push(c.name());
            }
        }
        let truthful = vec![info::Message::Characters(in_play)];
        let misinformation: Vec<info::Message> = ctx
            .state()
            .script()
            .iter()
            .filter(|c| c.kind() == character::Kind::Outsider)
            .map(|c| info::Message::Characters(vec![c.name()]))
            .chain([info::Message::Characters(Vec::new())])
            .collect();
        ctx.wake(me);
        ctx.inform(me, truthful, misinformation);
        ctx.sleep(me);
        return;
    }
    let candidates = ctx.state().alive_ids();
    ctx.wake(me);
    let target = ctx.ask_one(me, "Who do you want to kill?", &candidates);
    ctx.sleep(me);
    if let Some(target) = target {
        ability::kill(ctx, me, &target);
    }
}
