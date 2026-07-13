//! The interaction context handed to every ability hook.
//!
//! `Ctx` bundles the mutable grimoire, the storyteller I/O, and read-only
//! access to the registry and script. It exposes:
//!
//! * **labelled I/O** (`wake`/`sleep`/`reveal`/`ask`/`choose`) that formats
//!   player names for the storyteller;
//! * the **death pipeline** (`demon_kill`/`execute`/`ability_kill`) that
//!   resolves protection, immunity, redirection, and on-death reactions by
//!   consulting abilities rather than hardcoding roles;
//! * **registration queries** (`registers_evil`/`registers_as_demon`/…) that
//!   route misregistration (Recluse/Spy) through storyteller discretion;
//! * an **information helper** (`deliver`) that substitutes false info for
//!   impaired players in one place instead of once per role.
//!
//! ### The borrow trick
//! Abilities are stored in the registry, which `Ctx` holds by shared reference.
//! To dispatch an event to an ability we copy that `&Registry` out of `self`
//! first (`let reg = self.reg;`), look the ability up through the copy, then
//! call it with `&mut self`. Because the ability reference is tied to the
//! registry's lifetime — not to a borrow of `self` — this satisfies the borrow
//! checker without interior mutability or cloning.

use crate::event::DeathSource;
use crate::grimoire::Grimoire;
use crate::ids::{CharacterId, PlayerId};
use crate::registry::Registry;
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::script::Script;
use crate::storyteller::{Candidate, Storyteller};
use crate::token::Token;

pub struct Ctx<'a> {
    pub grim: &'a mut Grimoire,
    pub st: &'a mut dyn Storyteller,
    reg: &'a Registry,
    script: &'a Script,
}

impl<'a> Ctx<'a> {
    pub fn new(
        grim: &'a mut Grimoire,
        st: &'a mut dyn Storyteller,
        reg: &'a Registry,
        script: &'a Script,
    ) -> Self {
        Ctx {
            grim,
            st,
            reg,
            script,
        }
    }

    #[must_use]
    pub fn registry(&self) -> &Registry {
        self.reg
    }

    #[must_use]
    pub fn script(&self) -> &Script {
        self.script
    }

    // --- Labelling & I/O ----------------------------------------------------

    /// "Name(Role)" — the storyteller's-eye label used throughout the transcript.
    #[must_use]
    pub fn label(&self, id: PlayerId) -> String {
        let p = self.grim.get(id);
        let role = self.reg.info(p.role).name;
        format!("{}({})", p.name, role)
    }

    /// The player's bare name — what other players actually see, used in
    /// player-facing information ("one of Alice and Bob is the Chef").
    #[must_use]
    pub fn name(&self, id: PlayerId) -> String {
        self.grim.get(id).name.clone()
    }

    pub fn wake(&mut self, me: PlayerId) {
        let who = self.label(me);
        self.st.wake(&who);
    }

    pub fn sleep(&mut self, me: PlayerId) {
        let who = self.label(me);
        self.st.sleep(&who);
    }

    pub fn reveal(&mut self, me: PlayerId, message: &str) {
        let who = self.label(me);
        self.st.reveal(&who, message);
    }

    /// Ask `me` to point at one of `candidates`; returns the chosen player.
    pub fn ask(&mut self, me: PlayerId, prompt: &str, candidates: &[PlayerId]) -> PlayerId {
        let who = self.label(me);
        let cands: Vec<Candidate> = candidates
            .iter()
            .map(|&id| Candidate::new(id, self.label(id)))
            .collect();
        self.st.ask(&who, prompt, &cands)
    }

    /// A discretionary storyteller choice among `options`; returns the index.
    pub fn choose(&mut self, prompt: &str, options: &[String]) -> usize {
        self.st.choose(prompt, options)
    }

    pub fn log(&mut self, message: &str) {
        self.st.log(message);
    }

    // --- Queries ------------------------------------------------------------

    #[must_use]
    pub fn is_impaired(&self, id: PlayerId) -> bool {
        self.grim.is_impaired(id)
    }

    /// Living players other than `me`, in seat order.
    #[must_use]
    pub fn alive_others(&self, me: PlayerId) -> Vec<PlayerId> {
        self.grim.alive().filter(|&id| id != me).collect()
    }

