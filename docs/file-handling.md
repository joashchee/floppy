# File handling: where dropped files go

A living document. Floppy reads the **Drop choices** table below when
it's built (`drops.rs`, `include_str!`), and follows it when a dropped
file, folder or zip has no clear winner and the user hasn't answered for
that kind of item yet. `scripts/merge-findings.py` fills it from users'
findings (Export Findings…); a unit test fails on a malformed row.

## How Floppy decides

Each dropped item is judged by its contents first, then its name, then
the tab it was dropped on (`drops::options`), and every place it could
go gets a score:

| Score | Meaning | Examples |
|---|---|---|
| 3 | Its contents say so | A DOS `MZ` header in an `.exe`, an Amiga hunk executable, an AmigaDOS floppy, an HFS volume, a Mac `APPL`, a Mac archive, a ROM or startup disk setup still needs, an IFF or Mac type a known app opens |
| 2 | Its name says so, or where it was dropped | `.com`/`.bat`, a disk image extension, an extension a known app opens, the tab it was dropped on (documents only, when nothing in it says otherwise) |
| 1 | It could go there | Any guest's documents, a zip kept whole |

One option scoring highest, at 2 or more, wins. Otherwise, in order:

1. the user's own remembered answer for this kind of item,
2. this table (the choice with the most answers among the options),
3. what other users chose, learned from findings dropped on Floppy
   (`learned.rs`),
4. a dialog listing every option, best first, with why.

The dialog's answer is kept in `library/drop-choices.json`, remembered
unless the user unticks it, and shared by the next Export Findings.

## Signatures

A kind of item, never a name:

- **Kind:** `file`, `folder` or `zip`.
- **Extension:** lowercase, no dot (empty for none and for folders).
- **Contents:** for files `mz`, `hunk`, `adf`, `hfs`, `iff:<type>`,
  `mac-type:<type>`, `macbinary:<type>`, `setup:mac-rom`,
  `setup:mac-startup-disk`, `setup:kickstart`, `setup:workbench`,
  `text`, `binary` or `empty`; for folders and zips
  `programs:<guests>` (`dos`, `mac`, `amiga` joined by `+`, or `none`),
  plus `+setup` when setup files seem to be inside.

## Drop choices

"Goes to" is `App`, `Document` or `Setup file`, then the guest (`DOS`,
`Classic Mac`, `Amiga`). A signature may have several rows: the one
with the most answers wins, and a tie still asks.

<!-- drops:start -->
| Kind | Extension | Contents | Goes to | Answers | Last answered |
|---|---|---|---|---|---|
<!-- drops:end -->

## Check log

| Date | What was checked | Result |
|---|---|---|
| 2026-09-27 | Created, with an empty Drop choices table | Floppy asks for every drop without a clear winner until findings fill it |
