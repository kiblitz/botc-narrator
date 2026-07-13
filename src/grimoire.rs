//! The grimoire: the storyteller's private, mutable model of the game.
//!
//! It knows seating, each player's current role/kind/alignment, life status,
//! and their reminder tokens. It performs only *mechanical* mutations — placing
//! tokens, flipping alive, transforming a role. It contains **no** character
//! rules: nothing here knows what a Soldier or Scarlet Woman is. Those
//! interactions live in the event pipeline ([`crate::ctx`]), which keeps roles
//! append-only.

use crate::ids::{CharacterId, PlayerId};
use crate::role::{Alignment, CharacterInfo, Kind};
use crate::token::Token;

/// The current game phase. Night 1 is the "first night" with its own order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Setup,
    Night(u32),
    Day(u32),
}

impl Phase {
    #[must_use]
    pub fn is_first_night(self) -> bool {
        matches!(self, Phase::Night(1))
    }
}

/// A seated player and their current state.
#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    /// The player's *true* current role. Changes on transformation (starpass,
    /// Scarlet Woman promotion).
    pub role: CharacterId,
    pub kind: Kind,
    pub alignment: Alignment,
    /// The role the player *believes* they have. Differs from `role` only for
    /// the Drunk, who thinks they are a Townsfolk. Used to slot them into the
    /// night order and to phrase their (malfunctioning) information.
    pub believed_role: CharacterId,
    pub alive: bool,
    /// Dead players retain a single "ghost" vote until they spend it.
    pub ghost_vote: bool,
    tokens: Vec<Token>,
}

impl Player {
    #[must_use]
    pub fn has_token(&self, token: Token) -> bool {
        self.tokens.contains(&token)
    }

    #[must_use]
    pub fn has_token_kind(&self, kind: Token) -> bool {
        self.tokens.iter().any(|&t| t.same_kind(kind))
    }

    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// A player is impaired (their ability malfunctions) if any token impairs
    /// them — poison or drunkenness — *or* if they merely believe they are a
    /// character they are not. The latter is exactly the Drunk: they run their
    /// believed Townsfolk's ability but it never works. Expressing it here means
    /// the Drunk needs no special case anywhere else, and any future
    /// "you-think-you-are-X" role is impaired for free.
    #[must_use]
    pub fn is_impaired(&self) -> bool {
        self.believed_role != self.role || self.tokens.iter().any(|&t| t.impairs())
    }
}

/// Specification for one player at game creation.
pub struct PlayerSpec {
    pub name: String,
    pub role: CharacterInfo,
    /// For the Drunk: the Townsfolk they believe they are. `None` means they
    /// believe their true role.
    pub believed_role: Option<CharacterInfo>,
}

impl PlayerSpec {
    #[must_use]
    pub fn new(name: impl Into<String>, role: CharacterInfo) -> Self {
        PlayerSpec {
            name: name.into(),
            role,
            believed_role: None,
        }
    }

    /// Construct a Drunk-style player who believes they are `believed`.
    #[must_use]
    pub fn believing(mut self, believed: CharacterInfo) -> Self {
        self.believed_role = Some(believed);
        self
    }
}

/// The full game state.
#[derive(Debug, Clone)]
pub struct Grimoire {
    players: Vec<Player>,
    pub phase: Phase,
    /// Set once a win condition fires (Saint executed, Demon dead with no
    /// Scarlet Woman, Mayor final-three, …). `None` while the game is live.
    pub winner: Option<Alignment>,
}

impl Grimoire {
    /// Build a grimoire from seat-ordered specs (index 0 is seat 0, clockwise).
    #[must_use]
    pub fn new(specs: Vec<PlayerSpec>) -> Self {
        let players = specs
            .into_iter()
            .enumerate()
            .map(|(i, s)| Player {
                id: PlayerId(i),
                name: s.name,
                role: s.role.id,
                kind: s.role.kind,
                alignment: s.role.alignment,
                believed_role: s.believed_role.map_or(s.role.id, |b| b.id),
                alive: true,
                ghost_vote: false,
                tokens: Vec::new(),
            })
            .collect();
        Grimoire {
            players,
            phase: Phase::Setup,
            winner: None,
        }
    }

    /// Record a game-ending result. The first declaration wins; later ones are
    /// ignored so a single resolution can't be overwritten by a cascade.
    pub fn declare_winner(&mut self, team: Alignment) {
        if self.winner.is_none() {
            self.winner = Some(team);
        }
    }

    #[must_use]
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    #[must_use]
    pub fn get(&self, id: PlayerId) -> &Player {
        &self.players[id.0]
    }

