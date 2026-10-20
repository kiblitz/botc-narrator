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
}
