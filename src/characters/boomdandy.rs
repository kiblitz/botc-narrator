use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::player;
use crate::storyteller;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "boomdandy"
    }

    fn name(&self) -> &'static str {
        "Boomdandy"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// If executed (even if they survive), all but 3 players die; then
    /// everyone points, and the player with the most fingers dies.
    fn on_executed(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        if ctx.state().is_malfunctioning(me) {
            return;
        }
        explode(ctx, me);
        if ctx.state().winner().is_none() {
            point(ctx, me);
        }
    }
}

/// Every living player but 3 dies: the Demons survive (all of them, in the
/// rare case of more than 3) and the storyteller
/// picks the rest. The executed Boomdandy is not one of the 3.
fn explode(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    let demons = ctx.state().demons();
    let (mut survivors, rest): (Vec<player::Id>, Vec<player::Id>) = ctx
        .state()
        .alive_except(me)
        .into_iter()
        .partition(|p| demons.contains(p));
    let reason = ability::reason(me, &Def, "who survives the explosion?");
    let picks = 3usize.saturating_sub(survivors.len());
    survivors.extend(ctx.choose(reason, rest.clone(), picks, |p| {
        storyteller::Choice::Player(p.clone())
    }));
    ctx.announce(format!("{me} explodes"));
    for p in rest.iter().filter(|p| !survivors.contains(p)) {
        ctx.kill(p, event::Cause::Ability { by: me.clone() });
    }
}

/// After a 10 to 1 countdown every player, dead or alive, points at a living
/// player; the one with the most fingers dies (nobody on a tie).
fn point(ctx: &mut ctx::Write<'_>, me: &player::Id) {
    let candidates = ctx.state().alive_except(me);
    let mut fingers: Vec<(player::Id, usize)> =
        candidates.iter().map(|p| (p.clone(), 0)).collect();
    for pointer in ctx.state().seat_order() {
        if let Some(target) = ctx.ask_one(&pointer, "Who do you point at?", &candidates)
            && let Some((_, count)) = fingers.iter_mut().find(|(p, _)| *p == target)
        {
            *count += 1;
        }
    }
    let most = fingers.iter().map(|(_, n)| *n).max().unwrap_or(0);
    let top: Vec<player::Id> = fingers
        .into_iter()
        .filter(|(_, n)| *n == most)
        .map(|(p, _)| p)
        .collect();
    if let [target] = &top[..] {
        ctx.kill(target, event::Cause::Ability { by: me.clone() });
    }
}
