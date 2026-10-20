use crate::character;
use crate::setup;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "sentinel"
    }

    fn name(&self) -> &'static str {
        "Sentinel"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Fabled
    }

    /// There might be 1 extra or 1 fewer Outsider in play.
    fn setup_counts(&self, counts: setup::Counts) -> Vec<setup::Counts> {
        let mut options = vec![counts, counts.shift_outsiders(1)];
        if counts.outsiders > 0 {
            options.push(counts.shift_outsiders(-1));
        }
        options
    }
}
