//! Shared logic for the "learn that one of these two players is the X" roles:
//! Washerwoman (Townsfolk), Librarian (Outsider), Investigator (Minion).
//!
//! The storyteller picks the genuine player (which may be a misregistering
//! Recluse or Spy), the character to name, and a decoy. An impaired role's
//! information is entirely the storyteller's to invent.

use crate::ctx::Ctx;
use crate::ids::PlayerId;
use crate::role::Kind;

pub fn learn_one_of_two(ctx: &mut Ctx, me: PlayerId, kind: Kind, noun: &str) {
    ctx.wake(me);
    let others = ctx.alive_others(me);
    if others.len() < 2 {
        // Degenerate board; nothing meaningful to reveal.
        ctx.sleep(me);
        return;
    }
    let impaired = ctx.is_impaired(me);

    // The genuine player: one who can register as the wanted kind. An impaired
    // role may be shown any player.
    let real_pool: Vec<PlayerId> = if impaired {
        others.clone()
    } else {
        let pool: Vec<PlayerId> = others
            .iter()
            .copied()
            .filter(|&p| ctx.could_register_kind(p, kind))
            .collect();
        if pool.is_empty() {
            others.clone()
        } else {
            pool
        }
    };
    let real_labels: Vec<String> = real_pool.iter().map(|&id| ctx.label(id)).collect();
    let real = real_pool[ctx.choose(&format!("{noun}: the genuine player"), &real_labels)];

    // The character to name.
    let character_name = if impaired {
        let names = ctx.roster_character_names();
        names[ctx.choose(&format!("{noun}: character shown"), &names)].clone()
    } else {
        let cid = ctx.registers_as_character(real, noun);
        ctx.registry().info(cid).name.to_string()
    };

    // The decoy: any other player.
    let decoys: Vec<PlayerId> = others.into_iter().filter(|&p| p != real).collect();
    let decoy_labels: Vec<String> = decoys.iter().map(|&id| ctx.label(id)).collect();
    let decoy = decoys[ctx.choose(&format!("{noun}: the decoy player"), &decoy_labels)];

    let a = ctx.name(real);
    let b = ctx.name(decoy);
    ctx.reveal(me, &format!("One of {a} and {b} is the {character_name}"));
    ctx.sleep(me);
}
