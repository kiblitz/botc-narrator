# botc-narrator: agent context

Rust Blood on the Clocktower storyteller engine. Read README.md for the architecture.

## Decisions (do not revisit without the user)

- Rewritten from OCaml to Rust (2026-10-20). The OCaml free monad was replaced by ability contexts (`ctx::Read` / `ctx::Write`) passed to plain functions; the read/write split replaces the phantom permission types.
- Storyteller discretion goes through `storyteller::Decision` with only legal options and a typed reason. The AI storyteller is Claude Code via the `claude -p` CLI (prompt on stdin), not the API. Tests use `storyteller::Scripted`.
- Info abilities supply truthful and misinformation options; poisoning is handled centrally by `ctx::Read::inform`, never per character.
- Grimoire status is reminder tokens (`game::Reminder`), not per-effect boolean fields.
- No GitHub Actions. `scripts/check.sh` is the merge gate.

## Conventions

- Module-qualified paths: `use crate::player;` then `player::Id`. Types don't repeat the module name (each character is `chef::Def`).
- Tests are `expect-test` transcripts in `tests/`. Update them with `UPDATE_EXPECT=1 cargo test`, then review the diff.
