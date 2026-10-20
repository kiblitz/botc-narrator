# botc-narrator

A Blood on the Clocktower storyteller engine in Rust, with the 22 Trouble Brewing characters and more being added (see [docs/adding-characters.md](docs/adding-characters.md)).

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

### Registration

Info abilities never read other players' alignment or character from the grimoire directly: they ask `ctx::Read::registers_evil` / `registered_alignment`, `registered_character` and `registers_as(asker, target, kind)`. A character that may register falsely says so with `character::Def::misregistration` (the Recluse as an evil Minion or Demon, the Spy as a good Townsfolk or Outsider). The storyteller decides each query (`Reason::Registration`, the truthful answer first), and is only asked when a misregistration would change the answer; a malfunctioning character registers truly. The Fortune Teller's red herring is a reminder token chosen at setup.

### Read vs write

Night actions are tagged `NightAction::ReadOnly` (gets `ctx::Read`, which cannot change the grimoire) or `NightAction::ReadWrite` (gets `ctx::Write`). The narrator groups consecutive read-only actions into a batch and announces it to the table (`batch_started` / `batch_ended`), since those actions are independent; read-write actions are sync points. Batches currently run one after another.

### Rule interactions

`ctx::Write` resolves interactions centrally, so characters do not know about each other:

- `kill` checks Monk protection (a reminder) for demon kills, then asks every functioning living player's abilities (`intercept_death`) in seat order whether to let the death happen, prevent it (Soldier) or redirect it (Mayor; never onto the demon). The same applies to executions and any other cause.
- Minions and demon learn each other on the first night only with 7 or more players (Travellers do not count and are not shown).
- `die` calls every living player's `on_death` hook, then the dead player's own (Scarlet Woman, Imp starpass), then checks win conditions.
- `execute` calls `on_executed` (Saint), then `kill` with `Cause::Execution`.

### Grimoire state

`game::State` keeps players in seat order with **reminder tokens** (`game::Reminder { source, effect, expiry }`) instead of hardcoded flags. Poisoned, Drunk, Protected and AbilityUsed have rules meaning; `Effect::Mad(character)` marks a player mad about being a character; `Effect::Custom` covers tokens only their character interprets (Butler's master, Fortune Teller's red herring). Placing a token moves it, like a physical token. `Expiry::Dusk` tokens are removed when the next night begins, or when the player whose ability placed them dies. The game keeps a typed `event::Event` history.

Each player also has:

- a **believed character** (`Player::believes`): what they were told they are, when it is not their real character. The Drunk is told a not-in-play Townsfolk at setup; they wake and act as it, but a player who believes a character they are not is malfunctioning, so its ability never works and their information may be false.
- **abilities** (`State::abilities`): their own character's (unless lost), the believed character's and any gained ones (`ctx::Write::gain_ability` / `lose_ability`). Night actions and every player hook run for each ability, so a gained ability wakes in its own place in the night order.
- an **alignment** set at seating (`game::Seat`): the character's, or chosen for a Traveller.

Character kinds are Townsfolk, Outsider, Minion, Demon, **Traveller** (seated, chosen alignment, outside the setup counts) and **Fabled** (storyteller-side: `ctx::Write::add_fabled`, never seated).

### Adding characters and scripts

A character is a unit struct implementing `character::Def` in its own module (`src/characters/<id>.rs`) plus one line in the `registry!` list in `src/characters.rs`; abilities are mostly composed from the building blocks in `ability` (tell a count / yes-no / character / "one of two", count registered evil, mark a player, use once, kill, the demon attack, storyteller picks). **See [docs/adding-characters.md](docs/adding-characters.md).** It overrides only the hooks it uses:

| Hook | When |
|---|---|
| `night_order` | its official first-night / other-nights positions (clocktower.online numbering) |
| `night_action`, `wakes_tonight` | what it does at its turn; whether it wakes tonight (default: alive), checked when the turn comes |
| `day_action` | a public day ability (Slayer) |
| `setup_counts`, `duplicates_in_setup` | how it changes the setup counts (Baron: +2 Outsiders; options for "+0 or +1"); copies filling extra slots (Legion, Riot) |
| `on_setup` | storyteller setup choices before the first night (Drunk, Fortune Teller red herring); `game::State::script` is set by then |
| `on_night_start`, `on_day_start`, `on_day_end` | phase changes, for living players |
| `on_nominated`, `on_nominates`, `on_vote` | the nominee, the nominator, everyone after a vote |
| `on_executed`, `intercept_death`, `on_death` | executions, preventing or redirecting any death, reacting to deaths |
| `on_madness_broken` | a player this ability made mad broke madness (`narrator::check_madness`, judged by the table) |
| `misregistration` | how it may register falsely |

Extra deaths at other times are plain `ctx::Write::kill` calls from any hook.

A script is a JSON file in `data/scripts/<id>.json` in the standard script format (a `_meta` entry with the name, then official character ids, as strings or `{"id"}` objects), listed in `script::SCRIPTS`. Official ids are matched to `Def::id` ignoring underscores (`fortuneteller` is `fortune_teller`); unknown ids are an error listing them. Night orders are computed by sorting the script's characters by `night_order`; the minion and demon info sit at positions 6 and 9 of the first night.

`setup::choose` builds a legal set of characters for a player count: it picks characters at random and re-applies every chosen character's `setup_counts` until the picks match, filling kinds that run out with Townsfolk.

### Not yet modelled

Butler voting restriction, nomination/voting rules, and reminder tokens in the Spy's grimoire view. Misinformation for "one of these two is the X", and misregistration, only offer characters in play, not the whole script. The Saint's loss fires on execution before any ability can prevent the death. The simulator's player view shows a Drunk their real character (it does not read `believes` yet). The Fang Gu's jump only notices Monk protection (other ways an Outsider would survive still let it jump). A Cerenovus execution is judged once, at the end of the day.

## Simulator site

`site/` is a static page that generates random, plausible Trouble Brewing games in the browser (the engine compiled to WebAssembly from `web/`) and lets you step through them. Players sit in a circle; each step shows who is awake, what they were told or asked, storyteller decisions and public events. Click a player to see everything they know so far, or switch the view to one player's point of view to hide what they couldn't see. Pick a script (only Trouble Brewing so far; new scripts in `script::all()` appear automatically). Games are reproducible from `#script=...&seed=...&players=...` in the URL. Information steps highlight the players and characters they name.

Generation (`sim.rs`) uses legal setups for 5-15 players (`setup::choose`), players who act on simple heuristics (evil targets good), a random storyteller, daily nominations with a majority vote (no ghost votes), mad players who break madness with a fixed probability, and night deaths announced at dawn. Day actions such as the Slayer's shot are public.

```sh
sh scripts/serve-web.sh              # build the wasm and serve at http://localhost:8080
```

Building needs `wasm-pack` and the `wasm32-unknown-unknown` target; the Playwright tests (`site/tests`) run in `scripts/check.sh`.

## Running

```sh
sh scripts/check.sh                  # fmt, clippy, tests (the merge gate)
cargo test -- --ignored              # also calls the real Claude Code CLI
cargo run -- random 42               # demo game, random storyteller, seed 42
cargo run -- claude                  # demo game with Claude Code as storyteller
```
