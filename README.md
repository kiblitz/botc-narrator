# botc-narrator

A Blood on the Clocktower storyteller engine in Rust, with every character on the scripts in `data/scripts` (see [docs/adding-characters.md](docs/adding-characters.md) to add more).

## Design

The engine separates *rules* from *how the game is run*:

- **Characters** (`src/characters/`) describe their ability against an ability context. They never print or prompt directly.
- **The table** (`table::Io`) is how the storyteller talks to players: `wake`, `sleep`, `tell`, `ask`, and `confirm` for yes/no questions (`table::Question`: use an optional ability, claim a character, said the secret word, lost roshambo). Public announcements are `ctx::Write::announce` events. Implement it for a console, a GUI, a networked app or a test recorder.
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

Info abilities never read other players' alignment or character from the grimoire directly: they ask `ctx::Read::registers_evil` / `registered_alignment`, `registered_character` and `registers_as(asker, target, kind)`. A character that may register falsely says so with `character::Def::misregistration` (the Recluse as an evil Minion or Demon, the Spy as a good Townsfolk or Outsider). The storyteller decides each query (`Reason::Registration`, the truthful answer first), and is only asked when a misregistration would change the answer; a malfunctioning character registers truly. Misregistration may be as any script character of the allowed kinds. The Fortune Teller's red herring is a reminder token chosen at setup.

### Read vs write

Night actions are tagged `NightAction::ReadOnly` (gets `ctx::Read`, which cannot change the grimoire) or `NightAction::ReadWrite` (gets `ctx::Write`). The narrator groups consecutive read-only actions into a batch and announces it to the table (`batch_started` / `batch_ended`), since those actions are independent; read-write actions are sync points. Batches currently run one after another.

### Rule interactions

`ctx::Write` resolves interactions centrally, so characters do not know about each other:

- `kill` checks Monk protection (a reminder) for demon kills, then asks every functioning living player's abilities (`intercept_death`) in seat order, the dying player's own abilities last (so a Tea Lady's protection is used before the Fool's), whether to let the death happen, prevent it (Soldier) or redirect it (Mayor; never onto the demon). The same applies to executions and any other cause.
- Minions and demon learn each other on the first night only with 7 or more players (Travellers do not count and are not shown), unless a living, functioning ability hides it (`hides_evil_info`: Poppy Grower; `narrator::evil_info` runs it later). Abilities that `joins_evil_info` (Magician) appear in the team they are shown.
- A Poisoned or Drunk token placed by another player only works while that player is sober and healthy themselves (`State::is_malfunctioning`): a poisoned Courtier's or Philosopher's target sobers.
- `die` calls every living player's `on_death` hook, then the dead player's own (Scarlet Woman, Imp starpass), then checks win conditions.
- `kill_unstoppably` skips protection and `intercept_death` (the Assassin); the death itself (`die`) is the same.
- `execute` calls `on_executed` (Saint, Goblin, Boomdandy), then every player's `on_execution` (Fearmonger, Mastermind), then `kill` with `Cause::Execution`.
- `check_win` asks every player's `vetoes_win` before declaring a winner (the Mastermind's extra day).

### Grimoire state

