# botc-narrator: agent context

Rust Blood on the Clocktower storyteller engine. Read README.md for the architecture.

## Decisions (do not revisit without the user)

- Rewritten from OCaml to Rust (2026-10-20). The OCaml free monad was replaced by ability contexts (`ctx::Read` / `ctx::Write`) passed to plain functions; the read/write split replaces the phantom permission types.
- Storyteller discretion goes through `storyteller::Decision` with only legal options and a typed reason. The AI storyteller is Claude Code via the `claude -p` CLI (prompt on stdin), not the API. Tests use `storyteller::Scripted`.
- Info abilities supply truthful and misinformation options; poisoning is handled centrally by `ctx::Read::inform`, never per character.
- Grimoire status is reminder tokens (`game::Reminder`), not per-effect boolean fields.
- The simulator (`src/sim.rs` -> `web/` wasm -> `site/`) records every step with an audience (public / player / storyteller) and a grimoire snapshot; the table gets the grimoire with each event (`table::Io::event`), so every grimoire change must go through `ctx::Write::record`. In a player's view the site shows only that player's knowledge, and night deaths are announced at dawn without their cause.
- Character icons are the official art, hotlinked from the townsquare (clocktower.online) repo pinned to a commit in `site/app.js`; never copy them into this repository.
- Scripts are registered in one place, `script::all()` (from `script::SCRIPTS`); the simulator's Script picker lists whatever is there.
- No GitHub Actions. `scripts/check.sh` is the merge gate.
- Scripts are data: `data/scripts/<id>.json` (official ids, matched to `Def::id` ignoring underscores) through the `characters::all()` registry, listed in `script::SCRIPTS`. Night orders are computed from each character's `night_order` (clocktower.online numbering), never hand-written.
- Information abilities learn about other players only through registration (`ctx::Read::registers_evil` / `registered_character` / `registers_as`), so misregistration (Recluse, Spy) is central; the storyteller decides each query.
- A player who believes they are a character they are not (`Player::believes`) acts as it but is malfunctioning. Hooks run for every ability a player has (`State::abilities`: own unless lost, believed, gained).
- Any death (demon, execution, ability) goes through `ctx::Write::kill`, where abilities may prevent or redirect it (`intercept_death`). Setup counts come from `setup_counts` adjustments applied by `setup::choose`.

## Conventions

- `scripts/check.sh` needs `wasm-pack`, the `wasm32-unknown-unknown` target and node (Playwright tests in `site/tests`). Write Python helper output as UTF-8 (`PYTHONUTF8=1`) on this machine; cp1252 corrupts non-ASCII text.

- Module-qualified paths: `use crate::player;` then `player::Id`. Types don't repeat the module name (each character is `chef::Def`).
- Tests are `expect-test` transcripts in `tests/`. Update them with `UPDATE_EXPECT=1 cargo test`, then review the diff.
