use crate::character;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "baron"
    }

    fn name(&self) -> &'static str {
        "Baron"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Minion
    }

    /// [+2 Outsiders]
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        vec![counts.shift_outsiders(2)]
    }
}
