# CR and encounter guidance in SRD 5.2

Research for ticket 009. Researched 2026-10-24.

## Short answer

SRD 5.2.1 gives three numeric things that depend on CR or party level: the XP value of each CR, the proficiency bonus of each CR, and an XP budget per character for each party level at three difficulties. It also gives one prose rule of thumb about what CR means. It does not give expected hit points, armor class, attack bonus, damage per round or save DC by CR. I searched the full text for such a table and found none. Whether the 2024 Dungeon Master's Guide (DMG) contains one could not be confirmed, because the DMG body text is paywalled and I did not read it.

## Sources and how they were read

All page numbers below are the printed page numbers in the SRD PDF footer, which match the PDF page index. Citations of the form "SRD p. N" mean that page.

| Source | Location | Notes |
| --- | --- | --- |
| SRD 5.2.1 PDF (Wizards of the Coast, 364 pages, PDF created 2025-04-23) | https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf | Primary source for every number below. Text extracted with `pdftotext` and read page by page. |
| SRD 5.2 PDF (earlier release, 361 pages) | https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.pdf | Read only the legal page (p. 1) to compare attribution wording. |
| Official SRD landing page and FAQ | https://www.dndbeyond.com/srd (mirrored at https://dnd.wizards.com/resources/systems-reference-document) | License and revocation statements. |
| CC BY 4.0 legal code | https://creativecommons.org/licenses/by/4.0/legalcode.en | License text. |
| Official 2024 DMG table of contents | https://www.dndbeyond.com/sources/dnd/dmg-2024 | Headings only. Chapter bodies redirect to the D&D Beyond marketplace, so they are paywalled. |
| Official free Basic Rules 2024, "DM's Toolbox" | https://www.dndbeyond.com/sources/dnd/br-2024/dms-toolbox | Headings checked to see whether a monster-creation section is free. It is not. |
| D&D Beyond article on building combat encounters | https://www.dndbeyond.com/posts/1901-creating-combat-encounters-using-the-new-dungeon | First-party article about DMG chapter 4. |

The local copy in `data/srd/2024/` is a derived import from the 5e-database project (see `data/srd/README.md`), not a Wizards document. I used it only as a cross-check of the XP and proficiency bonus tables. Details are in "Local data check" below.

## What SRD 5.2.1 contains

### 1. What CR means

SRD p. 178 ("Rules Glossary", Challenge Rating): "Challenge Rating (CR) summarizes the threat a monster poses to a group of four player characters." If the CR is higher than the characters' level the monster is likely a danger, and if lower it likely poses little threat. The same entry says circumstances and the number of player characters can significantly alter how threatening a monster is, and points to "Gameplay Toolbox" ("Combat Encounters") for planning guidance.

SRD p. 202 gives the one calibration sentence: "a single monster generally presents a low-difficulty challenge for a party of four characters whose level equals the monster's Challenge Rating."

SRD p. 203 ("Powerful Creatures") warns that a creature whose CR is above the party's level might take out a character with one action, with an Ogre (CR 2) against a level 1 Wizard as the example. It also advises not using a monster with a hard-to-overcome feature against characters whose level is below its CR.

### 2. XP value by CR

SRD p. 256 ("Monsters", Experience Points by Challenge Rating). XP is awarded for defeating the monster in combat or otherwise neutralizing it. A summoned monster is worth the XP in its stat block unless a rule says otherwise (same page).

| CR | XP | CR | XP |
| --- | --- | --- | --- |
| 0 | 0 or 10 | 14 | 11,500 |
| 1/8 | 25 | 15 | 13,000 |
| 1/4 | 50 | 16 | 15,000 |
| 1/2 | 100 | 17 | 18,000 |
| 1 | 200 | 18 | 20,000 |
| 2 | 450 | 19 | 22,000 |
| 3 | 700 | 20 | 25,000 |
| 4 | 1,100 | 21 | 33,000 |
| 5 | 1,800 | 22 | 41,000 |
| 6 | 2,300 | 23 | 50,000 |
| 7 | 2,900 | 24 | 62,000 |
| 8 | 3,900 | 25 | 75,000 |
| 9 | 5,000 | 26 | 90,000 |
| 10 | 5,900 | 27 | 105,000 |
| 11 | 7,200 | 28 | 120,000 |
| 12 | 8,400 | 29 | 135,000 |
| 13 | 10,000 | 30 | 155,000 |

