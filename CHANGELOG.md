# Changelog

All notable changes to Floppy are recorded here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/), newest first.

Update this file whenever a change lands. A short bullet is enough, dated
against the day the change was made.

## Unreleased

- **Back up Floppy's system: Burn A CD** (2026-09-27): once the system
  is complete (the Mac has its ROM and startup disk, or the Amiga its
  Kickstart, and nothing is half set up), a strip offers to Burn A CD:
  one compressed disc image (.iso) of the setup files and their
  settings, for restoring after a reinstall or on a new computer. **Not
  Now** holds until the files change. **Backup Floppy System…** in the
  gear menu does the same whenever you like. Floppy writes the ISO
  itself (no tools needed, same on macOS and Linux). To restore, open or
  drop the disc (or leave it in Downloads): Floppy reads it without
  mounting, checks each file's SHA-256, and fills only what isn't set
  up, keeping the rest. A 25 MB Mac setup made a 72 KB disc in testing.
- **AROS source notice in every release** (2026-09-27): the release
  script's source step (`scripts/fetch-sources.sh`) also writes
  `AROS-SOURCE.txt`, naming the AROS build inside the bundled FS-UAE (read
  from its `fs-uae.dat`), its licence (AROS Public License) and where its
  source is, to attach to the release with the emulator sources. It fails
  if FS-UAE stops carrying an AROS ROM it can identify.
