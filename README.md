# botc-narrator

A [Blood on the Clocktower](https://bloodontheclocktower.com/) narrator
(storyteller) engine, written in Rust. It models the storyteller's job — waking
players, giving them information, resolving abilities and deaths — for the
**Trouble Brewing** edition, with an architecture built so that adding a
character, a status, or a whole script is *append-only*.

```sh
cargo test              # scenario + simulation suite
cargo run               # a scripted console game
cargo run --bin server  # multiplayer web server on http://localhost:3000
```

## The design in one breath

State is an open bag of **reminder tokens** on players. Behaviour lives in
stateless **ability** objects held in a **registry** and selected by a data-only
**script**. Roles never touch each other: they raise intents through a shared
**context** — *"the demon attacks X"*, *"does Y register as evil?"* — and a
**pipeline** resolves them by consulting every ability's hooks. I/O is behind a
**storyteller** trait, so the same logic runs a console, a UI, or a
deterministic test.

## Why it's built this way

The previous (OCaml) version worked but wasn't *appendable*: characters reached
into each other by hardcoded string identity, and status was hardcoded boolean
fields. Three things fix that here.

### 1. Status is tokens, not fields

`token.rs` is an open enum: `Poisoned`, `Protected`, `RedHerring`,
`Master(player)`, `DiedToday`, `UsedAbility`, … Adding a new status is adding a
variant, never widening the player record. Cross-cutting questions are asked
once over the bag:

```rust
// One definition every information role consults.
pub fn is_impaired(&self) -> bool {
    self.believed_role != self.role            // the Drunk, for free
        || self.tokens.iter().any(|t| t.impairs())  // poison, drunkenness
}
```

The Drunk needs *no* code of its own: a player who believes they are a character
they are not is impaired by construction, so it runs its believed Townsfolk's
ability and gets storyteller-chosen noise.

### 2. Interactions are a pipeline, not cross-references

The Imp does not know the Soldier, Monk, Mayor, or Scarlet Woman exist. It fires
one intent:

```rust
ctx.demon_kill(target);   // that's the entire kill
```

`Ctx::demon_kill` then resolves the death by consulting abilities:

* a `Protected` token (Monk) cancels it;
* the target's `blocks_demon_kill` (Soldier) can refuse it;
* the target's `redirect_death` (Mayor) can bounce it elsewhere;
* once applied, `on_any_death` fans out to every ability — the Scarlet Woman
  promotes, the Undertaker takes note, the Ravenkeeper wakes.

Soldier, Monk, Mayor, and Scarlet Woman are therefore each **one self-contained
file**. None of them is mentioned in the Imp, and none mentions the others.
Starpass-vs-Scarlet-Woman precedence falls out naturally: a starpass makes the
new Demon *before* the old one dies, so the resolved death already sees a living
Demon and the Scarlet Woman stays put.

### 3. Information routes through registration & impairment

Misregistration (Recluse, Spy) and droison are handled centrally, not
re-implemented per role. An information ability computes the *true* answer using
registration queries, and the context does the rest:

```rust
// Empath: count neighbours that *register* as evil (a Recluse may, a Spy may not),
// then deliver — impaired players get storyteller-chosen false info.
let n = i32::from(ctx.registers_evil(left, "empath"))
      + i32::from(ctx.registers_evil(right, "empath"));
let shown = ctx.deliver(me, n, &[0, 1, 2]);
```

`registers_evil` / `registers_as_demon` / `registers_as_kind` /
`registers_as_character` consult the target's declared registration span and ask
the storyteller to resolve any ambiguity. `deliver` is the single place droison
corrupts numeric/boolean info.

## Adding things

**A character** — implement `Ability`, register it, done:

```rust
character!(Gossip, "gossip", "Gossip", Kind::Townsfolk, Alignment::Good);

impl Ability for Gossip {
    fn info(&self) -> CharacterInfo { Self::INFO }
    fn on_day_ability(&self, ctx: &mut Ctx, me: PlayerId, target: PlayerId) { /* … */ }
}
// reg.register(Gossip);
```

Only the hooks you use are written; every other hook defaults to a no-op.

**A script** — a new file listing character ids and night orders (see
`scripts/trouble_brewing.rs`). No engine changes.

**A storyteller backend** — implement `Storyteller` (six methods). `ScriptedStoryteller`
(tests) and `ConsoleStoryteller` (demo) are the two provided.

## Layout

```
src/
  ids.rs           PlayerId (seat) & CharacterId (interned string)
  role.rs          Kind, Alignment, CharacterInfo
  token.rs         the open reminder-token vocabulary
  grimoire.rs      seating, life status, tokens — mechanical state only, no rules
  storyteller.rs   the narrator I/O trait
  interp/          scripted (test), console, and automated storyteller backends
  event.rs         DeathSource, Registration (misregistration spans)
  ability.rs       the Ability trait: every hook a character can implement
  ctx.rs           the interaction pipeline: I/O, deaths, registration, info
  registry.rs      id -> behaviour map
  script.rs        roster + night orders (data)
  setup.rs         random legal composition (Baron/Drunk aware)
  voting.rs        nomination tally: majority, ghost votes, master constraint
  engine.rs        night/day orchestration (incl. the day vote)
  characters/      one file per role (22 for Trouble Brewing, + Assassin demo)
  scripts/         one file per script (trouble_brewing, homebrew)
  server/          multiplayer web server (automated storyteller)
tests/
  slice.rs             pipeline interactions (immunity, poison, starpass, promotion)
  trouble_brewing.rs   full-script scenarios (Virgin, Slayer, Ravenkeeper, …)
  voting.rs            thresholds, Butler master, ghost votes, ties
  appendable.rs        a new character + a second script, with no engine changes
  auto_game.rs         40 fully-automated games played to a winner
```

## Voting

`voting.rs` tallies a nomination as a pure function of the grimoire: living
players have one vote, dead players a single ghost vote, and a master-restricted
voter (the Butler, read from its `Master` token) only counts when their master
votes too. `Engine::call_vote` / `resolve_day` track who is on the block across
a day and resolve the execution (a tie executes no one, which then runs the
Mayor's endgame hook). The voting layer never names the Butler — any
master-constrained role works through the same token.

## Multiplayer (automated storyteller)

`cargo run --bin server` starts a web server where players join from their
browsers and the **computer runs the storyteller** — no human narrator. It works
because the storyteller's job splits cleanly across the `Storyteller` trait:

- **`ask`** (who to poison / kill / read) and **votes** are *player* decisions,
  routed to that seat's browser;
- **`choose`** (false info under poison, demon bluffs, misregistration) is the
  storyteller's *discretionary* call, made automatically by a `DiscretionPolicy`;
- **`wake`/`sleep`/`reveal`** are private, per-player events.

So the server's storyteller is literally `AutoStoryteller<NetworkAgent,
RandomDiscretion>` — the engine and all 22 roles are used unchanged. The
synchronous engine runs on its own thread and blocks on a channel while the async
web layer (`axum`) shuttles a player's prompt to the browser and their answer
back; a blocking `ask` simply parks the game thread until the network replies.
`setup.rs` builds a random legal composition (5–15 players) and `interp::auto`
provides a headless simulation used by `tests/auto_game.rs`. That simulation is
how a real bug was caught: after an Imp starpass the new Imp stopped acting,
because the night order dispatches by *believed* role and `transform` had left it
stale — a transformed player must be woken as their new role.

## Status & fidelity

All 22 Trouble Brewing characters are implemented with full-rules fidelity:
poison/drunk impairment, Monk protection, Soldier immunity, Mayor bounce,
Scarlet Woman promotion, Imp starpass, Recluse/Spy misregistration, the Drunk,
red herrings, and the Saint/Mayor/Demon-death win conditions — plus a day-vote
layer. `tests/appendable.rs` demonstrates the core claim: the non-TB **Assassin**
(one new file) and a **homebrew script** (one new file) drop in with zero changes
to the engine or any existing role.
