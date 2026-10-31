//! Encounter log: structured entries kept in the state file and rendered as
//! one plain-text log per encounter.
//!
//! The public view is a filter over the same entries, never a second file.
//! Each line carries its own public rendering, decided when the line is
//! written, so the filter never has to parse text.

use serde::{Deserialize, Serialize};

/// How a log line reads in the public view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind", content = "text")]
pub enum Visibility {
    /// The public line is the same as the full line.
    #[default]
    Same,
    /// The public line drops hidden detail and keeps the fact.
    Replaced(String),
    /// The line only carries hidden detail, such as a monster recharge.
    Hidden,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LogLine {
    pub text: String,
    #[serde(default, skip_serializing_if = "is_same")]
    pub public: Visibility,
}

fn is_same(v: &Visibility) -> bool {
    *v == Visibility::Same
}

impl LogLine {
    pub fn same(text: impl Into<String>) -> Self {
        Self { text: text.into(), public: Visibility::Same }
    }

    pub fn replaced(text: impl Into<String>, public: impl Into<String>) -> Self {
        Self { text: text.into(), public: Visibility::Replaced(public.into()) }
    }

    pub fn hidden(text: impl Into<String>) -> Self {
        Self { text: text.into(), public: Visibility::Hidden }
    }

    /// The public rendering of this line, or `None` when it is hidden.
    pub fn public_text(&self) -> Option<&str> {
        match &self.public {
            Visibility::Same => Some(&self.text),
            Visibility::Replaced(text) => Some(text),
            Visibility::Hidden => None,
        }
    }
}

/// One log entry, written by one command.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct LogEntry {
    /// Entry number. Entry 0 is `bench start`.
    pub index: usize,
    /// Command that wrote the entry.
    pub command: String,
    /// Round and active combatant when the command ran, for the headers.
    pub round: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<String>,
    pub lines: Vec<LogLine>,
    /// Game hash of the state this entry wrote.
    pub state_hash: String,
}

/// Render the full plain-text log.
pub fn render(entries: &[LogEntry]) -> String {
    render_with(entries, false)
}

/// Render the public view of the log.
pub fn render_public(entries: &[LogEntry]) -> String {
    render_with(entries, true)
}

fn render_with(entries: &[LogEntry], public: bool) -> String {
    let mut out = String::new();
    let mut round = None;
    let mut turn: Option<&str> = None;
    for entry in entries {
        if entry.round > 0 && round != Some(entry.round) {
            out.push_str(&format!("\n== Round {} ==\n", entry.round));
            round = Some(entry.round);
            turn = None;
        }
        if let Some(active) = entry.turn.as_deref() {
            if turn != Some(active) {
                out.push_str(&format!("-- Turn: {active} --\n"));
                turn = Some(active);
            }
        }
        for line in &entry.lines {
            let text = if public { line.public_text() } else { Some(line.text.as_str()) };
            if let Some(text) = text {
                out.push_str(&format!("[{}] {text}\n", entry.index));
            }
        }
        if !public {
            out.push_str(&format!("[{}] state {}\n", entry.index, entry.state_hash));
        }
    }
    out.trim_start().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(index: usize, round: u32, turn: &str, lines: Vec<LogLine>) -> LogEntry {
        LogEntry {
            index,
            command: "test".into(),
            round,
            turn: Some(turn.into()),
            lines,
            state_hash: format!("h{index}"),
        }
    }

    #[test]
    fn render_writes_round_and_turn_headers_once() {
        let entries = vec![
            entry(1, 1, "mia", vec![LogLine::same("mia attacks")]),
            entry(2, 1, "mia", vec![LogLine::same("mia moves")]),
            entry(3, 1, "goblin", vec![LogLine::same("goblin attacks")]),
            entry(4, 2, "mia", vec![LogLine::same("mia attacks")]),
        ];
        let text = render(&entries);
        assert_eq!(text.matches("== Round 1 ==").count(), 1);
        assert_eq!(text.matches("-- Turn: mia --").count(), 2);
        assert!(text.contains("[2] state h2"));
    }

    #[test]
    fn public_render_applies_line_visibility() {
        let entries = vec![entry(
            1,
            1,
            "goblin",
            vec![
                LogLine::replaced("goblin regains 5 HP", "goblin regains HP"),
                LogLine::hidden("goblin recharges breath"),
                LogLine::same("goblin attacks mia"),
            ],
        )];
        let text = render_public(&entries);
        assert!(text.contains("goblin regains HP"));
        assert!(!text.contains("5 HP"));
        assert!(!text.contains("recharges"));
        assert!(text.contains("goblin attacks mia"));
        assert!(!text.contains("state h1"));
    }
}