- **Start the Amiga now with the free AROS Kickstart** (2026-09-27): with
  no Kickstart ROM, the Amiga setup offers **Use AROS for Now**, the
  open-source replacement already inside FS-UAE, with its caveats (some
  games and demos run, many programs don't, no Commodore Workbench). The
  Kickstart stays on every "still needed" list, and as soon as Floppy
  finds a real one (dropped, in Downloads, on a files disc, or chosen) it
  switches to it and says so. **Stop Using AROS** turns it off. A
  "didn't work" setup report on the Kickstart notes that AROS was
  running. CLAUDE.md's rule 3 names AROS as its one exception.
- **Less friction getting setup files** (2026-09-27):
  - Each missing ROM or startup disk lists where to get it: free, paid,
    or how to copy it from hardware you own, with plain notes (System
    7.5.3 is a free Apple release, but the Internet Archive copy is a
    mirror). The list is `docs/legal-setupfiles.md`'s new "Where Floppy
    points you" table, read when Floppy is built. **Open Page** opens it
    in the browser.
  - After that, Floppy checks the Downloads folder whenever its window
    comes back to the front and adds what it recognizes by content, so
    a downloaded System 7.5.3 image is set up without any dragging.
    **Look in Downloads** checks on demand.
  - **Report a Setup Problem…** keeps a note (source broken, file didn't
    work, better source) for the next **Export Findings…**, and
    `merge-findings.py` adds it to the new "Reported setup notes" table.
  - A missing emulator gets its own strip with **Locate…**, for DOSBox
    Staging and FS-UAE as well as Basilisk II, and the message no longer
    tells release users to run a build script.
  - Dropdowns are drawn by Floppy, so they follow the theme on Linux,
    where WebKitGTK's native ones came out pale and hard to read.
  - `docs/legal-setupfiles.md` re-checked: Amiga Forever Essentials'
    own page and E-Maculation's Mac ROM capture guide added.
- **Basilisk II for Linux** (2026-09-27): the Basilisk II workflow also
  builds a Linux x86-64 binary (the Unix build with SDL2 linked
  statically, so it needs only libc and libstdc++). A `platforms` input
  picks the builds, and publishing to an existing release only adds what
  it lacks. `fetch-basilisk.sh` fetches it on Linux, and Floppy bundles
  it there like on macOS.
- **Linux support, first pass** (2026-09-27):
  - Policy: macOS and Linux are supported, Windows comes later
    (`CLAUDE.md` rule 7, `docs/platform-parity.md`).
  - Builds on Linux: `flate2`'s pure-Rust backend is now a direct
    dependency. Only a macOS-only Tauri dependency had been switching it
    on, so zip imports failed to compile elsewhere.
  - `fetch-dosbox.sh` and `fetch-fs-uae.sh` fetch the pinned Linux x86-64
    releases, which Floppy bundles in its `.deb`/AppImage. Without them,
    `dosbox-staging` (or a `dosbox` that is Staging), `fs-uae` and
    `BasiliskII` on `PATH` are used. The DOSBox end-to-end tests pass on
    Linux.
  - Classic Mac imports work on Linux: resource forks and Finder info go
    in the `.rsrc/` and `.finf/` folders Basilisk II reads there.
  - Disc images (Import Files Disc, dropped setup files) mount read-only
    through udisks, with no root needed.
  - `build-release.sh` checks the Linux bundles for home-folder paths.
    "Show in Finder" reads "Show in Folder", and Locate Basilisk II… takes
    any program.
- **Errors field, names that follow Is and Version, exports that don't
  repeat** (2026-09-27):
  - Each app's details panel has an **Errors** field: what goes wrong
    running it. For a known app, Export Findings carries it (the user's
    home folder reads `~`), and `merge-findings.py` adds it to a new
    "Reported problems" table in `docs/app-handlers.md`. Another app's
    errors stay on the Mac unless you tick **Share in Export Findings**,
    which sends them with the app's name and program fingerprint.
  - Saying what an app is (Is and Version, "Which app was this?", or a
    known fingerprint on import) names it for that: "WordPerfect 5.1".
    An app you renamed keeps your name.
  - Export Findings holds only what's new since the last export
    (`library/findings.json`, and each test result is marked exported),
    because merging adds counts up. The gear menu counts new findings
    and says when you last exported.
- **Export Findings** (2026-09-27): the gear menu's Export Test Report is
  now **Export Findings…**, a zip of everything Floppy has learned that
  could help every Floppy: test results, apps and versions you
  identified (by program fingerprint), file types you added to a known
  app, and ROMs or Workbench floppies it didn't know. No documents,
  files or file names. `scripts/merge-findings.py` (was
  `merge-handler-tests.py`, still reads old reports) merges it into the
  living documents, which Floppy reads when it's built:
  - new "Reported file types" table (`docs/app-handlers.md`): users'
    extensions, offered in "Open with";
  - new "Reported by users" table (`docs/legal-setupfiles.md`): setup
    files the missing-files list now asks for by content;
  - program names in "Known versions" count as clues for "Which app
    was this?".
- **Setup files per tab** (2026-09-27): the "Setup files needed" strip,
  Import Files Disc and the drop overlay's setup target show only on the
  tab of the guest that needs them, listing just its files. The DOS tab,
  which needs none, no longer shows them.
- **App versions** (2026-09-27): Floppy tells DOS app versions, and
  different apps with the same program name (`WORD.EXE`), apart.
  - Each DOS app's programs are fingerprinted (size and SHA-256). One
    that matches a row of the new "Known versions" table in
    `docs/app-handlers.md` is recognized as that app and version.
  - After a session with an app Floppy only knows by its program's
    name, it asks "Which app was this?". The app's details have the
    same choice (**Is** and **Version**), and the library list shows
    each app's identity.
  - Keep several versions side by side, and mark one the **favorite**:
    "Open with" puts it first, even for a document last opened in
    another version. Import Apps Disc brings each version in once.
  - Test results record the confirmed app, version and fingerprint, so
    two `WORD.EXE`s keep separate records. Test reports are version 2,
    and `merge-handler-tests.py` adds confirmed versions to Known
    versions, reporting any fingerprint already listed as something else.

## 0.3.0 (2026-09-27)

- **Basilisk II is bundled** (2026-09-27): the classic Mac works out of
  the box, like DOS and Amiga. `scripts/fetch-basilisk.sh` pins Floppy's
  own universal build of macemu `892eeb7` (made by
  `.github/workflows/basilisk.yml`), and every release now ships its
  source plus the GMP and MPFR it links. A Basilisk II you pick with
  Locate Basilisk II… still takes precedence.
- **Locate Basilisk II…** (2026-09-26): a gear-menu item to point Floppy
  at a Basilisk II that isn't in `/Applications` or `~/Applications`
  (say, still in Downloads). Pick the `.app` (or its executable); Floppy
  remembers it in `library/emulators.json` and the status pill shows
  "(located)". If it moves, Floppy falls back to the usual places.
- **Ask Diskette** (2026-09-26): while Diskette is running and Floppy
  still needs setup files or old apps, a strip (and a gear-menu item)
  offers to ask it (`request.rs`). Floppy sends one list of everything
  missing, Diskette offers to Burn A CD, and the disc it makes comes
  straight back to Floppy, which imports its setup files and apps.
  - Dropping a Burn A CD disc anywhere on the window now imports both
    its setup files and its apps, even once setup is done.
  - List directives (`#gather:`, `#forks:`) now apply to the lines after
    them, so one list can ask for system files and apps.
  - Apps on a disc never go on the ignore list: only files that matched
    a missing-files line can.
- **Quieter log** (2026-09-26): logging starts at Info, so `tauri dev`
  no longer prints a flood of `[TRACE]` windowing events.
- **ANSIapps theme** (2026-09-26): the ansiapps apps' alternative
  old-school DOS look, switched on from the gear menu and remembered
  across launches (`docs/ansiapps-theme.md`). Blue double-framed
  panels, gray Turbo Vision dialogs, 16 text-mode colors, and the
  IBM VGA 8x16 font by VileR (CC BY-SA 4.0, shipped unmodified,
  credited in About Floppy). Floppy's green stays its accent.
- **Gears button** (2026-09-26): a gear at the header's top right opens
  the app-level menu. Save Wanted-Apps List…, Import Apps Disc…,
  Import Apps Folder… and Export Test Report… moved there from the
  library panel, and a new **About Floppy** shows the version, each
  guest's emulator and the license.
- **Release builds install themselves** (2026-09-26):
  `scripts/build-release.sh` now finishes by copying `Floppy.app` into
  `~/Applications/`, replacing any older copy.
- **Handler verification** (2026-09-26): a running record of which apps
  really open which file types.
  - After each document session, Floppy asks whether the app opened it
    correctly (Worked / Didn't Work, optional note). Answers stay
    local, in `library/verifications.json`, with no document names.
  - "Open with" shows each app's record for the file type (worked 3×,
    failed 1×) and ranks by it. An app that fails more than it works
    goes last.
  - **Export Test Report…** writes the totals as JSON.
    `scripts/merge-handler-tests.py` merges reports into the new "Tested
    in Floppy" table in `docs/app-handlers.md`, never counting a report
    twice, and suggests *believed* entries to mark verified.
    `verify.rs`.

- **Find the old apps you own** (2026-09-26).
  - The new living document `docs/app-handlers.md` lists old apps that
    open old formats: 13 DOS, 29 classic Mac and 15 Amiga apps. Each
    entry gives its program names, extensions, Mac type/creator codes or
    Amiga IFF types, and a confidence and source. Its code copy is
    `handlers.rs`, and document matching now uses it.
  - **Save Wanted-Apps List…** writes the missing-files list format for
    apps not in the library, with `#gather: folder` and
    `#forks: appledouble` directives for disc makers.
  - **Import Apps Disc…** imports each app found, with its folder.

- **Open an old DOS file in the app that made it** (2026-09-26): the DOS
  slice of "Open This Old File".
  - Drop a document (a .WP5, .WK1, .DBF…) on the DOS tab, use **Import
    Document…**, or run `floppy open <file>`. It's copied to `C:\DOCS`
    under an 8.3 name, and its original name is kept.
  - Floppy offers the library's apps that open it: a table of
    well-known DOS programs (WordPerfect, Word, WordStar, Works, 1-2-3,
    Quattro Pro, dBASE, Paradox, Harvard Graphics, Deluxe Paint, PC
    Paintbrush, AutoCAD, Turbo Pascal), plus each app's **Also opens**.
  - **Open** runs the program with the document's DOS path.
  - When DOSBox quits, a panel lists the new or changed files, with
    **Show in Finder** and **Export…**.
  - A headless end-to-end test runs a real document through DOSBox.
    `documents.rs`.

- **Files discs that don't bring the same bad copy twice** (2026-09-26),
  matching Diskette's Burn A CD contract (`discs.rs`).
  - Floppy recognizes a Burn A CD disc by its ISO Application ID, or by
    the `diskette-burn.json` manifest, which is never mistaken for a
    system file. It remembers each disc's ID, so a re-import says so.
  - Every copy on such a disc that Floppy couldn't use (not a system
    file, refused by its checks, or damaged beside a good copy) goes on
    an ignore list. The next missing-files list carries
    `#ignore: sha256 <hash> <reason>` lines, so the disc leaves that
    content out under any name.
  - The disc also says, line by line, what the user's drives had. A
    slot whose every line found nothing usable is left off later lists.
    The setup panel shows "Not on your drives" with **Ask Again**.
  - **Forget N Ignored Copies** clears the ignore list. State is in
    `library/files-discs.json`. A small SHA-1 (`sha1.rs`) matches a disc
    to the list it answers.

