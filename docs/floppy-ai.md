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
`pack` entry: its AI version and date, and `floppy-findings.sig`, the
maintainers' Ed25519 signature of that JSON) whose **materials** are the
living documents themselves: `app-handlers.md`, `legal-setupfiles.md`,
`file-handling.md` and this file. Dropped on Floppy, or picked with
Learn from Findings…, it's learned from like any findings
(`learned.rs`): Floppy parses the materials' tables with the same code
that reads them at build time. Only a pack **signed** with a key in "Pack keys" below
counts as one: it raises the Floppy's AI version when it's newer than
the build, and its setup sources replace the build's, for links on
sites the build already points to (a link elsewhere waits for a
release). An unsigned "pack" is learned from as ordinary findings.

Any findings zip may carry materials: plain `.md` or `.txt` files
under `materials/`, each listed in the JSON's `materials` with its
licence and where it came from, and legal to share on ansiapps.com (the
project's own documents, or text under a licence that allows
redistribution). Never guest software, ROMs, disk images or anything
executable (rule 3), which Floppy refuses anyway. Floppy keeps the
materials it's given in `library/learned/<id>/`, for the user to read.

## Safeguards

Findings and packs come from other people, so everything that learns
from them assumes they may be hostile. Whenever the code for Floppy's AI
processes changes (`drops.rs`, `learned.rs`, `findings.rs`, `ai.rs`, the
parts of `handlers.rs`, `cd.rs` and `documents.rs` they reach, and
`scripts/merge-findings.py` and `make-ai-pack.py`), it's reviewed for
abuse against this list, and the list grows with every new safeguard.

| Threat | Safeguard |
|---|---|
| A fake "official" pack: a higher AI version, setup links to pirated ROMs or malware on a shared host | Only packs signed with a key in "Pack keys" raise the AI version or change setup sources (`ai::verify`, Ed25519, strict); setup links only to sites the build already points to; the signing key never enters a repo |
| A pack's materials swapped after signing | Every material lists its SHA-256 in the signed JSON; a signed pack's material without one, or that doesn't match, is refused |
| Guest software, programs or disk images smuggled in as materials | Only plain `.md`/`.txt` with short plain names, UTF-8 without NULs, at most 1 MiB each, 4 MiB and 32 files per findings file, kept in the library's own folder, never run |
| Findings that overrule what Floppy knows | Floppy's built-in knowledge always wins; a conflicting fingerprint is skipped and listed; the user's own answers and identities beat anything learned |
| A learned identity passed off as certain | Apps identified from learned fingerprints say so (`IdentifiedBy::Learned`), and are never exported as the user's own (no laundering through re-export) |
| Other people's test results burying the user's working app | Only the user's own results can move an app to the end of "Open with"; learned results only break ties |
| Program files taught as document types | `.EXE`, `.COM`, `.BAT`, `.SYS`, `.DLL`, `.OVL`, `.DRV`, `.PIF` are never learned as document extensions |
| Junk or paths passed off as program names | A learned program name must be a plain file name: no path separators, quotes, backticks or markup, at most 64 characters (Floppy and `merge-findings.py` alike) |
| Huge findings stalling or bloating Floppy | 16 MiB per findings file; at most 5,000 entries of each kind per file, and 50,000 of each kind learned in all; counts saturate instead of overflowing |
| Spoofed text in names and messages (bidi overrides, zero-width characters) | Stripped or refused in every string learned; messages show at most 64 characters of a name |
| A dropped file read whole to judge it | Only the first 512 bytes, and a whole file only when it's at most 64 MiB (MacBinary) |
| Findings that break the living documents when merged (table markers, HTML, pipes) | `merge-findings.py` strips `<!--`, `-->`, `<`, `>`, backticks, pipes and newlines from every cell, and IDs to letters, digits, `-` and `_` |
| The same findings counted twice, or a Floppy learning from its own exports | Each findings ID is learned or merged once; a library's own export IDs are never learned |
| Anything learned turning out bad | Forget What Was Learned… empties it all, materials included |

## Pack keys

Public keys (Ed25519, hex) whose signatures make a pack official. Make
one with `cargo run --manifest-path src-tauri/Cargo.toml --example
ai-pack-key -- new <keyfile>` on the maintainer's own machine; the
secret stays in `<keyfile>`, outside every repo. Until a key is listed
here, no pack is official: packs still teach, as ordinary findings.

<!-- pack-keys:start -->
| Key | Added | Note |
|---|---|---|
<!-- pack-keys:end -->

## Versions

<!-- ai-versions:start -->
| AI version | Date | What changed |
|---|---|---|
| 1 | 2026-09-27 | First version: the handler table, known versions, reported file types, setup sources and known-good setup files, and drop choices as of this date |
<!-- ai-versions:end -->
