# App handlers

A living list of old apps that open old file formats, for the guests
Floppy runs. Floppy uses it three ways:

1. **Opening documents** (`documents.rs`): a document is offered to the
   apps in the user's library that open it.
2. **The wanted-apps list:** a list, like the missing-files list, of
   these apps' program files. A disc maker such as Diskette's Burn A CD
   gathers the user's own copies onto a disc, each with its whole folder
   (see [The wanted-apps list](#the-wanted-apps-list)).
3. **Import Apps Disc:** Floppy finds these apps on that disc and
   imports each into the right guest.

Floppy never downloads or ships an app (rule 3): only the user's own
copies are found.

**Last checked: 2026-09-27.** Re-check entries marked *believed* when
you can, and update the date and the check log at the bottom.

**The code copy** is `HANDLERS` in `src-tauri/src/handlers.rs`. Keep the
two in step: a unit test fails if an app or program name in the code is
missing from this file.

**Confidence:**
- *verified*: confirmed in the source given.
- *believed*: from general knowledge of the program. Probably right, but
  not yet checked against a source.
- **Tested in Floppy** (below) is separate evidence: whether an app
  actually opened a file type when run in Floppy.

## DOS

Documents are matched by extension. The program file is what the
wanted-apps list asks for.

| App | Program | Opens | Confidence | Source |
|---|---|---|---|---|
| WordPerfect | `WP.EXE` | .WP .WP5 .WP6 .WPD .DOC | verified | [DOS Days](https://www.dosdays.co.uk/topics/Software/wordperfect.php), [WPDOS](https://mendelson.org/wpdos/shell.html) |
| Microsoft Word (DOS) | `WORD.EXE` | .DOC | believed | |
| WordStar | `WS.EXE` | .WS .WS4 .WS5 .WS7 .WSD | verified (program) | [PCjs: WordStar 4.00](https://www.pcjs.org/software/pcx86/app/other/wordstar/4.00/) |
| Microsoft Works (DOS) | `WORKS.EXE` | .WPS .WKS .WDB | believed | |
| Lotus 1-2-3 | `123.EXE`, `LOTUS.COM` | .WKS .WK1 .WK3 | verified (`123.EXE`) | [PCjs: Lotus 1-2-3](https://www.pcjs.org/software/pcx86/app/lotus/123/1a/) |
| Quattro Pro | `Q.EXE` | .WQ1 .WQ2 .WKQ | believed (program), verified (.WQ1/.WQ2) | [Wikipedia](https://en.wikipedia.org/wiki/Quattro_Pro) |
| dBASE | `DBASE.EXE` | .DBF | believed | |
| Paradox | `PARADOX.EXE` | .DB | believed | |
| Harvard Graphics | `HG.EXE` | .CHT .SHW | verified | [Harvard Graphics 3.05 (Internet Archive)](https://archive.org/details/hg3_msdos) |
| Deluxe Paint II | `DPAINT.EXE` | .LBM .IFF .BBM | believed | [WinWorld: DeluxePaint II](https://winworldpc.com/product/deluxepaint/2x) |
| PC Paintbrush | `PBRUSH.EXE` | .PCX | believed | [WinWorld: PC Paintbrush IV](https://winworldpc.com/product/pc-paintbrush/iv-dos) |
| AutoCAD | `ACAD.EXE` | .DWG | believed | |
| Turbo Pascal | `TURBO.EXE` | .PAS | believed | |

A document whose extension several apps open (`.DOC`: WordPerfect and
Word) is offered to each of them.

### Versions, and programs that share a name

A DOS program keeps its name from version to version (`WP.EXE` is
WordPerfect 4.2 to 6.2), and different apps use the same name
(`WORD.EXE`). So a name is only a guess. Floppy settles what an app is
in one of two ways:

- **By fingerprint:** one of its programs matches a row of
  [Known versions](#known-versions) by size and SHA-256.
- **By the user:** after a session with an app that isn't settled yet,
  whose program is named like one above, Floppy asks "Which app was
  this?". The same choice is in the app's details (**Is** and
  **Version**).

The library keeps any number of versions of an app side by side. One of
them can be the **favorite**, which documents of its types open with,
even a document last opened in another version.

## Classic Mac

Documents are matched by their Finder type and creator codes, which are
stored with the file rather than in its name. The app file names are
what the wanted-apps list asks for. Mac apps are often renamed, so
several names are listed per app. Codes are from the
[whitefiles.org type/creator list](https://whitefiles.org/dta/pgs/f01_cr_typ_lst.pdf)
unless noted. Opening a Mac document automatically isn't built yet (see
the plan).

| App | App file names | Creator | Document types | Confidence |
|---|---|---|---|---|
| MacWrite | `MacWrite` | `MACA` | `WORD` | verified |
| MacWrite II | `MacWrite II` | `MWII` | `MW2D` | verified |
| MacPaint | `MacPaint` | `MPNT` | `PNTG` | verified |
| MacDraw | `MacDraw` | `MDRW` | `DRWG` | verified |
| MacDraw II | `MacDraw II` | `MDPL` | `DRWG` | verified |
| ClarisWorks | `ClarisWorks`, `AppleWorks` | `BOBO` | `CWWP` `CWDB` `CWSS` `CWGR` | verified ([ClarisWorks](http://justsolve.archiveteam.org/wiki/ClarisWorks)) |
| Microsoft Word | `Microsoft Word` | `MSWD` | `WDBN` `W6BN` | verified |
| Microsoft Excel | `Microsoft Excel` | `XCEL` | `XLS ` `XLS4` `XLW4` | verified |
| Microsoft Works | `Microsoft Works` | `MSWK` | `AWWP` `AWDB` `AWSS` `AWDR` | verified |
| PowerPoint | `Microsoft PowerPoint` | `PPNT` | `SLDS` | verified |
| FileMaker Pro | `FileMaker Pro` | `FMPR` | `FMPR` | verified |
| HyperCard | `HyperCard` | `WILD` | `STAK` | verified |
| PageMaker | `PageMaker`, `Aldus PageMaker` | `ALD3` `ALD4` `ALD5` | `ALB3` `ALB4` `ALB5` | verified |
| QuarkXPress | `QuarkXPress` | `XPR3` | `XDOC` | verified |
| WriteNow | `WriteNow` | `nX^n` | `*WNW` | verified |
| Nisus Writer | `Nisus Writer`, `Nisus` | `NISI` | `TEXT` | verified |
| WordPerfect (Mac) | `WordPerfect` | `WPC2` | `WPD0` `WPD1` `WPD2` | verified |
| MORE | `MORE` | `MOR2` | `MOR3` | verified |
| FullWrite | `FullWrite` | `FWRT` | | verified (creator) |
| SuperPaint | `SuperPaint` | `SPNT` | `SPTG` | verified |
| Canvas | `Canvas` | `DAD2` | `drw2` | verified |
| Photoshop | `Adobe Photoshop` | `8BIM` | `8BIM` | verified |
| FreeHand | `Aldus FreeHand`, `FreeHand` | `FHA3` | `FHD3` | verified |
| Persuasion | `Aldus Persuasion`, `Persuasion` | `PLP2` | `PRS1` `PRS2` | verified |
| MacProject II | `MacProject II` | `MPRX` | `MPRD` | verified |
| TeachText / SimpleText | `TeachText`, `SimpleText` | `ttxt` | `TEXT` `ttro` | verified |
| Compact Pro | `Compact Pro` | `CPCT` | `PACT` | verified |
| Disk Copy | `Disk Copy` | `dCpy` | `dImg` | verified |
| StuffIt Expander | `StuffIt Expander` | `SITx` | `SIT!` | believed (creator), verified (`SIT!`) |

## Amiga

Documents are matched by IFF FORM type, read from the file's first 12
bytes, since Amiga files often have no extension. Other formats are
matched by name, such as ProTracker's `mod.` prefix. Types are from the
[AmigaOS IFF FORM and chunk registry](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry).
Program names are *believed* unless noted, since Amiga program names
vary by version. Opening an Amiga document automatically isn't built
yet.

| App | Program | Opens | Confidence |
|---|---|---|---|
| Deluxe Paint | `DPaint` | IFF `ILBM`, `ANIM`, `PRSP` | believed (program), verified (`PRSP`: DPaint IV) |
| Personal Paint | `PPaint` | IFF `ILBM` | believed |
| Brilliance | `Brilliance` | IFF `ILBM`, `ANIM` | believed |
| ProWrite | `ProWrite` | IFF `WORD` | verified (registry: New Horizons) |
| Flow | `Flow` | IFF `HEAD` | verified (registry: New Horizons) |
| Deluxe Music Construction Set | `DMCS` | IFF `SMUS` | believed |
| AudioMaster | `AudioMaster` | IFF `8SVX` | believed |
| ProTracker | `ProTracker` | `mod.*` modules | believed |
| OctaMED | `OctaMED` | `.med` / MMD modules | verified (format) ([OctaMED module](http://fileformats.archiveteam.org/wiki/OctaMED_module_(MED))) |
| Imagine / Turbo Silver | `Imagine`, `TurboSilver` | IFF `TDDD` | verified (registry: Impulse) |
| Deluxe Video | `DVideo` | IFF `ANBM` | believed (program), verified (registry) |
| Final Writer | `FinalWriter` | its own documents | believed |
| Wordworth | `Wordworth` | its own documents | believed |
| PageStream | `PageStream` | its own documents | believed |
| MultiView | `MultiView` | IFF `ILBM` `8SVX` `FTXT`, AmigaGuide | believed (part of Workbench 3) |

## Known versions

Handler versions known by their program's fingerprint: size in bytes and
SHA-256 of the program file. A new import whose program matches a row is
identified as that app and version without asking.

Rows come from findings (**Export Findings…** in Floppy's gear menu,
merged with `scripts/merge-findings.py`):
a result for an app the user said was, for example, WordPerfect 5.1
carries that program's fingerprint, and merging it adds or updates the
row. Worked and Failed are totals across its tests, over all file types.
A version that has failed more often than it worked is still listed,
since it's still that version. Only fingerprints are published, never
the program itself (rule 3).

Floppy reads this table straight from this file (`handlers.rs`,
`known_versions`), so there's no code copy to keep in step. A unit test
fails if a row doesn't parse or names an app that isn't above.

<!-- versions:start -->
| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |
|---|---|---|---|---|---|---|---|---|
<!-- versions:end -->

## Reported file types

File types Floppy users say a known app opens, beyond the tables above:
extensions they added to the app's **Also opens** once they'd confirmed
which app it is. From merged findings (**Export Findings…**, then
`scripts/merge-findings.py`). Floppy reads this table straight from
this file, so every release offers these apps for these types, with
"Floppy users report that…" as the reason. Reports counts the findings
files that said so. Move an entry into the app's row above once it's
checked, and delete it here (a unit test fails while it's in both).

<!-- filetypes:start -->
| Guest | App | Extension | Reports | Last reported |
|---|---|---|---|---|
<!-- filetypes:end -->

## Reported problems

What Floppy users wrote in an app's **Errors** field in the details
panel, per app, version and program (its file name and the start of its
SHA-256, when Floppy had it). A known app's errors are shared with the
findings; another app's only when the user ticked Share, and it's listed
as "<its name in their library> (unidentified)". From merged findings. Floppy doesn't read
this table: it's for whoever looks into why an app misbehaves in its
emulator (per-app DOSBox settings, a missing driver, a bad copy). Users
typed these, so check a row before acting on it. Delete a row once it's
dealt with.

<!-- problems:start -->
| Guest | App | Version | Program | Errors | Reported |
|---|---|---|---|---|---|
<!-- problems:end -->

## Tested in Floppy

Whether an app opened a file type correctly in its emulator, from users'
answers. After each document session Floppy asks "Did it open
correctly?" and keeps the answer locally (`verify.rs`,
`library/verifications.json`). The answers rank the "Open with" choices.
Nothing leaves the machine until the user chooses **Export Findings…**,
which saves them (with the other findings, `findings.rs`) as a zip. To
merge findings into this document, run:

```sh
scripts/merge-findings.py "Floppy findings 2026-09-27.zip" [more…]
```

Each findings file is merged once (its ID is recorded below). The script adds a
check-log row, and suggests *believed* entries that tests now back up.
Reports carry no document names, only the file type. Counts are totals
across all merged reports. An entry that failed more often than it
worked needs a look before it's trusted. The App column is the handler
only when the user or a fingerprint confirmed it; otherwise it's the
app's name in that user's library, and the version is blank. Results
with a confirmed app, a version and a fingerprint also go into
[Known versions](#known-versions).

<!-- tested:start -->
| Guest | App | Version | Program | File type | Worked | Failed | Last tested | Notes |
|---|---|---|---|---|---|---|---|---|
<!-- tested:end -->
<!-- merged-reports: -->

## The wanted-apps list

**Save Wanted-Apps List…** writes a list in the missing-files list's
format (`cd.rs`). It asks for the program file of every app above that
isn't in the library yet, for all three guests. Two lines near the top
tell a disc maker how to gather apps:

```text
#columns: name size sha1
#gather: folder
#forks: appledouble
# WordPerfect (DOS): .WP .WP5 .WP6 .WPD .DOC
WP.EXE
```

- `#gather: folder`: for each matched file, gather the whole folder it's
  in, with its layout kept, not just the file. DOS and Amiga apps need
  their support files, and Mac apps their dictionaries and plug-ins. A
  maker may skip a folder that's a volume's root or unreasonably large,
  and say so.
- `#forks: appledouble`: keep classic Mac resource forks and Finder
  info by writing an AppleDouble `._name` file beside each file that has
  them. ISO 9660, Joliet and UDF can't hold forks, and a Mac app without
  its resource fork doesn't run. Floppy merges `._name` files back on
  import.
- Both start with `#`, so a reader that doesn't know them skips them. It
  then gathers only the program files, which won't run on their own, so
  a disc maker should support both before offering the list.

**Import Apps Disc…** reads such a disc (an image, or a folder):
- Every file named like a program above is imported as an app of that
  guest. DOS and Amiga apps are imported with their folder. A Mac app is
  imported with its folder, and its `._` files restore the forks.
- A DOS app is imported once per version: a copy whose program is byte
  for byte one already in the library, or already imported from the
  disc, is skipped, and a different version comes in beside it. Mac and
  Amiga apps are imported once, and not when the library has them.

## Check log

| Date | What was checked | Result |
|---|---|---|
| 2026-09-27 | Added version tracking: the Known versions table, and a Version column in Tested in Floppy | No versions known yet. They arrive with version 2 test reports, which carry fingerprints. |
| 2026-09-26 | Mac type/creator codes (whitefiles.org list), Amiga IFF FORM registry (AmigaOS wiki), DOS program names (DOS Days, WPDOS, PCjs, Internet Archive, WinWorld), Quattro Pro and OctaMED formats | Table above. DOS programs for Word, Works, dBASE, Paradox, Deluxe Paint, PC Paintbrush, AutoCAD, Turbo Pascal and Quattro Pro, and most Amiga program names, are still *believed*. |

## Sources

- [Creator code (Wikipedia)](https://en.wikipedia.org/wiki/Creator_code)
- [File Type and Creator Codes (whitefiles.org, PDF)](https://whitefiles.org/dta/pgs/f01_cr_typ_lst.pdf)
- [Macintosh type/creator code (Just Solve the File Format Problem)](http://justsolve.archiveteam.org/wiki/Macintosh_type/creator_code)
- [IFF FORM and Chunk Registry (AmigaOS wiki)](https://wiki.amigaos.net/wiki/IFF_FORM_and_Chunk_Registry)
- [Interchange File Format (Wikipedia)](https://en.wikipedia.org/wiki/Interchange_File_Format)
- [DOS Days: WordPerfect](https://www.dosdays.co.uk/topics/Software/wordperfect.php)
- [PCjs: Lotus 1-2-3](https://www.pcjs.org/software/pcx86/app/lotus/123/1a/), [PCjs: WordStar 4.00](https://www.pcjs.org/software/pcx86/app/other/wordstar/4.00/)
- [Quattro Pro (Wikipedia)](https://en.wikipedia.org/wiki/Quattro_Pro)
- [OctaMED module (Just Solve the File Format Problem)](http://fileformats.archiveteam.org/wiki/OctaMED_module_(MED))
