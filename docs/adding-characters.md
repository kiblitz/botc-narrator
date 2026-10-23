# Adding a character

A character is **one file** `src/characters/<id>.rs` plus **one line** in the
`registry!` list in `src/characters.rs`. No engine module (`ctx`, `game`,
`narrator`, `sim`, `storyteller`) changes. If an ability family really needs
engine support, add a general mechanism (a hook, a generic block), never a
special case for one character.

Ids are the official script ids with underscores allowed (`fang_gu` loads
from `fanggu`). Rules come from the official wiki; night order numbers from
`data/characters/<id>.json` (or the clocktower.online numbering).

## The shape

```rust
use crate::ability;
use crate::character;
use crate::ctx;
use crate::player;

pub struct Def;

impl character::Def for Def {
    fn id(&self) -> &'static str { "noble" }
    fn name(&self) -> &'static str { "Noble" }
    fn kind(&self) -> character::Kind { character::Kind::Townsfolk }

    fn night_order(&self) -> character::NightOrder {
        character::NightOrder { first: Some(44), other: None }
    }

    fn night_action(&self) -> Option<character::NightAction> {
        Some(character::NightAction::ReadOnly(act))
    }
}

fn act(ctx: &mut ctx::Read<'_>, me: &player::Id, _night: u32) { ... }
```

Override only the hooks the ability uses (the full list is in the README,
"Adding characters and scripts"). Use `ReadOnly` for pure information
(they can run in parallel) and `ReadWrite` when the grimoire changes.

## Rules the engine already handles

- **Who wakes**: every player with the ability, at its night-order position,
  if `wakes_tonight` (default: alive). Override it for "the night you die"
  (Ravenkeeper) or "if an Outsider died today" (Godfather).
- **Malfunctioning** (poisoned, drunk, or believing a character they are not):
  information goes through `ctx.inform`, which offers misinformation
  automatically; effects in `ability::mark` / `ability::kill` do nothing. Check
  `ctx.state().is_malfunctioning(me)` for anything else.
- **Registration**: ask `ctx.registers_evil`, `ctx.registers_as(me, p, kind)`,
  `ctx.registered_character`; never read another player's alignment or
  character from the grimoire for information.
- **Death**: `ctx.kill(target, cause)` applies protection and every
  `intercept_death`; execution goes through it too.

## Building blocks (`ability`)

