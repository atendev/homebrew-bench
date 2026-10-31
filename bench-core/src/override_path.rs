//! Dotted override paths over the encounter state.
//!
//! A path starts with a combatant id (`goblin-2.hp`) or a top-level state
//! field (`round`). Later segments walk object keys, array indexes, or the
//! `id` of an element in an array of objects (`stat_blocks.goblin.stat_block`).
//! Every change round-trips through the state schema, so a bad value fails
//! with the field path serde reports.

use serde_json::Value;

use crate::encounter::{EncounterState, StatBlockSnapshot};

/// Top-level fields an override may never touch.
const FORBIDDEN_TOP: &[&str] = &[
    "id",
    "engine_version",
    "tamper_hash",
    "log",
    "command_history",
    "initial_snapshot",
];

/// Leaf names an override may never touch at any depth.
const FORBIDDEN_LEAF: &[&str] = &["id", "hash", "tamper_hash", "state_hash", "stat_block_id", "side_id"];

/// What an override changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverrideTarget {
    /// Combatant the path starts at, if any.
    pub combatant: Option<String>,
    /// Path inside the combatant, or inside the state when `combatant` is `None`.
    pub field: Vec<String>,
}

impl OverrideTarget {
    /// Whether the public view must hide the override's value: true for
    /// monster hit points, resources, recharge state and per-round budgets.
    pub fn hides_value(&self, state: &EncounterState) -> bool {
        match &self.combatant {
            Some(id) if !state.is_party(id) => matches!(
                self.field.first().map(String::as_str),
                Some("hp" | "temp_hp" | "max_hp" | "resources" | "action_economy")
            ),
            Some(_) => false,
            None => !matches!(
                self.field.first().map(String::as_str),
                Some("round" | "active_turn" | "initiative_order" | "pending" | "sides" | "settings")
            ),
        }
    }
}

/// Apply `value` at `path`, or clear the field when `value` is `None`.
pub fn apply(state: &mut EncounterState, path: &str, value: Option<Value>) -> Result<OverrideTarget, String> {
    let segments: Vec<String> = path.split('.').map(str::to_string).collect();
    if segments.iter().any(String::is_empty) {
        return Err(format!("override path `{path}` has an empty segment"));
    }
    let combatant_index = state.combatants.iter().position(|c| c.id == segments[0]);
    let (combatant, field) = match combatant_index {
        Some(_) => (Some(segments[0].clone()), segments[1..].to_vec()),
        None => (None, segments.clone()),
    };
    if field.is_empty() {
        return Err(format!("override path `{path}` names a combatant but no field"));
    }
    if combatant.is_none() && FORBIDDEN_TOP.contains(&field[0].as_str()) {
        return Err(format!("`{}` cannot be overridden", field[0]));
    }
    if let Some(leaf) = field.last() {
        if FORBIDDEN_LEAF.contains(&leaf.as_str()) {
            return Err(format!("`{path}` is an id or hash and cannot be overridden"));
        }
    }
    if combatant.is_none() && !state_has_top_field(state, &field[0]) {
        return Err(format!("`{}` is neither a combatant id nor a state field", field[0]));
    }

    let mut root = serde_json::to_value(&*state).map_err(|e| e.to_string())?;
    let mut full: Vec<String> = Vec::new();
    if let Some(i) = combatant_index {
        full.push("combatants".into());
        full.push(i.to_string());
    }
    full.extend(field.iter().cloned());

    let value = value.map(|v| normalize_conditions(&full, v));
    let expected = value.clone();
    let removed_element = set_at(&mut root, &full, value).map_err(|e| format!("`{path}`: {e}"))?;

    let mut next: EncounterState = serde_path_to_error::deserialize(root)
        .map_err(|e| format!("`{path}` rejected at `{}`: {}", e.path(), e.inner()))?;

    let check = serde_json::to_value(&next).map_err(|e| e.to_string())?;
    let landed = get_at(&check, &full);
    match (&expected, landed) {
        (None, _) if removed_element => {}
        (Some(want), Some(got)) if want == got => {}
        (Some(_), _) => return Err(format!("`{path}` is not a state field")),
        (None, None) | (None, Some(Value::Null)) => {}
        (None, Some(Value::Array(a))) if a.is_empty() => {}
        (None, Some(Value::Object(o))) if o.is_empty() => {}
        (None, Some(_)) => return Err(format!("`{path}` is required and cannot be cleared")),
    }

    if full.first().map(String::as_str) == Some("stat_blocks") {
        next.stat_blocks = next
            .stat_blocks
            .into_iter()
            .map(|s| StatBlockSnapshot::from_stat_block(s.id, s.stat_block))
            .collect();
    }
    *state = carry_bookkeeping(next, state);
    Ok(OverrideTarget { combatant, field })
}