- **Every setup file is found by content** (2026-09-26). The
  missing-files list now has size and SHA-1 lines for Mac startup disks
  and Workbench disks as well as ROMs: 179 known-good copies in all.
  - 20 Mac ROMs from MAME.
  - 4 Mac startup disks from Internet Archive metadata: installed System
    7.0.1 and 7.5.3, and Mac OS 7.6's bootable Utilities floppies.
  - 74 Kickstarts from FS-UAE and TOSEC, including Amiga Forever's
    encrypted ROMs.
  - 81 Workbench boot disks from TOSEC and the FS-UAE launcher.
  - The table (`known_files.rs`, replacing `known_roms.rs`) is generated
    by the new `scripts/update-known-hashes.py`. `docs/legal-setupfiles.md`
    lists the sources and what's left out, and why.

- **Old disks macOS can't open** (2026-09-26): HFS floppies, CDs and
  drives, and Amiga drives.
  - When one is attached and nothing on it mounts, a notice offers
    **Copy and Open**. Floppy reads the disk through macOS's password
    prompt (`authopen`, read-only) into a library image, then opens it
    in Basilisk II or FS-UAE when that guest is set up and free.
  - The first 64 KB decide the guest, and anything else is abandoned.
  - **Ignore** hides a disk until it's detached. `media.rs`.

