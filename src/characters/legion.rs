use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::game;
use crate::player;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "legion"
    }

    fn name(&self) -> &'static str {
        "Legion"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(39),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// [Most players are Legion]: a majority of Legion, the rest Townsfolk,
    /// no Minions or Outsiders.
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        let total = counts.total();
        let legion = total / 2 + 1;
        vec![setup::Counts {
            townsfolk: total - legion,
            outsiders: 0,
            minions: 0,
            demons: legion,
        }]
    }

    fn duplicates_in_setup(&self) -> bool {
        true
    }

    /// You register as a Minion too (the storyteller decides each query).
    fn misregistration(&self) -> Option<character::Misregistration> {
        Some(character::Misregistration {
            alignment: character::Alignment::Evil,
            kinds: &[character::Kind::Minion],
        })
    }

    /// Executions fail if only evil voted.
    fn intercept_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        _me: &player::Id,
        target: &player::Id,
        cause: &event::Cause,
    ) -> character::Interception {
        if *cause != event::Cause::Execution {
            return character::Interception::Pass;
        }
        let state = ctx.state();
        let voters = ability::today(state).iter().rev().find_map(|e| match e {
            event::Event::Voted {
                nominee, voters, ..
            } if nominee == target => Some(voters),
            _ => None,
        });
        match voters {
            Some(voters) if !voters.is_empty() && voters.iter().all(|v| state.is_evil(v)) => {
                character::Interception::Prevent
            }
            _ => character::Interception::Pass,
        }
    }

    /// With one good player left, the storyteller may declare evil the winner.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        _dead: &player::Id,
        _cause: &event::Cause,
    ) {
        let state = ctx.state();
        let good_alive = state
            .alive_ids()
            .iter()
            .filter(|p| !state.is_evil(p))
            .count();
        if voice(state).as_ref() == Some(me)
            && good_alive == 1
            && ability::storyteller_agrees(&mut ctx.read(), me, &Def, "declare evil the winner?")
        {
            ctx.win(character::Alignment::Evil);
        }
    }
}

/// The Legion player the group's ability acts through: the first living,
/// functioning one in seat order.
fn voice(state: &game::State) -> Option<player::Id> {
    let legion = character::Def::id(&Def);
    state.seat_order().into_iter().find(|p| {
        state.is_alive(p) && !state.is_malfunctioning(p) && state.has_ability(p, legion)
    })
}

/// Each night*, a player might die: the storyteller decides, once for all
/// Legion (nobody wakes).
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    if voice(ctx.state()).as_ref() != Some(me) {
        return;
    }
    let mut read = ctx.read();
    if !ability::storyteller_agrees(&mut read, me, &Def, "does a player die tonight?") {
        return;
    }
    let candidates = read.state().alive_ids();
    let question = "who dies tonight?";
    if let Some(target) = ability::storyteller_picks_player(&mut read, me, &Def, question, candidates)
    {
        ability::demon_kill(ctx, me, &target);
    }
}
