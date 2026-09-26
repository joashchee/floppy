# Floppy notes: multi-guest emulation and the files disc

A condensed record of the work done on 2026-09-26: adding the classic Mac
and Amiga guests, and the missing-files list and files disc. Details live
in `docs/emulators.md`, `docs/platform-parity.md`, CLAUDE.md and
CHANGELOG.md. This file is the short version with the reasoning.

## Which emulators, and why

Floppy's scope is original-era desktop apps that open obsolete file
formats: archivers, word processors, paint programs, trackers. These are
DOS, classic Mac and Amiga software. Windows 3.x apps (Paintbrush,
WordPerfect for Windows) would run in DOSBox with a user-supplied
Windows. No target app is C64 software, so VICE isn't needed.

| Guest | Emulator | License | How Floppy gets it |
|---|---|---|---|
| DOS | DOSBox Staging v0.83.0 | GPL-2.0-or-later | Bundled (`scripts/fetch-dosbox.sh`) |
| Amiga | FS-UAE v3.2.35 | GPL-2.0 | Bundled (`scripts/fetch-fs-uae.sh`, SHA-256 pinned per arch) |
| Classic Mac | Basilisk II (kanjitalk755 fork) | GPL-2.0-or-later | Not bundled: found in `/Applications` or `~/Applications`, or `FLOPPY_BASILISK` |

All three run as separate executables with a per-launch config file
(aggregation, never linked), which satisfies rule 1.

**Reliability findings:**

- **FS-UAE.** Maintained, and official releases carry SHA-256 digests.
  - It's driven entirely by a config file, and `base_dir` keeps its files
    out of `~/Documents/FS-UAE`.
  - Its release tarball's bundle signature doesn't verify, although the
    binary runs. Release signing must re-sign it.
  - It needs OpenGL and a window server, and segfaults when run headless,
    so there's no end-to-end test for it.
  - Its download carries AROS, a free replacement Kickstart. Floppy never
    uses it. **Open question:** strip it at fetch time to respect rule 3's
    spirit?
- **Basilisk II.** The fork is active (last commit 2026-08-30).
  - It has no binary releases. macOS builds live on the Emaculation forum
    behind a Cloudflare check, and building from source needs full Xcode,
    GMP/MPFR and SDL2. Hence detection instead of bundling.
  - Checked in its source:
    - `--config <file>` loads a prefs file.
    - On a macOS host its shared folder (`extfs`) reads resource forks
      natively (`file/..namedfork/rsrc`) and Finder info from
      `com.apple.FinderInfo`.
    - The shared volume is named **Unix**.
    - `CheckROM` accepts only 32-bit clean ROMs (version `0x067C`,
      512 KB or 1 MB).
- **Alternatives set aside.**
  - Amiberry (GPL-3.0): the fallback if FS-UAE stalls.
  - Mini vMac: no shared folder.
  - SheepShaver: PPC, not needed yet, and the same distribution problem.
  - QEMU q800: no classic-Mac shared folder.
  - The CAPS/IPF plugin: separately licensed and unchecked, so not
    bundled.

## What was built

**Library.**
- `GuestOs` is now `dos | mac-classic | amiga`, with library folders
  `library/dos`, `library/mac` and `library/amiga`.
- User-supplied ROMs and boot disks are copied into `library/system/<os>/`
  after validation.
- IDs are unique slugs, and a manifest entry naming a folder that has
  vanished is replaced on reimport.

**Classic Mac (`mac.rs`).**
- Imports keep resource forks and Finder info:
  - Folders are copied with `fs::copy`, which copies forks on macOS.
  - AppleDouble data from zips (`__MACOSX/…/._x` and sibling `._x`) is
    merged back into the file.
  - MacBinary I/II/III is decoded.
  - StuffIt, BinHex and Compact Pro archives get type/creator codes for
    expanding in the guest.
- Launch writes Basilisk II prefs: Quadra 900, 68040 + FPU, 64 MB, the
  library as the Unix volume, and the app's disk image mounted.

**Amiga (`amiga.rs`).**
- Programs are ADF/ADZ/DMS/HDF images or AmigaDOS executables (hunk
  magic `000003F3`).
- The Kickstart version picks the default model: 1.x A500, 2.x A600,
  3.x A1200.
- A disk app boots from its disk, with its other floppies and Workbench
  in the swap list. A folder app boots Workbench. The library mounts as
  a non-booting `Floppy:` drive.

**Emulators and commands.**
- `emulator.rs` replaced `dosbox.rs`. It locates each emulator (env
  override, then bundled, then installed) and spawns it.
- Only one Mac and one Amiga run at a time, since each guest's apps share
  a writable startup disk. DOS still allows several instances.
- `floppy import --os dos|mac-classic|amiga <path>`.

