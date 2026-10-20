//! Every character, one module per character, and the registry scripts load
//! them from. Adding a character is its module plus one line here.

use crate::character;

/// Declares each character module and lists its `Def` in [`all`].
macro_rules! registry {
    ($($id:ident),* $(,)?) => {
        $(pub mod $id;)*

        /// Every character the engine has: scripts load their characters
        /// from it by id.
        pub fn all() -> &'static [character::Ref] {
            &[$(&$id::Def),*]
        }
    };
}

registry! {
    baron,
    butler,
    cerenovus,
    chef,
    drunk,
    empath,
    fang_gu,
    fortune_teller,
    godfather,
    imp,
    investigator,
    librarian,
    mayor,
    monk,
    noble,
    poisoner,
    ravenkeeper,
    recluse,
    saint,
    scarlet_woman,
    slayer,
    soldier,
    spy,
    undertaker,
    virgin,
    washerwoman,
}

/// The character with this id. Official script ids have no underscores
/// (`fortuneteller`), so underscores are ignored when matching.
pub fn find(id: &str) -> Option<character::Ref> {
    let key = |s: &str| s.replace('_', "");
    all().iter().copied().find(|c| key(c.id()) == key(id))
}
