# The standard party is four generated archetypes at every level, with no magic items

Batch mode needs one fixed party so a report's numbers mean something and stay comparable with the SRD's CR guidance, which calibrates CR against four characters (SRD p. 178, p. 202). The standard party is four stat blocks named by role: defender on the front line, a ranged-capable striker, a healer, and an arcane caster. Each approximates a classic class without reproducing it, and the approximated class is noted in a comment on the stat block. The four exist at every level from 1 to 20, generated from one designer-set progression table per archetype. The tables, the generator, and the generated stat blocks are all committed, so a report can cite the exact party that fought. The party carries no magic items. Gear exists only as numbers inside the stat blocks, and the report names this assumption.

The caster genuinely casts, with a small spell list per tier covering single-target damage, area damage, save-based control, and one buff, plus one at-will damage option so it is never inert. The healer carries two or three healing spells plus one at-will option. No summons and no zones, since zones are unmodeled. PCs at 0 hit points follow the ruleset death-save rules: unconscious, rolling death saves, healable, killable by further hits. Monsters die at 0 unless the agent judges otherwise. A user can replace the standard party with their own through `bench batch --party <name>`, with party files in `~/.bench/parties/` and a project layer at `./.bench/parties/` that wins on a same-named party, mirroring the house-rule layers. Live mode always uses the user's party; the standard party is batch scaffolding.

## Considered options

- Hand-authored stat blocks at the tier breakpoints (levels 1, 5, 11, 17). Rejected because the mid-levels are where homebrew CRs are usually misjudged, and the generator costs little next to four tables.
- Transcribing class progressions from the rulebooks into the tables. Rejected because those tables sit outside the SRD's CC license, and the party's job is calibration, not class fidelity. The tables are our own content, and the CR baseline work will verify them empirically against SRD monsters.
- Baked-in magic items at tier boundaries. Rejected for phase 1 as an invisible confound in every number. Revisit if the CR baseline shows the party falling off SRD monsters at high levels.
- A martial-plus-healer party with no real caster. Rejected because a party with no save-based offense cannot probe legendary resistances or condition saves.
- Writing the caster's spells as ordinary stat-block abilities that happen to be named Fireball. Rejected because that sidesteps the spell effects file, and the report could not tell the two paths apart.

## Consequences

- Dying per the ruleset (unconscious, death saves, healable, killable) becomes a PC-only behavioral field on the stat block schema.
- The spell effects file ticket now has a fixed minimum scope it must satisfy before the standard party can run.
- Changing a progression table silently changes the reference numbers behind every batch report, so treat table edits as calibration changes.
- High-level benchmark numbers assume a magic-item-free party, which understates real parties at level 11 and above.
