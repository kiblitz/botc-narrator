//! Rooms: who sits where, the host, the paired bridge, and the game's state as
//! the players see it. Rooms live in memory only; a restart ends every game.
//!
//! The game itself runs on its own thread (`engine`), which writes what the
//! table should see here through [`Room::update`]; every connected phone is
//! told something changed and builds its own view (`view`).

use std::collections::HashMap;
use std::sync;
use std::sync::mpsc;
use std::time;

use botc_bridge::wire;
use botc_narrator::character;
use botc_narrator::game;
use botc_narrator::script;
use botc_narrator::sim;
use rand::Rng;
use tokio::sync::watch;

use crate::inbox;

/// Room codes: short, easy to read out, no look-alike letters.
const CODE_LETTERS: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
const CODE_LENGTH: usize = 5;
/// The bridge's pairing code: long enough that it cannot be guessed, since
/// the bridge sees the whole grimoire. Shown to the host only.
const PAIRING_LETTERS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const PAIRING_GROUPS: usize = 4;
const PAIRING_GROUP_LENGTH: usize = 4;
/// Player tokens: 128 random bits, hex.
const TOKEN_BYTES: usize = 16;

pub const MOST_ROOMS: usize = 64;
/// Rooms one client address may host at once. Hosting needs no account, so
/// this, with [`LOBBY_IDLE`], keeps one visitor from taking every room.
pub const MOST_ROOMS_PER_CLIENT: usize = 3;
/// Phones that may be connected as one seat at a time.
pub const MOST_PHONES_PER_SEAT: usize = 3;
/// More than any script seats, so a few spare phones can join and be removed.
pub const MOST_SEATS: usize = 20;
pub const MOST_NAME_CHARS: usize = 20;
/// A room nobody has touched for this long is closed.
pub const IDLE: time::Duration = time::Duration::from_secs(4 * 60 * 60);
/// A room whose game never started is closed after this long untouched.
pub const LOBBY_IDLE: time::Duration = time::Duration::from_secs(60 * 60);
/// However busy, a room is closed this long after it opened, so an anonymous
/// host cannot keep rooms forever by touching them.
pub const MOST_LIFETIME: time::Duration = time::Duration::from_secs(12 * 60 * 60);
/// A finished game's room is closed this long after its last change.
pub const FINISHED: time::Duration = time::Duration::from_secs(30 * 60);
pub const DEFAULT_SCRIPT: &str = "trouble_brewing";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NoSuchRoom,
    TooManyRooms,
    /// This client already hosts [`MOST_ROOMS_PER_CLIENT`] rooms.
    HostingTooMany,
    Started,
    Full,
    BadName,
    NameTaken,
    NotHost,
    NoSuchScript,
    /// The seating is not a reordering of the players in the room.
    BadSeating,
    /// Not a player count the script can seat.
    PlayerCount {
        min: usize,
        max: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoSuchRoom => f.write_str("There is no room with that code."),
            Error::TooManyRooms => f.write_str("Too many rooms are open; try again later."),
            Error::HostingTooMany => {
                f.write_str("Too many rooms are hosted from your network; finish one or wait.")
            }
            Error::Started => f.write_str("That game has already started."),
            Error::Full => f.write_str("That room is full."),
            Error::BadName => write!(f, "Names are 1 to {MOST_NAME_CHARS} characters."),
            Error::NameTaken => f.write_str("Someone in the room already has that name."),
            Error::NotHost => f.write_str("Only the host can do that."),
            Error::NoSuchScript => f.write_str("Unknown script."),
            Error::BadSeating => f.write_str("The seating changed meanwhile; try again."),
            Error::PlayerCount { min, max } => {
                write!(f, "This script needs {min} to {max} players.")
            }
        }
    }
}

/// Every open room, by code.
#[derive(Default)]
pub struct Rooms {
    rooms: sync::Mutex<HashMap<String, sync::Arc<Room>>>,
}

