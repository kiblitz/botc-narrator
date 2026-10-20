use crate::ability;
use crate::character;
use crate::ctx;
use crate::event;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "plague_doctor"
    }

    fn name(&self) -> &'static str {
        "Plague Doctor"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// When the Plague Doctor dies, the storyteller gains a Minion ability.
    /// The storyteller holds no abilities here, so (as the official jinxes do
    /// for several Minions) a living Minion of the storyteller's choice gains
    /// it, and learns this.
    fn on_death(
        &self,
        ctx: &mut ctx::Write<'_>,
        me: &player::Id,
        dead: &player::Id,
        _cause: &event::Cause,
    ) {
        if dead != me || ctx.state().is_malfunctioning(me) {
            return;
        }
        let minions: Vec<character::Ref> = ctx
            .state()
            .script()
            .iter()
            .copied()
            .filter(|c| c.kind() == character::Kind::Minion)
            .collect();
        let holders = ctx.state().alive_of_kind(character::Kind::Minion);
        let mut read = ctx.read();
        let gained =
            ability::storyteller_picks_character(&mut read, me, &Def, "which Minion ability", minions);
        let holder = ability::storyteller_picks_player(
            &mut read,
            me,
            &Def,
            "which Minion gains the ability",
            holders,
        );
        let (Some(gained), Some(holder)) = (gained, holder) else {
            return;
        };
        ctx.gain_ability(&holder, gained);
        ctx.tell(&holder, &info::Message::Character(gained.name()));
    }
}
