use crate::character;
use crate::ctx;
use crate::event;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "al_hadikhia"
    }

    fn name(&self) -> &'static str {
        "Al-Hadikhia"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Demon
    }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: None,
            other: Some(51),
        }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadWrite(act))
    }
}

const CHOICES: usize = 3;

/// Each night*, may choose 3 players (everyone learns who): each silently
/// chooses to live (pointing at themself) or die (pointing at no-one); if
/// all 3 chose to live, all 3 die. A malfunctioning Al-Hadikhia's choices still
/// wake them but change nothing.
fn act(ctx: &mut ctx::Write<'_>, me: &player::Id, _night: u32) {
    let everyone = ctx.state().seat_order();
    ctx.wake(me);
    let chosen = ctx.ask_up_to(me, "Choose 3 players, if any", &everyone, CHOICES);
    ctx.sleep(me);
    if chosen.len() < CHOICES {
        return;
    }
    let names: Vec<&str> = chosen.iter().map(player::Id::as_str).collect();
    ctx.announce(format!("The Al-Hadikhia chose {}", names.join(", ")));
    let works = !ctx.state().is_malfunctioning(me);
    let mut all_live = true;
    for p in &chosen {
        ctx.wake(p);
        let prompt = "You were chosen: point at yourself to live, or at no-one to die";
        let lives = !ctx
            .ask_up_to(p, prompt, std::slice::from_ref(p), 1)
            .is_empty();
        ctx.sleep(p);
        all_live &= lives;
        if !works {
            continue;
        }
        if lives {
            ctx.revive(p);
        } else {
            ctx.kill(p, event::Cause::Demon);
        }
    }
    if works && all_live {
        for p in &chosen {
            ctx.kill(p, event::Cause::Demon);
        }
    }
}