    #[must_use]
    pub fn living_demon_exists(&self) -> bool {
        self.grim
            .alive()
            .any(|id| self.grim.get(id).kind.is_demon())
    }

    /// Whether `id` *could* register as `kind` for some reading, without
    /// consulting the storyteller. Used to build candidate lists for the
    /// Washerwoman/Librarian/Investigator before offering the actual choice.
    #[must_use]
    pub fn could_register_kind(&self, id: PlayerId, kind: Kind) -> bool {
        let really = self.grim.get(id).kind == kind;
        match self.misregistration(id) {
            None => really,
            Some(r) => really || r.may_be_kind(kind),
        }
    }

    /// The display names of every character on the current script — the pool a
    /// storyteller draws false information from for an impaired role.
    #[must_use]
    pub fn roster_character_names(&self) -> Vec<String> {
        self.script
            .roster
            .iter()
            .map(|&c| self.reg.info(c).name.to_string())
            .collect()
    }

    fn misregistration(&self, id: PlayerId) -> Option<crate::event::Registration> {
        let reg = self.reg;
        let role = self.grim.get(id).role;
        reg.get(role).misregistration()
    }

    // --- Registration (misregistration resolved by storyteller) -------------

    /// Does `id` register as evil for this reading? Honours Recluse/Spy
    /// ambiguity via storyteller discretion.
    pub fn registers_evil(&mut self, id: PlayerId, context: &str) -> bool {
        let truly_evil = self.grim.get(id).alignment.is_evil();
        match self.misregistration(id) {
            None => truly_evil,
            Some(r) if r.may_be_evil() && r.may_be_good() => {
                let who = self.label(id);
                let opts = vec![format!("{who} reads EVIL"), format!("{who} reads GOOD")];
                self.st.choose(&format!("registration [{context}]"), &opts) == 0
            }
            Some(r) => r.may_be_evil(),
        }
    }

    /// Does `id` register as the Demon for this reading? Red herrings always
    /// do; the Recluse may, at storyteller discretion.
    pub fn registers_as_demon(&mut self, id: PlayerId, context: &str) -> bool {
        if self.grim.has_token(id, Token::RedHerring) {
            return true;
        }
        let truly_demon = self.grim.get(id).kind.is_demon();
        match self.misregistration(id) {
            None => truly_demon,
            Some(r) if !truly_demon && r.may_be_kind(Kind::Demon) => {
                let who = self.label(id);
                let opts = vec![
                    format!("{who} reads as the Demon"),
                    format!("{who} does not"),
                ];
                self.st.choose(&format!("registration [{context}]"), &opts) == 0
            }
            Some(_) => truly_demon,
        }
    }

    /// Does `id` register as `kind` for this reading? Used by the
    /// Washerwoman/Librarian/Investigator. Misregistering roles let the
    /// storyteller decide the specific reading.
    pub fn registers_as_kind(&mut self, id: PlayerId, kind: Kind, context: &str) -> bool {
        let really = self.grim.get(id).kind == kind;
        match self.misregistration(id) {
            None => really,
            Some(r) if really || r.may_be_kind(kind) => {
                let who = self.label(id);
                let opts = vec![
                    format!("{who} reads as {kind:?}"),
                    format!("{who} does not"),
                ];
                self.st.choose(&format!("registration [{context}]"), &opts) == 0
            }
            Some(_) => false,
        }
    }

    /// Which character `id` registers as (Undertaker). Misregistering roles let
    /// the storyteller pick any script character of a kind they may show as.
    pub fn registers_as_character(&mut self, id: PlayerId, context: &str) -> CharacterId {
        let real = self.grim.get(id).role;
        match self.misregistration(id) {
            None => real,
            Some(r) => {
                let mut cands: Vec<CharacterId> = vec![real];
                for &cid in &self.script.roster {
                    if cid != real && r.may_be_kind(self.reg.info(cid).kind) {
                        cands.push(cid);
                    }
                }
                let labels: Vec<String> = cands
                    .iter()
                    .map(|&c| self.reg.info(c).name.to_string())
                    .collect();
                let i = self.st.choose(
                    &format!("registers as which character [{context}]"),
                    &labels,
                );
                cands[i]
            }
        }
    }

