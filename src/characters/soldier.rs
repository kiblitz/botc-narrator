use crate::character;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "soldier"
    }

    fn name(&self) -> &'static str {
        "Soldier"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    fn safe_from_demon(&self) -> bool {
        true
    }
}
