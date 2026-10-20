use crate::character;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "recluse"
    }

    fn name(&self) -> &'static str {
        "Recluse"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }

    /// Might register as evil and as a Minion or Demon, even if dead.
    fn misregistration(&self) -> Option<character::Misregistration> {
        Some(character::Misregistration {
            alignment: character::Alignment::Evil,
            kinds: &[character::Kind::Minion, character::Kind::Demon],
        })
    }
}