fn carry_bookkeeping(mut next: EncounterState, old: &EncounterState) -> EncounterState {
    next.log = old.log.clone();
    next.command_history = old.command_history.clone();
    next.initial_snapshot = old.initial_snapshot.clone();
    next.tamper_hash = old.tamper_hash.clone();
    next
}

fn state_has_top_field(state: &EncounterState, name: &str) -> bool {
    let schema = schemars::schema_for!(EncounterState);
    let in_schema = schema
        .schema
        .object
        .as_ref()
        .is_some_and(|o| o.properties.contains_key(name));
    in_schema || serde_json::to_value(state).ok().and_then(|v| v.get(name).cloned()).is_some()
}

/// `mia.conditions '["prone"]'` is shorthand for permanent condition entries.
fn normalize_conditions(path: &[String], value: Value) -> Value {
    if path.last().map(String::as_str) != Some("conditions") {
        return value;
    }
    match value {
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| match item {
                    Value::String(name) => serde_json::json!({"condition": name, "duration": "permanent"}),
                    other => other,
                })
                .collect(),
        ),
        other => other,
    }
}

fn child_key(node: &Value, segment: &str) -> Option<ChildKey> {
    match node {
        Value::Object(_) => Some(ChildKey::Key(segment.to_string())),
        Value::Array(items) => {
            if let Ok(i) = segment.parse::<usize>() {
                return Some(ChildKey::Index(i));
            }
            items
                .iter()
                .position(|item| item.get("id").and_then(Value::as_str) == Some(segment))
                .map(ChildKey::Index)
        }
        _ => None,
    }
}

enum ChildKey {
    Key(String),
    Index(usize),
}

/// Returns whether the change removed an array element.
fn set_at(root: &mut Value, path: &[String], value: Option<Value>) -> Result<bool, String> {
    let (last, parents) = path.split_last().ok_or("empty path")?;
    let mut node = root;
    for segment in parents {
        node = match child_key(node, segment) {
            Some(ChildKey::Key(k)) => node
                .as_object_mut()
                .and_then(|o| o.get_mut(&k))
                .ok_or_else(|| format!("no field `{segment}`"))?,
            Some(ChildKey::Index(i)) => node
                .as_array_mut()
                .and_then(|a| a.get_mut(i))
                .ok_or_else(|| format!("no element `{segment}`"))?,
            None => return Err(format!("`{segment}` is inside a value that has no fields")),
        };
    }
    match (child_key(node, last), value) {
        (Some(ChildKey::Key(k)), Some(v)) => {
            node.as_object_mut().expect("object").insert(k, v);
        }
        (Some(ChildKey::Key(k)), None) => {
            let obj = node.as_object_mut().expect("object");
            match obj.get(&k) {
                Some(Value::Array(_)) => {
                    obj.insert(k, Value::Array(vec![]));
                }
                Some(Value::Object(_)) => {
                    obj.insert(k, Value::Object(Default::default()));
                }
                _ => {
                    obj.insert(k, Value::Null);
                }
            }
        }
        (Some(ChildKey::Index(i)), Some(v)) => {
            let arr = node.as_array_mut().expect("array");
            let slot = arr.get_mut(i).ok_or_else(|| format!("no element `{last}`"))?;
            *slot = v;
        }
        (Some(ChildKey::Index(i)), None) => {
            let arr = node.as_array_mut().expect("array");
            if i >= arr.len() {
                return Err(format!("no element `{last}`"));
            }
            arr.remove(i);
            return Ok(true);
        }
        (None, _) => return Err(format!("`{last}` is inside a value that has no fields")),
    }
    Ok(false)
}

fn get_at<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    let mut node = root;
    for segment in path {
        node = match child_key(node, segment)? {
            ChildKey::Key(k) => node.get(&k)?,
            ChildKey::Index(i) => node.get(i)?,
        };
    }
    Some(node)
}
