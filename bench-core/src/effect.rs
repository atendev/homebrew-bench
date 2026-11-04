//! Closed effect vocabulary for modeled and partly-modeled abilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::stat_block::{Ability, Condition, DamageType};

/// The trigger on a reaction entry that makes the engine open a reaction window
/// (ADR 0018). The Opportunity Attack is not a variant: every creature has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReactionTrigger {
    /// The creature is hit by an attack roll the filter matches (Shield, Parry).
    HitByAttackRoll { attack: AttackFilter },
}

/// Which attack rolls a hit trigger answers. A melee attack is one with no range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttackFilter {
    Any,
    Melee,
    Ranged,
}

impl AttackFilter {
    /// Whether an attack roll made with `attack` matches the filter.
    pub fn matches(self, attack: &Attack) -> bool {
        match self {
            AttackFilter::Any => true,
            AttackFilter::Melee => attack.range.is_none(),
            AttackFilter::Ranged => attack.range.is_some(),
        }
    }
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
    /// Recharge on a d6 roll of "N-6" or "N" for N in 2 to 6 (default 5-6). Optional
    /// group id shared across abilities, such as metallic dragon breath weapons.
    Recharge {
        #[serde(default = "default_recharge_dice", deserialize_with = "deserialize_recharge_dice")]
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

/// Count and die size of an `NdM` dice expression, the only form the engine rolls.
pub fn parse_dice(expr: &str) -> Option<(u32, u32)> {
    let (count, sides) = expr.split_once('d')?;
    match (count.parse::<u32>(), sides.parse::<u32>()) {
        (Ok(count), Ok(sides)) if count > 0 && sides > 0 => Some((count, sides)),
        _ => None,
    }
}

fn default_recharge_dice() -> String {
    "5-6".into()
}

/// Lowest d6 roll that recharges an ability, from "N-6" or "N" with N in 2 to 6.
pub fn recharge_minimum(dice: &str) -> Option<u32> {
    let digits = dice.strip_suffix("-6").unwrap_or(dice);
    match digits.parse::<u32>() {
        Ok(n) if (2..=6).contains(&n) && digits.len() == 1 => Some(n),
        _ => None,
    }
}

fn deserialize_recharge_dice<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let dice = String::deserialize(deserializer)?;
    if recharge_minimum(&dice).is_none() {
        return Err(serde::de::Error::custom(format!(
            "recharge dice '{dice}' must be 'N-6' or 'N' for N in 2 to 6"
        )));
    }
    Ok(dice)
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

/// The encounter's lighting, and the lighting a roll modifier needs.
/// `unspecified` (formerly `normal`) means no lighting was stated and turns on no roll modifier.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Lighting {
    #[default]
    #[serde(alias = "normal")]
    Unspecified,
    DimLight,
    BrightLight,
    Sunlight,
}

impl Lighting {
    /// Whether an encounter lit like `self` meets a roll modifier keyed to `required`.
    /// Sunlight is Bright Light; `unspecified` meets nothing.
    pub fn meets(self, required: Lighting) -> bool {
        match (self, required) {
            (Lighting::Sunlight, Lighting::BrightLight) => true,
            (Lighting::Unspecified, _) => false,
            _ => self == required,
        }
    }

    /// Whether no lighting was stated.
    pub fn is_unspecified(&self) -> bool {
        *self == Lighting::Unspecified
    }

    /// The snake_case spelling used by flags, state files and override paths.
    pub fn as_str(self) -> &'static str {
        match self {
            Lighting::Unspecified => "unspecified",
            Lighting::DimLight => "dim_light",
            Lighting::BrightLight => "bright_light",
            Lighting::Sunlight => "sunlight",
        }
    }
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
    /// The entry applies only while the target has this condition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_target: Option<Condition>,
    /// With `if_target`, the condition must have been applied by the damaging creature ("Grappled by the mimic").
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub if_target_from_actor: bool,
}

/// Apply a condition.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConditionEffect {
    pub condition: Condition,
    pub duration: Duration,
    /// The condition ends when the creature takes damage, as with Sleep.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ends_on_damage: bool,
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
    /// Ends once the attack that opened the reaction window has resolved (Parry).
    /// Only an entry with a hit trigger may use it.
    TriggeringAttack,
    /// Lasts until the end of the bearer's next turn; a turn under way when it is applied
    /// does not count (Haste's lethargy). The engine turns it into `until_end_of_turn` when
    /// that next turn starts.
    UntilEndOfNextTurn,
    /// Ends when the bearer's current turn ends.
    UntilEndOfTurn,
}

/// Timed modifier applied to a combatant.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TimedModifier {
    pub target: ModifierTarget,
    pub change: ModifierChange,
    pub duration: Duration,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_ability: Option<String>,
    /// Who the modifier lands on: each target the effect resolves against (the default) or the
    /// creature using the ability.
    #[serde(default, skip_serializing_if = "ModifierBearer::is_target")]
    pub bearer: ModifierBearer,
    /// The modifier ends when its bearer takes damage, as Hypnotic Pattern's Speed 0 does.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ends_on_damage: bool,
    /// Effects that land on the bearer when this modifier ends, with the modifier's source as
    /// the actor (Haste's lethargy). They don't fire when the encounter ends.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_end: Vec<Effect>,
}

/// The creature a timed modifier lands on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModifierBearer {
    /// Each target the effect resolves against; under a save's `on_failure` or an attack's
    /// `on_hit`, that target.
    #[default]
    Target,
    /// The creature using the ability (Shield, Parry).
    #[serde(rename = "self")]
    Actor,
}