impl Rooms {
    /// Opens a room hosted from the address `client` (no account), whose
    /// host sits in it as `name`. Returns the room and the host's player
    /// token, which is all that makes them the host when they reconnect.
    pub fn open(&self, client: &str, name: &str) -> Result<(sync::Arc<Room>, String), Error> {
        let name = valid_name(name)?;
        let mut rooms = self.lock();
        let hosted = rooms
            .values()
            .filter(|r| r.read(|s| s.host_client == client))
            .count();
        if hosted >= MOST_ROOMS_PER_CLIENT {
            return Err(Error::HostingTooMany);
        }
        if rooms.len() >= MOST_ROOMS {
            return Err(Error::TooManyRooms);
        }
        let code = loop {
            let code = random_code(CODE_LETTERS, CODE_LENGTH);
            if !rooms.contains_key(&code) {
                break code;
            }
        };
        let token = random_token();
        let state = State {
            host: name.clone(),
            host_client: client.to_string(),
            seats: vec![Seat {
                name,
                token: token.clone(),
                connections: 0,
            }],
            script: DEFAULT_SCRIPT,
            pairing: random_pairing(),
            bridge: None,
            game: None,
            opened: time::Instant::now(),
            touched: time::Instant::now(),
            closed: false,
        };
        let room = sync::Arc::new(Room::new(code.clone(), state));
        rooms.insert(code, sync::Arc::clone(&room));
        Ok((room, token))
    }

    pub fn get(&self, code: &str) -> Option<sync::Arc<Room>> {
        self.lock().get(&code.trim().to_ascii_uppercase()).cloned()
    }

    /// The room a bridge pairing code belongs to.
    pub fn paired(&self, pairing: &str) -> Option<sync::Arc<Room>> {
        let pairing = normalise_pairing(pairing);
        self.lock()
            .values()
            .find(|r| r.read(|s| constant_time_eq(&s.pairing, &pairing)))
            .cloned()
    }

    /// Closes rooms idle for too long, or open past [`MOST_LIFETIME`];
    /// returns how many.
    pub fn sweep(&self, now: time::Instant) -> usize {
        let mut rooms = self.lock();
        let stale: Vec<String> = rooms
            .values()
            .filter(|r| {
                r.read(|s| {
                    now.saturating_duration_since(s.touched) > s.lifetime()
                        || now.saturating_duration_since(s.opened) > MOST_LIFETIME
                })
            })
            .map(|r| r.code.clone())
            .collect();
        for code in &stale {
            if let Some(room) = rooms.remove(code) {
                tracing::info!(room = %code, "closing idle room");
                room.close();
            }
        }
        stale.len()
    }

    pub fn count(&self) -> usize {
        self.lock().len()
    }

    fn lock(&self) -> sync::MutexGuard<'_, HashMap<String, sync::Arc<Room>>> {
        self.rooms
            .lock()
            .unwrap_or_else(sync::PoisonError::into_inner)
    }
}

pub struct Room {
    pub code: String,
    state: sync::Mutex<State>,
    changed: watch::Sender<u64>,
}

impl Room {
    fn new(code: String, state: State) -> Self {
        let (changed, _) = watch::channel(0);
        Room {
            code,
            state: sync::Mutex::new(state),
            changed,
        }
    }

    /// Changes the room and tells every connected phone.
    pub fn update<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        let result = {
            let mut state = self.lock();
            state.touched = time::Instant::now();
            f(&mut state)
        };
        self.changed.send_modify(|version| *version += 1);
        result
    }

    pub fn read<R>(&self, f: impl FnOnce(&State) -> R) -> R {
        f(&self.lock())
    }

    /// Wakes whenever the room changes.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    /// Stops the game and drops the bridge.
    pub fn close(&self) {
        self.update(|state| {
            if let Some(game) = &state.game {
                let _ = game.inbox.send(inbox::Input::Close);
            }
            state.bridge = None;
            state.closed = true;
        });
    }

    /// Sends the game thread an input; false if no game is running.
    pub fn send(&self, input: inbox::Input) -> bool {
        self.read(|state| {
            state
                .game
                .as_ref()
                .is_some_and(|game| game.inbox.send(input).is_ok())
        })
    }

    fn lock(&self) -> sync::MutexGuard<'_, State> {
        // A panic while holding the lock leaves the state as it was written
        // so far, which is still a room: keep serving it.
        self.state
            .lock()
            .unwrap_or_else(sync::PoisonError::into_inner)
    }
}

pub struct State {
    /// The host's seat name.
    pub host: String,
    /// The address the room was hosted from, to cap rooms per client.
    pub host_client: String,
    /// In seat order, clockwise.
    pub seats: Vec<Seat>,
    /// The script's id.
    pub script: &'static str,
    pub pairing: String,
    pub bridge: Option<Bridge>,
    pub game: Option<Game>,
    pub opened: time::Instant,
    pub touched: time::Instant,
    /// The room was closed (replaced or idle); phones are told to leave.
    pub closed: bool,
}

