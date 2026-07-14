//! The game thread: runs the synchronous engine to completion, blocking on
//! player commands from the network.
//!
//! The storyteller here is `AutoStoryteller<NetworkAgent, RandomDiscretion>` —
//! discretionary calls are automated, player `ask`s block on the network. The
//! engine and every ability are used unchanged; the whole multiplayer server is
//! "just another `Storyteller`."

use std::collections::HashSet;
use std::sync::mpsc::Receiver;

use tokio::sync::mpsc::UnboundedSender;

use crate::characters;
use crate::engine::Engine;
use crate::grimoire::Grimoire;
use crate::ids::PlayerId;
use crate::interp::{AutoStoryteller, PlayerAgent, RandomDiscretion, Rng};
use crate::role::Alignment;
use crate::scripts::trouble_brewing;
use crate::setup;
use crate::storyteller::Candidate;

use super::protocol::{
    ClientMsg, FinalRole, GameEvent, Opt, PlayerCmd, PublicPlayer, ServerMsg, Target,
};

/// A [`PlayerAgent`] backed by the network: it emits private events to players
/// and blocks for their answers.
pub struct NetworkAgent {
    out: UnboundedSender<GameEvent>,
    cmds: Receiver<PlayerCmd>,
    names: Vec<String>,
}

impl NetworkAgent {
    fn emit(&self, player: usize, msg: ServerMsg) {
        let _ = self.out.send(GameEvent {
            to: Target::One(player),
            msg,
        });
    }

    fn broadcast(&self, msg: ServerMsg) {
        let _ = self.out.send(GameEvent {
            to: Target::All,
            msg,
        });
    }

    /// Block until the next player command arrives.
    fn recv(&self) -> Option<PlayerCmd> {
        self.cmds.recv().ok()
    }

    fn name(&self, id: usize) -> String {
        self.names.get(id).cloned().unwrap_or_default()
    }
}

impl PlayerAgent for NetworkAgent {
    fn decide(&mut self, who: PlayerId, prompt: &str, options: &[Candidate]) -> PlayerId {
        let opts: Vec<Opt> = options
            .iter()
            .map(|c| Opt {
                id: c.id.0,
                name: self.name(c.id.0),
            })
            .collect();
        self.emit(
            who.0,
            ServerMsg::Prompt {
                prompt: prompt.to_string(),
                options: opts,
            },
        );
        // Block until this player answers with a valid option.
        loop {
            let Some(cmd) = self.recv() else {
                // Channel closed (game ending): fall back to the first option.
                return options[0].id;
            };
            if cmd.player == who.0 {
                if let ClientMsg::Answer { target } = cmd.msg {
                    if let Some(c) = options.iter().find(|c| c.id.0 == target) {
                        return c.id;
                    }
                }
            }
            // Ignore commands from other players / wrong type while waiting.
        }
    }

    fn on_wake(&mut self, who: PlayerId) {
        self.emit(who.0, ServerMsg::Wake);
    }

    fn on_sleep(&mut self, who: PlayerId) {
        self.emit(who.0, ServerMsg::Sleep);
    }

    fn on_reveal(&mut self, who: PlayerId, message: &str) {
        self.emit(
            who.0,
            ServerMsg::Info {
                message: message.to_string(),
            },
        );
    }
}

type St = AutoStoryteller<NetworkAgent, RandomDiscretion>;

/// Run one full game. Blocks until the game ends or the command channel closes.
pub fn run_game(
    names: Vec<String>,
    seed: u64,
    out: UnboundedSender<GameEvent>,
    cmds: Receiver<PlayerCmd>,
) {
    let reg = characters::registry();
    let script = trouble_brewing::script();
    let specs = setup::random_trouble_brewing(&reg, &script, &names, &mut Rng::new(seed));
    let grim = Grimoire::new(specs);
    let mut engine = Engine::new(grim, &reg, &script);

    let agent = NetworkAgent {
        out,
        cmds,
        names: names.clone(),
    };
    let mut st: St = AutoStoryteller::new(agent, RandomDiscretion::new(seed ^ 0x5EED));

    engine.run_setup(&mut st);
    send_roles(&engine.grim, &st);

    loop {
        st.agent.broadcast(ServerMsg::NightStarted);
        broadcast_public(&engine.grim, "night", &st);
        engine.run_night(&mut st);
        broadcast_public(&engine.grim, "night", &st);
        if engine.grim.winner.is_some() {
            break;
        }
        run_day(&mut engine, &mut st);
        if engine.grim.winner.is_some() {
            break;
        }
    }

    let winner = match engine.grim.winner {
        Some(Alignment::Good) => "Good",
        Some(Alignment::Evil) => "Evil",
        None => "None",
    };
    let roles: Vec<FinalRole> = engine
        .grim
        .seats()
        .map(|id| {
            let p = engine.grim.get(id);
            FinalRole {
                name: p.name.clone(),
                role: reg.info(p.role).name.to_string(),
            }
        })
        .collect();
    st.agent.broadcast(ServerMsg::GameOver {
        winner: winner.to_string(),
        roles,
    });
}

