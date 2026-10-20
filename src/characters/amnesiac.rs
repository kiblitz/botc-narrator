use crate::ability;
use crate::character;
use crate::ctx;
use crate::game;
use crate::info;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str {
        "amnesiac"
    }

    fn name(&self) -> &'static str {
        "Amnesiac"
    }

    fn kind(&self) -> character::Kind {
        character::Kind::Townsfolk
    }

    /// The hidden ability acts at its own positions; these are the Amnesiac's.
    fn night_order(&self) -> character::NightOrder {
        character::NightOrder {
            first: Some(32),
            other: Some(47),
        }
    }

    /// The storyteller designs their ability: here, that of a not-in-play
    /// Townsfolk, which they have without being told. A token on the
    /// Amnesiac names it.
    fn on_setup(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let options: Vec<character::Ref> =
            ability::not_in_play(ctx.state(), character::Kind::Townsfolk)
                .into_iter()
                .filter(|c| !character::same(*c, &Def))
                .collect();
        let question = "which ability they have";
        if let Some(hidden) =
            ability::storyteller_picks_character(&mut ctx.read(), me, &Def, question, options)
        {
            ctx.gain_ability(me, hidden);
            let token = game::Effect::Custom(hidden.name());
            let reminder = game::Reminder::new(&Def, me, token, game::Expiry::Never);
            ctx.place_reminder(me, reminder);
        }
    }

    /// Each day, privately guesses their ability and learns how close it is.
    fn on_day_start(&self, ctx: &mut ctx::Write<'_>, me: &player::Id) {
        let state = ctx.state();
        let hidden = state.player(me).reminders.iter().find_map(|r| match r.effect {
            game::Effect::Custom(name) if r.source == character::Def::id(&Def) => Some(name),
            _ => None,
        });
        let Some(hidden) = hidden else { return };
        let options: Vec<character::Ref> = state
            .script()
            .iter()
            .copied()
            .filter(|c| c.kind() == character::Kind::Townsfolk && !character::same(*c, &Def))
            .collect();
        let prompt = "Which ability do you think you have?";
        let Some(guess) = ctx.ask_character(me, prompt, &options) else {
            return;
        };
        use info::Accuracy;
        let truthful = if guess.name() == hidden {
            vec![Accuracy::Bingo]
        } else {
            // How warm a wrong guess is, is the storyteller's judgement.
            vec![Accuracy::Cold, Accuracy::Warm, Accuracy::Hot]
        };
        let all = [
            Accuracy::Cold,
            Accuracy::Warm,
            Accuracy::Hot,
            Accuracy::Bingo,
        ];
        let message = |a: &Accuracy| info::Message::Accuracy(*a);
        ctx.inform(
            me,
            truthful.iter().map(message).collect(),
            all.iter().map(message).collect(),
        );
    }
}
