//! Closed effect vocabulary for modeled and partly-modeled abilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::stat_block::{Ability, Condition, DamageType};

/// Reaction triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReactionTrigger {
    HitByAttack,
    MissedByAttack,
    EnemyEntersReach,
    AllyHitWithinRange,
    TargetFailsSave,
    StartOfTurn,
}

/// Passive triggers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PassiveTrigger {
    Initiative,
    PerceptionCheck,
    SavesAgainstMagic,
    D20TestAdvantage,
    DamageDealt,
    DeathSavingThrow,
}

/// Usage limit for an ability.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Usage {
    /// Unlimited uses.
    AtWill,
    /// Recharge on a die roll (default 5-6). Optional group id shared across
    /// abilities, such as metallic dragon breath weapons.
    Recharge {
        #[serde(default = "default_recharge_dice")]
        dice: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        group: Option<String>,
    },
    /// Fixed uses per day.
    PerDay { uses: u32 },
    /// Uses per short or long rest.
    PerRest { rest: RestLength, uses: u32 },
    /// Recharges at the start of the creature's next turn.
    CooldownUntilNextTurn,
}

fn default_recharge_dice() -> String {
    "5-6".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RestLength {
    Short,
    Long,
}

/// Area-of-effect shape.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    Cone { size: u32 },
    Line { length: u32, width: u32 },
    Emanation { radius: u32 },
    Sphere { radius: u32 },
    Cube { size: u32 },
    Cylinder { radius: u32, height: u32 },
}

/// Lighting setting for roll modifiers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lighting {
    #[default]
    Normal,
    Sunlight,
    BrightLight,
}

/// Advantage/neutral/disadvantage state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RollDisposition {
    #[default]
    Normal,
    Advantage,
    Disadvantage,
}

/// Attack effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Attack {
    pub modifier: i32,
    pub reach: Option<u32>,
    pub range: Option<(u32, u32)>,
    pub target_count: u32,
    pub damage: Vec<Damage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_hit: Vec<Effect>,
}

/// Saving-throw effect with its success semantics.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SavingThrow {
    pub ability: Ability,
    pub dc: i32,
    pub half_on_success: bool,
    pub no_damage_on_success: bool,
    pub repeat_save: Option<RepeatSave>,
    pub two_stage: Option<TwoStage>,
    #[serde(default)]
    pub immunity_after_success: bool,
    pub hp_gate: Option<u32>,
    pub creature_type_gate: Option<Vec<String>>,
    pub damage: Vec<Damage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_failure: Vec<Effect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RepeatSave {
    pub ability: Ability,
    pub frequency: String,
    pub ends_on_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TwoStage {
    pub first_ability: Ability,
    pub second_ability: Ability,
}

/// Damage instance.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Damage {
    pub dice: String,
    #[serde(rename = "type")]
    pub damage_type: DamageType,
    #[serde(default)]
    pub modifier: i32,
    #[serde(default)]
    pub magical: bool,
}

/// Apply a condition.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConditionEffect {
    pub condition: Condition,
    pub duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<String>,
}

/// Duration for timed modifiers and conditions.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    Rounds(u32),
    Minutes(u32),
    Hours(u32),
    Days(u32),
    UntilSave { ability: Ability },
    Concentration,
    Instant,
    Permanent,
}

/// Timed modifier applied to a combatant.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TimedModifier {
    pub target: ModifierTarget,
    pub change: ModifierChange,
    pub duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModifierTarget {
    Speed,
    ArmorClass,
    AttackRolls,
    SavingThrows { ability: Option<Ability> },
    AbilityChecks { ability: Option<Ability> },
    DamageRolls,
    Reactions,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModifierChange {
    Add(i32),
    Set(u32),
    Halve,
    Double,
    Advantage,
    Disadvantage,
    None,
}

/// Roll modifier tied to the encounter lighting setting.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RollModifier {
    pub lighting: Lighting,
    pub disposition: RollDisposition,
    pub target: ModifierTarget,
}

/// Forced movement.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ForcedMove {
    pub feet: u32,
    pub direction: String,
}

/// Heal or temporary HP.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Heal {
    pub dice: String,
    #[serde(default)]
    pub modifier: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TempHp {
    pub dice: String,
    #[serde(default)]
    pub modifier: i32,
}

/// Teleport.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Teleport {
    pub range: u32,
    pub destination: String,
}

/// Move-then-attack effect.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct MoveThenAttack {
    pub move_feet: u32,
    pub attack_id: String,
}

/// Multiattack step.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MultiattackStep {
    Action { action_id: String, count: u32 },
    Choice { options: Vec<String>, count: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Multiattack {
    pub steps: Vec<MultiattackStep>,
    #[serde(default)]
    pub swap: bool,
}

/// Use another action by id.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct UseAction {
    pub action_id: String,
}

/// Cast a spell from the engine-owned spell effects file.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Cast {
    pub spell_id: String,
    pub slot_level: Option<u8>,
    pub upcast: bool,
}

/// Where an area effect originates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AreaOrigin {
    /// Originates from the source combatant.
    Self_,
    /// Originates from a point chosen by the agent.
    TargetPoint,
    /// Originates from the target creature.
    TargetCreature,
}

/// Area effect wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AreaEffect {
    pub area: Area,
    pub origin: AreaOrigin,
    pub effect: Box<Effect>,
    pub target_count: Option<u32>,
}

/// One effect in the closed vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    Attack(Attack),
    SavingThrow(SavingThrow),
    Damage(Damage),
    Condition(ConditionEffect),
    TimedModifier(TimedModifier),
    RollModifier(RollModifier),
    ForcedMove(ForcedMove),
    Heal(Heal),
    TempHp(TempHp),
    Teleport(Teleport),
    MoveThenAttack(MoveThenAttack),
    Multiattack(Multiattack),
    UseAction(UseAction),
    Cast(Cast),
    Area(AreaEffect),
}

/// Resource map for usage tracking on a combatant.
pub type Resources = HashMap<String, i32>;
