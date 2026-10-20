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
    al_hadikhia,
    apprentice,
    balloonist,
    barista,
    baron,
    beggar,
    bishop,
    bone_collector,
    bureaucrat,
    butcher,
    butler,
    cerenovus,
    chambermaid,
    chef,
    clockmaker,
    dreamer,
    drunk,
    empath,
    fang_gu,
    fortune_teller,
    general,
    godfather,
    grandmother,
    harlot,
    imp,
    investigator,
    knaves,
    knight,
    legion,
    leviathan,
    librarian,
    lil_monsta,
    lleech,
    mathematician,
    matron,
    mayor,
    monk,
    no_dashii,
    noble,
    oracle,
    po,
    poisoner,
    pukka,
    ravenkeeper,
    recluse,
    saint,
    scarlet_woman,
    seamstress,
    sentinel,
    shabaloth,
    shugenja,
    slayer,
    soldier,
    spirit_of_ivory,
    spy,
    storm_catcher,
    undertaker,
    vigormortis,
    virgin,
    vortox,
    washerwoman,
    zombuul,
}

/// The character with this id. Official script ids have no underscores
/// (`fortuneteller`), so underscores are ignored when matching.
pub fn find(id: &str) -> Option<character::Ref> {
    let key = |s: &str| s.replace('_', "");
    all().iter().copied().find(|c| key(c.id()) == key(id))
}
