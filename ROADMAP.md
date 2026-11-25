# Floppy roadmap

Progress and next phases, in the order to do them. Each open item has an
effort rating: **S** an hour or less, **M** a few hours to a day, **L**
several days, **XL** a week or more (hands-on work once nothing blocks it;
blockers are named beside the item). Detail lives in `CLAUDE.md`'s "Not
built yet" and in the docs linked per item. Started 2026-11-25 from those
lists.

## Windows 9x guest (new)

- [ ] [M] Spike rust-dos (https://github.com/dividebysandwich/rust-dos,
  GPL-2.0, built-in Windows 3.1/95/98 via `--win32`) as the emulator for
  Win9x-only handlers. Blocked on a real Win9x-only test program (an
  `.exe` or installer folder the maintainer owns) run with
  `rust-dos -d <folder> --win32` on a Mac with a display. Spike of
  2026-11-25 (v1.9.3, macOS arm64, headless) found DOS batch runs, and
  `--no-config`, `-d`, `--log` and `--exit-with-parent` suit Floppy, but:
  the binary links Homebrew's SDL3 (bundle SDL3 or build our own), macOS
  zips are thin arm64/x86_64 and ad hoc signed (needs `lipo` merge plus
  hardened-runtime signing, as FS-UAE), and it writes
  `~/Library/Application Support/rust-dos/run-lock/` even with
  `--no-config` (redirect `HOME` to stay within rule 6). Rule 4: even with
  `--no-config` it asks rust-dos.com for a newer version at startup; a
  config with `version_check=never` stops it (set up in the gitignored
  `win9x/`). Targeted source audit of v1.9.3 (2026-11-25): no malicious code or
  telemetry found; network calls are fixed-URL and opt-in except the
  update check; `cargo audit` clean on a 2 Oct advisory DB; emulation
  core, `unsafe` blocks and binary-vs-source match NOT audited.
  Built-in Windows results (2026-11-25, logs in the gitignored `win9x/`):
  Windows Media Player 6.4 opens its windows, then quits by itself when
  `CoCreateInstance` for its ActiveMovie control returns 0x80040154 (class
  not registered): COM-based apps are out of reach for now. Winamp 2.95's
  installer runs and installs; `winamp.exe` starts, then ends with exit
  code FF because a plugin calls `KERNEL32.GetPrivateProfileStructA`,
  which the built-in Windows lacks (also missing: `wmaudsdk.dll`,
  `RichEd20.dll`). With only `IN_MP3`, `IN_WAVE` and `OUT_WAVE` left in
  `PLUGINS`, `winamp.exe` runs (confirmed by the maintainer, 2026-11-25):
  plain Win32 apps are viable, and `DEMO.MP3` plays through the Mac's
  speakers (confirmed by the maintainer, 2026-11-25). Next: put the
  parked plugins back one at a time to
  name the one that needs `GetPrivateProfileStructA`, then try FruityLoops
  or Buzz and a Win9x app that opens an obsolete format (Word Pro,
  Publisher); an upstream issue is drafted, not posted (the maintainer
  decides). Fallbacks if it
  fails: 86Box or DOSBox-X with a user-supplied Win9x image (a new
  `cd::SLOTS` entry). Detail: `docs/emulators.md`.
- [ ] [L] If the spike passes: add the `win9x` guest (fetch script with
  pinned hashes, launch config, `docs/emulators.md` entry, platform-parity
  row, handler rows marked Win9x). Blocked on the spike.

## Releases and signing

- [ ] [M] First signed and notarized run of `scripts/sign-macos.sh` with a
  real Developer ID: check notarization accepts it, and DOSBox, FS-UAE and
  Basilisk II launch under the hardened runtime on Apple Silicon and Intel
  (Basilisk II's entitlements are a guess). Needs the maintainer's Mac.
- [ ] [M] Windows installer signing (Authenticode) to stop SmartScreen's
  warning. Blocked on a certificate.

## Core features

- [ ] [M] Single-instance handling: a second `floppy import …` while
  Floppy runs opens a second window; forward it instead.
- [ ] [M] Per-app DOSBox settings (cycles, machine type, sound).
- [ ] [L] Auto-open a Mac app (alias in the startup disk's Startup Items)
  or an Amiga folder app (`user-startup`), so the guest opens it itself.
- [ ] [XL] SheepShaver for PowerPC-only Mac apps (Mac OS 8.5 to 9.0.4).
- [ ] [S] More known-good hashes as users report copies not listed (the
  IIci's single-file ROM dump, for one): edit
  `scripts/update-known-hashes.py`, never `known_files.rs`. Ongoing.

## Platform parity (`docs/platform-parity.md`)

- [ ] [L] Linux: old-media watcher (udev and a polkit prompt).
- [ ] [M] Linux: files opened from the desktop (`.desktop` MIME types plus
  single-instance forwarding; after single-instance handling).
- [ ] [M] Linux: the Diskette handoff.
- [ ] [S] Linux: check udisks disc mounting on a real desktop.
- [ ] [XL] Windows beyond the beta: Basilisk II for Windows (a job in
  `basilisk.yml`), old-media pickup, the Diskette handoff,
  single-instance forwarding.
- [ ] [S] Check the Windows beta on a real Windows desktop (CI only builds
  and unit-tests it).
