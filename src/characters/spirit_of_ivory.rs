use crate::character;
use crate::game;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "spirit_of_ivory"
    }

    fn name(&self) -> &'static str {
        "Spirit of Ivory"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Fabled
    }

    /// There can't be more than 1 extra evil player: a good player may only
    /// turn evil while no player who started good is evil.
    fn permits_alignment_change(
        &self,
        state: &game::State,
        _target: &player::Id,
        to: character::Alignment,
    ) -> bool {
        to == character::Alignment::Good || extra_evil(state) == 0
    }
}

/// Players who are evil now but started the game good.
fn extra_evil(state: &game::State) -> usize {
    state
        .players()
        .iter()
        .filter(|p| {
            p.alignment == character::Alignment::Evil
                && p.starting_alignment == character::Alignment::Good
        })
        .count()
}
