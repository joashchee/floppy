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
| NerdTracker II | `NT2.EXE` | .NED | verified (program) | [NerdTracker II source mirror](https://github.com/et1999cc/Nerdtracker-II) |

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
| EaglePlayer | `EaglePlayer` | music in the formats its external players cover, by name prefix; with Wanted Team's players `FP.` (Future Player), `SDR.` (Synth Dream), `OSP.` (Synth Pack), `DNS.` (Dynamic Synthesizer), `BSS.` (Beathoven Synthesizer), `MK2.` (Mark II), `TME.`, `SPL.` (Sound Programming Language), Quartet | verified (players) ([Aminet: Eagleplayer](https://aminet.net/package/mus/play/Eagleplayer), [UADE eagleplayer.conf](https://gitlab.com/uade-music-player/uade/-/blob/master/eagleplayer.conf)) |
| DeliTracker | `DeliTracker` | music by its players, by name prefix: `MK2.` (Mark II), `MSO.` (Medley), `TME.`, `MED.` | verified (players in 2.32) ([Aminet: DeliTracker232](https://aminet.net/package/mus/play/DeliTracker232)) |
| HippoPlayer | `HiP` | modules, and by its players `MK2.`, `BSS.`, `MSO.`, `OSP.`, `FP.`, `TME.` | believed (program), verified (players) ([HippoPlayer](https://github.com/koobo/HippoPlayer)) |
| mdxplay | `mdxplay` | X68000 `.MDX` music (with its `.PDX`), through AHI | believed (program) ([Aminet: mdxplay_68k](https://m68k.aminet.net/package/mus/play/mdxplay_68k)) |

## Windows 9x (candidates, no guest yet)

Floppy has no Windows 9x guest, so nothing here is in `HANDLERS` and
Floppy doesn't read this section: it's research for the rust-dos spike
(`ROADMAP.md`). These are 32-bit (or 16-bit) Windows 95/98-era apps that
open formats the DOS, Mac and Amiga apps above don't. "Win9x-only" here
means no DOS or Mac OS build of that version exists; none is blocked from
Windows NT/2000 by anything but its installer, so *9x-era* is the honest
label. Researched 2026-11-25 from search results only: platform and
extension facts come from the source named, everything else is *believed*.
Floppy never downloads or ships any of it (rule 3).

### Apps

| App | Opens | Platform | Confidence | Source |
|---|---|---|---|---|
| Lotus Word Pro 97 | `.LWP` (Ami Pro's successor, also reads `.SAM`) | Win95, NT 4 | verified (platform, `.LWP`) | [PRONOM fmt/340](https://www.nationalarchives.gov.uk/PRONOM/fmt/340), [WinWorld: Word Pro 97](https://winworldpc.com/product/lotus-word-pro/97) |
| Lotus Freelance Graphics 97 | `.PRZ` | Win95, NT 4 | verified (platform), believed (`.PRZ` version) | [Wikipedia: SmartSuite](https://www.wikipedia.org/wiki/Smartsuite), [WinWorld: SmartSuite 97](https://winworldpc.com/product/lotus-smartsuite/97) |
| Lotus Approach 97 | `.APR` | Win95, NT 4 | verified | [PRONOM x-fmt/333](https://www.nationalarchives.gov.uk/PRONOM/x-fmt/333) |
| Corel WordPerfect Suite 8 | `.WPD`, Quattro Pro `.QPW`, Presentations `.SHW` | Win95/NT | believed (formats), verified (it exists) | [WinWorld: WordPerfect 8.x](https://winworldpc.com/product/wordperfect/8x) |
| Microsoft Publisher 97 | `.PUB` | Win95 | believed (`.PUB`), verified (Win95) | [Microsoft, 1996](https://news.microsoft.com/source/1996/09/09/best-selling-microsoft-publisher-program-now-brings-easy-web-publishing-to-small-businesses-and-home-users/) |
| Paint Shop Pro 4/5 | `.PSP` and many bitmap formats | Win95/98 | believed (the source is weak) | [WinWorld: Paint Shop Pro 4.x](https://winworldpc.com/product/paint-shop-pro/4x) |
| Micrografx Picture Publisher 5 to 8 | its own bitmap files (`.PP4`) | Windows | believed (extension) | [WinWorld: Picture Publisher 5.0](https://winworldpc.com/product/micrografx-picture-publisher/50) |
| FruityLoops 1 to 3 | `.FLP` | Win9x | believed (`.FLP` and 1997 origin verified) | [Image-Line history](https://www.image-line.com/company-history) |
| Jeskola Buzz | `.BMX` tracker songs | Win9x-era, freeware | believed (the old builds' platform) | [Wikipedia: Jeskola Buzz](https://wikipedia.com/wiki/Jeskola_buzz) |
| Cakewalk Pro Audio | `.WRK` | Windows | believed (no source found) | |

Not yet researched, worth a pass: Microsoft Access 97 (`.MDB`) and Works 4,
Corel Draw 7 (`.CDR`), Adobe PageMaker 6.5 (`.PM6`), Macromedia Flash 3/4
(`.FLA`) and Director, Sound Forge and ACID, Fractal Design Painter.

### Players still available for download

Where each can be had legally (or from a long-standing archive). Floppy
points at these like "Where Floppy points you" does, and downloads
nothing. Verify hashes: several are third-party mirrors.

| Player | Plays | Win9x status | Where | Confidence |
|---|---|---|---|---|
| Windows Media Player 6.4 | `.ASF`, `.WMA`, `.AVI`, `.MPG`, `.MID` | Win95, 98, NT 4 | [Microsoft's archived page](https://msdn.microsoft.com/en-us/bb676150.aspx) (`Mpie4ful.exe`, about 2.3 MB) | verified |
| Winamp 2.95 | `.MP3`, `.MOD`, `.XM`, `.IT` and plug-ins | runs on Win9x (last official 9x line is 5.35 per a forum claim) | archive.org copies of nullsoft.com ([VOGONS](https://www.vogons.org/viewtopic.php?p=330687)); has known code-execution holes ([Hydrogenaudio](https://hydrogenaudio.org/index.php?msg=904231)) | believed |
| MusicMatch Jukebox 1.0 to 10 | `.MP3`, `.WMA` | Win9x for the early builds | [OldVersion.com](https://www.oldversion.com/windows/musicmatch-jukebox/) (1.0 lists an MD5) | believed |
| VLC 0.8.6i | most video and audio | the last line for Win95/98/ME; needs MSLU (unicows) | [VLC's Windows page](https://videolan.org/vlc/download-windows.php) points to its archives, or KernelEx for newer ones ([VLC wiki](https://wiki.videolan.org/VLC_Features_OS)) | verified (status), the mirror's MD5 unchecked |
| QuickTime 6.5.2 | `.MOV` | Win98/ME | an Apple support page named in 2006 ([Apple thread](https://discussions.apple.com/thread/620843)); may be gone, so try the Internet Archive | believed |
| RealPlayer 5 and G2 | `.RM`, `.RA`, `.RAM` | Win95/NT (32-bit) and Win3.1/95 (16-bit) for 5.0 | RealNetworks' Real Legacy Software Archive, as of 2008 ([snapshot](https://webarchive.library.unt.edu/eot2008/20080921153913mp_/http:/forms.real.com/real/player/blackjack.html)); the old ones carried adware | believed |
| XMPlay | modules, MP3 and more | 3.x on Win98 or later per Wikipedia; a Win95/98 fix exists for 4.0.0.1 | [un4seen.com](https://un4seen.com/xmplay.html) | believed |
| foobar2000 0.8.3 | common audio | claimed the last for Win9x | [foobar2000's old versions](https://foobar2000.org/old) (does not list it) | unverified (one forum post) |

Gaps: no source for Sonique (nothing found), and none says whether any
of these players' installers run under rust-dos's built-in Windows.

### What the spike needs

The test set for `ROADMAP.md`'s rust-dos spike should be a few of the
free ones above, since they need no purchase: Windows Media Player 6.4,
Winamp 2.95, FruityLoops 3 or Jeskola Buzz, and VLC 0.8.6i. All are
Win32, so they test rust-dos's built-in Windows (`--win32`) rather than
its DOS side. The maintainer supplies the copies (rule 3).

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
| 2026-11-25 | Windows 9x candidates and legacy players, by web search only (PRONOM, WinWorld, Microsoft, VOGONS, VLC wiki, Apple and OldVersion pages) | New "Windows 9x" section. Apps mostly *believed*; WMP 6.4 and VLC 0.8.6i's status verified. Not read by Floppy. |
| 2026-10-15 | Music players for formats today's players mostly don't cover: EaglePlayer's and DeliTracker's player lists, HippoPlayer's, mdxplay's (Aminet, UADE's eagleplayer.conf, HippoPlayer's source), NerdTracker II's program (its source mirror) | Rows above. Opening Amiga music by name prefix isn't built, so these reach Floppy through the wanted-apps list and Import Apps Disc. |
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
