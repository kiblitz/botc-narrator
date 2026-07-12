//! Character implementations for Trouble Brewing.
//!
//! Each character is a zero-sized behaviour struct implementing
//! [`crate::ability::Ability`]. The [`character!`] macro declares the struct and
//! its static [`CharacterInfo`](crate::role::CharacterInfo); every hook that
//! matters is then written out explicitly so the rules read top-to-bottom in
//! one file per role.

/// Declare a character struct with its static metadata.
///
/// ```ignore
/// character!(Empath, "empath", "Empath", Kind::Townsfolk, Alignment::Good);
/// ```
///
/// Resolved by textual scope in the character modules declared below.
macro_rules! character {
    ($ty:ident, $id:literal, $name:literal, $kind:expr, $align:expr) => {
        pub struct $ty;

        impl $ty {
            pub const INFO: $crate::role::CharacterInfo =
                $crate::role::CharacterInfo::new($id, $name, $kind, $align);
            pub const ID: $crate::ids::CharacterId = Self::INFO.id;
        }
    };
}

mod info_role;

pub mod baron;
pub mod butler;
pub mod chef;
pub mod drunk;
pub mod empath;
pub mod fortune_teller;
pub mod imp;
pub mod investigator;
pub mod librarian;
pub mod mayor;
pub mod monk;
pub mod poisoner;
pub mod ravenkeeper;
pub mod recluse;
pub mod saint;
pub mod scarlet_woman;
pub mod slayer;
pub mod soldier;
pub mod spy;
pub mod undertaker;
pub mod virgin;
pub mod washerwoman;

pub use baron::Baron;
pub use butler::Butler;
pub use chef::Chef;
pub use drunk::Drunk;
pub use empath::Empath;
pub use fortune_teller::FortuneTeller;
pub use imp::Imp;
pub use investigator::Investigator;
pub use librarian::Librarian;
pub use mayor::Mayor;
pub use monk::Monk;
pub use poisoner::Poisoner;
pub use ravenkeeper::Ravenkeeper;
pub use recluse::Recluse;
pub use saint::Saint;
pub use scarlet_woman::ScarletWoman;
pub use slayer::Slayer;
pub use soldier::Soldier;
pub use spy::Spy;
pub use undertaker::Undertaker;
pub use virgin::Virgin;
pub use washerwoman::Washerwoman;

use crate::registry::Registry;

/// A registry containing every Trouble Brewing character.
#[must_use]
pub fn registry() -> Registry {
    let mut reg = Registry::new();
    reg.register(Baron);
    reg.register(Butler);
    reg.register(Chef);
    reg.register(Drunk);
    reg.register(Empath);
    reg.register(FortuneTeller);
    reg.register(Imp);
    reg.register(Investigator);
    reg.register(Librarian);
    reg.register(Mayor);
    reg.register(Monk);
    reg.register(Poisoner);
    reg.register(Ravenkeeper);
    reg.register(Recluse);
    reg.register(Saint);
    reg.register(ScarletWoman);
    reg.register(Slayer);
    reg.register(Soldier);
    reg.register(Spy);
    reg.register(Undertaker);
    reg.register(Virgin);
    reg.register(Washerwoman);
    reg
}
