use crate::character;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "butcher"
    }

    fn name(&self) -> &'static str {
        "Butcher"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Traveller
    }

    /// Each day, after the first execution, may nominate again.
    fn nominates_again(&self) -> bool {
        true
    }
}
