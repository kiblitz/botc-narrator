//! What one phone is shown: the room as that player may see it. Other
//! players' characters, the storyteller's log and the grimoire stay hidden
//! until the game is over; the pairing code is the host's alone.

use botc_narrator::character;
use botc_narrator::game;
use botc_narrator::script;

use crate::room;

#[derive(serde::Serialize)]
pub struct View {
    pub room: String,
    pub you: String,
    pub host: bool,
    /// "lobby" before the game, then the game's stage.
    pub stage: &'static str,
    pub phase: Option<String>,
    pub script: ScriptChoice,
    /// Lobby, for the host: the scripts to choose from.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<ScriptChoice>,
    pub seats: Vec<Seat>,
    pub me: Option<Me>,
    pub log: Vec<room::Entry>,
    pub prompt: Option<room::Prompt>,
    pub vote: Option<Vote>,
    pub may_nominate: bool,
    /// Players this player may nominate now.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nominees: Vec<String>,
    pub day_actions: Vec<room::DayAction>,
    pub thinking: bool,
    pub bridge: Bridge,
    pub winner: Option<character::Alignment>,
    pub failure: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct ScriptChoice {
    pub id: &'static str,
    pub name: String,
    pub max_players: usize,
}

#[derive(serde::Serialize)]
pub struct Seat {
    pub name: String,
    pub connected: bool,
    pub host: bool,
    pub alive: bool,
    pub ghost_vote: bool,
    /// Only once the game is over (and your own, in `me`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<character::Alignment>,
}

#[derive(serde::Serialize)]
pub struct Me {
    /// The character you believe you are (a Drunk sees their Townsfolk).
    pub character: &'static str,
    pub kind: character::Kind,
    pub alignment: character::Alignment,
    pub alive: bool,
    pub awake: bool,
}

#[derive(serde::Serialize)]
pub struct Vote {
    pub nominee: String,
    pub nominator: Option<String>,
    pub needed: usize,
    pub eligible: bool,
    pub hands: Vec<(String, bool)>,
}

#[derive(serde::Serialize)]
pub struct Bridge {
    pub connected: bool,
    /// The host's alone.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairing: Option<String>,
    pub bridged: usize,
    pub fallbacks: usize,
}

fn choice(script: &script::Script) -> ScriptChoice {
    ScriptChoice {
        id: script.id,
        name: script.name.clone(),
        max_players: room::most_players(script.id),
    }
}

/// Every registered script, for the host's picker.
pub fn scripts() -> Vec<ScriptChoice> {
    script::all().iter().map(choice).collect()
}

/// The room as `you` (a seated player's name) may see it.
pub fn of(code: &str, state: &room::State, you: &str) -> View {
    let host = state.is_host(you);
    let script = script::by_id(state.script)
        .map(|s| choice(&s))
        .unwrap_or(ScriptChoice {
            id: state.script,
            name: state.script.to_string(),
            max_players: 0,
        });
    let game = state.game.as_ref();
    let over = game.is_some_and(|g| g.stage == room::Stage::Over);
    // Deaths show only once the table is told (night deaths at dawn); after
    // the game, the grimoire says.
    let known_dead = |name: &str| match game {
        Some(g) if over => g
            .grimoire
            .players()
            .iter()
            .any(|p| p.id.as_str() == name && !p.alive),
        Some(g) => g.known_dead.iter().any(|d| d == name),
        None => false,
    };
    let seats = state
        .seats
        .iter()
        .map(|seat| {
            let player = game.and_then(|g| {
                g.grimoire
                    .players()
                    .iter()
                    .find(|p| p.id.as_str() == seat.name)
            });
            Seat {
                name: seat.name.clone(),
                connected: seat.connections > 0,
                host: state.is_host(&seat.name),
                alive: !known_dead(&seat.name),
                ghost_vote: known_dead(&seat.name) && player.is_some_and(|p| p.ghost_vote),
                character: player.filter(|_| over).map(|p| p.character.name()),
                alignment: player.filter(|_| over).map(|p| p.alignment),
            }
        })
        .collect();
    let me = game.and_then(|g| {
        let p = g.grimoire.players().iter().find(|p| p.id.as_str() == you)?;
        let believed = p.believed();
        let alignment = match p.believes {
            Some(c) => c.kind().alignment().unwrap_or(p.alignment),
            None => p.alignment,
        };
        Some(Me {
            character: believed.name(),
            kind: believed.kind(),
            alignment,
            alive: !known_dead(you),
            awake: g.awake.iter().any(|a| a == you),
        })
    });
    let log = game
        .map(|g| {
            g.log
                .iter()
                .filter(|e| match &e.audience {
                    room::Audience::Public => true,
                    room::Audience::Player(p) => p == you,
                    room::Audience::Storyteller => over,
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let may_nominate = game
        .is_some_and(|g| g.stage == room::Stage::Day && g.may_nominate.iter().any(|p| p == you));
    let nominees = match game {
        Some(g) if may_nominate => state
            .seats
            .iter()
            .map(|s| s.name.clone())
            .filter(|n| !g.nominees.contains(n))
            .collect(),
        _ => Vec::new(),
    };
    View {
        room: code.to_string(),
        you: you.to_string(),
        host,
        stage: match game.map(|g| g.stage) {
            None => "lobby",
            Some(room::Stage::Night) => "night",
            Some(room::Stage::Day) => "day",
            Some(room::Stage::Vote) => "vote",
            Some(room::Stage::Dusk) => "dusk",
            Some(room::Stage::Over) => "over",
        },
        phase: game
            .map(|g| g.phase)
            .filter(|p| *p != game::Phase::Setup)
            .map(|p| p.to_string()),
        script,
        scripts: if host && game.is_none() {
            scripts()
        } else {
            Vec::new()
        },
        seats,
        me,
        log,
        prompt: game.and_then(|g| g.prompts.iter().find(|p| p.player == you).cloned()),
        vote: game.and_then(|g| g.vote.as_ref()).map(|v| Vote {
            nominee: v.nominee.clone(),
            nominator: v.nominator.clone(),
            needed: v.needed,
            eligible: v.eligible.iter().any(|e| e == you),
            hands: v.hands.clone(),
        }),
        may_nominate,
        nominees,
        day_actions: game
            .filter(|g| g.stage == room::Stage::Day)
            .map(|g| {
                g.day_actions
                    .iter()
                    .filter(|a| a.player == you)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default(),
        thinking: game.is_some_and(|g| g.thinking),
        bridge: Bridge {
            connected: state.bridge.is_some(),
            pairing: host.then(|| state.pairing.clone()),
            bridged: game.map_or(0, |g| g.bridged),
            fallbacks: game.map_or(0, |g| g.fallbacks),
        },
        winner: game.and_then(|g| g.winner),
        failure: game.and_then(|g| g.failure.clone()),
    }
}