| Need | Block |
|---|---|
| learn a number / yes-no / a character | `tell_count`, `tell_yes_no`, `tell_character` |
| "one of these two is the X" | `tell_one_of(ctx, me, kind)` |
| learn a player / a player's character / a direction / who is winning | `info::Message::Player`, `Is`, `Direction`, `Winning` with `ctx.inform` (it returns what was told) |
| good or evil characters information may name | `nameable(state, kinds)` (the script's, or in play) |
| "woke tonight due to their ability" | `wakes_for_ability(state, p, night)` |
| how many are evil | `count_evil`, `alive_neighbours`, `groups` |
| choose players / a character | `ctx.ask`, `ctx.ask_one`, `ctx.ask_character` |
| protect / poison / any token | `mark(ctx, me, &Def, target, effect, expiry)`, `mark_all` (several players), `has_mark`, `marked` |
| once per game | `use_once(ctx, me, &Def)`, `used`, `used_tonight` |
| take a token off | `ctx.remove_reminder(target, &reminder)` |
| kill | `kill(ctx, me, target)`; the demon's attack: `demon_attack`; a demon kill of a given player: `demon_kill`; "even if they could not": `ctx.kill_unstoppably` |
| "you may choose" (no-one is legal) | `ctx.ask_up_to` |
| several identical tokens (No Dashii's two Poisoned) | `sync_marks`, `marked` |
| nearest neighbours of a kind, alive or dead (No Dashii, Vigormortis) | `nearest_of_kind` |
| bring a dead player back | `ctx.revive` |
| tell everyone something | `ctx.announce` |
| a dead player keeps their ability | a `game::Effect::KeepsAbility` token (lasts while its placer lives and functions) |
| "today" | `today` (the day's events), `died_today` |
| longer-lasting tokens | `game::Expiry::Night(n)` (until night `n`), `Expiry::Death` (while the placer's ability lasts) |
| keep a player asleep tonight | `game::Effect::DoesNotWake` (Exorcist) |
| everyone an ability ever marked | `ever_marked` |
| storyteller choices | `storyteller_agrees`, `storyteller_picks_player`, `storyteller_picks_character` |
| setup modifier | `setup_counts` with `setup::Counts::shift_outsiders` / `with` |
| not-in-play characters, the script | `not_in_play`, `ctx.state().script()` |
| madness | `game::Effect::Mad(name)` + `on_madness_broken` |
| "died today", "nominated today" | `died_today`, `nominators_today` |
| a public claim ("X is the Y"), read back that night | `claim`, `claims_today` (tagged with the ability that claimed) |
| what the player was told | `ctx.inform` returns the chosen message |
| "[+the Damsel]" | hook `requires` |
| "if you nominated" (with the nominee) | `nominations_today` |
| a player's yes/no choice or claim | `ctx.confirm(player, &table::Question::...)` |
| see the grimoire | `grimoire(state)` |
| "until you die" | `game::Expiry::Death` |
| react to any nomination / execution, delay a win | hooks `on_nomination`, `on_execution`, `vetoes_win` |
| becomes another character / alignment | `ctx.transform`, `ctx.set_alignment`, `ctx.set_believed` |
| gains or loses an ability | `ctx.gain_ability`, `ctx.lose_ability` (acts the same night if its turn is still to come) |
| alternate demon / forced false info | hooks `counts_as_demon`, `forces_false_info` |
| evil info / no Demon in the game | hooks `joins_evil_info` (Magician), `hides_evil_info` + `narrator::evil_info` (Poppy Grower), `demonless` (Atheist) |
| the Storyteller can be nominated and executed | hooks `storyteller_nominable`, `on_storyteller_executed` (Atheist: decides the winner) |
| survive a death | hook `intercept_death` returning `Prevent` (Fool, Sailor, Tea Lady, Pacifist, Innkeeper); the dying player's own abilities are asked last |
| day ability | hook `day_action` (gets its targets from `day_targets`, chosen first); `narrator::day_actions` offers it (to the server and the simulator) while unused; `day_action_users` lets others use it (Damsel guess); `claim` lets any living player claim it, in public (`Visibility::Public`, announced) or as a private visit (`Visibility::Private`: the narrator wakes the player around it, so do not wake them yourself); the bluff asks the same remaining questions (those after the targets), with no effect (share them in one helper, like the Juggler's `guesses`); an empty bluff is right exactly when the ability asks nothing after its targets (Fisherman, Psychopath) |
| may make a player mad | hook `causes_madness` (players can then report breaking it) |
| ask a player yes/no | `ctx.ask_yes_no` (the Harlot's visit) |
| sober / acts twice / restored / execution-only | `game::Effect::Sober`, `ActsTwice`, `AbilityRestored`, `ExecutionOnly` with `mark`; hook `always_sober` |
| votes and nominations | hooks `vote_weight`, `storyteller_nominations`, `nominates_again`; `ctx.remove_reminder`, `ctx.spend_ghost_vote` |
| seats | `ctx.swap_seats` |
| Fabled and Loric | hooks `fabled_setup`, `fabled_dusk`, `permits_alignment_change`, `lies_to`; storyteller tokens `ctx.mark_fabled` / `State::fabled_has`; `storyteller::Reason::Fabled` |
| nomination limits | hook `can_nominate` (Golem) |
| "when you learn that you died" | hook `on_learns_death` (Klutz, Moonchild) |
| "the first player to choose you" / whom a player chose | hooks `on_chosen` (Goon) / `on_choose` (Lunatic) |
| end-of-game judgement | hook `on_win` (Politician) |
| judged claims | `ctx.is_mad_about(player, about)` (Mutant) |
| a token for a later night | `ctx.place_reminder` + `wakes_tonight` checking it, then `ctx.remove_reminder` (Moonchild, Barber) |
| believes they are good (Marionette) | `ctx.set_believed` in `on_setup`; the evil team info skips them; hook `seated_next_to_demon` |

## Worked examples

- **Noble** (`noble.rs`): registration-aware evil per player, `groups(&others, 3)`
  split into truthful (exactly one evil) and misinformation, `ctx.inform`.
- **Godfather** (`godfather.rs`): `setup_counts` with two options (-1 / +1
  Outsider), first-night `info::Message::Characters`, `wakes_tonight` using
  `died_today`, then `ability::kill`.
- **Cerenovus** (`cerenovus.rs`): `ctx.ask_character` from the script's good
  characters, `mark` with `Effect::Mad`, waking the target to tell them, and
  `on_madness_broken` asking `storyteller_agrees` before `ctx.execute`.
- **Fang Gu** (`fang_gu.rs`): `setup_counts` (+1 Outsider), `registers_as`
  Outsider, `use_once`, `transform` + `set_alignment`, then killing itself.
- Demons: **Pukka** (`pukka.rs`, `marked` + `sync_marks`: the poisoned player dies next night),
  **No Dashii** (`no_dashii.rs`, `sync_marks` on `nearest_of_kind`
  Townsfolk at every phase change), **Po** (`ask_up_to` and a charge token),
  **Legion** (`duplicates_in_setup`, one voice for the group), **Lil' Monsta**
  (a carried ability: `transform` + `gain_ability`).
- Travellers, Fabled and Loric (`bureaucrat.rs`, `harlot.rs`, `knaves.rs`, ...):
  tests in `tests/travellers.rs`.
- Trouble Brewing (`imp.rs`, `monk.rs`, `empath.rs`, ...) are a few lines each.

## Tests

Every character gets an expect-test scenario in `tests/` showing the ability,
and the poisoned/drunk case where it matters (see `tests/new_characters.rs`).
Build a script with `script::Script::new` when the character is not on a
registered script yet. Update expectations with `UPDATE_EXPECT=1 cargo test`
and read the diff.
