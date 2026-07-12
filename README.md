# botc-narrator

A [Blood on the Clocktower](https://bloodontheclocktower.com/) narrator
(storyteller) engine, written in Rust. It models the storyteller's job — waking
players, giving them information, resolving abilities and deaths — for the
**Trouble Brewing** edition, with an architecture built so that adding a
character, a status, or a whole script is *append-only*.

```sh
cargo test    # 20 scenario tests
cargo run     # a scripted console game
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
  interp/          scripted (test) and console storyteller backends
  event.rs         DeathSource, Registration (misregistration spans)
  ability.rs       the Ability trait: every hook a character can implement
  ctx.rs           the interaction pipeline: I/O, deaths, registration, info
  registry.rs      id -> behaviour map
  script.rs        roster + night orders (data)
  engine.rs        night/day orchestration
  characters/      one file per role (22 for Trouble Brewing)
  scripts/         one file per script
tests/
  slice.rs             pipeline interactions (immunity, poison, starpass, promotion)
  trouble_brewing.rs   full-script scenarios (Virgin, Slayer, Ravenkeeper, …)
```

## Status & fidelity

All 22 Trouble Brewing characters are implemented with full-rules fidelity:
poison/drunk impairment, Monk protection, Soldier immunity, Mayor bounce,
Scarlet Woman promotion, Imp starpass, Recluse/Spy misregistration, the Drunk,
red herrings, and the Saint/Mayor/Demon-death win conditions. Voting is
represented only by the Butler's master token; a vote-tallying layer would be
the natural next append.