- **Releases ship the bundled emulators' source** (2026-09-26).
  `scripts/build-release.sh` now ends by running the new
  `scripts/fetch-sources.sh`. It downloads the DOSBox Staging and FS-UAE
  source matching the pinned binaries into
  `src-tauri/target/release/bundle/source/`, checking each against a
  SHA-256 pinned beside the binary's version (`SRC_URL`/`SRC_SHA256` in
  the fetch scripts). The GPL requires releasing those alongside the DMG.

## 0.2.0 (2026-09-26)

- **Drop setup files onto the window** (2026-09-26).
  - While a Mac or Amiga system file is missing, a strip under the
    toolbar says which ones are still needed, with a **Choose Files…**
    button.
  - Dragging onto the window splits the overlay into "import into
    <guest>" and "add setup files". The drop goes to the target under
    the cursor.
  - Setup files can be the files themselves (under any name), folders
    (any depth), zips, or disc images. Each is recognized by its
    contents, and a dropped encrypted Amiga Forever ROM brings its
    `rom.key`.
  - Once nothing is missing, the strip, the setup target and **Import
    Files Disc…** disappear, and drops import apps as before.
    `cd::import_dropped`, `import_setup_files`.

- **The missing-files list finds ROMs by content** (2026-09-26). It now
  starts with a `#columns: name size sha1` header.
  - Name lines are unchanged.
  - Each known-good ROM gets a nameless tab-separated line with its size
    and SHA-1, so a tool can find the ROM stored under any name. There
    are 20 Basilisk II-usable Mac ROMs from MAME and 49 Kickstarts from
    FS-UAE's ROM table (`known_roms.rs`).
  - Readers that only know plain name lists see the header as a comment
    and still match the names.