pub struct Seat {
    pub name: String,
    pub token: String,
    /// Phones connected as this player now.
    pub connections: usize,
}

/// The paired bridge: where storyteller decisions go.
pub struct Bridge {
    /// Tells this connection from a later one that replaced it.
    pub connection: u64,
    pub requests: tokio::sync::mpsc::UnboundedSender<wire::ToBridge>,
}

impl State {
    /// The seat whose token this is.
    pub fn seat_of(&self, token: &str) -> Option<&Seat> {
        self.seats
            .iter()
            .find(|s| constant_time_eq(&s.token, token))
    }

    pub fn is_host(&self, name: &str) -> bool {
        self.host == name
    }

    fn lifetime(&self) -> time::Duration {
        match &self.game {
            Some(game) if game.stage == Stage::Over => FINISHED,
            Some(_) => IDLE,
            None => LOBBY_IDLE,
        }
    }

    /// Seats a new player in the lobby; returns their token.
    pub fn join(&mut self, name: &str) -> Result<String, Error> {
        if self.game.is_some() {
            return Err(Error::Started);
        }
        let name = valid_name(name)?;
        if self.seats.len() >= MOST_SEATS {
            return Err(Error::Full);
        }
        if self
            .seats
            .iter()
            .any(|s| s.name.to_lowercase() == name.to_lowercase())
        {
            return Err(Error::NameTaken);
        }
        let token = random_token();
        self.seats.push(Seat {
            name,
            token: token.clone(),
            connections: 0,
        });
        Ok(token)
    }

    /// The host reorders the circle: `order` must name every seated player once.
    pub fn reseat(&mut self, order: &[String]) -> Result<(), Error> {
        if self.game.is_some() {
            return Err(Error::Started);
        }
        let same = order.len() == self.seats.len()
            && self
                .seats
                .iter()
                .all(|s| order.iter().filter(|n| **n == s.name).count() == 1);
        if !same {
            return Err(Error::BadSeating);
        }
        self.seats
            .sort_by_key(|s| order.iter().position(|n| *n == s.name));
        Ok(())
    }

    /// The host removes a player from the lobby (never themself).
    pub fn remove(&mut self, name: &str) -> Result<(), Error> {
        if self.game.is_some() {
            return Err(Error::Started);
        }
        if name == self.host {
            return Err(Error::NotHost);
        }
        self.seats.retain(|s| s.name != name);
        Ok(())
    }

    pub fn choose_script(&mut self, id: &str) -> Result<(), Error> {
        if self.game.is_some() {
            return Err(Error::Started);
        }
        let script = script::by_id(id).ok_or(Error::NoSuchScript)?;
        self.script = script.id;
        Ok(())
    }

    /// The chosen script, if the room's players can start it.
    pub fn startable(&self) -> Result<script::Script, Error> {
        if self.game.is_some() {
            return Err(Error::Started);
        }
        let script = script::by_id(self.script).ok_or(Error::NoSuchScript)?;
        let max = most_players(script.id);
        let min = sim::MIN_PLAYERS;
        if !(min..=max).contains(&self.seats.len()) {
            return Err(Error::PlayerCount { min, max });
        }
        Ok(script)
    }

    /// A fresh pairing code; the paired bridge is dropped.
    pub fn new_pairing(&mut self) {
        self.pairing = random_pairing();
        self.bridge = None;
    }
}

/// The largest game `script_id` can seat (computed once per script).
pub fn most_players(script_id: &str) -> usize {
    static MOST: sync::OnceLock<HashMap<&'static str, usize>> = sync::OnceLock::new();
    MOST.get_or_init(|| {
        script::all()
            .iter()
            .map(|s| (s.id, sim::max_players(s)))
            .collect()
    })
    .get(script_id)
    .copied()
    .unwrap_or(sim::MIN_PLAYERS)
}