The PDF prints CR 11 to 13 and CR 28 to 30 as a short continuation block at the top of p. 256, above the main table. The CR 0 value is "0 or 10" as printed; the SRD does not say when each applies.

Stat blocks repeat the value in the form "CR 10 (XP 5,900; PB +4)" (for example the Aboleth). Some stat blocks give an extra lair value, such as "CR 10 (XP 5,900, or 7,200 in lair; PB +4)".

### 3. Proficiency bonus by CR

SRD p. 256 ("Monsters", Proficiency Bonus by Challenge Rating). The text says a monster's proficiency bonus (PB) "is reflected in saving throws, skills, and other statistics".

| CR | PB |
| --- | --- |
| 0 to 4 | +2 |
| 5 to 8 | +3 |
| 9 to 12 | +4 |
| 13 to 16 | +5 |
| 17 to 20 | +6 |
| 21 to 24 | +7 |
| 25 to 28 | +8 |
| 29 to 30 | +9 |

### 4. XP budget per character (encounter difficulty)

SRD p. 202 ("Gameplay Toolbox", Combat Encounter Difficulty, Step 2). Multiply the per-character number by the number of characters in the party to get the encounter's XP budget.

| Party level | Low | Moderate | High |
| --- | --- | --- | --- |
| 1 | 50 | 75 | 100 |
| 2 | 100 | 150 | 200 |
| 3 | 150 | 225 | 400 |
| 4 | 250 | 375 | 500 |
| 5 | 500 | 750 | 1,100 |
| 6 | 600 | 1,000 | 1,400 |
| 7 | 750 | 1,300 | 1,700 |
| 8 | 1,000 | 1,700 | 2,100 |
| 9 | 1,300 | 2,000 | 2,600 |
| 10 | 1,600 | 2,300 | 3,100 |
| 11 | 1,900 | 2,900 | 4,100 |
| 12 | 2,200 | 3,700 | 4,700 |
| 13 | 2,600 | 4,200 | 5,400 |
| 14 | 2,900 | 4,900 | 6,200 |
| 15 | 3,300 | 5,400 | 7,800 |
| 16 | 3,800 | 6,100 | 9,800 |
| 17 | 4,500 | 7,200 | 11,700 |
| 18 | 5,000 | 8,700 | 14,200 |
| 19 | 5,500 | 10,700 | 17,200 |
| 20 | 6,400 | 13,200 | 22,000 |

Definitions of the three difficulty levels, SRD p. 202 (Step 1):

- Low: "likely to have one or two scary moments for the players, but their characters should emerge victorious with no casualties", though some may need healing resources.
- Moderate: "Absent healing and other resources, an encounter of moderate difficulty could go badly for the adventurers", with weaker characters possibly taken out and "a slim chance that one or more characters might die".
- High: "could be lethal for one or more characters", needing smart tactics, quick thinking, and maybe luck.

Step 3 (SRD p. 202 to 203): spend as much of the budget as you can without going over, using the XP in each stat block. Worked examples confirm the arithmetic: four level 1 characters at Low gives 50 x 4 = 200 XP; five level 3 characters at Moderate gives 225 x 5 = 1,125 XP; six level 15 characters at High gives 7,800 x 6 = 46,800 XP.

Troubleshooting notes on SRD p. 203 that matter for a simulator:

- More than two creatures per character raises the chance that a lucky streak deals more damage than expected, so add fragile creatures. This matters most at levels 1 and 2.
- CR 0 creatures, especially ones worth 0 XP, should be used sparingly.
- Referencing more than two or three distinct stat blocks in one encounter gets hard to run.

What the table does not say: the SRD gives no multiplier for the number of monsters, no party-size correction beyond "multiply by the number of characters", and no budget per adventuring day. The phrase "adventuring day" does not appear anywhere in the SRD text.

