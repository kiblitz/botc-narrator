//! The storyteller of a room: decisions go to the paired bridge (Codex on
//! someone's computer) as the Claude storyteller's prompt, and its reply is
//! parsed the same way. With no bridge, or no usable reply in time, the
//! choice is random, so the game never waits on a computer that went away.

use std::cell;
use std::rc;
use std::sync;
use std::time;

use botc_bridge::wire;
use botc_narrator::game;
use botc_narrator::storyteller;

use crate::inbox;
use crate::room;

pub struct Storyteller {
    room: sync::Arc<room::Room>,
    inbox: rc::Rc<cell::RefCell<inbox::Inbox>>,
    /// How long the bridge has for each decision.
    timeout: time::Duration,
    fallback: storyteller::Random,
    next_request: u64,
}

impl Storyteller {
    pub fn new(
        room: sync::Arc<room::Room>,
        inbox: rc::Rc<cell::RefCell<inbox::Inbox>>,
        timeout: time::Duration,
        fallback: storyteller::Random,
    ) -> Self {
        Storyteller {
            room,
            inbox,
            timeout,
            fallback,
            next_request: 1,
        }
    }

    /// The bridge's picks, or why there are none.
    fn ask_bridge(
        &mut self,
        decision: &storyteller::Decision,
        view: &game::State,
    ) -> Result<Vec<usize>, String> {
        let requests = self
            .room
            .read(|s| s.bridge.as_ref().map(|b| b.requests.clone()))
            .ok_or("no bridge is paired")?;
        let id = self.next_request;
        self.next_request += 1;
        let prompt = storyteller::claude::prompt(storyteller::claude::DEFAULT_GOAL, decision, view);
        requests
            .send(wire::ToBridge::Decide { id, prompt })
            .map_err(|_| "the bridge went away".to_string())?;
        set_thinking(&self.room, true);
        let deadline = time::Instant::now() + self.timeout;
        let reply = self
            .inbox
            .borrow_mut()
            .wait(Some(deadline), |input| match input {
                inbox::Input::Decided { request, .. } if *request == id => inbox::Verdict::Take,
                other => inbox::keep_commands(other),
            });
        set_thinking(&self.room, false);
        match reply {
            Some(inbox::Input::Decided {
                reply: Ok(text), ..
            }) => storyteller::claude::parse_picks(&text, decision),
            Some(inbox::Input::Decided { reply: Err(e), .. }) => Err(e),
            _ => Err(format!("no reply within {}s", self.timeout.as_secs())),
        }
    }
}

fn set_thinking(room: &room::Room, thinking: bool) {
    room.update(|state| {
        if let Some(game) = &mut state.game {
            game.thinking = thinking;
        }
    });
}

impl storyteller::Strategy for Storyteller {
    fn decide(&mut self, decision: &storyteller::Decision, view: &game::State) -> Vec<usize> {
        let asked = self.ask_bridge(decision, view);
        let bridged = asked.is_ok();
        self.room.update(|state| {
            if let Some(game) = &mut state.game {
                if bridged {
                    game.bridged += 1;
                } else {
                    game.fallbacks += 1;
                }
            }
        });
        match asked {
            Ok(picks) => picks,
            Err(reason) => {
                tracing::info!(room = %self.room.code, %reason, "storyteller decision chosen at random");
                self.fallback.decide(decision, view)
            }
        }
    }

    /// While a bridge is paired, its LLM reads the players' own questions.
    /// Should it fail to answer one after all, the random fallback answers.
    fn reads_text(&self) -> bool {
        self.room.read(|s| s.bridge.is_some())
    }
}
