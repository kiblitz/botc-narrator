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
        vec![counts.shift_outsiders(-1), counts.shift_outsiders(1)]
    }
}

/// Starts knowing which Outsiders are in play. If one died today, chooses a
/// player tonight: they die.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    if night == 1 {
        let state = ctx.state();
        let names = |cs: Vec<character::Ref>| cs.iter().map(|c| c.name()).collect::<Vec<_>>();
        let in_play: Vec<character::Ref> = state
            .players()
            .iter()
            .map(|p| p.character)
            .filter(|c| c.kind() == character::Kind::Outsider)
            .collect();
        let truthful = vec![info::Message::Characters(names(in_play))];
        let misinformation: Vec<info::Message> = state
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
