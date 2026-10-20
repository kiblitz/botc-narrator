//! What an ability can do. [`Read`] can talk to players and the storyteller
//! but cannot change the grimoire; [`Write`] can also change it, and resolves
//! rule interactions (protection, redirects, death triggers, win checks).

use crate::character;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;
use crate::table;

pub struct Read<'a> {
    state: &'a game::State,
    io: &'a mut dyn table::Io,
    storyteller: &'a mut dyn storyteller::Strategy,
}

impl<'a> Read<'a> {
    pub fn new(
        state: &'a game::State,
        io: &'a mut dyn table::Io,
        storyteller: &'a mut dyn storyteller::Strategy,
    ) -> Self {
        Self {
            state,
            io,
            storyteller,
        }
    }

    pub fn state(&self) -> &game::State {
        self.state
    }

    pub fn wake(&mut self, player: &player::Id) {
        self.io.wake(player);
    }

    pub fn sleep(&mut self, player: &player::Id) {
        self.io.sleep(player);
    }

    pub fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.io.tell(player, message);
    }

    /// Ask a player to point at `count` distinct candidates.
    pub fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        let count = count.min(candidates.len());
        if count == 0 {
            return Vec::new();
        }
        let answer = self.io.ask(player, prompt, candidates, count);
        if storyteller::is_selection(&answer, count, |a| candidates.contains(a)) {
            answer
        } else {
            candidates[..count].to_vec()
        }
    }

    pub fn ask_one(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
    ) -> Option<player::Id> {
        self.ask(player, prompt, candidates, 1).into_iter().next()
    }

    /// Ask a player to choose one of `options` (characters).
    pub fn ask_character(
        &mut self,
        player: &player::Id,
        prompt: &str,
        options: &[character::Ref],
    ) -> Option<character::Ref> {
        if options.is_empty() {
            return None;
        }
        let answer = self.io.ask_character(player, prompt, options);
        Some(options.get(answer).copied().unwrap_or(options[0]))
    }

    /// Let the storyteller choose `count` of `options`; `describe` says how
    /// each option is presented. When there is no real choice (as many options
    /// as picks) the storyteller is not consulted.
    pub fn choose<T: Clone>(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<T>,
        count: usize,
        describe: impl Fn(&T) -> storyteller::Choice,
    ) -> Vec<T> {
        let count = count.min(options.len());
        if count == 0 {
            return Vec::new();
        }
        let decision = storyteller::Decision {
            reason,
            options: options.iter().map(describe).collect(),
            count,
        };
        let picks = if options.len() == count {
            (0..count).collect()
        } else {
            let picks = self.storyteller.decide(&decision, self.state);
            if decision.is_valid(&picks) {
                picks
            } else {
                (0..count).collect()
            }
        };
        self.io.decided(&decision, &picks);
        picks.into_iter().map(|i| options[i].clone()).collect()
    }

    pub fn choose_player(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<player::Id>,
    ) -> Option<player::Id> {
        self.choose(reason, options, 1, |p| {
            storyteller::Choice::Player(p.clone())
        })
        .into_iter()
        .next()
    }

    /// Give `player` information. The storyteller picks one of the `truthful`
    /// messages; if the player is malfunctioning, `misinformation` is legal too.
    pub fn inform(
        &mut self,
        player: &player::Id,
        truthful: Vec<info::Message>,
        misinformation: Vec<info::Message>,
    ) {
        let malfunctioning = self.state.is_malfunctioning(player);
        let mut options: Vec<(info::Message, bool)> =
            truthful.into_iter().map(|m| (m, true)).collect();
        if malfunctioning {
            for message in misinformation {
                if !options.iter().any(|(m, _)| *m == message) {
                    options.push((message, false));
                }
            }
        }
        let reason = storyteller::Reason::Information {
            player: player.clone(),
            character: self.state.believed(player).name(),
            malfunctioning,
        };
        let chosen = self.choose(reason, options, 1, |(message, truthful)| {
            storyteller::Choice::Info {
                message: message.clone(),
                truthful: *truthful,
            }
        });
        if let Some((message, _)) = chosen.into_iter().next() {
            self.tell(player, &message);
        }
    }

    // Registration: information abilities learn about other players through
    // these, never from the grimoire directly, so a character that may
    // misregister (`character::Def::misregistration`) is handled centrally.
    // The truthful answer is always the first option; the storyteller is only
    // asked when a misregistration would change the answer.

    /// The alignment `target` registers as to `asker`'s ability.
    pub fn registered_alignment(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
    ) -> character::Alignment {
        let truth = self.state.alignment(target);
        let mut options = vec![truth];
        if let Some(m) = self.misregistration(target)
            && m.alignment != truth
        {
            options.push(m.alignment);
        }
        self.register(asker, target, options, |a| {
            storyteller::Choice::Alignment(*a)
        })
    }

    pub fn registers_evil(&mut self, asker: &player::Id, target: &player::Id) -> bool {
        self.registered_alignment(asker, target) == character::Alignment::Evil
    }

    /// The character `target` registers as to `asker`'s ability: their own,
    /// or an in-play character of a kind they may misregister as.
    pub fn registered_character(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
    ) -> character::Ref {
        let mut options = vec![self.state.character(target)];
        if let Some(m) = self.misregistration(target) {
            for kind in m.kinds {
                for c in self.in_play(*kind) {
                    if !options.iter().any(|o| character::same(*o, c)) {
                        options.push(c);
                    }
                }
            }
        }
        self.register(asker, target, options, |c| {
            storyteller::Choice::Character(c.name())
        })
    }

    /// Whether `target` registers as a `kind` character to `asker`'s ability,
    /// and as which.
    pub fn registers_as(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
        kind: character::Kind,
    ) -> Option<character::Ref> {
        let real = self.state.character(target);
        let truth = (real.kind() == kind).then_some(real);
        let mut options = vec![truth];
        if let Some(m) = self.misregistration(target) {
            for misregistered in m.kinds {
                let answers: Vec<Option<character::Ref>> = if *misregistered == kind {
                    self.in_play(kind).into_iter().map(Some).collect()
                } else {
                    vec![None]
                };
                for answer in answers {
                    let known = options.iter().any(|o| match (o, answer) {
                        (Some(a), Some(b)) => character::same(*a, b),
                        (None, None) => true,
                        _ => false,
                    });
                    if !known {
                        options.push(answer);
                    }
                }
            }
        }
        self.register(asker, target, options, |answer| match answer {
            Some(c) => storyteller::Choice::Character(c.name()),
            None => storyteller::Choice::NotOfKind(kind),
        })
    }

    /// How `target` may misregister: never while malfunctioning.
    fn misregistration(&self, target: &player::Id) -> Option<character::Misregistration> {
        if self.state.is_malfunctioning(target) {
            return None;
        }
        self.state.character(target).misregistration()
    }

    /// Distinct in-play characters of `kind`, in seat order.
    fn in_play(&self, kind: character::Kind) -> Vec<character::Ref> {
        let mut characters: Vec<character::Ref> = Vec::new();
        for p in self.state.players() {
            if p.character.kind() == kind
                && !characters.iter().any(|c| character::same(*c, p.character))
            {
                characters.push(p.character);
            }
        }
        characters
    }

    fn register<T: Clone>(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
        options: Vec<T>,
        describe: impl Fn(&T) -> storyteller::Choice,
    ) -> T {
        if options.len() == 1 {
            return options[0].clone();
        }
        let reason = storyteller::Reason::Registration {
            asker: asker.clone(),
            target: target.clone(),
        };
        let first = options[0].clone();
        self.choose(reason, options, 1, describe)
            .into_iter()
            .next()
            .unwrap_or(first)
    }
}