**UI.**
- Guest OS tiles and per-guest import.
- A system setup box: ROM, startup disk or Workbench, Amiga model.
- "Start Mac OS" and "Start Workbench" buttons, plus launch hints.

## Missing-files list and files disc (option A: name list)

A plain-file contract, specified in `cd.rs`'s module doc. Floppy writes a
`.txt` of the system files it still needs. Any tool (or the user by hand)
gathers copies into a disc image or folder, and Floppy imports that.
Diskette's Burn A CD is one tool that builds the disc from a catalog of
the user's drives. The two files are the whole interface (rule 2).

- **Save Missing-Files List…** (`cd.rs`) saves a list of only the missing
  slots (Mac ROM, Mac startup disk, Kickstart, Workbench). Each slot lists
  the names such files commonly go by: emulator and Amiga Forever
  conventions, checksum-named Mac ROM dumps, TOSEC names, and `rom.key`.
- **Import Files Disc…** mounts a disc image read-only with `hdiutil`
  (**Import Folder of Files…** reads a folder) and recognizes files by
  content:
  - Basilisk II ROMs by the version word.
  - Kickstarts by their header plus the end-around-carry checksum
    (confirmed in FS-UAE's `rommgr.cpp`).
  - Mac startup disks by a blessed System Folder: raw, Disk Copy 4.2,
    Apple-partitioned, and HFS+ (including HFS-wrapped).
  - Workbench floppies by volume name and bootblock checksum.
- **No assumptions about layout.** A tool may place each name
  independently and keep duplicates under deep paths, so the disc is
  walked with no depth limit. A ROM and its `rom.key` can land in
  different folders, so an encrypted Amiga Forever ROM is tried against
  every `rom.key` on the disc. The one that decrypts it wins,
  checked the way FS-UAE's `decode_cloanto_rom_do` does: XOR with the
  key, then look for a Kickstart header.
- **Picking between copies.** Passing checksums beat damaged copies, and
  a Workbench matching the Kickstart release wins. Only empty slots are
  filled. A copy the library rejects falls through to the next.
- **Fix on the way.** The Mac ROM check now matches Basilisk II's; it had
  accepted 256 KB Mac II ROMs that Basilisk II refuses.
- **Why option A first.** Floppy can't know users' file names, and ROM
  dumps go by many. A plain name list works with existing tools.
- **Options B and C, sizes and hashes (built 2026-09-26).** The list now
  starts with a `#columns: name size sha1` header. Name lines stay bare
  names (no tabs). Each known-good ROM gets a nameless line: a tab, its
  size, a tab, its SHA-1. A tool that hashes candidates by size finds
  the dump whatever it's called.
  - The header starts with `#`, so a name-only reader treats it as a
    comment. Name lines still work there, and the nameless lines read as
    odd names that match nothing, so one list suits both kinds of
    reader.
  - The hashes come from published lists: Mac ROMs from MAME, Mac
    startup disks from Internet Archive metadata, Kickstarts from FS-UAE
    and TOSEC, Workbench disks from TOSEC and the FS-UAE launcher
    (`known_files.rs`, generated by `scripts/update-known-hashes.py`,
    sources in `docs/legal-setupfiles.md`).
  - Every slot gets hash lines. A used startup disk or Workbench no
    longer matches, but the user's original download or disk image
    does.
  - Floppy still recognizes every file by content, so an unlisted dump
    still imports.

## Testing status

- **Automated.** 40 unit tests plus the DOSBox end-to-end test pass, and
  `tsc` is clean. Fork, xattr and HFS tests run only on a macOS host.
- **Not verified.**
  - No Mac or Amiga guest has actually booted. FS-UAE can't run headless
    here, and Basilisk II needs a real ROM, which Floppy must never
    download.
  - ISO mounting fails in the sandbox, so `import_cd` is tested on a
    folder laid out like the ISO.
  - The Mac ROM checksum was written from memory and never checked
    against a real ROM. It's used only for ranking copies, never to
    reject one.
  - The new UI hasn't been viewed (the sandbox has no display).
- **Licenses.** The only new crate is `libc` (MIT OR Apache-2.0), which
  was already in the lockfile. No npm changes.

## Open items

- Bundle Basilisk II via a reproducible, pinned source build.
- Auto-open apps inside the guest: a Mac Startup Items alias, or Amiga
  `user-startup`.
- SheepShaver for PPC-only Mac apps.
- Decide on AROS in FS-UAE's download (above).
- Code signing: re-sign the nested `DOSBox Staging.app` and `FS-UAE.app`.
- Universal arm64 + x86-64 build: the FS-UAE fetch is per-arch.
- Windows/Linux: Mac fork handling, FS-UAE paths, Basilisk II detection,
  ISO mounting (`docs/platform-parity.md`).
