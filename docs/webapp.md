# The multiplayer web app

<https://botc.kiblitz.xyz> runs a game at a real table: everyone plays on their
own phone, and nobody has to be the storyteller. The engine runs the game, and
an LLM on someone's computer (Codex by default) makes the storyteller's choices.

## Hosting

1. Open the site and choose **Sign in or sign up** (a Kiblitz account; only the
   host needs one), then **Host a game**. You get a five-letter room code. The
   host is a player like everyone else.
2. Everyone else opens the site, types the code and their name, and taps
   **Join**. A link `https://botc.kiblitz.xyz/?join=CODE` fills the code in.
3. Arrange the seating to match the real circle, clockwise: drag the ⠿ handle,
   or use the arrows. ✕ removes someone who joined by mistake.
4. Pick the script. The player count is however many have joined; the picker
   shows each script's range.
5. Optionally pair a computer as the storyteller (below), then **Start the game**.

During the game, the host's phone has the phase controls at the bottom:
**End the day** (the nominee on the block is executed), **Close the vote**,
and **Everyone close your eyes: start the night**. Dawn comes by itself when
the night's actions are done.

## Playing

- Your character is hidden until you tap the token; tap again to hide it.
- At night, keep your eyes closed. When the storyteller wakes you, your phone
  buzzes and asks you to choose players, a character, or yes/no. If you don't
  answer in time (90 seconds), the first choice is taken.
- Everything you have been told is in **What you know**, with players and
  characters highlighted. Public events (deaths at dawn, nominations, votes,
  executions) go to everyone.
- By day, tap **Nominate** next to a player. Everyone with a vote (the living,
  and the dead who still have their ghost vote) then sees **Raise hand** / **No**.
  Public day abilities (the Slayer's shot) show up as a button for their holder.
- When the game ends, everyone sees every character and the storyteller's notes.

A phone that sleeps or reloads reconnects to its seat (the seat is kept in the
browser's storage). Rooms live in the server's memory: a server restart ends
running games.

## The storyteller bridge

Without a bridge, the storyteller chooses at random among the legal options.
To let Codex choose (aiming for a balanced, interesting game), run the bridge on
any computer with Codex CLI installed and logged in:

```sh
cargo run --release -p botc_bridge -- <PAIRING-CODE>
```

The host's lobby shows the pairing code and the exact command (**Copy**). The
code is the host's alone, because the bridge sees the whole grimoire; **New
code** drops the paired bridge. The host sees a green dot while one is connected. Whoever runs the bridge can
read the grimoire in its prompts, so a host who plays should let someone else's
computer (or one nobody looks at) be the storyteller.

Each decision is sent as a prompt (the grimoire, the history, the numbered legal
options) to the command's stdin, and the command's output is parsed for option
numbers by the server, so a bridge can never make an illegal choice. By default
the command is `codex exec --skip-git-repo-check --ephemeral --sandbox=read-only -`.
Any other command works after `--`, for example Claude Code:

```sh
cargo run --release -p botc_bridge -- <PAIRING-CODE> -- claude -p
```

Options: `--server URL` (default `https://botc.kiblitz.xyz`), `--timeout SECONDS`
for the command (default 80, under the server's 90). The server waits 90 seconds per decision; if the
bridge is gone, fails, or answers nonsense, that decision is random and the game
goes on. The bridge reconnects by itself if the connection drops.

## How it is built

- `server/` (`botc_server`): axum + WebSockets. `room` holds rooms in memory;
  `engine` runs each game on its own thread with the engine's narrator;
  `phone::Table` is the `table::Io` over phones (prompts block the game thread
  until answered or timed out); `bridged::Storyteller` is the
  `storyteller::Strategy` (bridge, else `storyteller::Random`); `view` builds
  each player's private view; `inbox` is what the sockets send the game thread.
  Sockets authenticate in their first message, never in the URL.
- `server/static/`: the phone client, plain HTML/CSS/JS with no build step.
- `bridge/` (`botc_bridge`): the CLI and the wire protocol (`bridge::wire`).
- Sign-in is the shared account service via the pinned `account-session`
  library (client id `botc`); only hosting needs it.
- Deploy: `deploy/` (see `deploy/README.md`) and `scripts/deploy.sh`, on slot1,
  port 8081.

Tests: `server/tests/game.rs` (whole games over sockets with bot phones, and
that no phone sees another's secrets), `bridge/tests/bridge.rs` (pairing and
decisions with a fake `codex`), `server/e2e` (Playwright on phone viewports,
against `cargo run -p botc_server --example e2e`).

Limits: a seat takes at most three phones; past 60 wrong room codes, seat
tokens or pairing codes in a minute, the server refuses lookups for the rest of
that minute. A host whose game is running cannot open another room.

Not yet modelled: Travellers joining, the
storyteller's judgement of madness (mad players never break madness), and
"did you say the word / lose roshambo" are asked to the player themself.
