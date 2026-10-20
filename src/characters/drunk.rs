use crate::character;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "drunk"
    }

    fn name(&self) -> &'static str {
        "Drunk"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Outsider
    }
}