pub struct Write<'a> {
    state: &'a mut game::State,
    io: &'a mut dyn table::Io,
    storyteller: &'a mut dyn storyteller::Strategy,
}

impl<'a> Write<'a> {
    pub fn new(
        state: &'a mut game::State,
        io: &'a mut dyn table::Io,
        storyteller: &'a mut dyn storyteller::Strategy,
    ) -> Self {
        Self {
            state,
            io,
            storyteller,
        }
    }

    /// A read-only view of this context, for talking to players and the storyteller.
    pub fn read(&mut self) -> Read<'_> {
        Read::new(self.state, self.io, self.storyteller)
    }

    pub fn state(&self) -> &game::State {
        self.state
    }

    pub(crate) fn io(&mut self) -> &mut dyn table::Io {
        self.io
    }

    pub fn record(&mut self, event: event::Event) {
        self.state.record(event.clone());
        self.io.event(&event, self.state);
    }

    /// A storyteller-facing note, kept in the history.
    pub fn note(&mut self, text: impl Into<String>) {
        self.record(event::Event::Note(text.into()));
    }

    /// Moves to the next phase; living players' night-start or day-start
    /// abilities fire.
    pub fn advance_phase(&mut self) -> game::Phase {
        let phase = self.state.advance_phase();
        self.record(event::Event::PhaseBegan(phase));
        for (id, ability) in self.living_abilities() {
            match phase {
                game::Phase::Night(_) => ability.on_night_start(self, &id),
                game::Phase::Day(_) => ability.on_day_start(self, &id),
                game::Phase::Setup => {}
            }
        }
        phase
    }

    /// Every living player's abilities, in seat order.
    pub fn living_abilities(&self) -> Vec<(player::Id, character::Ref)> {
        self.abilities_of(&self.state.alive_ids())
    }

    /// The abilities of `players`, in the given order.
    pub fn abilities_of(&self, players: &[player::Id]) -> Vec<(player::Id, character::Ref)> {
        players
            .iter()
            .flat_map(|id| {
                self.state
                    .abilities(id)
                    .into_iter()
                    .map(move |ability| (id.clone(), ability))
            })
            .collect()
    }

    pub fn place_reminder(&mut self, target: &player::Id, reminder: game::Reminder) {
        self.state.place_reminder(target, reminder.clone());
        self.record(event::Event::ReminderPlaced {
            player: target.clone(),
            reminder,
        });
    }

    /// Tell `target` they are `character`, which they are not (or, with
    /// `None`, that they are their real character again).
    pub fn set_believed(&mut self, target: &player::Id, character: Option<character::Ref>) {
        self.state.set_believed(target, character);
        self.record(event::Event::Believes {
            player: target.clone(),
            character: self.state.believed(target).name(),
        });
    }

    pub fn gain_ability(&mut self, target: &player::Id, ability: character::Ref) {
        self.state.gain_ability(target, ability);
        self.record(event::Event::AbilityGained {
            player: target.clone(),
            ability: ability.name(),
        });
    }

    pub fn lose_ability(&mut self, target: &player::Id, ability: character::Ref) {
        self.state.lose_ability(target, ability);
        self.record(event::Event::AbilityLost {
            player: target.clone(),
            ability: ability.name(),
        });
    }

    /// `target` changes alignment (their character stays).
    pub fn set_alignment(&mut self, target: &player::Id, alignment: character::Alignment) {
        self.state.set_alignment(target, alignment);
        self.record(event::Event::AlignmentChanged {
            player: target.clone(),
            alignment,
        });
    }

    /// The script being played (see `game::State::script`).
    pub fn set_script(&mut self, characters: Vec<character::Ref>) {
        self.state.set_script(characters);
    }

    /// Put a Fabled character in play (it is never seated).
    pub fn add_fabled(&mut self, fabled: character::Ref) {
        assert_eq!(
            fabled.kind(),
            character::Kind::Fabled,
            "{} is not Fabled",
            fabled.name()
        );
        self.state.add_fabled(fabled);
        self.record(event::Event::FabledAdded(fabled.name()));
    }

    /// Attempt to kill `target`. Monk protection stops demon kills; then every
    /// functioning living player's abilities may prevent or redirect the
    /// death (`character::Def::intercept_death`). Returns whether someone died.
    pub fn kill(&mut self, target: &player::Id, cause: event::Cause) -> bool {
        if !self.state.is_alive(target) {
            return false;
        }
        if cause == event::Cause::Demon
            && let Some(by) = self.state.effect_source(target, game::Effect::Protected)
        {
            self.record(event::Event::KillPrevented {
                player: target.clone(),
                by,
            });
            return false;
        }
        for (id, ability) in self.living_abilities() {
            if self.state.is_malfunctioning(&id) {
                continue;
            }
            match ability.intercept_death(self, &id, target, &cause) {
                character::Interception::Pass => {}
                character::Interception::Prevent => {
                    self.record(event::Event::KillPrevented {
                        player: target.clone(),
                        by: ability.id(),
                    });
                    return false;
                }
                character::Interception::Redirect(to) if &to != target => {
                    self.record(event::Event::KillRedirected {
                        from: target.clone(),
                        to: to.clone(),
                    });
                    return self.kill(&to, cause);
                }
                character::Interception::Redirect(_) => break,
            }
        }
        self.die(target, cause);
        true
    }

    fn die(&mut self, target: &player::Id, cause: event::Cause) {
        self.state.mark_dead(target);
        // A dead player's ability stops: tokens it placed until dusk are removed.
        self.state.remove_reminders(target, game::Expiry::Dusk);
        self.record(event::Event::Died {
            player: target.clone(),
            cause: cause.clone(),
        });
        let mut watchers = self.living_abilities();
        watchers.extend(self.abilities_of(std::slice::from_ref(target)));
        for (id, ability) in watchers {
            ability.on_death(self, &id, target, &cause);
        }
        self.check_win();
    }

    /// Execute `target`: their execution triggers fire, then they die unless
    /// an ability prevents it. Returns whether they died.
    pub fn execute(&mut self, target: &player::Id) -> bool {
        if !self.state.is_alive(target) {
            return false;
        }
        self.state.mark_executed(target);
        self.record(event::Event::Executed(target.clone()));
        for (id, ability) in self.abilities_of(std::slice::from_ref(target)) {
            ability.on_executed(self, &id);
        }
        self.kill(target, event::Cause::Execution)
    }

    /// `target` becomes `into`; their old ability's lingering tokens are
    /// removed, and they now know their character.
    pub fn transform(&mut self, target: &player::Id, into: character::Ref) {
        self.state.remove_reminders(target, game::Expiry::Dusk);
        self.state.set_character(target, into);
        self.state.set_believed(target, None);
        self.record(event::Event::Transformed {
            player: target.clone(),
            into: into.name(),
        });
    }

    pub fn win(&mut self, alignment: character::Alignment) {
        if self.state.winner().is_none() {
            self.state.set_winner(alignment);
            self.record(event::Event::Won(alignment));
        }
    }

    /// Good wins when no demon lives; evil wins when two players remain.
    pub fn check_win(&mut self) {
        if !self
            .state
            .players()
            .iter()
            .any(|p| p.alive && p.character.kind() == character::Kind::Demon)
        {
            self.win(character::Alignment::Good);
        } else if self.state.alive_ids().len() <= 2 {
            self.win(character::Alignment::Evil);
        }
    }

    // Everything `Read` can do, so read-write abilities need not reborrow.

    pub fn wake(&mut self, player: &player::Id) {
        self.read().wake(player);
    }

    pub fn sleep(&mut self, player: &player::Id) {
        self.read().sleep(player);
    }

    pub fn tell(&mut self, player: &player::Id, message: &info::Message) {
        self.read().tell(player, message);
    }

    pub fn ask(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        count: usize,
    ) -> Vec<player::Id> {
        self.read().ask(player, prompt, candidates, count)
    }

    pub fn ask_one(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
    ) -> Option<player::Id> {
        self.read().ask_one(player, prompt, candidates)
    }

    pub fn ask_character(
        &mut self,
        player: &player::Id,
        prompt: &str,
        options: &[character::Ref],
    ) -> Option<character::Ref> {
        self.read().ask_character(player, prompt, options)
    }

    pub fn choose<T: Clone>(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<T>,
        count: usize,
        describe: impl Fn(&T) -> storyteller::Choice,
    ) -> Vec<T> {
        self.read().choose(reason, options, count, describe)
    }

    pub fn choose_player(
        &mut self,
        reason: storyteller::Reason,
        options: Vec<player::Id>,
    ) -> Option<player::Id> {
        self.read().choose_player(reason, options)
    }

    pub fn inform(
        &mut self,
        player: &player::Id,
        truthful: Vec<info::Message>,
        misinformation: Vec<info::Message>,
    ) {
        self.read().inform(player, truthful, misinformation);
    }

    pub fn registered_alignment(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
    ) -> character::Alignment {
        self.read().registered_alignment(asker, target)
    }

    pub fn registers_evil(&mut self, asker: &player::Id, target: &player::Id) -> bool {
        self.read().registers_evil(asker, target)
    }

    pub fn registered_character(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
    ) -> character::Ref {
        self.read().registered_character(asker, target)
    }

    pub fn registers_as(
        &mut self,
        asker: &player::Id,
        target: &player::Id,
        kind: character::Kind,
    ) -> Option<character::Ref> {
        self.read().registers_as(asker, target, kind)
    }
}
