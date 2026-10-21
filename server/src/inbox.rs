//! What a room sends its game thread: players' answers and votes, the host's
//! commands and the bridge's replies, in arrival order.
//!
//! The game thread waits for one kind of input at a time (an answer to the
//! prompt it just showed, the bridge's reply to its decision). Inputs that
//! arrive meanwhile and matter later (a nomination made while the storyteller
//! is thinking) are kept for the next wait; stale ones are dropped.

use std::collections;
use std::sync::mpsc;
use std::time;

/// At most this many inputs are kept for later; the oldest go first.
const MOST_KEPT: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// A player answered prompt `prompt`.
    Answer {
        prompt: u64,
        player: String,
        answer: Answer,
    },
    Nominate {
        nominator: String,
        nominee: String,
    },
    Vote {
        player: String,
        yes: bool,
    },
    /// The host closes the vote: hands not raised are no.
    CloseVote,
    /// A player uses a public day ability (the Slayer's shot), by character id.
    DayAction {
        player: String,
        ability: String,
    },
    EndDay,
    StartNight,
    /// The bridge's reply to storyteller decision `request`, or why there is none.
    Decided {
        request: u64,
        reply: Result<String, String>,
    },
    /// The room is gone: the game stops.
    Close,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Players(Vec<String>),
    /// The index of a character option.
    Option(usize),
    YesNo(bool),
}

/// What [`Inbox::wait`] does with an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The input waited for.
    Take,
    /// Not now, but later: kept for a following wait.
    Keep,
    /// Stale or invalid: thrown away.
    Drop,
}

/// Commands are kept for later; answers and replies nobody waits for are stale.
pub fn keep_commands(input: &Input) -> Verdict {
    match input {
        Input::Answer { .. } | Input::Decided { .. } | Input::Vote { .. } | Input::CloseVote => {
            Verdict::Drop
        }
        Input::Nominate { .. }
        | Input::DayAction { .. }
        | Input::EndDay
        | Input::StartNight
        | Input::Close => Verdict::Keep,
    }
}

pub struct Inbox {
    receiver: mpsc::Receiver<Input>,
    kept: collections::VecDeque<Input>,
    closed: bool,
}

impl Inbox {
    pub fn new() -> (mpsc::Sender<Input>, Inbox) {
        let (sender, receiver) = mpsc::channel();
        let inbox = Inbox {
            receiver,
            kept: collections::VecDeque::new(),
            closed: false,
        };
        (sender, inbox)
    }

    /// Whether the room closed (or every sender went): every wait now returns
    /// `None` at once, so the game winds down without blocking.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// The first input `judge` takes, kept ones first, or `None` once
    /// `deadline` passes or the room closes.
    pub fn wait(
        &mut self,
        deadline: Option<time::Instant>,
        mut judge: impl FnMut(&Input) -> Verdict,
    ) -> Option<Input> {
        if self.closed {
            return None;
        }
        let mut i = 0;
        while i < self.kept.len() {
            match judge(&self.kept[i]) {
                Verdict::Take => return self.kept.remove(i),
                Verdict::Keep => i += 1,
                Verdict::Drop => {
                    self.kept.remove(i);
                }
            }
        }
        loop {
            let received = match deadline {
                Some(deadline) => {
                    let left = deadline.saturating_duration_since(time::Instant::now());
                    if left.is_zero() {
                        return None;
                    }
                    match self.receiver.recv_timeout(left) {
                        Ok(input) => Some(input),
                        Err(mpsc::RecvTimeoutError::Timeout) => return None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => None,
                    }
                }
                None => self.receiver.recv().ok(),
            };
            let input = match received {
                Some(Input::Close) | None => {
                    self.closed = true;
                    return None;
                }
                Some(input) => input,
            };
            match judge(&input) {
                Verdict::Take => return Some(input),
                Verdict::Keep => {
                    if self.kept.len() == MOST_KEPT {
                        // The host's phase commands are never the ones dropped.
                        let spam = self
                            .kept
                            .iter()
                            .position(|k| !matches!(k, Input::EndDay | Input::StartNight));
                        if let Some(i) = spam {
                            self.kept.remove(i);
                        }
                    }
                    self.kept.push_back(input);
                }
                Verdict::Drop => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time;

    use crate::inbox;

    fn soon() -> Option<time::Instant> {
        Some(time::Instant::now() + time::Duration::from_millis(50))
    }

    #[test]
    fn kept_inputs_wait_for_a_later_wait_and_stale_ones_go() {
        let (sender, mut inbox) = inbox::Inbox::new();
        sender.send(inbox::Input::EndDay).unwrap();
        sender
            .send(inbox::Input::Decided {
                request: 1,
                reply: Ok("stale".into()),
            })
            .unwrap();
        sender
            .send(inbox::Input::Decided {
                request: 2,
                reply: Ok("0".into()),
            })
            .unwrap();
        let taken = inbox.wait(soon(), |input| match input {
            inbox::Input::Decided { request: 2, .. } => inbox::Verdict::Take,
            other => inbox::keep_commands(other),
        });
        assert_eq!(
            taken,
            Some(inbox::Input::Decided {
                request: 2,
                reply: Ok("0".into())
            })
        );
        // The end of the day was kept; the stale reply was not.
        let mut seen = Vec::new();
        let next = inbox.wait(soon(), |input| {
            seen.push(input.clone());
            inbox::Verdict::Take
        });
        assert_eq!(next, Some(inbox::Input::EndDay));
        assert_eq!(seen, vec![inbox::Input::EndDay]);
    }

    #[test]
    fn a_wait_ends_at_its_deadline() {
        let (_sender, mut inbox) = inbox::Inbox::new();
        let started = time::Instant::now();
        assert_eq!(inbox.wait(soon(), |_| inbox::Verdict::Take), None);
        assert!(started.elapsed() >= time::Duration::from_millis(50));
        assert!(!inbox.is_closed());
    }

    #[test]
    fn closing_ends_every_wait() {
        let (sender, mut inbox) = inbox::Inbox::new();
        sender.send(inbox::Input::Close).unwrap();
        assert_eq!(inbox.wait(None, |_| inbox::Verdict::Take), None);
        assert!(inbox.is_closed());
        assert_eq!(inbox.wait(None, |_| inbox::Verdict::Take), None);
    }
}
