//! What an ability can do. [`Read`] can talk to players and the storyteller
//! but cannot change the grimoire; [`Write`] can also change it, and resolves
//! rule interactions (protection, redirects, death triggers, win checks).

use crate::ability;
use crate::character;
use crate::event;
use crate::game;
use crate::info;
use crate::player;
use crate::storyteller;
use crate::table;

/// A night choice: who chose, and whom. Choices made through a `Read` are
/// kept until the narrator resolves them (`Write::resolve_choices`), since a
/// read-only ability cannot change the grimoire.
pub type Choice = (player::Id, Vec<player::Id>);

pub struct Read<'a> {
    state: &'a game::State,
    io: &'a mut dyn table::Io,
    storyteller: &'a mut dyn storyteller::Strategy,
    choices: &'a mut Vec<Choice>,
}

impl<'a> Read<'a> {
    pub fn new(
        state: &'a game::State,
        io: &'a mut dyn table::Io,
        storyteller: &'a mut dyn storyteller::Strategy,
        choices: &'a mut Vec<Choice>,
    ) -> Self {
        Self {
            state,
            io,
            storyteller,
            choices,
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
        let answer = if storyteller::is_selection(&answer, count, |a| candidates.contains(a)) {
            answer
        } else {
            candidates[..count].to_vec()
        };
        self.chose(player, &answer);
        answer
    }

    /// Keeps a night choice for `Write::resolve_choices`.
    fn chose(&mut self, player: &player::Id, chosen: &[player::Id]) {
        if let game::Phase::Night(_) = self.state.phase()
            && !chosen.is_empty()
        {
            self.choices.push((player.clone(), chosen.to_vec()));
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

    /// Ask a player who "may choose" for up to `max` distinct candidates;
    /// an empty answer is choosing no-one. Invalid answers count as no-one.
    pub fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        let max = max.min(candidates.len());
        if max == 0 {
            return Vec::new();
        }
        let answer = self.io.ask_up_to(player, prompt, candidates, max);
        let answer = if answer.len() <= max
            && storyteller::is_selection(&answer, answer.len(), |a| candidates.contains(a))
        {
            answer
        } else {
            Vec::new()
        };
        self.chose(player, &answer);
        answer
    }

    /// Whether `player` tried to convince the group they are `about`: the
    /// table says so, or they reported breaking madness today (a report is
    /// not about anything in particular: it counts for every madness).
    pub fn is_mad_about(&mut self, player: &player::Id, about: &'static str) -> bool {
        reported_madness(self.state, player) || self.io.is_mad_about(player, about)
    }

    /// Whether `player`, mad about being `about`, broke their madness: they
    /// reported it today, or the table says so.
    pub fn broke_madness(&mut self, player: &player::Id, about: &'static str) -> bool {
        reported_madness(self.state, player) || self.io.broke_madness(player, about)
    }

    /// Ask a player a yes/no question (do they agree?).
    pub fn ask_yes_no(&mut self, player: &player::Id, prompt: &str) -> bool {
        self.io.ask_yes_no(player, prompt)
    }

    /// Ask a player to type a short free text; `None` if they typed nothing.
    /// Whitespace is collapsed and the text cut to [`info::TEXT_LIMIT`].
    pub fn ask_text(&mut self, player: &player::Id, prompt: &str) -> Option<String> {
        let typed = self.io.ask_text(player, prompt);
        let text: String = typed
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(info::TEXT_LIMIT)
            .collect();
        (!text.is_empty()).then_some(text)
    }

    /// Whether the storyteller understands free text, so may [`Read::answer`]
    /// a player's own question.
    pub fn storyteller_reads_text(&self) -> bool {
        self.storyteller.reads_text()
    }

    /// The storyteller answers `player`'s own yes/no `question` and tells
    /// them. The engine cannot judge the question, so the decision says
    /// which answers are legal (`storyteller::Honesty`): only the true one
    /// for a functioning player, either when malfunctioning, the untrue one
    /// when their information must be false. Ask only a storyteller that
    /// [`Read::storyteller_reads_text`]. Returns the answer.
    pub fn answer(&mut self, player: &player::Id, question: &str) -> bool {
        let malfunctioning = self.state.is_malfunctioning(player);
        let honesty =
            if self.forced_false(player) || (!malfunctioning && self.lying_storyteller(player)) {
                storyteller::Honesty::False
            } else if malfunctioning {
                storyteller::Honesty::Either
            } else {
                storyteller::Honesty::Truthful
            };
        let reason = storyteller::Reason::Question {
            player: player.clone(),
            character: self.state.believed(player).name(),
            question: question.to_string(),
            honesty,
        };
        let yes = self
            .choose(reason, vec![true, false], 1, |&yes| {
                storyteller::Choice::YesNo(yes)
            })
            .first()
            .copied()
            .unwrap_or(true);
        let message = info::Message::Answer {
            question: question.to_string(),
            yes,
        };
        self.tell(player, &message);
        yes
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

    /// Ask `player` a yes/no question (or the table's judgement about them).
    pub fn confirm(&mut self, player: &player::Id, question: &table::Question) -> bool {
        self.io.confirm(player, question)
    }

    /// Give `player` information. The storyteller picks one of the `truthful`
    /// messages; if the player is malfunctioning, `misinformation` is legal too.
    /// Returns what the player was told.
    pub fn inform(
        &mut self,
        player: &player::Id,
        truthful: Vec<info::Message>,
        misinformation: Vec<info::Message>,
    ) -> Option<info::Message> {
        let malfunctioning = self.state.is_malfunctioning(player);
        let forced = !misinformation.is_empty() && self.forced_false(player);
        // A malfunctioning player may be told either anyway, so a lying
        // storyteller (Knaves) is only asked about for a functioning one.
        let must_lie = forced
            || (!malfunctioning && !misinformation.is_empty() && self.lying_storyteller(player));
        let mut options: Vec<(info::Message, bool)> = if must_lie {
            Vec::new()
        } else {
            truthful.iter().cloned().map(|m| (m, true)).collect()
        };
        if malfunctioning || must_lie {
            for message in misinformation {
                if !options.iter().any(|(m, _)| *m == message) && !truthful.contains(&message) {
                    options.push((message, false));
                }
            }
        }
        if options.is_empty() {
            // Nothing false to say: the truth is all that is left.
            options = truthful.into_iter().map(|m| (m, true)).collect();
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
        let (message, _) = chosen.into_iter().next()?;
        self.tell(player, &message);
        Some(message)
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
                for c in self.misregistration_options(*kind) {
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
                    self.misregistration_options(kind)
                        .into_iter()
                        .map(Some)
                        .collect()
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

    /// Whether a living, functioning player's ability forces `receiver`'s
    /// information to be false (`character::Def::forces_false_info`).
    fn forced_false(&self, receiver: &player::Id) -> bool {
        if self.state.has_effect(receiver, game::Effect::Sober) {
            return false;
        }
        self.state.alive_ids().iter().any(|id| {
            !self.state.is_malfunctioning(id)
                && self
                    .state
                    .abilities(id)
                    .iter()
                    .any(|c| c.forces_false_info(self.state, id, receiver))
        })
    }

    /// Whether an in-play Fabled or Loric has a lying storyteller give
    /// `receiver` this information (`character::Def::lies_to`). A sober
    /// player (the Barista) always gets the truth.
    fn lying_storyteller(&mut self, receiver: &player::Id) -> bool {
        if self.state.has_effect(receiver, game::Effect::Sober) {
            return false;
        }
        let fabled = self.state.fabled().to_vec();
        let mut lies = false;
        for f in fabled {
            lies |= f.lies_to(self, receiver);
        }
        lies
    }

    /// How `target` may misregister: never while malfunctioning.
    fn misregistration(&self, target: &player::Id) -> Option<character::Misregistration> {
        if self.state.is_malfunctioning(target) {
            return None;
        }
        self.state.character(target).misregistration()
    }

    /// Characters of `kind` a player may misregister as: those on the script,
    /// or the in-play ones when no script is recorded.
    fn misregistration_options(&self, kind: character::Kind) -> Vec<character::Ref> {
        if !self.state.script().is_empty() {
            return self
                .state
                .script()
                .iter()
                .copied()
                .filter(|c| c.kind() == kind)
                .collect();
        }
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
    /// Night choices not yet resolved (see [`Write::resolve_choices`]).
    choices: Vec<Choice>,
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
            choices: Vec::new(),
        }
    }

    /// A read-only view of this context, for talking to players and the storyteller.
    pub fn read(&mut self) -> Read<'_> {
        Read::new(self.state, self.io, self.storyteller, &mut self.choices)
    }

    /// Night choices made so far react: the chooser's abilities
    /// (`character::Def::on_choose`), then each chosen player's
    /// (`on_chosen`, the Goon). Choices through `Write::ask` resolve at once;
    /// the narrator resolves those of read-only abilities after each action.
    pub fn resolve_choices(&mut self) {
        for (chooser, chosen) in std::mem::take(&mut self.choices) {
            for (id, ability) in self.abilities_of(std::slice::from_ref(&chooser)) {
                ability.on_choose(self, &id, &chosen);
            }
            for (id, ability) in self.abilities_of(&chosen) {
                if id != chooser {
                    ability.on_chosen(self, &id, &chooser);
                }
            }
        }
    }

    pub fn state(&self) -> &game::State {
        self.state
    }

    /// The table itself, for asking players something that is not an
    /// ability's choice (no `on_choose` hooks run), such as whom the Butcher
    /// nominates again.
    pub fn io(&mut self) -> &mut dyn table::Io {
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

    /// Something everyone at the table is told.
    pub fn announce(&mut self, text: impl Into<String>) {
        self.record(event::Event::Announced(text.into()));
    }

    /// Moves to the next phase; active players' night-start or day-start
    /// abilities fire.
    pub fn advance_phase(&mut self) -> game::Phase {
        let phase = self.state.advance_phase();
        self.record(event::Event::PhaseBegan(phase));
        if let game::Phase::Day(_) = phase {
            // Night deaths are learnt at dawn.
            let dead = self.state.deaths_tonight().to_vec();
            self.learn_deaths(&dead);
        }
        for (id, ability) in self.active_abilities() {
            match phase {
                game::Phase::Night(_) => ability.on_night_start(self, &id),
                game::Phase::Day(_) => ability.on_day_start(self, &id),
                game::Phase::Setup => {}
            }
        }
        phase
    }

    /// The abilities of every player whose ability is active (alive, or dead
    /// but keeping it: `game::State::ability_active`), in seat order.
    pub fn active_abilities(&self) -> Vec<(player::Id, character::Ref)> {
        let active: Vec<player::Id> = self
            .state
            .seat_order()
            .into_iter()
            .filter(|id| self.state.ability_active(id))
            .collect();
        self.abilities_of(&active)
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
        self.place_reminders(std::slice::from_ref(target), reminder);
    }

    /// Puts the same token on each of `targets` (the Minstrel makes everyone
    /// drunk), moving that ability's earlier tokens of the kind.
    pub fn place_reminders(&mut self, targets: &[player::Id], reminder: game::Reminder) {
        self.state.place_reminders(targets, reminder.clone());
        for target in targets {
            self.record(event::Event::ReminderPlaced {
                player: target.clone(),
                reminder: reminder.clone(),
            });
        }
    }

    /// Adds a token without moving its copies on other players (at most one
    /// equal token per player), for abilities with several identical tokens
    /// (see `ability::sync_marks`).
    pub fn add_reminder(&mut self, target: &player::Id, reminder: game::Reminder) {
        self.state.add_reminder(target, reminder.clone());
        self.record(event::Event::ReminderPlaced {
            player: target.clone(),
            reminder,
        });
    }

    /// Take `reminder` off `target`, if it is there.
    pub fn remove_reminder(&mut self, target: &player::Id, reminder: &game::Reminder) {
        if self.state.remove_reminder(target, reminder) {
            self.record(event::Event::ReminderRemoved {
                player: target.clone(),
                reminder: reminder.clone(),
            });
        }
    }

    /// A dead player comes back to life (their abilities work again).
    pub fn revive(&mut self, target: &player::Id) {
        if self.state.is_alive(target) {
            return;
        }
        self.state.mark_alive(target);
        self.record(event::Event::Revived(target.clone()));
    }

    /// Puts a storyteller-side token on an in-play Fabled or Loric.
    pub fn mark_fabled(&mut self, fabled: character::Ref, effect: game::Effect) {
        self.state.mark_fabled(fabled, effect);
        self.record(event::Event::FabledMarked {
            fabled: fabled.name(),
            effect,
        });
    }

    /// Two players swap seats (the Matron).
    pub fn swap_seats(&mut self, a: &player::Id, b: &player::Id) {
        self.state.swap_seats(a, b);
        self.record(event::Event::SeatsSwapped(a.clone(), b.clone()));
    }

    /// A dead player gives away their vote token (to the Beggar).
    pub fn spend_ghost_vote(&mut self, player: &player::Id) {
        self.state.spend_ghost_vote(player);
        self.record(event::Event::GhostVoteSpent(player.clone()));
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

    /// `target` changes alignment (their character stays), unless an
    /// in-play Fabled forbids it (`character::Def::permits_alignment_change`).
    /// Returns whether it changed.
    pub fn set_alignment(&mut self, target: &player::Id, alignment: character::Alignment) -> bool {
        if self.state.alignment(target) == alignment {
            return false;
        }
        let refused = self
            .state
            .fabled()
            .iter()
            .find(|f| !f.permits_alignment_change(self.state, target, alignment));
        if let Some(f) = refused {
            let note = format!("{target} stays {} ({})", alignment.opposite(), f.name());
            self.note(note);
            return false;
        }
        self.state.set_alignment(target, alignment);
        self.record(event::Event::AlignmentChanged {
            player: target.clone(),
            alignment,
        });
        true
    }

    /// The script being played (see `game::State::script`).
    pub fn set_script(&mut self, characters: Vec<character::Ref>) {
        self.state.set_script(characters);
    }

    /// Put a Fabled or Loric character in play (it is never seated).
    pub fn add_fabled(&mut self, fabled: character::Ref) {
        assert!(
            fabled.kind().storyteller_side(),
            "{} is not Fabled or Loric",
            fabled.name()
        );
        self.state.add_fabled(fabled);
        self.record(event::Event::FabledAdded(fabled.name()));
    }

    /// Attempt to kill `target`. Monk protection stops demon kills, and an
    /// `ExecutionOnly` token every death but execution; then every
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
        if cause != event::Cause::Execution
            && let Some(by) = self
                .state
                .effect_source(target, game::Effect::ExecutionOnly)
        {
            self.record(event::Event::KillPrevented {
                player: target.clone(),
                by,
            });
            return false;
        }
        // The dying player's own abilities (the Fool) come last, so another
        // player's protection (the Tea Lady) is used first.
        let (own, others): (Vec<_>, Vec<_>) = self
            .active_abilities()
            .into_iter()
            .partition(|(id, _)| id == target);
        for (id, ability) in others.into_iter().chain(own) {
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

    /// `target` dies even if something would stop it (the Assassin): no
    /// protection or `intercept_death` applies. Returns whether they died.
    pub fn kill_unstoppably(&mut self, target: &player::Id, cause: event::Cause) -> bool {
        if !self.state.is_alive(target) {
            return false;
        }
        self.die(target, cause);
        true
    }

    fn die(&mut self, target: &player::Id, cause: event::Cause) {
        self.state.mark_dead(target);
        // A dead player's ability stops: its lingering tokens are removed.
        self.state.remove_lingering(target);
        self.record(event::Event::Died {
            player: target.clone(),
            cause: cause.clone(),
        });
        let mut watchers = self.active_abilities();
        watchers.extend(self.abilities_of(std::slice::from_ref(target)));
        for (id, ability) in watchers {
            ability.on_death(self, &id, target, &cause);
        }
        self.check_win();
        // A day death is learnt at once; night deaths at dawn (`advance_phase`).
        if let game::Phase::Day(_) = self.state.phase() {
            self.learn_deaths(std::slice::from_ref(target));
        }
    }

    /// The `dead` learn they died (`character::Def::on_learns_death`).
    fn learn_deaths(&mut self, dead: &[player::Id]) {
        for (id, ability) in self.abilities_of(dead) {
            if self.state.winner().is_none() {
                ability.on_learns_death(self, &id);
            }
        }
    }

    /// Execute `target`: their execution triggers fire, then every player's
    /// `on_execution`, then they die unless an ability prevents it. Returns
    /// whether they died.
    pub fn execute(&mut self, target: &player::Id) -> bool {
        if !self.state.is_alive(target) {
            return false;
        }
        self.state.mark_executed(target);
        self.record(event::Event::Executed(target.clone()));
        for (id, ability) in self.abilities_of(std::slice::from_ref(target)) {
            ability.on_executed(self, &id);
        }
        let everyone = self.state.seat_order();
        for (id, ability) in self.abilities_of(&everyone) {
            ability.on_execution(self, &id, target);
        }
        self.kill(target, event::Cause::Execution)
    }

    /// `target` becomes `into`; their old ability's lingering tokens are
    /// removed, and they now know their character.
    pub fn transform(&mut self, target: &player::Id, into: character::Ref) {
        self.state.remove_lingering(target);
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
            let everyone = self.state.seat_order();
            for (id, ability) in self.abilities_of(&everyone) {
                ability.on_win(self, &id, alignment);
            }
        }
    }

    /// Good wins when no demon lives (unless the game has none, see
    /// `character::Def::demonless`); evil wins when two players remain;
    /// unless an ability vetoes the win (`character::Def::vetoes_win`).
    pub fn check_win(&mut self) {
        if self.state.winner().is_some() {
            return;
        }
        let winner = if !self.state.demonless() && !self.state.demon_alive() {
            character::Alignment::Good
        } else if self.state.alive_ids().len() <= 2 {
            character::Alignment::Evil
        } else {
            return;
        };
        let everyone = self.state.seat_order();
        for (id, ability) in self.abilities_of(&everyone) {
            if ability.vetoes_win(self, &id, winner) {
                return;
            }
        }
        self.win(winner);
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
        let answer = self.read().ask(player, prompt, candidates, count);
        self.resolve_choices();
        answer
    }

    pub fn ask_one(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
    ) -> Option<player::Id> {
        self.ask(player, prompt, candidates, 1).into_iter().next()
    }

    pub fn ask_up_to(
        &mut self,
        player: &player::Id,
        prompt: &str,
        candidates: &[player::Id],
        max: usize,
    ) -> Vec<player::Id> {
        let answer = self.read().ask_up_to(player, prompt, candidates, max);
        self.resolve_choices();
        answer
    }

    pub fn ask_yes_no(&mut self, player: &player::Id, prompt: &str) -> bool {
        self.read().ask_yes_no(player, prompt)
    }

    pub fn ask_character(
        &mut self,
        player: &player::Id,
        prompt: &str,
        options: &[character::Ref],
    ) -> Option<character::Ref> {
        self.read().ask_character(player, prompt, options)
    }

    pub fn ask_text(&mut self, player: &player::Id, prompt: &str) -> Option<String> {
        self.read().ask_text(player, prompt)
    }

    pub fn storyteller_reads_text(&self) -> bool {
        self.storyteller.reads_text()
    }

    pub fn answer(&mut self, player: &player::Id, question: &str) -> bool {
        self.read().answer(player, question)
    }

    pub fn is_mad_about(&mut self, player: &player::Id, about: &'static str) -> bool {
        self.read().is_mad_about(player, about)
    }

    pub fn broke_madness(&mut self, player: &player::Id, about: &'static str) -> bool {
        self.read().broke_madness(player, about)
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

    pub fn confirm(&mut self, player: &player::Id, question: &table::Question) -> bool {
        self.read().confirm(player, question)
    }

    pub fn inform(
        &mut self,
        player: &player::Id,
        truthful: Vec<info::Message>,
        misinformation: Vec<info::Message>,
    ) -> Option<info::Message> {
        self.read().inform(player, truthful, misinformation)
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

/// Whether `player` reported breaking madness today (`narrator::report_madness`).
pub fn reported_madness(state: &game::State, player: &player::Id) -> bool {
    ability::today(state)
        .iter()
        .any(|e| matches!(e, event::Event::MadnessReported(p) if p == player))
}
