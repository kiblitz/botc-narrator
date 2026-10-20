use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

/// The token on the evil player the Bounty Hunter knows.
const KNOWN: game::Effect = game::Effect::Custom("Known");

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "bounty_hunter"
    }

    fn name(&self) -> &'static str {
        "Bounty Hunter"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(46),
            other: Some(64),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }

    /// Until they know someone, and again once the player they know is dead.
    fn wakes_tonight(&self, state: &game::State, me: &player::Id) -> bool {
        state.is_alive(me) && known(state, me).is_none_or(|p| !state.is_alive(&p))
    }

    /// [1 Townsfolk is evil]: the storyteller turns another Townsfolk evil,
    /// who learns so. Not for a Drunk who only believes they are the Bounty
    /// Hunter.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        if !character::same(ctx.state().character(me), &Def) {
            return;
        }
        let state = ctx.state();
        let townsfolk: Vec<player::Id> = ability::others(state, me)
            .into_iter()
            .filter(|p| state.kind(p) == character::Kind::Townsfolk && !state.is_evil(p))
            .collect();
        let question = "which Townsfolk is evil";
        let Some(evil) =
            ability::storyteller_picks_player(&mut ctx.read(), me, &Def, question, townsfolk)
        else {
            return;
        };
        ctx.set_alignment(&evil, character::Alignment::Evil);
        let you_are_evil = info::Message::IsAligned {
            player: evil.clone(),
            alignment: character::Alignment::Evil,
        };
        ctx.wake(&evil);
        ctx.tell(&evil, &you_are_evil);
        ctx.sleep(&evil);
    }
}

/// The player `me` knows now: their Known token.
fn known(state: &game::State, me: &player::Id) -> Option<player::Id> {
    state
        .players()
        .iter()
        .find(|p| ability::has_mark(state, me, &Def, &p.id, KNOWN))
        .map(|p| p.id.clone())
}

/// Learns an evil player (as they register) not known before, who gets the
/// Known token; when malfunctioning, possibly a good one.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let before = ability::ever_marked(ctx.state(), me, &Def, KNOWN);
    let candidates: Vec<player::Id> = ctx
        .state()
        .alive_except(me)
        .into_iter()
        .filter(|p| !before.contains(p))
        .collect();
    let mut evil = Vec::new();
    let mut good = Vec::new();
    for p in candidates {
        if ctx.registers_evil(me, &p) {
            evil.push(info::Message::Player(p));
        } else {
            good.push(info::Message::Player(p));
        }
    }
    ctx.wake(me);
    let told = ctx.inform(me, evil, good);
    ctx.sleep(me);
    if let Some(info::Message::Player(target)) = told {
        let reminder = game::Reminder::new(&Def, me, KNOWN, game::Expiry::Never);
        ctx.place_reminder(&target, reminder);
    }
}
