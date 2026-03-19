# botc-narrator

A Blood on the Clocktower narrator engine for the Trouble Brewing edition, written in OCaml with Jane Street conventions.

## How the narrator works

In a real BotC game, the narrator (storyteller) orchestrates the game: they wake players at night, tell them information, ask them to point at other players, and make discretionary decisions (e.g. what fake info to give a poisoned player). This library models all of those interactions as a **free monad** (`Botc_exec.t`), which separates the game logic from how those interactions actually happen.

### The free monad (`Botc_exec`)

Every character action is a value of type `unit Botc_exec.t` — a description of what should happen, not an imperative program. The primitives are:

| Primitive | Type | What it represents |
|-----------|------|--------------------|
| `wake id` | `unit t` | Wake a player up (they open their eyes) |
| `sleep id` | `unit t` | Put a player back to sleep |
| `tell id msg` | `unit t` | Tell a player something ("You see 2 evil neighbors") |
| `ask id question candidates` | `Player_id.t t` | Ask a player a question; they point at one of the candidates ("Who do you want to poison?") |
| `narrator_pick xs` | `'a t` | The narrator freely chooses one element from a list (narrator discretion) |
| `log msg` | `unit t` | Log a narrator-facing message |
| `get_state` / `set_state` / `modify_state` | `Game_state.t t` / `unit t` | Read or update game state |

These compose with `let%bind.Botc_exec` and `let%map.Botc_exec`.

### Asking vs telling vs narrator discretion

There are three distinct kinds of interaction:

- **`tell`** — one-way information from narrator to player. The Empath learns "1" (one evil neighbor). The Chef learns "2" (two adjacent evil pairs). The character formats its own message string.

- **`ask`** — the narrator asks a player a question, and the player responds by pointing at someone. The Poisoner is asked "Who do you want to poison?" and picks a candidate. The Fortune Teller is asked "Choose a player" twice. Each character provides its own question text.

- **`narrator_pick`** — the *narrator* (storyteller) makes a discretionary choice, not the player. This happens when the game rules give the storyteller freedom:
  - A poisoned Chef should see a wrong number — the narrator picks which wrong number.
  - The Washerwoman is shown "one of these two players is the Chef" — the narrator picks which non-Chef player to pair them with.
  - Demon bluffs — the narrator picks which 3 not-in-play characters to show.

In tests, `ask` and `narrator_pick` are both controlled via queues in the deterministic interpreter. `player_choices` feeds answers to `ask`; `narrator_choices` feeds indices to `narrator_pick`.

### Interpreters (`Interp_S`)

To actually run a `Botc_exec.t` program, you provide an interpreter — a module satisfying `Botc_exec.Interp_S`:

```ocaml
module type Interp_S = sig
  val wake : Player_id.t -> unit
  val sleep : Player_id.t -> unit
  val tell : Player_id.t -> string -> unit
  val ask : Player_id.t -> string -> Player_id.t list -> Player_id.t
  val narrator_pick : 'a list -> 'a
  val log : string -> unit
end
```

Then `Botc_exec.run (module MyInterp) initial_state program` executes it. Different interpreters serve different purposes:

- **Console interpreter** (`bin/main.ml`): prints to stdout, always picks the first option.
- **Test interpreter** (`test/helpers/test_helpers.ml`): feeds deterministic answers from queues, collects `tell` messages for assertions.
- A future GUI/web interpreter would prompt real users.

## Architecture

```
src/
  core/                        Core types and engine (botc_narrator_core)
    kind.ml                    Townsfolk | Outsider | Minion | Demon
    alignment.ml               Good | Evil
    char_display.ml            Lightweight character record for display/storage
    player_id.ml               Comparable integer wrapper
    player.ml                  Player record (id, name, character, alive, etc.)
    game_state.ml              Full game state (players, phase, status effects)
    botc_exec.ml               Free monad for narrator instructions
    character_intf.ml          Module type signatures (Base_S, S, Character)
    character.ml               Make functor — shared helpers for all characters
    narrator.ml                Night phase orchestration (minion/demon info, run_order)
    script_intf.ml             Module type for scripts (all, night orders)

  characters/                  Character implementations (botc_narrator_characters)
    import.ml                  Prelude (re-exports Char_display)
    characters.ml              Module re-exports for all 22 characters
    <character>.ml             One file per character
    <character>.mli            Each is just: include Character_intf.S

  scripts/                     Script definitions (botc_narrator_scripts)
    trouble_brewing.ml         Character roster + night orders for Trouble Brewing

  botc_narrator_lib.ml         Top-level library re-exporting everything

test/
  helpers/                     Shared test utilities
    test_helpers.ml            Deterministic interpreter, state builders, assertions
  trouble_brewing/             Tests for the Trouble Brewing script
    test_trouble_brewing.ml    Night and day action tests
```

## Characters

Each character is a module satisfying `Character_intf.S`. A character is defined by:

1. **`type t`** — abstract per-character state. Most characters use `{ kind : Kind.t; alignment : Alignment.t }`, but this can be extended (e.g. Slayer could track `used : bool`).

2. **`Character.Make` functor** — provides shared helpers (`to_display`, `narrator_pick_from`, `pick_n`, `alive_except`, `if_alive`) from a `Base_S` input.

3. **Actions and hooks** defined after the `include Character.Make(...)`:

| Function | When it fires |
|----------|--------------|
| `night_action ~player_id ~night` | During the night phase, in script order |
| `day_action ~player_id` | During the day phase (e.g. Slayer shoot) |
| `on_setup ~player_id` | During game setup (e.g. Baron adds outsiders) |
| `on_nominated ~player_id ~nominator` | When this player is nominated (e.g. Virgin) |
| `on_executed ~player_id` | When this player is executed (e.g. Saint) |
| `on_night_kill ~player_id` | When this player would die at night (e.g. Soldier blocks, Mayor redirects) |

All return `unit Botc_exec.t option` — `None` means the hook doesn't apply, `Some m` means run `m`.

## Running tests

```sh
opam exec -- dune runtest
```

## Running the demo

```sh
opam exec -- dune exec botc_narrator
```

This runs a hardcoded 6-player Night 1 scenario with console output.