    pub fn get_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id.0]
    }

    /// All players in seat order.
    pub fn seats(&self) -> impl Iterator<Item = PlayerId> + '_ {
        (0..self.players.len()).map(PlayerId)
    }

    /// All living players in seat order.
    pub fn alive(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players.iter().filter(|p| p.alive).map(|p| p.id)
    }

    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.alive).count()
    }

    #[must_use]
    pub fn is_alive(&self, id: PlayerId) -> bool {
        self.get(id).alive
    }

    /// The two living neighbours of `id` around the ring of *living* players
    /// (dead players are skipped). Returns `None` if fewer than two others are
    /// alive to have distinct neighbours; with exactly two alive both entries
    /// are the same other player.
    #[must_use]
    pub fn alive_neighbours(&self, id: PlayerId) -> Option<(PlayerId, PlayerId)> {
        let ring: Vec<PlayerId> = self.alive().collect();
        let n = ring.len();
        if n < 2 {
            return None;
        }
        let i = ring.iter().position(|&p| p == id)?;
        let left = ring[(i + n - 1) % n];
        let right = ring[(i + 1) % n];
        Some((left, right))
    }

    /// The living player, if any, currently holding role `character`.
    #[must_use]
    pub fn find_role(&self, character: CharacterId) -> Option<PlayerId> {
        self.players
            .iter()
            .find(|p| p.role == character)
            .map(|p| p.id)
    }

    /// The player (living or dead) whose *believed* role is `character`. This
    /// is how the night order finds the Drunk when the Townsfolk they believe
    /// they are is called.
    #[must_use]
    pub fn find_believed_role(&self, character: CharacterId) -> Option<PlayerId> {
        self.players
            .iter()
            .find(|p| p.believed_role == character)
            .map(|p| p.id)
    }

    /// The set of true roles currently in play (living or dead).
    #[must_use]
    pub fn roles_in_play(&self) -> Vec<CharacterId> {
        let mut v: Vec<CharacterId> = self.players.iter().map(|p| p.role).collect();
        v.sort_by_key(|c| c.as_str());
        v.dedup();
        v
    }

    #[must_use]
    pub fn is_impaired(&self, id: PlayerId) -> bool {
        self.get(id).is_impaired()
    }

    #[must_use]
    pub fn has_ghost_vote(&self, id: PlayerId) -> bool {
        self.get(id).ghost_vote
    }

    /// The master a voting-restricted player (the Butler) must follow, if any.
    /// Read from the `Master` token, so any master-constrained role works — the
    /// voting layer never names the Butler.
    #[must_use]
    pub fn master_of(&self, id: PlayerId) -> Option<PlayerId> {
        self.get(id).tokens().iter().find_map(|&t| match t {
            Token::Master(m) => Some(m),
            _ => None,
        })
    }

    // --- Token mutation ---------------------------------------------------

    pub fn add_token(&mut self, id: PlayerId, token: Token) {
        let p = self.get_mut(id);
        if !p.tokens.contains(&token) {
            p.tokens.push(token);
        }
    }

    pub fn remove_token(&mut self, id: PlayerId, token: Token) {
        self.get_mut(id).tokens.retain(|&t| t != token);
    }

    /// Remove every token of the same variant as `kind` from `id`, ignoring
    /// payload. Used to "move" a single-instance token (e.g. re-poison).
    pub fn remove_tokens_of_kind(&mut self, id: PlayerId, kind: Token) {
        self.get_mut(id).tokens.retain(|&t| !t.same_kind(kind));
    }

    /// Remove a token variant from *every* player. Used to enforce
    /// single-target statuses like poison across the whole board.
    pub fn clear_token_kind_everywhere(&mut self, kind: Token) {
        for p in &mut self.players {
            p.tokens.retain(|&t| !t.same_kind(kind));
        }
    }

    #[must_use]
    pub fn has_token(&self, id: PlayerId, token: Token) -> bool {
        self.get(id).has_token(token)
    }

    // --- Life & role mutation --------------------------------------------

    /// Mark a player dead and grant their ghost vote. Purely mechanical: this
    /// does *not* trigger Scarlet Woman promotion, starpass, or any on-death
    /// ability — those are the event pipeline's responsibility.
    pub fn kill(&mut self, id: PlayerId) {
        let p = self.get_mut(id);
        p.alive = false;
        p.ghost_vote = true;
    }

    pub fn spend_ghost_vote(&mut self, id: PlayerId) {
        self.get_mut(id).ghost_vote = false;
    }

    /// Replace a player's current role/kind/alignment (starpass, Scarlet Woman).
    ///
    /// Their *believed* role is updated to match: a transformed player knows and
    /// acts as their new character (the new Imp wakes and kills), so the night
    /// order — which dispatches by believed role to accommodate the Drunk — must
    /// find them under the new role. Transform targets are never Drunks, so this
    /// preserves the "believed ≠ role ⟺ deluded" invariant.
    pub fn transform(&mut self, id: PlayerId, info: CharacterInfo) {
        let p = self.get_mut(id);
        p.role = info.id;
        p.kind = info.kind;
        p.alignment = info.alignment;
        p.believed_role = info.id;
    }

    // --- Phase transitions ------------------------------------------------

    /// Advance to the next phase, clearing tokens whose scope has ended.
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            Phase::Setup => Phase::Night(1),
            Phase::Night(n) => {
                // Dawn: nightly protections expire.
                self.clear_tokens(Token::clears_at_dawn);
                Phase::Day(n)
            }
            Phase::Day(n) => Phase::Night(n + 1),
        };
    }

    fn clear_tokens(&mut self, pred: impl Fn(Token) -> bool) {
        for p in &mut self.players {
            p.tokens.retain(|&t| !pred(t));
        }
    }
}
