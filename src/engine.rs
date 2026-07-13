//! Night/day orchestration: owns the grimoire and drives it through the script.
//!
//! The engine is deliberately thin. It decides *who acts when* (night order,
//! first-night info) and exposes the day interactions (nominate, execute, day
//! abilities); all the actual rules live in the abilities it dispatches to.

use crate::ctx::Ctx;
use crate::grimoire::{Grimoire, Phase};
use crate::ids::{CharacterId, PlayerId};
use crate::registry::Registry;
use crate::role::Kind;
use crate::script::Script;
use crate::storyteller::Storyteller;
use crate::voting::{self, VoteResult};

/// The running tally for a single day: who currently holds the most valid votes
/// and whether that top spot is tied (a tie means no execution).
#[derive(Default)]
struct Block {
    highest: usize,
    on_block: Option<PlayerId>,
    tied: bool,
}

pub struct Engine<'a> {
    pub grim: Grimoire,
    reg: &'a Registry,
    script: &'a Script,
    block: Block,
}

impl<'a> Engine<'a> {
    #[must_use]
    pub fn new(grim: Grimoire, reg: &'a Registry, script: &'a Script) -> Self {
        Engine {
            grim,
            reg,
            script,
            block: Block::default(),
        }
    }

    /// Run `f` with a freshly-built [`Ctx`] borrowing this engine's grimoire and
    /// the given storyteller.
    pub fn with_ctx<R>(&mut self, st: &mut dyn Storyteller, f: impl FnOnce(&mut Ctx) -> R) -> R {
        let mut ctx = Ctx::new(&mut self.grim, st, self.reg, self.script);
        f(&mut ctx)
    }

    /// The players (living or dead) who act as `character` — those whose
    /// *believed* role matches, so the Drunk is found in their Townsfolk slot.
    fn actors_for(&self, character: CharacterId) -> Vec<PlayerId> {
        self.grim
            .seats()
            .filter(|&id| self.grim.get(id).believed_role == character)
            .collect()
    }

    // --- Setup --------------------------------------------------------------

    /// Fire every in-play role's setup hook in roster order.
    pub fn run_setup(&mut self, st: &mut dyn Storyteller) {
        let order = self.script.roster.clone();
        for cid in order {
            for actor in self.actors_for(cid) {
                let reg = self.reg;
                self.with_ctx(st, |ctx| reg.get(cid).on_setup(ctx, actor));
            }
        }
    }

    // --- Night --------------------------------------------------------------

    /// Advance the phase and run the night: first-night info if applicable,
    /// then the character wake order. Assumes setup has run.
    pub fn run_night(&mut self, st: &mut dyn Storyteller) {
        if matches!(self.grim.phase, Phase::Setup | Phase::Day(_)) {
            self.grim.advance_phase();
        }
        let Phase::Night(n) = self.grim.phase else {
            return;
        };
        if n == 1 {
            self.minion_info(st);
            self.demon_info(st);
        }
        let order = self.script.night_order(n).to_vec();
        for cid in order {
            self.act(st, cid, n);
        }
    }

    /// Run a single character's night action for whoever acts as them. Exposed
    /// for fine-grained tests that drive one role at a time.
    pub fn act(&mut self, st: &mut dyn Storyteller, character: CharacterId, night: u32) {
        for actor in self.actors_for(character) {
            let reg = self.reg;
            self.with_ctx(st, |ctx| reg.get(character).on_night(ctx, actor, night));
        }
    }

    fn minion_info(&mut self, st: &mut dyn Storyteller) {
        let minions: Vec<PlayerId> = self
            .grim
            .alive()
            .filter(|&id| self.grim.get(id).kind == Kind::Minion)
            .collect();
        if minions.is_empty() {
            return;
        }
        self.with_ctx(st, |ctx| {
            let team: Vec<String> = ctx
                .grim
                .seats()
                .filter(|&id| ctx.grim.get(id).alignment.is_evil())
                .map(|id| ctx.label(id))
                .collect();
            let msg = format!("Your evil team: {}", team.join(", "));
            for &m in &minions {
                ctx.wake(m);
            }
            for &m in &minions {
                ctx.reveal(m, &msg);
            }
            for &m in &minions {
                ctx.sleep(m);
            }
        });
    }

