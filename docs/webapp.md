# The multiplayer web app

<https://botc.kiblitz.xyz> runs a game at a real table: everyone plays on their
own phone, and nobody has to be the storyteller. The engine runs the game, and
an LLM on someone's computer (Codex by default) makes the storyteller's choices.

## Hosting

1. Open the site, type your name and choose **Host a game**; no account is
   needed. You get a five-letter room code. The host is a player like everyone
   else; what makes you the host is the seat token your phone keeps, so the
   same phone gets back in as host after a reload.
2. Everyone else opens the site, types the code and their name, and taps
   **Join**. A link `https://botc.kiblitz.xyz/?join=CODE` fills the code in.
3. Arrange the seating to match the real circle, clockwise: drag the ⠿ handle,
   or use the arrows. ✕ removes someone who joined by mistake.
4. Pick the script. The player count is however many have joined; the picker
   shows each script's range. Everyone can open **Characters on …** in the
   lobby to read the chosen script's characters and abilities.
5. Optionally pair a computer as the storyteller (below), then **Start the game**.

During the game, the host's phone has the phase controls at the bottom:
**End the day** (the nominee on the block is executed), **Close the vote**,
and **Everyone close your eyes: start the night**. Dawn comes by itself when
the night's actions are done. The host also enters nominations and hands; see
Playing.

## Playing

- Your character is hidden until you tap the token; tap again to hide it.
- At night, keep your eyes closed. When the storyteller wakes you, your phone
  buzzes and asks you to choose players, a character, or yes/no. If you don't
  answer in time (90 seconds), the first choice is taken.
- Everything you have been told is in **What you know**, with players and
  characters highlighted. Public events (deaths at dawn, nominations, votes,
  executions) go to everyone.
- By day, nominations and votes happen out loud at the table, and the host
  enters them on their phone (decided 2026-10-22, so a stray tap on someone
  else's phone never nominates or votes). In the town square circle the host
  taps who nominates, then the nominee (only legal choices can be tapped: a
  living player who may nominate and has not today; someone not yet nominated
  today), then **Confirm**; **Cancel** starts over. During the vote the host
  goes round the circle tapping each voter whose hand is up (the living, and
  the dead who still have their ghost vote; tap again to lower it), then
  **Close the vote** to count. Everyone else's phone shows the vote and the
  raised hands live; the vote stays open until the host closes it. The
  storyteller's nominations (the Bishop) and the Butcher's second nomination
  (asked on the Butcher's phone) are voted on the same way.
- **Day actions** (user, 2026-10-22): by day every living phone has one
  **Day actions** button. It opens the same list on every phone, whatever
  the character: each day ability on the script (with its ability text),
  public ones (Slayer, Psychopath, Damsel guess, Gossip, Juggler) and
  private visits to the storyteller (Artist, Fisherman, Savant,
  Puzzlemaster), so using one shows nothing about who really has it. An
  action with targets opens the town square: tap the seats, then
  **Confirm**; **Cancel** goes back and sends nothing. A public use is
  announced with its targets ("X shoots Y"), then works for a real holder
  and does nothing for anyone else ("Nothing happens"); a kill shows on
  every phone as a death ("Y dies"), never its cause. A private visit is
  asked the same questions in private; a player without the ability is told
  "The storyteller has nothing for you". Each action is offered within its
  limit (once per game, each day, the first day). Travellers' abilities
  (their characters are public) and the Atheist's (it ends the game) are on
  their holder's list only, marked "yours alone".
- Honour system: the storyteller cannot hear the table. On scripts with
  madness (Cerenovus, Harpy, Pixie, Mutant), the list also has **I broke
  madness**, for everyone, once a day: the storyteller notes it (the same
  reply for anyone) and judges it at the end of the day as the ability says
  (it may execute a mad player; a Mutant who reports it claimed to be an
  Outsider). A report from someone who is not mad does nothing.
- When the game ends, everyone sees every character and the storyteller's notes.

A phone that sleeps or reloads reconnects to its seat (the seat is kept in the
browser's storage). Rooms live in the server's memory: a server restart ends
running games.

## The storyteller bridge

Without a bridge, the storyteller chooses at random among the legal options.
To let Codex or Claude Code choose (aiming for a balanced, interesting game),
paste the line the host's lobby shows into a terminal on any computer with
Node 22+ and Codex CLI or Claude Code installed and logged in. Nothing is
downloaded by hand:

```sh
# macOS, Linux, Git Bash
curl -fsSL https://botc.kiblitz.xyz/bridge.sh | sh -s -- <PAIRING-CODE>
```

```powershell
# Windows PowerShell
& ([scriptblock]::Create((irm https://botc.kiblitz.xyz/bridge.ps1))) <PAIRING-CODE>
```

Each fetches `bridge.mjs` from the site (`server/static/`) and runs it with
Node. It uses Codex if installed, else Claude Code; `-- COMMAND...` after the
code names any other command (in PowerShell, quote it: `'--'`). The Rust
bridge (`cargo run --release -p botc_bridge -- <PAIRING-CODE>`) speaks the
same protocol and stays as an alternative; it always uses Codex unless given
a command.

The host's lobby shows the pairing code inside both lines (**Copy**). The
code is the host's alone, because the bridge sees the whole grimoire; **New
code** drops the paired bridge. The host sees a green dot while one is connected. Whoever runs the bridge can
read the grimoire in its prompts, so a host who plays should let someone else's
computer (or one nobody looks at) be the storyteller.

Each decision is sent as a prompt (the grimoire, the history, the numbered legal
options) to the command's stdin, and the command's output is parsed for option
numbers by the server, so a bridge can never make an illegal choice. By default
the command is `codex exec --skip-git-repo-check --ephemeral --sandbox=read-only -`.
Any other command works after `--`, for example
`curl -fsSL https://botc.kiblitz.xyz/bridge.sh | sh -s -- <PAIRING-CODE> -- claude -p`.

To pick which Codex login answers, pass `--codex-home DIR` (or set Codex's own
`CODEX_HOME`), e.g. a second account when the first is usage-limited.

Options (both bridges): `--server URL` (default `https://botc.kiblitz.xyz`), `--timeout SECONDS`
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
- Nobody signs in: hosting and joining are anonymous (decided 2026-10-21).
- Deploy: `deploy/` (see `deploy/README.md`) and `scripts/deploy.sh`, on slot1,
  port 8081.

Tests: `server/tests/game.rs` (whole games over sockets with bot phones, and
that no phone sees another's secrets), `bridge/tests/bridge.rs` (pairing and
decisions with a fake `codex`), `server/e2e` (Playwright on phone viewports: a whole table, the script view, and both bridge one-liners run as shown with a fake `codex`,
against `cargo run -p botc_server --example e2e`).

Limits: a seat takes at most three phones; a client (by the address Caddy
forwards) that tries 60 wrong room codes in a minute cannot join any room for
the rest of that minute. Seat tokens and pairing codes are too long to guess,
so reconnecting phones and bridges are never limited. One client address hosts
at most three rooms at once (requests that do not come through Caddy, as in
local runs, are not limited); a room whose game never starts closes after an
hour untouched, a running one after four, a finished one after thirty minutes,
and the server holds at most 64 rooms.

Not yet modelled: Travellers joining, the
storyteller's own judgement of madness (only self-reports count), a fake
private visit answers at once while a real one waits for the storyteller, and
"did you say the word / lose roshambo" are asked to the player themself.

The live site can be played end to end from this machine with `server/e2e/live.mjs` (an anonymous host, five guests and a bridge on this PC; `VIDEO=dir` records it; see its header).
