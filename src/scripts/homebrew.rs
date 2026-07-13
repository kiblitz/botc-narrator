//! A small homebrew script, included to demonstrate that a new edition is pure
//! data over the existing registry. It reuses Trouble Brewing roles and adds the
//! [`Assassin`](crate::characters::Assassin) — no engine or character changes
//! were needed to introduce it.

use crate::characters::{
    Assassin, Butler, Empath, Imp, Monk, Recluse, Slayer, Soldier, Washerwoman,
};
use crate::script::Script;

#[must_use]
pub fn script() -> Script {
    Script {
        id: "homebrew_demo",
        name: "Homebrew Demo",
        roster: vec![
            Washerwoman::ID,
            Empath::ID,
            Slayer::ID,
            Soldier::ID,
            Monk::ID,
            Butler::ID,
            Recluse::ID,
            Assassin::ID,
            Imp::ID,
        ],
        first_night: vec![Washerwoman::ID, Empath::ID, Butler::ID],
        other_nights: vec![Monk::ID, Assassin::ID, Imp::ID, Empath::ID, Butler::ID],
    }
}
