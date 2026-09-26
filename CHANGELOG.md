# Changelog

All notable changes to Floppy are recorded here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/), newest first.

Update this file whenever a change lands. A short bullet is enough, dated
against the day the change was made.

## Unreleased (0.1.0)

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
