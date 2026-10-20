# botc-narrator

A Blood on the Clocktower storyteller engine in Rust, with the 22 Trouble Brewing characters.

## Design

The engine separates *rules* from *how the game is run*:

- **Characters** (`src/characters/`) describe their ability against an ability context. They never print or prompt directly.
- **The table** (`table::Io`) is how the storyteller talks to players: `wake`, `sleep`, `tell`, `ask`. Implement it for a console, a GUI, a networked app or a test recorder.
- **The storyteller** (`storyteller::Strategy`) makes every choice the rules leave to the storyteller's discretion.

### Storyteller decisions

Each discretionary choice is a `storyteller::Decision`:

- a typed `Reason` (information for a player, demon bluffs, starpass target, Mayor redirect),
- only **legal** `options`, so any answer is valid under the rules,
- the number of options to pick.

The strategy also gets a read-only view of the grimoire. Its answer is validated: an invalid answer falls back to the first options, and a decision with no real choice is not asked at all.

Strategies:

| Strategy | Use |
|---|---|
| `storyteller::Random` | uniform random legal choices (seedable) |
| `storyteller::claude::Storyteller` | asks Claude Code (`claude -p`, prompt on stdin) to choose, by default aiming for a balanced, interesting game; falls back to random if the CLI fails |
| `storyteller::Scripted` | queued picks for tests, then the first options |

### Information and misinformation

Info abilities don't handle poisoning themselves. They call `ctx::Read::inform(player, truthful, misinformation)`. The storyteller picks from the truthful messages, or, if the player is malfunctioning (poisoned or drunk), from the misinformation too. Options carry a `truthful` flag so a strategy knows which is which. Messages are typed (`info::Message`) rather than strings.

### Read vs write

Night actions are tagged `NightAction::ReadOnly` (gets `ctx::Read`, which cannot change the grimoire) or `NightAction::ReadWrite` (gets `ctx::Write`). The narrator groups consecutive read-only actions into a batch the table may run concurrently, and treats read-write actions as sync points.

### Rule interactions

`ctx::Write` resolves interactions centrally, so characters do not know about each other:

- `kill` with `Cause::Demon` checks Monk protection (a reminder), `safe_from_demon` (Soldier) and `redirect_demon_kill` (Mayor). A malfunctioning player's protection doesn't apply.
- `die` calls every living player's `on_death` hook, then the dead player's own (Scarlet Woman, Imp starpass), then checks win conditions.
- `execute` calls `on_executed` (Saint), then `die`.

### Grimoire state

`game::State` keeps players in seat order with **reminder tokens** (`game::Reminder { source, effect, expiry }`) instead of hardcoded flags. Poisoned, Drunk, Protected and AbilityUsed have rules meaning; `Effect::Custom` covers tokens only their character interprets (Butler's master). Placing a token moves it, like a physical token. `Expiry::Dusk` tokens are removed when the next night begins. The game keeps a typed `event::Event` history.

### Adding characters and scripts

A character is a unit struct implementing `character::Def`. It overrides only the hooks it uses (`night_action`, `day_action`, `on_setup`, `on_nominated`, `on_executed`, `on_death`, `safe_from_demon`, `redirect_demon_kill`). A script (`script::Script`) is a character list plus first-night and other-night orders. See `src/trouble_brewing.rs`.

### Not yet modelled

Drunk (thinks they are a Townsfolk), Recluse/Spy misregistration, Fortune Teller red herring, Butler voting restriction, and nomination/voting rules.

## Running

```sh
sh scripts/check.sh                  # fmt, clippy, tests (the merge gate)
cargo test -- --ignored              # also calls the real Claude Code CLI
cargo run -- random 42               # demo game, random storyteller, seed 42
cargo run -- claude                  # demo game with Claude Code as storyteller
```
