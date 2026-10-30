---
id: 035
title: SRD importer vocabulary gaps
type: grilling
mode: HITL
status: closed
blocked_by: []
claimed: false
---

## Question

The ticket 028 importer graduates 186 of 341 monsters. The other 155 are blocked by four vocabulary gaps (multiattack shapes, "Medium or small" size, conditional and later-clause damage, recharge ranges and recharge after rest). How should each be represented?

## Notes

Raised while building ticket 028. A read-only subagent compared options against ADR 0005, ADR 0008, `bench-core/src/effect.rs` and `bench-core/src/engine.rs`; the user approved all four recommendations. Most blockers came from the importer's converter policy (`multiattack()` refuses anything but a choose-1 set of equal-count actions), not from the schema.

## Resolution

1. **Multiattack shapes (about 95 entries).** `MultiattackStep::Choice { options: Vec<ChoiceOption>, count }` with `ChoiceOption { action_id, count (default 1) }`. A swap-style multiattack becomes a fixed `Action { X, N-K }` plus a `Choice` of `[X(1), Y(1)]` with count K. Retire `Multiattack.swap`, which the engine never reads. The engine's `Choice` branch must make K distinct picks instead of repeating `options.first()`: live mode through a pending prompt (ADR 0004), batch through the tactics policy (ADR 0012). Variable counts (`hydra`, `medusa`, `barbed-devil`) stay blocked. Amends ADR 0005 and ADR 0008 (swap fold).
2. **Size "Medium or small" (30 entries).** Keep `size: Size` and add `size_choice: Vec<Size>` (default empty). The fixture stores the first listed size. No `Size::MediumOrSmall` variant.
3. **Conditional and extra damage (51 multi-damage, 20 later-clause).** Later-clause unconditional damage goes into `Attack.on_hit` as `Effect::Damage`. Conditional damage gets `Damage.if_target: Option<Condition>`; the engine skips the entry unless the target has the condition. Corrections stay reserved for upstream faults. Open point for the build ticket: "grappled by this creature" needs source tracking that `Condition` lacks, and the 16 class-a Hit-clause entries need review against the same split.
4. **Recharge.** `Usage::Recharge { dice }` accepts "N-6" or "N" for N in 2..=6, parsed in `roll_recharge`; an unparseable value becomes a load-time error instead of a silent `false`. Recharge after rest (Cloaker) maps to `PerRest { rest: short, uses: 1 }` since one encounter never sees a rest.

ADR impact: ADR 0005 consequences (Choice semantics, `if_target` field, recharge dice format) and ADR 0008 (swap fold text). No new ADR.