- **`docs/legal-setupfiles.md`** (2026-09-26): a living list of free,
  legal sources for the files setup asks for. Only Mac startup disks
  (System 7.0.1 and 7.5.3, which Apple released free) and Basilisk II
  itself qualify. There's no free legal source for Mac ROMs, Kickstarts
  or Workbench.

## 0.1.0 (2026-09-26)

- **New app icon: a classic 5¼" floppy disk** (2026-09-26), replacing
  the 3½" one. It's a dark grey jacket with a light rim on the dark tile,
  and a cream label striped in Floppy's green. It has the large hub
  opening with the disk's hub ring, an index hole, a long head window
  with stress-relief notches on either side, and a write-protect notch.
  The header's brand mark (`AppMarkIcon`) is redrawn to match.
  `src-tauri/build.rs` now reruns when `icons/` changes: the dev Dock
  icon is compiled in, and cargo didn't treat the icons as inputs, so
  `tauri dev` kept showing the old one.
  `src-tauri/icons/source-icon.svg` is the source (also
  `public/favicon.svg`). Every size is generated from it with `npx tauri
  icon`, keeping only the desktop icons.

- **Basilisk II build workflow** (2026-09-26), a first step towards
  bundling it. `.github/workflows/basilisk.yml` (run by hand) builds a
  universal `BasiliskII.app` from a pinned kanjitalk755/macemu commit
  with pinned SDL2, GMP and MPFR (each checked by SHA-256). It then
  publishes the app as a GitHub release, next to the exact source and the
  GMP/MPFR tarballs. Floppy doesn't bundle it yet: that still needs
  `scripts/fetch-basilisk.sh` and the resource wiring.

- **Release builds no longer contain the builder's home path**
  (2026-09-26). A test build found 272 `/Users/<name>/.cargo/registry/…`
  paths that dependencies compile into their panic messages.
  `scripts/build-release.sh` wraps `tauri build` with
  `--remap-path-prefix` (`$HOME` becomes `~`, the project folder becomes
  `.`), then fails if `$HOME` is still anywhere in the built app.

