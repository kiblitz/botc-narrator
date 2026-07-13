//! Trouble Brewing: the roster and night wake orders.
//!
//! This is pure data — the entire edition is expressed as three lists of
//! character ids. Adding a whole new script is another file like this one; no
//! engine code changes.

use crate::characters::{
    Baron, Butler, Chef, Drunk, Empath, FortuneTeller, Imp, Investigator, Librarian, Mayor, Monk,
    Poisoner, Ravenkeeper, Recluse, Saint, ScarletWoman, Slayer, Soldier, Spy, Undertaker, Virgin,
    Washerwoman,
};
use crate::script::Script;

#[must_use]
pub fn script() -> Script {
    Script {
        id: "trouble_brewing",
        name: "Trouble Brewing",
        roster: vec![
            Washerwoman::ID,
            Librarian::ID,
            Investigator::ID,
            Chef::ID,
            Empath::ID,
            FortuneTeller::ID,
            Undertaker::ID,
            Monk::ID,
            Ravenkeeper::ID,
            Virgin::ID,
            Slayer::ID,
            Soldier::ID,
            Mayor::ID,
            Butler::ID,
            Drunk::ID,
            Recluse::ID,
            Saint::ID,
            Poisoner::ID,
            Spy::ID,
            Baron::ID,
            ScarletWoman::ID,
            Imp::ID,
        ],
        first_night: vec![
            Poisoner::ID,
            Washerwoman::ID,
            Librarian::ID,
            Investigator::ID,
            Chef::ID,
            Empath::ID,
            FortuneTeller::ID,
            Butler::ID,
            Spy::ID,
        ],
        other_nights: vec![
            Poisoner::ID,
            Monk::ID,
            Imp::ID,
            Ravenkeeper::ID,
            Undertaker::ID,
            Empath::ID,
            FortuneTeller::ID,
            Butler::ID,
            Spy::ID,
        ],
    }
}
