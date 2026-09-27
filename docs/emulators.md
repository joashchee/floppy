# Emulators

Which emulator runs each guest OS, and the reliability and licensing checks
behind each choice. Re-check a row whenever its version is bumped.

Floppy's scope is original-era desktop apps that open obsolete file
formats (archivers, word processors, paint programs, trackers). These are
DOS, classic Mac and Amiga software. Windows 3.x apps would run in DOSBox
with a user-supplied Windows, so they need no extra emulator. No target
app is C64 software, so VICE isn't needed yet.

| Guest | Emulator | Version | License | How Floppy gets it |
|---|---|---|---|---|
| DOS | DOSBox Staging | v0.83.0 | GPL-2.0-or-later | Bundled: `scripts/fetch-dosbox.sh` |
| Amiga | FS-UAE | v3.2.35 | GPL-2.0 (UAE/WinUAE lineage) | Bundled: `scripts/fetch-fs-uae.sh` |
| Classic Mac (68k) | Basilisk II (kanjitalk755 fork) | macemu `892eeb7` (2026-08-30) | GPL-2.0-or-later | Bundled (macOS): `scripts/fetch-basilisk.sh`, Floppy's own build. Also found in `/Applications` or `~/Applications`, or located from the gear menu |

All three run as separate executables, started with a config file Floppy
writes for each launch (aggregation, never linked in), so a GPLv2-only
emulator is fine to ship next to the GPL-2.0-or-later Floppy binary
(CLAUDE.md rule 1). None of them includes copyrighted guest ROMs or OSes.

## DOSBox Staging (DOS)

See CLAUDE.md "How DOS mode works". Pinned release DMG with SHA-256;
the end-to-end test runs a real program in it headlessly.

## FS-UAE (Amiga), checked 2026-09-26

- **Reliability:** actively maintained (v3.2.35 released 2025-09-07,
  repository updated 2025-09). Official GitHub releases ship macOS ARM64
  and x86-64 builds as `.tar.xz` and `.dmg`, each with a SHA-256 digest,
  so the fetch script can pin them. It's configured entirely by a config
  file (`fs-uae <file>`), with `base_dir` keeping its logs, floppy
  overlays and saves inside Floppy's library instead of
  `~/Documents/FS-UAE`. Directory hard drives (`hard_drive_N = <folder>`)
  share the Amiga library as `Floppy:`.
- **Caveats found:**
  - The tarball's bundle signature doesn't verify (`codesign -v`:
    "code or signature have been modified"), although the binary runs.
    Release signing has to re-sign the nested `FS-UAE.app` anyway.
  - FS-UAE needs OpenGL and a window server. With `headless = 1` and
    SDL's dummy video driver it segfaults while creating its window, so
    unlike DOSBox there's no headless end-to-end test. Only its config
    generation is unit-tested.
  - FS-UAE still reads `~/Library/Preferences/fs-uae/*-dir` files if the
    user has them, before `base_dir`. They only move FS-UAE's own folders.
  - Its archive carries the AROS replacement Kickstart (`fs-uae.dat`,
    AROS Public License), used when no ROM is configured. That's free
    software inside FS-UAE's own distribution, not a copyrighted ROM, but
    Floppy doesn't rely on it: it always passes the user's own Kickstart.
- **Alternative considered:** Amiberry (GPL-3.0, also fine as a separate
  executable, universal macOS DMG, very active). FS-UAE was picked for
  its long record as a scriptable, config-file-driven emulator with
  folder-backed hard drives. Amiberry is the fallback if FS-UAE stalls.
- **Not bundled:** the CAPS/IPF plugin (`capsimg`), which is licensed
  separately by the Software Preservation Society and hasn't been
  checked. IPF images therefore aren't offered.

## Basilisk II (classic Mac), checked 2026-09-26

- **Why Basilisk II:** it runs System 7 to Mac OS 8.1 on an emulated
  68040, which covers the classic-Mac Handlers (MacWrite, MacPaint,
  ClarisWorks/AppleWorks, StuffIt Expander, Compact Pro, Disk Copy). Its
  `extfs` shares a host folder as a Mac volume. On a macOS host
  (`extfs_macosx.cpp`) it reads resource forks natively
  (`file/..namedfork/rsrc`) and Finder info from `com.apple.FinderInfo`,
  so Floppy's fork-preserving import is all a transfer needs.
  `--config <file>` loads a prefs file instead of `~/.basilisk_ii_prefs`
  (`main_unix.cpp`, `prefs_unix.cpp`). The macOS Xcode build uses those
  Unix sources.
