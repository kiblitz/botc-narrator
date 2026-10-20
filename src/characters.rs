//! Every character, one module per character, and the registry scripts
//! load them from.

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

use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

/// Every character the engine has, in one registry: scripts load their
/// characters from it by id.
pub fn all() -> &'static [character::Ref] {
    &[
        &baron::Def,
        &butler::Def,
        &chef::Def,
        &drunk::Def,
        &empath::Def,
        &fortune_teller::Def,
        &imp::Def,
        &investigator::Def,
        &librarian::Def,
        &mayor::Def,
        &monk::Def,
        &poisoner::Def,
        &ravenkeeper::Def,
        &recluse::Def,
        &saint::Def,
        &scarlet_woman::Def,
        &slayer::Def,
        &soldier::Def,
        &spy::Def,
        &undertaker::Def,
        &virgin::Def,
        &washerwoman::Def,
    ]
}

/// The character with this id. Official script ids have no underscores
/// (`fortuneteller`), so underscores are ignored when matching.
pub fn find(id: &str) -> Option<character::Ref> {
    let key = |s: &str| s.replace('_', "");
    all().iter().copied().find(|c| key(c.id()) == key(id))
}

/// Washerwoman-style information: "one of these two players is the X", where
/// X is a `kind` character (as they register to `me`). Shown in both seat
/// orders so the storyteller also picks the order. Misinformation is any two
/// other players and any in-play character of that kind.
pub(crate) fn inform_one_of(ctx: &mut ctx::Read<'_>, me: &player::Id, kind: character::Kind) {
    let others: Vec<player::Id> = ctx
        .state()
        .seat_order()
        .into_iter()
        .filter(|p| p != me)
        .collect();
    let registered: Vec<(player::Id, character::Ref)> = others
        .iter()
        .filter_map(|p| ctx.registers_as(me, p, kind).map(|c| (p.clone(), c)))
        .collect();
    let state = ctx.state();
    let pairs_with = |a: &player::Id| -> Vec<[player::Id; 2]> {
        others
            .iter()
            .filter(|b| *b != a)
            .flat_map(|b| [[a.clone(), b.clone()], [b.clone(), a.clone()]])
            .collect()
    };
    let mut truthful = Vec::new();
    for (target, character) in &registered {
        let character = character.name();
        for players in pairs_with(target) {
            truthful.push(info::Message::OneOf { players, character });
        }
    }
    let none = info::Message::Count {
        count: 0,
        noun: noun(kind),
    };
    if truthful.is_empty() {
        truthful.push(none.clone());
    }
    let mut misinformation = vec![none];
    let names: Vec<&'static str> = state
        .players()
        .iter()
        .filter(|p| p.character.kind() == kind)
        .map(|p| p.character.name())
        .collect();
    for (i, a) in others.iter().enumerate() {
        for b in &others[i + 1..] {
            for &character in &names {
                misinformation.push(info::Message::OneOf {
                    players: [a.clone(), b.clone()],
                    character,
                });
            }
        }
    }
    ctx.wake(me);
    ctx.inform(me, truthful, misinformation);
    ctx.sleep(me);
}

fn noun(kind: character::Kind) -> &'static str {
    match kind {
        character::Kind::Townsfolk => "townsfolk",
        character::Kind::Outsider => "outsiders",
        character::Kind::Minion => "minions",
        character::Kind::Demon => "demons",
        character::Kind::Traveller => "travellers",
        character::Kind::Fabled => "fabled",
    }
}

/// Misinformation for "learn a character" abilities: any in-play character.
pub(crate) fn any_character(state: &game::State) -> Vec<info::Message> {
    state
        .character_names()
        .into_iter()
        .map(info::Message::Character)
        .collect()
}

pub(crate) fn counts(
    range: std::ops::RangeInclusive<usize>,
    noun: &'static str,
) -> Vec<info::Message> {
    range
        .map(|count| info::Message::Count { count, noun })
        .collect()
}