`game::State` keeps players in seat order with **reminder tokens** (`game::Reminder { source, effect, expiry }`) instead of hardcoded flags. Poisoned, Drunk, Protected, AbilityUsed and KeepsAbility (a dead player's ability still works while the placer lives and functions: the Vigormortis's Minions) have rules meaning; `Effect::Mad(character)` marks a player mad about being a character; `Effect::Custom` covers tokens only their character interprets (Butler's master, Fortune Teller's red herring). Placing a token moves it, like a physical token (`ctx::Write::place_reminders` puts one ability's token on several players at once; `remove_reminder` takes one off); `ctx::Write::add_reminder` leaves the copies on other players instead (the No Dashii's two Poisoned tokens; at most one equal token per player). `Effect::DoesNotWake` keeps a player asleep for their abilities tonight (the Exorcist's Demon). `Expiry::Dusk` tokens are removed when the next night begins, `Expiry::Night(n)` when night `n` begins ("drunk for 3 nights & 3 days"), `Expiry::Death` never by time; all three are also removed when the player whose ability placed them dies or changes character. `Expiry::Never` tokens stay. The game keeps a typed `event::Event` history.

Each player also has:

- a **believed character** (`Player::believes`): what they were told they are, when it is not their real character. The Drunk is told a not-in-play Townsfolk at setup; they wake and act as it, but a player who believes a character they are not is malfunctioning, so its ability never works and their information may be false.
- **abilities** (`State::abilities`): their own character's (unless lost), the believed character's and any gained ones (`ctx::Write::gain_ability` / `lose_ability`). Night actions and every player hook run for each ability; who acts is decided when each character's turn comes, so an ability gained earlier that night wakes in its own place in the night order. Once-per-game and other tokens are kept per ability (`source`).
- an **alignment** set at seating (`game::Seat`): the character's, or chosen for a Traveller.

Character kinds are Townsfolk, Outsider, Minion, Demon, **Traveller** (seated, chosen alignment, outside the setup counts) **Fabled** and **Loric** (storyteller-side: never seated; every Fabled and Loric on a script is put in play by `narrator::setup`, and its `setup_counts` apply to the setup).

Reminder tokens with rules meaning beyond poison and protection: `Sober` (sober, healthy and true info, the Barista; a character may also be `always_sober`, the Beggar), `ActsTwice` (night actions run twice), `AbilityRestored` (a dead player's ability works until dusk, the Bone Collector) and `ExecutionOnly` (only execution kills, the Storm Catcher). Votes are tallied by `narrator::vote` with each functioning ability's `vote_weight` (Bureaucrat x3, a Beggar without a token 0). Players may answer yes/no questions (`table::Io::ask_yes_no`: the Harlot's visit, another Matron swap). `ctx::Write::swap_seats` reorders the circle.

### Adding characters and scripts

A character is a unit struct implementing `character::Def` in its own module (`src/characters/<id>.rs`) plus one line in the `registry!` list in `src/characters.rs`; abilities are mostly composed from the building blocks in `ability` (tell a count / yes-no / character / "one of two", count registered evil, mark a player, use once, kill, the demon attack, storyteller picks). **See [docs/adding-characters.md](docs/adding-characters.md).** It overrides only the hooks it uses:

| Hook | When |
|---|---|
| `night_order` | its official first-night / other-nights positions (clocktower.online numbering) |
| `night_action`, `wakes_tonight` | what it does at its turn; whether it wakes tonight (default: alive), checked when the turn comes |
| `day_action`, `day_action_users` | a public day ability (Slayer); who may use it (default the holder; the Damsel lets every Minion guess) |
| `can_nominate` | whether the holder may nominate (the Golem only once); `narrator::nominate` refuses otherwise |
| `setup_counts`, `duplicates_in_setup` | how it changes the setup counts (Baron: +2 Outsiders; options for "+0 or +1"); copies filling extra slots (Legion, Riot) |
| `requires` | characters that must be in play with it ("[+the Damsel]"): `setup::choose` swaps one in for a Townsfolk |
| `on_setup` | storyteller setup choices before the first night (Drunk, Fortune Teller red herring); `game::State::script` is set by then |
| `on_night_start`, `on_day_start`, `on_day_end` | phase changes, for living players |
| `on_nominated`, `on_nominates`, `on_nomination`, `on_vote` | the nominee, the nominator, everyone after a nomination (Witch), everyone after a vote (dead players too) |
| `on_executed`, `on_execution` | the executed player; everyone (dead too), when anyone is executed |
| `intercept_death`, `on_death` | preventing or redirecting any death, reacting to deaths |
| `on_learns_death` | the dead player learns they died: at once by day, at dawn for a night death (Klutz, Moonchild) |
| `on_choose`, `on_chosen` | at night, the chooser's abilities, then each chosen player's (Lunatic, Goon), whenever a player chooses players with `ctx.ask` (any night choice counts, e.g. the Demon's Barber swap) |
| `on_win` | the game was won; every player's abilities, dead ones too (Politician) |
| `on_madness_broken` | a player this ability made mad broke madness (`narrator::check_madness`, judged by the table) |
| `misregistration` | how it may register falsely |
| `counts_as_demon` | whether a living holder keeps evil in the game (good wins when none lives; a Lil' Monsta-like carrier gains such an ability) |
| `forces_false_info` | makes other players' information false (Vortox) |
| `always_sober` | the player can never be drunk or poisoned (Beggar) |
| `vote_weight` | how much a vote counts (Bureaucrat, Beggar) |
| `storyteller_nominations`, `nominates_again` | only the storyteller nominates, and whom (Bishop); a second nomination after the day's first execution (Butcher) |
| `fabled_setup`, `fabled_dusk`, `permits_alignment_change`, `lies_to` | Fabled and Loric (no player): setup (Storm Catcher), dusk (Knaves switch), vetoing alignment changes (Spirit of Ivory), a lying storyteller for a player's information (Knaves) |
| `joins_evil_info`, `hides_evil_info` | appears in the evil team's first-night info (Magician); keeps evil from learning each other (Poppy Grower) |
| `demonless` | the game has no Demon, so good does not win by the Demon dying (Atheist) |
| `vetoes_win` | stops a win for now (Mastermind) |
| `seated_next_to_demon` | must sit beside a Demon (Marionette; the simulator's seating arranges it) |

Extra deaths at other times are plain `ctx::Write::kill` calls from any hook.

Night choices: `ctx::Write::ask` resolves `on_choose` / `on_chosen` at once; a read-only ability's choices are kept and resolved by the narrator right after its action (it cannot change the grimoire mid-action). When a character's turn comes, players who only believe they are it act first (the Lunatic before the real Demon). The table is asked `table::Io::is_mad_about` when an ability must judge whether a player tried to convince the group they are something (the Mutant, an Outsider); tokens can be removed again with `ctx::Write::remove_reminder`.

A script is a JSON file in `data/scripts/<id>.json` in the standard script format (a `_meta` entry with the name, then official character ids, as strings or `{"id"}` objects), listed in `script::SCRIPTS`. Official ids are matched to `Def::id` ignoring underscores (`fortuneteller` is `fortune_teller`); unknown ids are an error listing them. Night orders are computed by sorting the script's characters by `night_order`; the minion and demon info sit at positions 6 and 9 of the first night.

`setup::choose` builds a legal set of characters for a player count: it picks characters at random and re-applies every chosen character's `setup_counts` until the picks match, filling kinds that run out with Townsfolk.

### Not yet modelled

Travellers: they are seated at setup only (no joining later, no exile). The Bishop's storyteller nominates one opposing player a day (not everyone on the final day), and storyteller nominations do not trigger the Virgin. The Barista's "works twice" repeats night actions only and does not refresh a used once-per-game ability; the Bone Collector's restored player acts with their ability until dusk (night wakes and every hook) but used once-per-game abilities stay used. Only one Butcher's second nomination happens per day. The Harlot's storyteller is never offered the deaths when the chosen player is a demon. Fabled and Loric: the Storm Catcher's evil players learn the named player at setup, before the first night, and the protection stays with that player; the Knaves' players choose a storyteller for each piece of ability information, and the two storytellers' choices (who lies, whether to switch) are one decision each. The Bureaucrat combined with a Thief is not modelled.

Protection and rule-changing Townsfolk: an Alchemist's Minion ability does not change the setup counts (an Alchemist-Baron adds no Outsiders), and the storyteller cannot make them choose again. The Amnesiac's hidden ability is a not-in-play Townsfolk's, not one the storyteller designs. The Atheist's storyteller never breaks rules, and nominating the Storyteller is the living Atheist claimant's day action rather than anyone's nomination. Players cannot decline once-per-game choices (Courtier, Engineer, Philosopher always use them the first night they can). The Engineer gives changed Minions the chosen characters in seat order (no storyteller choice). The Magician is shown as part of the evil team, without the Demon/Minion distinction (team info is not role-labelled). The Poppy Grower's optional rule (evil learn each other when it becomes drunk or leaves play) is not used. A Courtier's or Minstrel's drunkenness ends if they die.

Nominating a Traveller's exile, and reminder tokens in the Spy's grimoire view. Misinformation for "one of these two is the X" only offers characters in play, not the whole script. Dead players' votes in the simulator and the Butler's voting restriction are not modelled; a believed character makes every ability of that player malfunction (right for the Drunk, not for a Lunatic who gains another ability). The Saint's loss fires on execution before any ability can prevent the death. The simulator's player view shows a Drunk their real character (it does not read `believes` yet). The Fang Gu's jump only notices Monk protection (other ways an Outsider would survive still let it jump). A Cerenovus execution is judged once, at the end of the day. The Mathematician's count is an approximation: the engine does not record which abilities misfired, so any number up to how many other players are malfunctioning counts as true and the storyteller picks. The Chambermaid counts a player as woken by their ability if one of their abilities has a turn tonight that it wakes for, judged when she acts (a player who dies before a later turn still counts). The Seamstress passes by pointing at herself. The Dreamer may be shown the Vortox, and the Grandmother does not yet trigger the Leviathan jinx.

Demons:

- The Zombuul's first death is not announced and it still counts as alive for nominations, votes and information.
- Lil' Monsta's babysitter does not register as the Demon to information abilities, the player dealt it (now a Minion) gets the first night's demon info, and Minions choose among living Minions only.
- Legion's evil-only vote makes the execution fail (the nominee survives) rather than zeroing the tally.
- The Leviathan's evil win on a second good execution is judged at dusk.
- Poison from the No Dashii, Vigormortis, Lleech and Pukka is updated at phase changes and deaths, not the moment the demon becomes drunk or a neighbour changes character.
- A regurgitated or revived player does not regain used abilities.
- The Al-Hadikhia's choices are announced at once rather than at dawn.
- The Pukka's and Shabaloth's tokens are only updated when they act or die (a Pukka that loses its ability some other way leaves its victim poisoned).
- Lil' Monsta is not a player, so a poisoned or drunk babysitter still lets a player die (a deliberate reading of the rules).
- Demon jinxes are not modelled.
Free-form day abilities are narrowed to typed choices: the Gossip's statement and the Artist's question are "this player is this character" (the Artist is never told "I don't know"), the Savant's statements are about one other player's alignment or character (when malfunctioning, both true or both false is the opposite of one of the storyteller's two picks), and the Fisherman's advice is whom to trust. The Gossip's statement is judged at night, against the characters then. The Juggler guesses only on day 1 (not a later first day as the Juggler), always as many distinct players as they can, up to 5. The Professor and the Huntsman cannot decline while there is someone to choose. The Town Crier ignores Traveller exiles only because exiles are not modelled. The High Priestess sits at 50 / 69, right after the Spy, since the clocktower.online numbering lacks her. In the simulator's player view a revived player still looks dead.

Outsiders:

- The Barber's Demon cannot decline the swap, and a Barber who dies at night after its turn gives haircuts the next night.
- A Goon chosen by a read-only ability (Fortune Teller) makes the chooser drunk only after that ability's information.
- The Plague Doctor's Minion ability goes to a living Minion of the storyteller's choice (as the official jinxes do for several Minions), since the storyteller holds no abilities; with no living Minion, nothing happens.
- The Puzzlemaster's drunk player stays drunk while the Puzzlemaster is drunk or poisoned.
- The Damsel's Spy and Widow jinxes are not applied.
- The Lunatic's choices get no Chosen tokens (the Demon is told them).
- The Tinker dies only at night; the Mutant is judged once, at the end of the day.
- The Politician's "most responsible" is a storyteller yes/no at the end of the game; their alignment changes after the win is recorded (the recorded winner is the team that won).

Minions: the Harpy wakes at 26 / 17 (officially 26.5 / 17.5, after the Fearmonger). When the Pit-Hag makes a Demon, "arbitrary deaths" are one optional extra kill of the storyteller's choice (Demon attacks still happen), and a character it creates with a first-night-only ability does not act. The Marionette never replaces a third Minion at setup, and only the simulator seats it beside the Demon. A Widow malfunctioning when choosing never poisons (rather than "while that lasts"). Whether a good player said the Mezepheles's word is judged once at the end of each day. If the Mastermind dies during the extra night or day, good wins at once. A Widow's poison does not lapse while she is drunk or poisoned. The Harpy's mad player is told the 2nd player as "Evil: <player>" (the claim they must make). A malfunctioning Fearmonger's choice leaves the previous Fear target in place. Only the Goblin is asked whether to claim (a false claim has no effect anyway). The Boomdandy's pointing is one round of choices, not a live countdown.

## Simulator site

`site/` is a static page that generates random, plausible games of any registered script in the browser (the engine compiled to WebAssembly from `web/`) and lets you step through them. Players sit in a circle; each step shows who is awake, what they were told or asked, storyteller decisions and public events. Click a player to see everything they know so far, or switch the view to one player's point of view to hide what they couldn't see. Pick a script: every script in `script::all()` is listed, and a test plays each one across many seeds and player counts. Games are reproducible from `#script=...&seed=...&players=...` in the URL. Information steps highlight the players and characters they name. **View script** lists the script's characters by kind with their ability text (`site/abilities.json`, generated and gitignored: `scripts/build-web.sh` runs `scripts/abilities.mjs` over `data/characters/*.json` and `data/abilities/*.json`). The **Names** box sets seat names clockwise (`sim::simulate_named`); the game is the same as with the default names, and the names are kept in the URL.

Generation (`sim.rs`) uses legal setups for 5-15 players (`setup::choose`), players who act on simple heuristics (evil targets good), a random storyteller, daily nominations with a majority vote (no ghost votes; vote weights apply), up to two Travellers with random alignments when the script has them, mad players who break madness with a fixed probability, players who "may choose" choosing no-one now and then (the Po charging), and night deaths announced at dawn. Day actions such as the Slayer's shot are public.

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