/// Tell each player their (believed) role and team privately.
fn send_roles(grim: &Grimoire, st: &St) {
    let reg = characters::registry();
    for id in grim.seats() {
        let p = grim.get(id);
        let info = reg.info(p.believed_role);
        let team = if info.alignment.is_evil() {
            "Evil"
        } else {
            "Good"
        };
        st.agent.emit(
            id.0,
            ServerMsg::Role {
                role: info.name.to_string(),
                team: team.to_string(),
            },
        );
    }
}

fn broadcast_public(grim: &Grimoire, phase: &str, st: &St) {
    let players: Vec<PublicPlayer> = grim
        .seats()
        .map(|id| {
            let p = grim.get(id);
            PublicPlayer {
                id: id.0,
                name: p.name.clone(),
                alive: p.alive,
            }
        })
        .collect();
    st.agent.broadcast(ServerMsg::Public {
        phase: phase.to_string(),
        players,
    });
}

/// The day: players nominate and vote. A nomination that reaches the threshold
/// ends the day with an execution; otherwise the day ends once every living
/// player has passed.
fn run_day(engine: &mut Engine, st: &mut St) {
    engine.begin_day();
    st.agent.broadcast(ServerMsg::DayStarted);
    broadcast_public(&engine.grim, "day", st);

    let mut nominated: HashSet<usize> = HashSet::new();
    let mut passed: HashSet<usize> = HashSet::new();

    loop {
        let Some(cmd) = st.agent.recv() else {
            return;
        };
        let actor = PlayerId(cmd.player);
        if !engine.grim.is_alive(actor) {
            continue;
        }
        match cmd.msg {
            ClientMsg::Nominate { nominee } => {
                let nominee = PlayerId(nominee);
                if nominated.contains(&cmd.player) || !engine.grim.is_alive(nominee) {
                    continue;
                }
                nominated.insert(cmd.player);
                passed.clear(); // fresh chance to pass after this resolves

                let threshold = crate::voting::threshold(engine.grim.alive_count());
                st.agent.broadcast(ServerMsg::VoteOpen {
                    nominee: nominee.0,
                    nominee_name: st.agent.name(nominee.0),
                    threshold,
                });
                let yes = collect_votes(engine, st, nominee);
                let result = engine.call_vote(st, actor, nominee, &yes);
                let executed = engine.on_the_block() == Some(nominee);
                st.agent.broadcast(ServerMsg::VoteResult {
                    nominee: nominee.0,
                    nominee_name: st.agent.name(nominee.0),
                    votes: result.votes,
                    executed,
                });
                if executed {
                    break;
                }
                broadcast_public(&engine.grim, "day", st);
            }
            ClientMsg::Pass => {
                passed.insert(cmd.player);
                if passed.len() >= engine.grim.alive_count() {
                    break;
                }
            }
            _ => {}
        }
    }

    engine.resolve_day(st);
    broadcast_public(&engine.grim, "day", st);
}

/// Collect one Yes/No vote from every living player on `nominee`.
fn collect_votes(engine: &Engine, st: &St, nominee: PlayerId) -> Vec<PlayerId> {
    let voters: Vec<usize> = engine.grim.alive().map(|p| p.0).collect();
    let needed: HashSet<usize> = voters.iter().copied().collect();
    let mut voted: HashSet<usize> = HashSet::new();
    let mut yes: Vec<PlayerId> = Vec::new();

    while voted.len() < needed.len() {
        let Some(cmd) = st.agent.recv() else {
            break;
        };
        if !needed.contains(&cmd.player) || voted.contains(&cmd.player) {
            continue;
        }
        if let ClientMsg::Vote { yes: y } = cmd.msg {
            voted.insert(cmd.player);
            if y {
                yes.push(PlayerId(cmd.player));
            }
        }
    }
    let _ = nominee;
    yes
}