### 5. Character XP thresholds

SRD p. 23 (Level Advancement, Character Advancement table) gives the XP needed to reach each level and the character proficiency bonus. SRD p. 182 ("Rules Glossary", Experience Points) says a character levels up when the XP total crosses these thresholds.

| Level | XP | PB | Level | XP | PB |
| --- | --- | --- | --- | --- | --- |
| 1 | 0 | +2 | 11 | 85,000 | +4 |
| 2 | 300 | +2 | 12 | 100,000 | +4 |
| 3 | 900 | +2 | 13 | 120,000 | +5 |
| 4 | 2,700 | +2 | 14 | 140,000 | +5 |
| 5 | 6,500 | +3 | 15 | 165,000 | +5 |
| 6 | 14,000 | +3 | 16 | 195,000 | +5 |
| 7 | 23,000 | +3 | 17 | 225,000 | +6 |
| 8 | 34,000 | +3 | 18 | 265,000 | +6 |
| 9 | 48,000 | +4 | 19 | 305,000 | +6 |
| 10 | 64,000 | +4 | 20 | 355,000 | +6 |

This is character leveling XP. It is not the 2014 "XP threshold per character" encounter system, which the SRD 5.2.1 does not use.

### 6. Running a monster

SRD p. 255 ("Running a Monster") gives behavior rules meant to keep a monster "in accordance with its Challenge Rating": use limited-use damaging abilities as soon and as often as possible, use Multiattack on turns without a stronger ability, and use Bonus Actions, Reactions and Legendary Actions as often as possible. This is relevant to the batch tactics policy, because it is the SRD's own statement of how a monster is assumed to behave when CR is computed.

### 7. Other level-scaled numbers that are not CR guidance

These are not monster CR benchmarks but exist in the Gameplay Toolbox and might tempt reuse:

- Trap damage and save DC by character level bands (for example the poison needle trap table on SRD p. 201: levels 5 to 10, 11 (2d10) damage, DC 13; levels 11 to 16, 22 (4d10), DC 15; levels 17 to 20, 55 (10d10), DC 17).
- Sample fear and mental stress DCs of 10, 15 and 20 and Psychic damage of 1d6, 3d6 and 9d6 (Fear and Mental Stress, Gameplay Toolbox, SRD p. 196).

### 8. What is absent

I searched the full SRD 5.2.1 text for "damage per round", "Monster Statistics", "Attack Bonus", "Save DC" and "adventuring day", and read the Monsters introduction (SRD pp. 254 to 257). The only hits for attack bonus and save DC are the character formulas and individual spell and stat block entries. Nothing gives expected HP, AC, attack bonus, damage per round, or save DC for a given CR. The SRD also never names the Dungeon Master's Guide or the Monster Manual. The only monster-design content is the stat block layout, hit dice by size (SRD p. 255, for example Medium d8 averaging 4.5 and Large d10 averaging 5.5), the proficiency bonus table, and the XP table.

The only indirect source of benchmark numbers in the SRD is its monster stat blocks ("Monsters A to Z", SRD p. 258 onward; the local import has 341 entries), each of which states CR, AC, HP, attack bonuses, damage and save DCs. A CR curve could be fitted to these empirically. That is a method choice for a later ticket, not something the SRD asserts.

## What exists only in the 2024 DMG, or could not be confirmed

Could not be confirmed. The DMG body is behind the D&D Beyond marketplace paywall (a request for `.../dmg-2024/dms-toolbox` redirects to the marketplace), so I read only what Wizards publishes openly.

