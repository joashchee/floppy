# Floppy AI

A living document. "Floppy AI" is everything Floppy knows about old
files, apps and setup that it didn't have to be told by the user: which
program is which app and version, which apps open which file types,
which setup files are good copies and where to get them legally, and
where dropped files go. It lives in the living documents (Floppy reads
their tables when it's built), and grows from users' findings
(`findings.rs`, `scripts/merge-findings.py`).

Its **version** is the top row of the table below. Floppy reads it
when it's built (`ai.rs`) and shows it next to the app version
("v0.3.0 · AI 1"). A Floppy that learned from a newer pack shows that
version instead, and says so.

## When the version goes up

Whenever what Floppy knows changes, whether or not the app changes:

- `scripts/merge-findings.py` merged findings that changed a table
  Floppy reads ("Known versions", "Reported file types", "Reported by
  users", "Drop choices"): the script adds the row itself;
- "Where Floppy points you" in `docs/legal-setupfiles.md` changed;
- `HANDLERS`, `known_files.rs` or how `drops.rs` judges items changed.

Then `scripts/make-ai-pack.py --site <ansiapps-site>` builds the
knowledge pack for that version and puts it on ansiapps.com, so users
can update their Floppy's knowledge without a new release.

## Knowledge packs

A knowledge pack is a findings zip (`floppy-findings.json` with a
`pack` entry: its AI version and date) whose **materials** are the
living documents themselves: `app-handlers.md`, `legal-setupfiles.md`,
`file-handling.md` and this file. Dropped on Floppy, or picked with
Learn from Findings…, it's learned from like any findings
(`learned.rs`): Floppy parses the materials' tables with the same code
that reads them at build time. A pack's setup sources replace the
built-in ones only when the pack is newer than the build, and only for
links on sites the build already points to; a link elsewhere waits for
a release.

Any findings zip may carry materials: plain `.md` or `.txt` files
under `materials/`, each listed in the JSON's `materials` with its
licence and where it came from, and legal to share on ansiapps.com (the
project's own documents, or text under a licence that allows
redistribution). Never guest software, ROMs, disk images or anything
executable (rule 3), which Floppy refuses anyway. Floppy keeps the
materials it's given in `library/learned/<id>/`, for the user to read.

## Versions

<!-- ai-versions:start -->
| AI version | Date | What changed |
|---|---|---|
| 1 | 2026-09-27 | First version: the handler table, known versions, reported file types, setup sources and known-good setup files, and drop choices as of this date |
<!-- ai-versions:end -->