    // --- Information delivery ------------------------------------------------

    /// Return the true value to show `me`, unless they are impaired, in which
    /// case the storyteller picks any value from `plausible`. This is the one
    /// place droison corrupts numeric/boolean information, so every simple info
    /// role gets it for free.
    pub fn deliver<T>(&mut self, me: PlayerId, truth: T, plausible: &[T]) -> T
    where
        T: Clone + std::fmt::Display,
    {
        if self.is_impaired(me) {
            let who = self.label(me);
            let opts: Vec<String> = plausible.iter().map(ToString::to_string).collect();
            let i = self.st.choose(&format!("false info for {who}"), &opts);
            plausible[i].clone()
        } else {
            truth
        }
    }

    // --- Death pipeline -----------------------------------------------------

    /// The Demon attacks `target` tonight. Resolves Monk protection, Soldier
    /// immunity, and Mayor redirection, then applies the death if it lands.
    pub fn demon_kill(&mut self, target: PlayerId) {
        if !self.grim.is_alive(target) {
            return;
        }
        if self.grim.has_token(target, Token::Protected) {
            let who = self.label(target);
            self.log(&format!("{who} is protected — the demon's kill fails"));
            return;
        }
        let reg = self.reg;
        let role = self.grim.get(target).role;
        if reg.get(role).blocks_demon_kill(self, target) {
            let who = self.label(target);
            self.log(&format!("{who} is immune — the demon's kill fails"));
            return;
        }
        let actual = reg
            .get(role)
            .redirect_death(self, target, DeathSource::Demon)
            .unwrap_or(target);
        self.apply_death(actual, DeathSource::Demon);
    }

    /// Execute `target` (day, or Virgin). Fires `on_executed` (Saint) first,
    /// then applies the death and marks them as having died today.
    pub fn execute(&mut self, target: PlayerId) {
        if !self.grim.is_alive(target) {
            return;
        }
        let reg = self.reg;
        let role = self.grim.get(target).role;
        reg.get(role).on_executed(self, target);
        self.apply_death(target, DeathSource::Execution);
    }

    /// Kill `target` via a named ability (Slayer, starpass). No protection or
    /// immunity applies — these deaths are already the result of a resolved
    /// ability.
    pub fn ability_kill(&mut self, target: PlayerId, source: &'static str) {
        self.apply_death(target, DeathSource::Ability(source));
    }

    /// Apply a death that has already been decided: flip the player dead, tag
    /// executions for the Undertaker, then fan the event out to every living
    /// ability (and the dead player's own, for the Ravenkeeper).
    fn apply_death(&mut self, dead: PlayerId, source: DeathSource) {
        if !self.grim.is_alive(dead) {
            return;
        }
        self.grim.kill(dead);
        if source.is_execution() {
            self.grim.add_token(dead, Token::DiedToday);
        }
        let who = self.label(dead);
        self.log(&format!("{who} dies"));

        let reg = self.reg;
        let seats: Vec<PlayerId> = self.grim.seats().collect();
        for obs in seats {
            // Living abilities react; a dead player reacts only to their own
            // death (so the Ravenkeeper fires while everyone else stays quiet).
            if self.grim.is_alive(obs) || obs == dead {
                let role = self.grim.get(obs).role;
                reg.get(role).on_any_death(self, obs, dead, source);
            }
        }

        // With every reaction resolved (including a Scarlet Woman becoming the
        // Demon), a board with no living Demon is a win for the good team.
        if !self.living_demon_exists() {
            self.declare_winner(Alignment::Good, "the Demon is dead");
        }
    }

    /// Transform `target` into another character (starpass, Scarlet Woman
    /// promotion). Leaves their believed role and tokens intact.
    pub fn transform(&mut self, target: PlayerId, into: CharacterInfo) {
        self.grim.transform(target, into);
        let who = self.label(target);
        self.log(&format!("{who} becomes the {}", into.name));
    }

    pub fn declare_winner(&mut self, team: Alignment, reason: &str) {
        self.grim.declare_winner(team);
        self.log(&format!("{team:?} wins — {reason}"));
    }
}