- **Public repo tidy-up** (2026-09-26). Floppy no longer points at a
  closed-source sister app's internals. The files-disc handoff is
  described as Floppy's own plain-file contract that any tool can
  produce (Diskette's Burn A CD is one). Buttons renamed "Save
  Missing-Files List…", "Import Files Disc…" and the new "Import Folder
  of Files…", and the `import_diskette_cd` command became
  `import_files_disc`. Design-system comments no longer cite other apps'
  source files. Private context moved to a gitignored `CLAUDE.local.md`.

- **Find missing system files** (2026-09-26): the missing-files list and
  files disc (`cd.rs`).
  - "Save Missing-Files List…" in the Mac and Amiga setup saves a `.txt` of the
    system files still missing (Mac ROM, Mac startup disk, Kickstart,
    Workbench), under the names those files commonly go by: emulator and
    Amiga Forever conventions, checksum-named Mac ROM dumps, TOSEC names.
  - "Import Files Disc…" mounts a disc image of gathered files (read-only,
    `hdiutil`) and recognizes files by content, not name: Basilisk II ROMs
    by their 32-bit clean version word, Kickstarts by header, bootable Mac
    disk images by a blessed System Folder (raw, Disk Copy 4.2,
    partitioned, HFS+), and Workbench floppies by volume name. Copies with
    a passing checksum win over damaged ones, and a Workbench matching the
    Kickstart wins. Only empty slots are filled, and a copy the library
    refuses is skipped for the next one.
  - Fixes for real-world disc layouts: the disc is searched at any depth
    (duplicates can keep deep volume-relative paths, e.g. 8 folders
    down), and an encrypted Amiga Forever ROM is paired with any
    `rom.key` on the disc that actually decrypts it, since a tool may
    place the two in different folders.
    `set_system_file` now takes the key path explicitly, and also reads
    the model from an encrypted ROM.
  - Fix: Mac ROMs are now checked the way Basilisk II checks them (32-bit
    clean, version `0x067C`, 512 KB or 1 MB). Before, a 256 KB Mac II ROM
    was accepted and then refused by Basilisk II.

- **Classic Mac and Amiga guests** (2026-09-26). Emulator choices, with
  their licensing and reliability checks, are in `docs/emulators.md`.
  - Amiga: bundled FS-UAE v3.2.35 (`scripts/fetch-fs-uae.sh`, pinned per
    arch by SHA-256). Imports `.adf`/`.adz`/`.dms`/`.hdf` disk images,
    AmigaDOS programs, folders and zips. A disk app boots from its disk
    (other disks in FS-UAE's swap list), and a folder app boots the
    user's Workbench. The library is the `Floppy:` drive.
  - Classic Mac: Basilisk II, found as an install in `/Applications` or
    `~/Applications` (it has no pinnable binary release). Boots the
    user's startup disk as a Quadra 900, with the library as the Unix
    volume and an app's disk image mounted. Imports keep resource forks
    and Finder info: fork-preserving folder copies, AppleDouble
    (`__MACOSX`, `._name`) merged back from zips, and MacBinary decoding.
    StuffIt, BinHex and Compact Pro archives are tagged for expanding in
    the Mac.
  - Guest setup: the user adds their own Mac ROM and startup disk, or
    Kickstart and Workbench. Floppy checks them (ROM size, Kickstart
    header and version, Amiga Forever `rom.key`) and copies them into
    `library/system/<os>/`. The Amiga model follows the Kickstart version
    and can be changed.
  - Only one Mac and one Amiga run at a time, since each guest's apps
    share one writable startup disk. DOS still allows several.
  - `floppy import --os mac-classic|amiga <path>`.
  - UI: guest OS tiles, per-guest import and setup, and "Start Mac OS" /
    "Start Workbench" next to Launch.
  - `dosbox.rs` became `emulator.rs`, which covers all three emulators.

- **Project started: DOS mode** (2026-09-26). Tauri + React/TS app,
  `com.ansiapps.floppy`, GPL-2.0-or-later.
  - DOS library: import a folder, a zip, or a single `.exe`/`.com`/`.bat`
    (dialog or drag and drop). Imports are staged first and renamed to 8.3,
    and macOS clutter (`.DS_Store`, `__MACOSX`, `._*`) is dropped.
    Zip-slip entries are skipped, and a zip that unpacks to more than 4 GB
    is refused.
  - Launch picks the most likely program (installers rank low), with a
    picker to override it, plus DOS Prompt, Show in Finder, rename, and
    Remove (with confirmation).
  - Bundled DOSBox Staging v0.83.0 (`scripts/fetch-dosbox.sh`, pinned by
    SHA-256), started with `--noprimaryconf` and a per-launch config. The
    whole DOS library is mounted as C:.
  - `floppy import [--os dos] <path>` command-line handoff for other apps
    (such as Diskette).
  - Unit tests for 8.3 naming, program ranking, config generation, import
    (zip, folder, single program, zip-slip, name collisions), and CLI
    parsing. There's also an ignored end-to-end test that runs a real
    `.COM` in the bundled DOSBox headlessly.
