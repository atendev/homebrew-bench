//! State digest returned with every live response, and its public view.

use serde::Serialize;
use serde_json::Value;

use crate::encounter::{Combatant, EncounterState};
use crate::engine;

/// Public stand-in for a monster's hit points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum WoundedState {
    #[serde(rename = "unhurt")]
    Unhurt,
    #[serde(rename = "wounded")]
    Wounded,
    #[serde(rename = "bloodied")]
    Bloodied,
    #[serde(rename = "at death's door")]
    AtDeathsDoor,
}

impl WoundedState {
    /// Unhurt at full, wounded below full, bloodied at half or less, at
    /// death's door at a quarter or less.
    pub fn of(hp: i32, max_hp: i32) -> Self {
        let max_hp = max_hp.max(1);
        if hp >= max_hp {
            Self::Unhurt
        } else if hp * 4 <= max_hp {
            Self::AtDeathsDoor
        } else if hp * 2 <= max_hp {
            Self::Bloodied
        } else {
            Self::Wounded
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Unhurt => "unhurt",
            Self::Wounded => "wounded",
            Self::Bloodied => "bloodied",
            Self::AtDeathsDoor => "at death's door",
        }
    }
}

/// Wounded state of a combatant.
pub fn wounded(c: &Combatant) -> WoundedState {
    WoundedState::of(c.hp, c.max_hp)
}

/// The DM digest: everything the agent needs to narrate the next step.
/// `legendary` lists the creatures that may take a legendary action when the
/// active turn ends, with their uses left and ready actions.
pub fn digest(state: &EncounterState) -> Value {
    build(state, false)
}

/// The public digest: monster numbers replaced by wounded states.
pub fn public_digest(state: &EncounterState) -> Value {
    build(state, true)
}

fn build(state: &EncounterState, public: bool) -> Value {
    let combatants: Vec<Value> = state
        .combatants
        .iter()
        .map(|c| {
            let ac = state
                .stat_blocks
                .iter()
                .find(|s| s.id == c.stat_block_id)
                .and_then(|s| s.stat_block.armor_class.first().map(|a| a.value));
            let mut v = serde_json::json!({
                "id": c.id,
                "name": c.name,
                "side": c.side_id,
                "ac": ac,
                "conditions": c.conditions,
                "concentration": c.concentration,
                "engagement": c.engagement,
                "ranges": c.ranges,
                "end_state": c.end_state,
            });
            let obj = v.as_object_mut().expect("object");
            if public && !state.is_party(&c.id) {
                obj.insert("wounded".into(), Value::from(wounded(c).word()));
            } else {
                obj.insert("hp".into(), Value::from(c.hp));
                obj.insert("temp_hp".into(), Value::from(c.temp_hp));
                obj.insert("max_hp".into(), Value::from(c.max_hp));
                obj.insert("resources".into(), serde_json::to_value(&c.resources).unwrap_or_default());
                obj.insert("action_economy".into(), serde_json::to_value(&c.action_economy).unwrap_or_default());
                obj.insert("death_saves".into(), serde_json::to_value(&c.death_saves).unwrap_or_default());
            }
            v
        })
        .collect();
    let pending = state.pending.as_ref().map(|p| {
        if public && !state.is_party(&p.actor_id) {
            serde_json::json!({"id": p.id, "kind": p.kind, "actor_id": p.actor_id})
        } else {
            serde_json::to_value(p).unwrap_or_default()
        }
    });
    let mut digest = serde_json::json!({
        "encounter": state.id,
        "round": state.round,
        "active_turn": state.active_turn,
        "initiative_order": state.initiative_order,
        "combatants": combatants,
        "pending": pending,
        "ended": state.ended,
    });
    if !public {
        digest["legendary"] = serde_json::to_value(engine::legendary_holders(state)).unwrap_or_default();
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wounded_state_thresholds() {
        assert_eq!(WoundedState::of(20, 20), WoundedState::Unhurt);
        assert_eq!(WoundedState::of(19, 20), WoundedState::Wounded);
        assert_eq!(WoundedState::of(11, 20), WoundedState::Wounded);
        assert_eq!(WoundedState::of(10, 20), WoundedState::Bloodied);
        assert_eq!(WoundedState::of(6, 20), WoundedState::Bloodied);
        assert_eq!(WoundedState::of(5, 20), WoundedState::AtDeathsDoor);
        assert_eq!(WoundedState::of(0, 20), WoundedState::AtDeathsDoor);
        assert_eq!(WoundedState::of(7, 7), WoundedState::Unhurt);
        assert_eq!(WoundedState::of(3, 7), WoundedState::Bloodied);
        assert_eq!(WoundedState::of(1, 7), WoundedState::AtDeathsDoor);
    }
}
