use crate::events::{TaskEvent, SUPPORTED};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PetState {
    #[default]
    Idle,
    Thinking,
    Working,
    Waiting,
    Done,
    Interrupted,
}
#[derive(Default)]
struct Session {
    state: PetState,
    at: u64,
    turn: String,
    calls: HashSet<String>,
    waiting: HashSet<String>,
}
#[derive(Default)]
pub struct TaskStates {
    sessions: HashMap<String, Session>,
}
#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub state: PetState,
    pub active_tasks: usize,
    pub sessions: usize,
    pub last_event_at: Option<u64>,
}
impl TaskStates {
    pub fn apply(&mut self, e: TaskEvent, now: u64) -> bool {
        if e.session.is_empty()
            || e.session.len() > 640
            || !SUPPORTED.contains(&e.event.as_str())
            || now.saturating_sub(e.at) > 86_400_000
            || e.at > now + 60_000
        {
            return false;
        }
        if let Some(old) = self.sessions.get(&e.session) {
            if old.at > e.at
                || (e.event != "UserPromptSubmit"
                    && !e.turn.is_empty()
                    && !old.turn.is_empty()
                    && e.turn != old.turn)
            {
                return false;
            }
            if e.event == "SessionStart" {
                return false;
            }
            // Late tool completions must not resurrect a completed turn.
            if matches!(old.state, PetState::Done | PetState::Interrupted)
                && matches!(
                    e.event.as_str(),
                    "PreToolUse" | "PostToolUse" | "PermissionRequest"
                )
            {
                return false;
            }
        }
        if e.event == "SessionEnd" {
            self.sessions.remove(&e.session);
            return true;
        }
        let s = self.sessions.entry(e.session).or_default();
        if !e.turn.is_empty() {
            s.turn = e.turn;
        }
        s.at = e.at;
        s.state = match e.event.as_str() {
            "SessionStart" => PetState::Idle,
            "UserPromptSubmit" => {
                s.calls.clear();
                s.waiting.clear();
                PetState::Thinking
            }
            "PreToolUse" => {
                s.waiting.remove(&e.call);
                s.calls.insert(e.call);
                if s.waiting.is_empty() {
                    PetState::Working
                } else {
                    PetState::Waiting
                }
            }
            "PostToolUse" => {
                s.calls.remove(&e.call);
                s.waiting.remove(&e.call);
                if !s.waiting.is_empty() {
                    PetState::Waiting
                } else if s.calls.is_empty() {
                    PetState::Thinking
                } else {
                    PetState::Working
                }
            }
            "PermissionRequest" => {
                s.waiting.insert(e.call);
                PetState::Waiting
            }
            "Stop" => {
                s.calls.clear();
                s.waiting.clear();
                PetState::Done
            }
            "Interrupt" => {
                s.calls.clear();
                s.waiting.clear();
                PetState::Interrupted
            }
            _ => return false,
        };
        true
    }
    pub fn snapshot(&mut self, now: u64, completed_seconds: u64) -> TaskSnapshot {
        self.sessions
            .retain(|_, s| now.saturating_sub(s.at) < 86_400_000);
        let active = self
            .sessions
            .values()
            .filter(|s| {
                matches!(
                    s.state,
                    PetState::Thinking | PetState::Working | PetState::Waiting
                )
            })
            .collect::<Vec<_>>();
        let latest = self.sessions.values().max_by_key(|s| s.at);
        let state = if active.iter().any(|s| s.state == PetState::Waiting) {
            PetState::Waiting
        } else if active.iter().any(|s| s.state == PetState::Working) {
            PetState::Working
        } else if !active.is_empty() {
            PetState::Thinking
        } else {
            latest
                .filter(|s| now.saturating_sub(s.at) < completed_seconds * 1000)
                .map(|s| s.state)
                .unwrap_or_default()
        };
        TaskSnapshot {
            state,
            active_tasks: active.len(),
            sessions: self.sessions.len(),
            last_event_at: latest.map(|s| s.at),
        }
    }
}