    fn demon_info(&mut self, st: &mut dyn Storyteller) {
        let Some(demon) = self
            .grim
            .alive()
            .find(|&id| self.grim.get(id).kind.is_demon())
        else {
            return;
        };
        self.with_ctx(st, |ctx| {
            let minions: Vec<String> = ctx
                .grim
                .seats()
                .filter(|&id| ctx.grim.get(id).kind == Kind::Minion)
                .map(|id| ctx.label(id))
                .collect();

            // Bluffs: good characters not in play; the storyteller picks three.
            let in_play = ctx.grim.roles_in_play();
            let mut pool: Vec<(CharacterId, String)> = ctx
                .script()
                .roster
                .iter()
                .filter(|&&cid| {
                    let info = ctx.registry().info(cid);
                    !info.alignment.is_evil() && !in_play.contains(&cid)
                })
                .map(|&cid| (cid, ctx.registry().info(cid).name.to_string()))
                .collect();

            ctx.wake(demon);
            if !minions.is_empty() {
                ctx.reveal(demon, &format!("Your minions: {}", minions.join(", ")));
            }
            let mut bluffs = Vec::new();
            for _ in 0..3 {
                if pool.is_empty() {
                    break;
                }
                let labels: Vec<String> = pool.iter().map(|(_, l)| l.clone()).collect();
                let i = ctx.choose("demon bluff", &labels);
                bluffs.push(pool.remove(i).1);
            }
            ctx.reveal(demon, &format!("Bluffs: {}", bluffs.join(", ")));
            ctx.sleep(demon);
        });
    }

    // --- Day interactions ---------------------------------------------------

    /// `nominator` nominates `nominee`; fires the nominee's on-nominated hook
    /// (Virgin).
    pub fn nominate(&mut self, st: &mut dyn Storyteller, nominator: PlayerId, nominee: PlayerId) {
        let reg = self.reg;
        let role = self.grim.get(nominee).role;
        self.with_ctx(st, |ctx| {
            reg.get(role).on_nominated(ctx, nominee, nominator);
        });
    }

    /// Execute `target` (the day's execution).
    pub fn execute(&mut self, st: &mut dyn Storyteller, target: PlayerId) {
        self.with_ctx(st, |ctx| ctx.execute(target));
    }

    /// End the day. With an execution, resolve it; without one, fire every
    /// living ability's no-execution hook (the Mayor's endgame).
    pub fn end_day(&mut self, st: &mut dyn Storyteller, executed: Option<PlayerId>) {
        match executed {
            Some(target) => self.execute(st, target),
            None => {
                let living: Vec<PlayerId> = self.grim.alive().collect();
                for id in living {
                    let reg = self.reg;
                    let role = self.grim.get(id).role;
                    self.with_ctx(st, |ctx| reg.get(role).on_no_execution(ctx, id));
                }
            }
        }
    }

    /// `actor` uses their once-per-day ability on `target` (Slayer shot).
    pub fn day_ability(&mut self, st: &mut dyn Storyteller, actor: PlayerId, target: PlayerId) {
        let reg = self.reg;
        let role = self.grim.get(actor).believed_role;
        self.with_ctx(st, |ctx| reg.get(role).on_day_ability(ctx, actor, target));
    }

    // --- Voting -------------------------------------------------------------

    /// Advance from night into the day and clear the day's vote tally.
    pub fn begin_day(&mut self) {
        if matches!(self.grim.phase, Phase::Night(_)) {
            self.grim.advance_phase();
        }
        self.block = Block::default();
    }

    /// The player currently facing execution (highest un-tied vote count).
    #[must_use]
    pub fn on_the_block(&self) -> Option<PlayerId> {
        if self.block.tied {
            None
        } else {
            self.block.on_block
        }
    }

    /// Run a nomination and its vote. Fires the nominee's on-nominated hook
    /// (Virgin) first; if they survive, tallies `raised_hands`, spends any ghost
    /// votes used, and updates who is on the block. Returns the tally.
    pub fn call_vote(
        &mut self,
        st: &mut dyn Storyteller,
        nominator: PlayerId,
        nominee: PlayerId,
        raised_hands: &[PlayerId],
    ) -> VoteResult {
        self.nominate(st, nominator, nominee);

        let result = voting::tally(&self.grim, nominee, raised_hands);
        // Dead voters spend their ghost vote whether or not the nomination passes.
        let spent: Vec<PlayerId> = result.ghost_voters(&self.grim).collect();
        for g in spent {
            self.grim.spend_ghost_vote(g);
        }

        if self.grim.is_alive(nominee) && result.reached() {
            if result.votes > self.block.highest {
                self.block.highest = result.votes;
                self.block.on_block = Some(nominee);
                self.block.tied = false;
            } else if result.votes == self.block.highest {
                // Ties the current leader: nobody is executed unless broken.
                self.block.tied = true;
            }
        }
        result
    }

    /// Resolve the day: execute whoever is on the block, or fire the
    /// no-execution hooks (Mayor) on a tie or an empty block. Returns the
    /// executed player, if any.
    pub fn resolve_day(&mut self, st: &mut dyn Storyteller) -> Option<PlayerId> {
        let executed = self.on_the_block();
        self.end_day(st, executed);
        executed
    }
}