What I did confirm from the official table of contents (https://www.dndbeyond.com/sources/dnd/dmg-2024):

- DMG chapter 3 ("DM's Toolbox") has a section headed "Creating a Creature". Its contents are unknown to me. I cannot say whether it holds a table of HP, AC, attack bonus, damage per round and save DC by CR.
- DMG chapter 4 ("Creating Adventures") has "Plan Encounters" and "Combat Encounters". The D&D Beyond article (https://www.dndbeyond.com/posts/1901-creating-combat-encounters-using-the-new-dungeon) names the sections "Combat Encounter Difficulty", "XP Budget per Character" and "Troubleshooting", and its one numeric example, 750 XP for a level 5 character at Moderate difficulty, matches the SRD p. 202 table. The free Basic Rules 2024 "DM's Toolbox" page lists the same Combat Encounter Difficulty steps. So the encounter budget table and its troubleshooting advice appear to be shared between the DMG, the free Basic Rules and the SRD, not DMG-only. I did not diff the DMG text against the SRD text.
- The free Basic Rules 2024 "DM's Toolbox" page has no "Creating a Creature" section, so that section is not part of the free rules.

Unconfirmed claim, not from a primary source: a web search surfaced community write-ups (for example Blog of Holding) saying the 2024 DMG drops the 2014 DMG's monster statistics by CR table and offers little monster-building guidance beyond reskinning. These are secondary sources that I did not follow back to the DMG text, so treat the claim as unverified. Someone who owns the 2024 DMG should read "Creating a Creature" in chapter 3 and record whether it has a by-CR statistics table.

Also confirmed absent from SRD 5.1 (the 2014-era SRD): a search of its text found no "Monster Statistics" table and no encounter difficulty or XP threshold section. The 2014 DMG table (reported by community sources as being at page 274) is therefore not available under Creative Commons from any SRD, and it is a 2014 rules artifact in any case.

## License terms and attribution

### What the SRD says

SRD p. 1 ("Legal Information", SRD 5.2.1): the document is provided by Wizards of the Coast LLC under CC-BY-4.0. "You are free to use the content in this document in any manner permitted under CC-BY-4.0, provided that you include the following attribution statement in any of your work":

> This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.

It adds: "Please do not include any other attribution to Wizards or its parent or affiliates other than that provided above." You may state that your work is "compatible with fifth edition" or "5E compatible." Section 5 of CC-BY-4.0 (disclaimer of warranties and limitation of liability) applies.

The SRD 5.2 release (SRD p. 1 of the 361 page PDF) has the same text with "SRD 5.2" in place of "SRD 5.2.1" and the same URL.

The official SRD page FAQ states that CC-BY-4.0 is irrevocable for SRD 5.2, that Wizards "cannot revoke or alter SRD 5.2 or remove it from Creative Commons", that each future SRD version is independently licensed, and that SRD 5.2 may be used commercially, including crowdfunding, "as long as you follow attribution guidelines". It says the only requirement is attribution.

### What CC-BY-4.0 requires (legal code, https://creativecommons.org/licenses/by/4.0/legalcode.en)

- Section 3(a)(1): when you share the material, including modified, retain the creator identification, copyright notice, license notice, disclaimer notice and a URI or hyperlink where reasonably practicable, and indicate whether you modified it.
- Section 3(a)(2): you may satisfy those conditions in any reasonable manner for the medium, including a link to a page that has the information.
- Section 3(a)(3): remove the attribution information on request of the licensor, to the extent reasonably practicable.
- Section 3(a)(4): a license you apply to adapted material must not stop recipients from complying with CC-BY-4.0.
- Section 2(a)(5)(B): no extra terms or technical measures that restrict recipients' exercise of the licensed rights.
- Section 2(a)(6): the license does not permit implying that you or your use is endorsed by or connected to the licensor.
- Section 2(b)(2): no trademark rights are granted.
- Section 4: if all or a substantial part of a database is included in your own database, your database as a whole becomes adapted material, and you follow Section 3(a) when sharing it. This is relevant to shipping the JSON data files.

### Consequences for this repo

1. Shipping and quoting the SRD numbers is permitted with attribution. The XP table, proficiency bonus table, XP budget table and the quoted guidance sentences all come from the SRD and fall under CC-BY-4.0. No permission is needed, including for commercial use.
2. The current attribution in `data/srd/README.md` does not match the wording Wizards now requires. It cites "SRD 5.2" and the URL `https://dnd.wizards.com/resources/systems-reference-document`. The SRD 5.2.1 legal page, and the 5.2 legal page, both give `https://www.dndbeyond.com/srd` as the "available at" URL. The data under `data/srd/2024/` also may be an SRD 5.2 or 5.2.1 import (I did not establish which version the 5e-database commit tracks). Action for a later ticket: pick the SRD version the project follows, and use that version's statement verbatim, with no extra attribution to Wizards. This is the main license finding for this ticket. It is not legal advice.
3. Modification must be indicated. If the project reshapes the tables into its own format or code constants (the project plans to import everything into its own format), say so in the notice, for example "reformatted and converted to JSON and Rust".
4. The statement must travel with the data. Because the repo ships prebuilt binaries and data files (per the engine and packaging decisions), the notice should be reachable from each shipped artifact (a `NOTICE` or `LICENSE-SRD` file in the release, plus the repo README), not only from a `data/srd/` README. Section 3(a)(2) allows a link, which keeps this light.
5. Do not ship or quote numbers that come from the DMG but not the SRD. Anything outside the SRD stays under Wizards' ordinary copyright, and CC-BY-4.0 does not cover it. If the DMG's "Creating a Creature" section does contain a by-CR statistics table, copying its rows into the repo would not be covered by the SRD license. Because I could not read that section, the safe rule is: use only numbers cited to an SRD page. Facts and arithmetic derived from SRD stat blocks (for example a fitted CR curve) are our own analysis, but should say they were derived from SRD stat blocks.
6. The project's own report may compare results to the SRD CR guidance with a citation to the SRD page, with the SRD attribution in the report footer or in a linked notice.
7. Trademarks and branding are not licensed (Section 2(b)(2)). The "compatible with fifth edition" or "5E compatible" statement is the only branding the SRD invites. SRD 5.2 monsters that Wizards kept out (for example the Beholder) must not be added.

The official SRD FAQ also says some SRD 5.2 spell, item and monster names differ from the core rulebooks to avoid protected trademarks, while the functionality is the same. That is a naming point, not a CR point.

## Local data check (`data/srd/2024/`)

`5e-SRD-Monsters.json` holds 341 monsters with `challenge_rating`, `xp`, `proficiency_bonus`, `hit_points` and armor class. Comparing its `xp` and `proficiency_bonus` per CR against SRD p. 256 found them identical to the PDF tables, with two notes:

- CR 0 monsters carry either 0 or 10 XP, matching the "0 or 10" in the table.
- CR 12 shows two XP values: the Erinyes has 8,400 (SRD p. 283, matches the table) and the Archmage has 8,000 (SRD p. 305, "CR 12 (XP 8,000; PB +4)"). The Archmage stat block itself says 8,000, which disagrees with the table on p. 256. This is an inconsistency in the SRD, not an import error. The engine needs a rule for which value wins; the SRD import cross-check decided in ticket 005 (monster schema and state model) is the place to flag it.

The local data has no encounter budget table and no per-CR benchmark stats, as expected.

## Open points and things not confirmed

- Contents of DMG chapter 3 "Creating a Creature": not read, not confirmed either way.
- Whether the DMG contains anything in its encounter chapter beyond the SRD's Combat Encounter Difficulty text: not confirmed. The sections I could compare by heading and one numeric example match.
- Which SRD version (5.2 or 5.2.1) the `data/srd/` import follows: not established.
- The CR 0 rule for "0 or 10" XP: the SRD does not say when to use which.
- The method Wizards used to assign CR to SRD monsters: the SRD does not publish one.

## What this means for the project

For the CR baseline and standard party build tickets: the SRD supplies an encounter-level baseline (XP budget at Low, Moderate and High for party levels 1 to 20, with one monster of CR equal to party level being Low for four characters) and the XP and proficiency bonus tables. It does not supply per-CR damage per round, HP, AC, attack bonus or save DC, so a report that compares a homebrew monster's damage per round with "CR guidance" cannot cite an SRD table for those. Options for the later grilling ticket: compare only on XP and CR bracket; derive an empirical curve from the SRD monster stat blocks and label it as derived; or have the user supply 2024 DMG numbers locally and keep them out of the repo.
