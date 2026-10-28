# SRD data

JSON and TypeScript schemas copied from [5e-bits/5e-database](https://github.com/5e-bits/5e-database) at the commit in `COMMIT`. Re-import with `scripts/import-srd.sh [commit]`. Don't hand-edit these files. Put corrections and additions in our own layer.

- `2024/` is SRD 5.2.1 content (2024 rules), the only edition the engine models.
- `2024/schemas/` holds the upstream TypeScript schemas.
- `corrections.json` holds the checked-in, cited corrections applied during conversion to fix upstream data faults against the commit in `COMMIT`.

## Licenses

The JSON structure and schemas come from 5e-database under the MIT license (`LICENSE-5e-database.md`). The game content is Wizards of the Coast material from SRD 5.2.1, used under CC-BY-4.0, and modified here by reformatting and restructuring into JSON. Attribution statement:

This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License, available at https://creativecommons.org/licenses/by/4.0/legalcode.

Include no other attribution to Wizards of the Coast or its affiliates; the statement above is the only one the license asks for.

Only SRD content goes in this repo. No non-SRD monsters, classes, or other material.