- **ROMs it accepts:** `CheckROM` (rom_patches.cpp) needs a 32-bit
  clean ROM, version word `0x067C` at offset 8 (512 KB or 1 MB: IIci,
  IIsi, IIfx, LC, Quadra, Centris…). Mac Plus, SE/Classic and original
  Mac II ROMs are refused, so Floppy refuses them too.
- **License:** GPL-2.0-or-later (source headers), `COPYING` is GPLv2.
- **Reliability:** the maintained fork (github.com/kanjitalk755/macemu) is
  active (last commit 2026-08-30) and supports macOS x86_64 (JIT) and
  arm64 (no JIT). **But it publishes no binary releases**: its only
  GitHub releases are 2017 SheepShaver pre-releases. Current macOS builds
  are posted on the Emaculation forum, which sits behind a Cloudflare
  browser check, so a script can't fetch and pin one.
- **So Floppy builds its own** (2026-09-27): `.github/workflows/basilisk.yml`
  builds a universal `BasiliskII.app` from a pinned commit on a GitHub
  macOS runner (Xcode, SDL2 2.32.10 framework, GMP 6.3.0 and MPFR 4.2.2
  static) and publishes it as a `basilisk-ii-<date>-<commit>` release of
  this repo, with the exact source and the GMP/MPFR tarballs.
  `scripts/fetch-basilisk.sh` pins it by SHA-256 and `fetch-sources.sh`
  ships all three sources with each Floppy release. Checked for
  `892eeb7`: the source tarball's contents are identical to GitHub's
  archive of that commit, the GMP and MPFR hashes are GNU's, the binary
  links only system frameworks and its own SDL2, and it needs macOS 11
  (the arm64 floor). It's signed ad hoc. A copy the user installs or
  locates still works, and a located one wins over the bundled one.
- **Linux** (2026-09-27): the same workflow's Linux job builds the Unix
  port of the same commit on Ubuntu 22.04. It uses SDL2 2.32.10 from its
  release source (SDL2's own SHA-256 pin), linked statically, with no
  GTK and no GMP/MPFR (x86-64 uses the JIT and the IEEE FPU core). SDL
  loads X11/Wayland and ALSA/PulseAudio/PipeWire at runtime, so the binary
  links only libc and libstdc++, and the job checks that. Its extfs keeps
  forks in `.rsrc/` and `.finf/` (`extfs_unix.cpp`), which is what Floppy
  writes on Linux. Checked for `892eeb7`: it reads Floppy's generated
  prefs and refuses a blank ROM cleanly. Booting needs a user's ROM, so
  it isn't tested beyond that.
- **Alternatives considered:**
  - Mini vMac (GPL-2.0): Mac Plus/II only, no shared host folder (disk
    images only), so transfers would need HFS image writing.
  - SheepShaver (same repo and prefs format): PowerPC, Mac OS 8.5 to
    9.0.4. No current Handler needs PPC. It's the natural next addition,
    with the same distribution problem.
  - QEMU `q800`: runs Mac OS 7.1 to 8.1 with a Quadra ROM, but has no
    classic-Mac shared folder.

## What Floppy does per launch

| Guest | Launch | Boot only |
|---|---|---|
| DOS | `[autoexec]` runs the program, then exits | DOS prompt in the app's folder |
| Classic Mac | Boots the user's startup disk with the library as the **Unix** volume, plus the app's disk image when that's what it opens. Apps in folders are opened by the user from the Unix volume. | Same, without the app's disk image |
| Amiga | A floppy or hard-disk image app boots itself (other floppies of the app go in FS-UAE's swap list, F12). A folder app boots the user's Workbench, and the user opens it from `Floppy:`. | Boots Workbench |

A Mac or Amiga app shares one writable startup disk with every other app
of that guest, so only one Basilisk II and one FS-UAE run at a time.
Several DOSBox windows can run at once.