/// The game as the table sees it, written by the game thread.
pub struct Game {
    pub inbox: mpsc::Sender<inbox::Input>,
    pub stage: Stage,
    pub phase: game::Phase,
    /// The grimoire after the latest event. It reaches a phone only through
    /// `view`, filtered to what that player may see.
    pub grimoire: game::State,
    pub log: Vec<Entry>,
    /// Who the table knows is dead: a night death only once it is announced at dawn.
    pub known_dead: Vec<String>,
    /// What each player is being asked now; at most one each.
    pub prompts: Vec<Prompt>,
    /// Players awake now (by day: privately visiting the storyteller).
    pub awake: Vec<String>,
    pub vote: Option<Vote>,
    /// Today's nominators and nominees: each nominates and is nominated once a day.
    pub nominators: Vec<String>,
    pub nominees: Vec<String>,
    /// Who may nominate now.
    pub may_nominate: Vec<String>,
    /// Public day abilities players may use now.
    pub day_actions: Vec<DayAction>,
    /// The storyteller is deciding something.
    pub thinking: bool,
    /// The host asked to end the day / start the night; set until the game
    /// thread acts, so a second tap does nothing.
    pub end_requested: bool,
    pub night_requested: bool,
    pub winner: Option<character::Alignment>,
    /// Why the game stopped, if it broke.
    pub failure: Option<String>,
    /// Storyteller decisions answered by the bridge, and by the random fallback.
    pub bridged: usize,
    pub fallbacks: usize,
}

