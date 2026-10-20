use crate::character;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "magician"
    }

    fn name(&self) -> &'static str {
        "Magician"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(5),
            other: None,
        }
    }

    /// The Demon thinks they are a Minion and the Minions think they are a
    /// Demon: they appear in the evil team both are shown.
    fn joins_evil_info(&self) -> bool {
        true
    }
}
