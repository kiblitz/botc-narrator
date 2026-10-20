use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;
use crate::setup;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "lil_monsta"
    }

    fn name(&self) -> &'static str {
        "Lil' Monsta"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(30),
            other: Some(53),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// [+1 Minion]: no player is the Lil' Monsta. The player dealt it becomes
    /// a not-in-play Minion (one that does not change the setup counts) and
    /// babysits it until the Minions first choose.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let counts = setup::Counts::base(ctx.state().resident_count());
        let minions: Vec<character::Ref> = ability::not_in_play(ctx.state(), character::Kind::Minion)
            .into_iter()
            .filter(|m| m.setup_counts(counts) == vec![counts])
            .collect();
        let question = "which Minion does the player dealt Lil' Monsta become?";
        let mut read = ctx.read();
        let Some(minion) = ability::storyteller_picks_character(&mut read, me, &Def, question, minions)
        else {
            return;
        };
        ctx.transform(me, minion);
        ctx.gain_ability(me, &Def);
    }
}

/// Each night the Minions choose who babysits (and "is the Demon"); the
/// storyteller settles disagreements. Each night*, a player other than the
/// babysitter might die. The holder of this ability is the babysitter.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    let minions = ctx.state().alive_of_kind(character::Kind::Minion);
    let mut picks: Vec<player::Id> = Vec::new();
    for minion in &minions {
        ctx.wake(minion);
        let pick = ctx.ask_one(minion, "Who babysits Lil' Monsta?", &minions);
        ctx.sleep(minion);
        if let Some(pick) = pick
            && !picks.contains(&pick)
        {
            picks.push(pick);
        }
    }
    let reason = storyteller::Reason::Ability {
        player: me.clone(),
        character: character::Def::name(&Def),
        question: "the Minions disagree: who babysits?",
    };
    let babysitter = ctx.choose_player(reason, picks).unwrap_or_else(|| me.clone());
    if &babysitter != me {
        ctx.lose_ability(me, &Def);
        ctx.gain_ability(&babysitter, &Def);
    }
    if night == 1 {
        return;
    }
    let mut read = ctx.read();
    if !ability::storyteller_agrees(&mut read, &babysitter, &Def, "does a player die tonight?") {
        return;
    }
    let candidates = read.state().alive_except(&babysitter);
    let question = "who dies tonight?";
    if let Some(target) =
        ability::storyteller_picks_player(&mut read, &babysitter, &Def, question, candidates)
    {
        ctx.kill(&target, event::Cause::Demon);
    }
}
