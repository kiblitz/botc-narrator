use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::narrator;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "lunatic"
    }

    fn name(&self) -> &'static str {
        "Lunatic"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(8),
            other: Some(20),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Thinks they are a Demon on the script (of the storyteller's choice):
    /// they wake and choose as it, before the real Demon, without effect.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let demons: Vec<character::Ref> = ctx
            .state()
            .script()
            .iter()
            .copied()
            .filter(|c| c.kind() == character::Kind::Demon)
            .collect();
        let question = "which Demon they believe they are";
        if let Some(believed) =
            ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, demons)
        {
            ctx.set_believed(me, Some(believed));
        }
    }

    /// The Demon learns whom the Lunatic chose; if the Lunatic is poisoned or
    /// drunk, the storyteller may show other players.
    fn on_choose(&self, ctx: &mut ctx::Write<'_>, me: &player::Id, chosen: &[player::Id]) {
        let state = ctx.state();
        let impaired = state.has_effect(me, game::Effect::Poisoned)
            || state.has_effect(me, game::Effect::Drunk);
        let demons = state.demons();
        let mut options = vec![(chosen.to_vec(), true)];
        if impaired {
            let alive = state.alive_ids();
            options.extend(
                ability::groups(&alive, chosen.len())
                    .into_iter()
                    .filter(|g| g.as_slice() != chosen)
                    .map(|g| (g, false)),
            );
        }
        for demon in demons {
            let reason = storyteller::Reason::Ability {
                player: me.clone(),
                character: character::Def::name(&Def),
                question: "whom the Demon is shown the Lunatic chose",
            };
            let shown = ctx
                .choose(reason, options.clone(), 1, |(players, truthful)| {
                    storyteller::Choice::Info {
                        message: info::Message::Players(players.clone()),
                        truthful: *truthful,
                    }
                })
                .into_iter()
                .next()
                .map_or_else(|| chosen.to_vec(), |(players, _)| players);
            ctx.wake(&demon);
            ctx.tell(&demon, &info::Message::Grimoire(vec![(me.clone(), character::Def::name(&Def))]));
            ctx.tell(&demon, &info::Message::Players(shown));
            ctx.sleep(&demon);
        }
    }
}

/// The first night: the Lunatic gets Demon info as if they were the Demon
/// (as many "Minions" as there are, any players, and three good bluffs, which
/// may be in play), and the real Demon learns who the Lunatic is. Later
/// nights they act at their believed Demon's turn.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, night: u32) {
    if night != 1 {
        return;
    }
    if ctx.state().resident_count() >= narrator::EVIL_INFO_MIN_PLAYERS {
        let minions = ctx.state().alive_of_kind(character::Kind::Minion).len();
        let mut candidates = ability::others(ctx.state(), me);
        let mut team = vec![me.clone()];
        for _ in 0..minions {
            let question = "who the Lunatic is shown as a Minion";
            let Some(pick) = ability::storyteller_picks_player(
                &mut ctx.read(),
                me,
                &Def,
                question,
                candidates.clone(),
            ) else {
                break;
            };
            candidates.retain(|p| *p != pick);
            team.push(pick);
        }
        let seats = ctx.state().seat_order();
        team.sort_by_key(|p| seats.iter().position(|s| s == p));
        let good: Vec<&'static str> = ctx
            .state()
            .script()
            .iter()
            .filter(|c| c.kind().alignment() == Some(character::Alignment::Good))
            .map(|c| c.name())
            .collect();
        let bluffs = ctx.choose(storyteller::Reason::DemonBluffs, good, 3, |name| {
            storyteller::Choice::Character(name)
        });
        ctx.wake(me);
        ctx.tell(me, &info::Message::Team { evil: team });
        ctx.tell(me, &info::Message::Bluffs(bluffs));
        ctx.sleep(me);
    }
    for demon in ctx.state().demons() {
        ctx.wake(&demon);
        ctx.tell(&demon, &info::Message::Grimoire(vec![(me.clone(), character::Def::name(&Def))]));
        ctx.sleep(&demon);
    }
}
