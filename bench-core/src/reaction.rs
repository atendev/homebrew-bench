//! Reaction windows (ADR 0018): what opened one, what it offers, and who decides.
//!
//! The engine lists the windows a trigger opens and asks the encounter's
//! [`ReactionDecider`] about each one. Live mode answers from the user's or the
//! agent's `bench respond`, and batch passes every window until the tactics
//! policy takes reactions (ticket 063).

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::encounter::EncounterState;

/// What opened a reaction window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowTrigger {
    /// The mover is leaving the reactor's reach (an Opportunity Attack).
    Opportunity { mover_id: String },
    /// The reactor was hit by an attack roll. `ac` is its AC before any reaction.
    Hit { attacker_id: String, total: i32, ac: i32, critical: bool },
}

/// One reaction a window offers: a reaction entry for a hit, or an attack
/// action for an Opportunity Attack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionOption {
    pub id: String,
    pub name: String,
}

/// An offer of reactions to one combatant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionWindow {
    pub reactor_id: String,
    pub trigger: WindowTrigger,
    pub options: Vec<ReactionOption>,
}

impl ReactionWindow {
    /// The option with this id.
    pub fn option(&self, id: &str) -> Option<&ReactionOption> {
        self.options.iter().find(|o| o.id == id)
    }
}

/// An answer to one window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionAnswer {
    /// Take the option with this id.
    Take(String),
    Pass,
}

/// What the decider says about a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Answer(ReactionAnswer),
    /// No answer yet: the engine records the window and the caller prompts for it.
    Wait,
}

/// Who answers reaction windows while one request resolves. It sits on the
/// state beside the player dice and is never stored.
#[derive(Debug, Clone, Default)]
pub enum ReactionDecider {
    /// Batch: pass every window. Policy v2 takes no reactions.
    #[default]
    PassAll,
    /// Live: hit-window answers queued from earlier prompts, in window order.
    /// Opportunity Attack answers live in the held move's `opportunity` and
    /// `declined` lists, so an Opportunity Attack window always waits.
    Queued(VecDeque<ReactionAnswer>),
}

impl ReactionDecider {
    /// Decide one window, given the state as the engine presents it there.
    pub fn decide(&mut self, _state: &EncounterState, window: &ReactionWindow) -> Decision {
        match self {
            ReactionDecider::PassAll => Decision::Answer(ReactionAnswer::Pass),
            ReactionDecider::Queued(_) if matches!(window.trigger, WindowTrigger::Opportunity { .. }) => Decision::Wait,
            ReactionDecider::Queued(answers) => answers.pop_front().map_or(Decision::Wait, Decision::Answer),
        }
    }
}

/// Reaction state for the request being resolved. Never stored.
#[derive(Debug, Clone, Default)]
pub struct Reactions {
    pub decider: ReactionDecider,
    /// The first window the decider could not answer. Once set, the engine
    /// passes every later window, and the caller discards the run and prompts.
    pub waiting: Option<ReactionWindow>,
}