impl Game {
    pub fn new(inbox: mpsc::Sender<inbox::Input>, grimoire: game::State) -> Self {
        Game {
            inbox,
            stage: Stage::Night,
            phase: game::Phase::Setup,
            grimoire,
            log: Vec::new(),
            known_dead: Vec::new(),
            prompts: Vec::new(),
            awake: Vec::new(),
            vote: None,
            nominators: Vec::new(),
            nominees: Vec::new(),
            may_nominate: Vec::new(),
            day_actions: Vec::new(),
            thinking: false,
            end_requested: false,
            night_requested: false,
            winner: None,
            failure: None,
            bridged: 0,
            fallbacks: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Night,
    /// Nominations are open.
    Day,
    Vote,
    /// The day is over: waiting for the host to start the night.
    Dusk,
    Over,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum Audience {
    Public,
    Player(String),
    /// Only the storyteller knows; everyone sees it once the game is over.
    Storyteller,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Entry {
    pub phase: game::Phase,
    pub audience: Audience,
    pub text: String,
    /// Players and characters the text names, for highlighting.
    pub players: Vec<String>,
    pub characters: Vec<&'static str>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Prompt {
    pub id: u64,
    pub player: String,
    pub text: String,
    #[serde(flatten)]
    pub kind: PromptKind,
    /// Seconds the player had when the prompt was shown; the default answer
    /// is taken then.
    pub seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PromptKind {
    Players {
        candidates: Vec<String>,
        min: usize,
        max: usize,
    },
    Character {
        options: Vec<&'static str>,
    },
    YesNo,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Vote {
    pub nominee: String,
    pub nominator: Option<String>,
    pub needed: usize,
    /// Living players, and dead ones with their ghost vote.
    pub eligible: Vec<String>,
    /// Hands so far: (player, raised).
    pub hands: Vec<(String, bool)>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DayAction {
    pub player: String,
    /// The character id, sent back to use it.
    pub ability: &'static str,
    pub name: &'static str,
}

/// A name with its whitespace collapsed, if its length and characters are fine.
pub fn valid_name(name: &str) -> Result<String, Error> {
    let name: String = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars = name.chars().count();
    if chars == 0 || chars > MOST_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(Error::BadName);
    }
    Ok(name)
}

fn random_code(letters: &[u8], length: usize) -> String {
    let mut rng = rand::rng();
    (0..length)
        .map(|_| letters[rng.random_range(0..letters.len())] as char)
        .collect()
}

fn random_pairing() -> String {
    (0..PAIRING_GROUPS)
        .map(|_| random_code(PAIRING_LETTERS, PAIRING_GROUP_LENGTH))
        .collect::<Vec<_>>()
        .join("-")
}

/// A pairing code as typed: case and separators do not matter.
fn normalise_pairing(pairing: &str) -> String {
    let letters: Vec<char> = pairing
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    letters
        .chunks(PAIRING_GROUP_LENGTH)
        .map(|g| g.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

fn random_token() -> String {
    let bytes: [u8; TOKEN_BYTES] = rand::rng().random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Compares secrets without stopping at the first difference.
fn constant_time_eq(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

#[cfg(test)]
mod tests {
    use crate::room;

    fn lobby() -> (room::Rooms, std::sync::Arc<room::Room>) {
        let rooms = room::Rooms::default();
        let (room, _) = rooms.open("203.0.113.9", "Ada").unwrap();
        for name in ["Bram", "Cleo", "Dev", "Esme"] {
            room.update(|s| s.join(name)).unwrap();
        }
        (rooms, room)
    }

    fn names(room: &room::Room) -> Vec<String> {
        room.read(|s| s.seats.iter().map(|s| s.name.clone()).collect())
    }

    #[test]
    fn players_join_by_code_with_unique_names() {
        let (rooms, room) = lobby();
        let found = rooms.get(&room.code.to_lowercase()).unwrap();
        assert_eq!(found.code, room.code);
        assert_eq!(room.update(|s| s.join("bram")), Err(room::Error::NameTaken));
        assert_eq!(room.update(|s| s.join("   ")), Err(room::Error::BadName));
        assert_eq!(
            room.update(|s| s.join("A name far longer than twenty")),
            Err(room::Error::BadName)
        );
        assert_eq!(names(&room), ["Ada", "Bram", "Cleo", "Dev", "Esme"]);
        let token = room.update(|s| s.join("  Finn   the  Fox ")).unwrap();
        assert_eq!(
            room.read(|s| s.seat_of(&token).map(|s| s.name.clone())),
            Some("Finn the Fox".to_string())
        );
    }

    #[test]
    fn the_host_reorders_the_circle_with_every_player_once() {
        let (_rooms, room) = lobby();
        let order: Vec<String> = ["Esme", "Ada", "Dev", "Bram", "Cleo"]
            .map(String::from)
            .to_vec();
        room.update(|s| s.reseat(&order)).unwrap();
        assert_eq!(names(&room), order);
        let missing: Vec<String> = ["Esme", "Ada", "Dev", "Bram"].map(String::from).to_vec();
        assert_eq!(
            room.update(|s| s.reseat(&missing)),
            Err(room::Error::BadSeating)
        );
        let twice: Vec<String> = ["Esme", "Esme", "Dev", "Bram", "Cleo"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            room.update(|s| s.reseat(&twice)),
            Err(room::Error::BadSeating)
        );
        assert_eq!(names(&room), order);
    }

    #[test]
    fn a_script_needs_enough_players() {
        let (_rooms, room) = lobby();
        assert!(room.read(|s| s.startable()).is_ok());
        room.update(|s| s.remove("Esme")).unwrap();
        assert!(matches!(
            room.read(|s| s.startable()),
            Err(room::Error::PlayerCount { min: 5, .. })
        ));
        assert_eq!(room.update(|s| s.remove("Ada")), Err(room::Error::NotHost));
    }

    #[test]
    fn a_bridge_pairs_by_its_code_however_it_is_typed() {
        let (rooms, room) = lobby();
        let pairing = room.read(|s| s.pairing.clone());
        assert_eq!(pairing.len(), 19);
        let typed = pairing.replace('-', " ").to_lowercase();
        assert_eq!(rooms.paired(&typed).unwrap().code, room.code);
        room.update(|s| s.new_pairing());
        assert!(rooms.paired(&pairing).is_none());
        assert!(rooms.paired("").is_none());
    }

    #[test]
    fn a_client_hosts_a_few_rooms_at_most() {
        let rooms = room::Rooms::default();
        for _ in 0..room::MOST_ROOMS_PER_CLIENT {
            rooms.open("203.0.113.9", "Ada").unwrap();
        }
        assert_eq!(
            rooms.open("203.0.113.9", "Ada").err(),
            Some(room::Error::HostingTooMany)
        );
        assert!(rooms.open("198.51.100.7", "Bram").is_ok());
    }

    #[test]
    fn lobbies_close_when_idle_and_every_room_when_too_old() {
        let (rooms, idle) = lobby();
        let now = std::time::Instant::now();
        let second = std::time::Duration::from_secs(1);
        assert_eq!(rooms.sweep(now + room::LOBBY_IDLE - second), 0);
        assert_eq!(rooms.sweep(now + room::LOBBY_IDLE + second), 1);
        assert!(idle.read(|s| s.closed));

        // However busy a room is, it closes once it is too old.
        let (rooms, busy) = lobby();
        busy.update(|s| s.touched = now + room::MOST_LIFETIME);
        assert_eq!(rooms.sweep(now + room::MOST_LIFETIME + second), 1);
        assert!(busy.read(|s| s.closed));
    }
}
