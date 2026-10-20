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
    alchemist,
    amnesiac,
    apprentice,
    artist,
    atheist,
    balloonist,
    barista,
    baron,
    beggar,
    bishop,
    bone_collector,
    bounty_hunter,
    bureaucrat,
    butcher,
    butler,
    cannibal,
    cerenovus,
    chambermaid,
    chef,
    clockmaker,
    courtier,
    dreamer,
    drunk,
    empath,
    engineer,
    exorcist,
    fang_gu,
    farmer,
    fisherman,
    fool,
    fortune_teller,
    gambler,
    general,
    godfather,
    gossip,
    grandmother,
    harlot,
    high_priestess,
    huntsman,
    imp,
    innkeeper,
    investigator,
    juggler,
    knaves,
    knight,
    legion,
    leviathan,
    librarian,
    lil_monsta,
    lleech,
    magician,
    mathematician,
    matron,
    mayor,
    minstrel,
    monk,
    no_dashii,
    noble,
    oracle,
    pacifist,
    philosopher,
    pixie,
    po,
    poisoner,
    poppy_grower,
    professor,
    pukka,
    ravenkeeper,
    recluse,
    sailor,
    saint,
    savant,
    scarlet_woman,
    seamstress,
    sentinel,
    shabaloth,
    shugenja,
    slayer,
    snake_charmer,
    soldier,
    spirit_of_ivory,
    spy,
    storm_catcher,
    tea_lady,
    town_crier,
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
