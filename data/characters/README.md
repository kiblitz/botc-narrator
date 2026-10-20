# Character rules specs

One JSON file per character, `data/characters/<id>.json`, where `<id>` is the
official script id (as used in `data/scripts`). These are research notes for the
engine: what the ability does, sourced from the official wiki
(https://wiki.bloodontheclocktower.com/<Name>), plus test scenarios the
implementation turns into expect-tests. `tests/character_specs.rs` checks that
every file parses and has every field below.

| field | type | meaning |
|---|---|---|
| `id` | string | official id; equals the file name |
| `name` | string | display name |
| `kind` | string | `townsfolk`, `outsider`, `minion`, `demon`, `traveller` or `fabled` |
| `ability` | string | the exact official ability text |
| `first_night` | number or null | official first-night order number (null: does not act) |
| `other_nights` | number or null | official other-nights order number |
| `reminders` | string[] | reminder token names |
| `setup` | string or null | setup effects (e.g. outsider count changes); null if none |
| `wakes` | string | whether and when the player wakes |
| `interaction` | string | what the player is told and what they choose |
| `drunk_or_poisoned` | string | behaviour while drunk or poisoned |
| `jinxes` | object[] | `{ "with": <id>, "text": <official jinx text> }` for characters in `data/scripts` |
| `rulings` | object[] | `{ "text": ..., "source": <url> }` notable rulings and edge cases |
| `building_blocks` | string[] | generic engine mechanisms the ability needs (see below) |
| `test_scenarios` | object[] | 3-6 of `{ "name", "setup", "actions", "expected" }` (all strings) |

Building-block vocabulary (add new names when none fits, and keep them kebab-case):
`learn-number`, `learn-player`, `learn-character`, `choose-players`,
`choose-character`, `protect`, `kill`, `once-per-game`, `registers-as`,
`setup-modifier`, `madness`, `believes-character`, `day-ability`,
`on-nominated`, `on-death`, `on-execution`, `vote-modifier`,
`extra-execution`, `ability-gain`, `character-change`, `poison`, `drunk`,
`win-condition`, `demon-death-survival`, `false-info`, `delayed-effect`,
`learn-yes-no`, `alignment-check` (learns or compares alignments),
`seating` (uses neighbours or distance around the circle),
`storyteller-judgement` (the Storyteller decides from the game state, e.g. who
is winning), `track-history` (needs a record of past nights or days),
`wake-count` (needs who woke tonight due to their ability),
`ability-malfunction-count` (needs which abilities worked abnormally since dawn).