impl ModifierBearer {
    /// True for the default, `target`.
    pub fn is_target(&self) -> bool {
        *self == ModifierBearer::Target
    }
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
    /// One extra action on each of the bearer's turns, limited to `only`; an empty list allows
    /// any action. Its change is `none`.
    ExtraAction {
        #[serde(default)]
        only: Vec<ExtraActionUse>,
    },
}

/// What an extra action may be spent on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtraActionUse {
    /// An action whose effects are exactly one single-target attack.
    OneAttack,
    /// The Disengage action.
    Disengage,
}

impl ExtraActionUse {
    /// The snake_case spelling used in state files, refusals and the digest.
    pub fn as_str(self) -> &'static str {
        match self {
            ExtraActionUse::OneAttack => "one_attack",
            ExtraActionUse::Disengage => "disengage",
        }
    }
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

/// Roll modifier on a trait, applied to the bearer's matching rolls while the encounter's
/// lighting meets `lighting`.
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
    /// Dice rolled; absent for flat healing such as the Heal spell's 70.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dice: Option<String>,
    #[serde(default)]
    pub modifier: i32,
    /// Whether a cast adds the caster's spellcasting ability modifier.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub spellcasting_modifier: bool,
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
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

/// One selectable option inside a `MultiattackStep::Choice`. With `cast` set,
/// the option is a use of the `action_id` Spellcasting action to cast that spell.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ChoiceOption {
    pub action_id: String,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast: Option<Cast>,
}

/// Multiattack step. `Action` with `cast` set casts that spell through the
/// `action_id` Spellcasting action. `Alternatives` runs exactly one of its
/// step sets; an empty set makes the whole step optional. Until live prompts
/// and the batch tactics policy choose, the engine runs the first set, so a
/// creature always takes an optional step unless the agent intervenes.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MultiattackStep {
    Action {
        action_id: String,
        count: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cast: Option<Cast>,
    },
    Choice { options: Vec<ChoiceOption>, pick: u32 },
    Alternatives { sets: Vec<Vec<MultiattackStep>> },
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Multiattack {
    pub steps: Vec<MultiattackStep>,
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
    /// The creature never provokes an Opportunity Attack by moving (Flyby, Agile).
    NoProvoke,
}

impl Effect {
    /// Whether this effect is a roll modifier or holds one in an `on_hit`, `on_failure` or area.
    pub fn holds_roll_modifier(&self) -> bool {
        match self {
            Effect::RollModifier(_) => true,
            Effect::Attack(a) => a.on_hit.iter().any(Effect::holds_roll_modifier),
            Effect::SavingThrow(s) => s.on_failure.iter().any(Effect::holds_roll_modifier),
            Effect::Area(a) => a.effect.holds_roll_modifier(),
            _ => false,
        }
    }

    /// Whether this effect, or one nested in it, lasts for the triggering attack.
    pub fn holds_triggering_attack(&self) -> bool {
        match self {
            Effect::TimedModifier(m) => matches!(m.duration, Duration::TriggeringAttack),
            Effect::Condition(c) => matches!(c.duration, Duration::TriggeringAttack),
            Effect::Attack(a) => a.on_hit.iter().any(Effect::holds_triggering_attack),
            Effect::SavingThrow(s) => s.on_failure.iter().any(Effect::holds_triggering_attack),
            Effect::Area(a) => a.effect.holds_triggering_attack(),
            _ => false,
        }
    }
}

/// Resource map for usage tracking on a combatant.
pub type Resources = HashMap<String, i32>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recharge_minimum_accepts_ranges_and_single_values() {
        assert_eq!(recharge_minimum("5-6"), Some(5));
        assert_eq!(recharge_minimum("2-6"), Some(2));
        assert_eq!(recharge_minimum("6"), Some(6));
        assert_eq!(recharge_minimum("4"), Some(4));
    }

    #[test]
    fn recharge_minimum_rejects_everything_else() {
        for bad in ["", "1", "7", "1-6", "7-6", "5-5", "4-5", "+5", "05-6", "five", "5-6-6", "-6"] {
            assert_eq!(recharge_minimum(bad), None, "{bad}");
        }
    }

    #[test]
    fn unparseable_recharge_dice_fail_at_load() {
        assert!(serde_json::from_str::<Usage>(r#"{"recharge":{"dice":"1d6"}}"#).is_err());
        let ok: Usage = serde_json::from_str(r#"{"recharge":{"dice":"3-6"}}"#).unwrap();
        assert!(matches!(ok, Usage::Recharge { dice, .. } if dice == "3-6"));
        let default: Usage = serde_json::from_str(r#"{"recharge":{}}"#).unwrap();
        assert!(matches!(default, Usage::Recharge { dice, .. } if dice == "5-6"));
    }

    #[test]
    fn lighting_meets_its_own_key_and_sunlight_meets_bright_light() {
        use Lighting::*;
        let all = [Unspecified, DimLight, BrightLight, Sunlight];
        let met = [(DimLight, DimLight), (BrightLight, BrightLight), (Sunlight, BrightLight), (Sunlight, Sunlight)];
        for setting in all {
            for required in all {
                assert_eq!(setting.meets(required), met.contains(&(setting, required)), "{setting:?} meets {required:?}");
            }
        }
    }

    #[test]
    fn normal_lighting_loads_as_unspecified() {
        assert_eq!(serde_json::from_str::<Lighting>(r#""normal""#).unwrap(), Lighting::Unspecified);
        assert_eq!(serde_json::to_string(&Lighting::Unspecified).unwrap(), r#""unspecified""#);
        assert_eq!(serde_json::from_str::<Lighting>(r#""dim_light""#).unwrap(), Lighting::DimLight);
    }
}
