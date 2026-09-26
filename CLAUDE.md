# Floppy

Emulation frontend and launcher for original-era desktop apps: DOS
(bundled DOSBox Staging), Amiga (bundled FS-UAE) and classic Mac OS
(bundled Basilisk II, Floppy's own build). `docs/emulators.md` records
why each emulator was picked and its licensing and reliability checks,
and `docs/floppy-notes.md` is a condensed history of the work so far.
`docs/legal-setupfiles.md` is a living list of free, legal sources for
the system files setup asks for. `docs/app-handlers.md` is a living list
of old apps that open old formats (confidence and sources per entry).
Its code copy is `HANDLERS` in `handlers.rs`, and a test keeps the two in
step. Re-check it and update its date when
touching setup.

This repo is public. Anything about closed-source sister apps (where
they live, their internals, their side of a handoff) belongs in the
gitignored `CLAUDE.local.md`, never in committed files.

**`CHANGELOG.md`**: add a bullet whenever a change lands.

## Non-negotiable rules

1. **Open source, GPL-2.0-or-later.** Anything linked into the Floppy
   binary must be GPL-compatible. MIT, BSD, ISC, zlib, MPL-2.0 and
   Unicode are fine. The few Apache-2.0-only crates (`tao`, Tauri's
   windowing layer, plus `sync_wrapper` and `target-lexicon`) are fine too, because "or later" lets the binary
   be distributed under GPLv3, which Apache-2.0 is compatible with. Avoid
   **AGPL** and anything proprietary. Emulators (DOSBox Staging, FS-UAE,
   Basilisk II) run as separate executables (aggregation), so a
   GPLv2-only core is fine to bundle but must never be linked in. Check licenses whenever a
   lockfile changes (`cargo metadata --offline`, `package-lock.json`).
2. **Floppy stays a separate program from anything that feeds it.**
   Other programs talk to it only across a process boundary, through
   documented plain interfaces: `floppy import [--os
   dos|mac-classic|amiga] <path>` and `floppy open <file>` (`cli.rs`), the missing-files list and
   files disc (`cd.rs`), the request list and `#reply-to:` disc
   (`request.rs`), and later perhaps a URL scheme. Never read
   another app's private data (its database or internal files), never
   let Floppy code be linked into a closed-source program, and never copy
   closed-source code into this repo, which would publish it under the
   GPL. UI chrome (tokens, Dialog, ProgressBar, icon glyphs) comes from a
   shared design system on purpose.
3. **Never bundle or download guest software.** That means no ROMs,
   operating systems (Mac OS, Kickstart, Windows 3.x), or applications,
   abandonware included. Users supply their own. DOS needs none: DOSBox
   provides the DOS. Mac ROMs, startup disks, Kickstarts and Workbench
   are copied from the user's own files into `library/system/<os>/`.
4. **No network access, no telemetry, no accounts.** Floppy works fully
   offline. (`scripts/fetch-*.sh` are build-time steps, not app code.)
5. **Bundle identifier is fixed: `com.ansiapps.floppy`.** macOS keys the
   app-data folder (the whole library) on it.
6. **All user data lives under Tauri's app-data dir** (`library/`), never
   a hardcoded path.
7. **macOS first, then Windows/Linux.** Distribution: itch.io (free or
   pay-what-you-want) plus GitHub releases. Never the Mac App Store,
   whose terms are incompatible with the GPL.

## How the library works

- `library.rs`: `library/library.json` manifest (apps, plus each guest's
  system files) and one folder per guest: `library/dos/<8.3 dir>/`,
  `library/mac/<name>/`, `library/amiga/<name>/`. Each guest's folder is
  shared into its emulator whole. Imports stage into `library/.staging/`,
  get prepared for the guest, find programs, then move into place. A
  failed import leaves nothing behind. User-supplied ROMs and boot disks
  live in `library/system/<os>/`, outside the shared folders.
- `emulator.rs`: finds each emulator (`FLOPPY_DOSBOX` / `FLOPPY_FSUAE` /
  `FLOPPY_BASILISK` env, then a copy the user located, kept in
  `library/emulators.json`, then bundled, then an install) and spawns it with
  only Floppy's per-launch config (`library/run/<emulator>/<id>.*`).
- `cd.rs`: the missing-files list and files disc, a plain-file contract
  any tool can take part in (the module doc is the spec). Floppy writes
  a `.txt` of missing system files under their common names, and some
  tool gathers copies into a disc image or folder, laid out any way, at
  any depth. Floppy mounts it read-only and fills empty slots by
  recognizing files by content, never by name or position. The same
  recognition handles setup files dropped on the window
  (`import_dropped`): files, folders, zips and disc images in any mix.
  The setup strip and the overlay's "add setup files" target show only
  while a Mac or Amiga system file is missing.
- `discs.rs`: files discs made from the list. A Burn A CD disc carries
  the ISO Application ID `DISKETTE BURN A CD` and a `diskette-burn.json`
  manifest (disc `id`, the answered list's SHA-1, and per-line counts).
  From such a disc Floppy:
  - remembers the disc ID;
  - puts every copy it couldn't use (unrecognized, refused, or damaged)
    on the ignore list, which later lists carry as
    `#ignore: sha256 <hash> <reason>`;
  - leaves off later lists any slot whose every line found nothing
    usable, until **Ask Again**.

  State is in `library/files-discs.json`. `sha1.rs` exists only to match
  a disc to the list it answers.

## Old media (`media.rs`)

- macOS no longer mounts HFS (since 10.15), or Amiga disks. A watcher
  thread polls `/dev` for new whole disks and, after 5 s, asks `diskutil`
  whether anything on one mounted. An external disk with nothing mounted
  and no modern partitions gets a "Copy and Open" notice.
- Raw disks belong to root, so the copy is read through
  `/usr/libexec/authopen` (macOS's own password prompt, read-only). The
  first 64 KB decide the guest (Apple partition map or HFS/HFS+/MFS: Mac;
  Rigid Disk Block or AmigaDOS: Amiga), and anything else is abandoned.
  The image is imported like a dropped disk image and launched when that
  guest is ready. The original is never written to.

## Documents (`documents.rs`)

- An old file is opened in the app that made it: it's imported as a
  document, matched to apps already in the library, and launched with
  the app. DOS only so far; Amiga and classic Mac are next (see the
  plan page linked from `CLAUDE.local.md`).
- DOS documents live in `C:\DOCS` (`library/dos/DOCS/`, reserved: no
  app folder takes that name) under 8.3 names. The original name is
  kept in `library.json` (`documents`) and used on export.
- Matching: `DOS_APPS`, a public table of well-known programs and the
  extensions they open, plus extensions the user adds per app ("Also
  opens"). The app a document last opened with is offered first.
- `open_document` runs `PROGRAM C:\DOCS\FILE` in `[autoexec]`. Every
  DOS launch snapshots drive C: and, when DOSBox quits, emits
  `session-ended` with the files that are new or changed (Show in Finder
  and Export, which uses the original name). Files saved into `C:\DOCS`
  become documents.
- `floppy open [--os dos] <file>` hands Floppy a document (`cli.rs`),
  like `floppy import` does an app. A dropped file that isn't a folder,
  zip or program becomes a document on the DOS tab.

## Handler apps (`handlers.rs`)

- `HANDLERS`: per guest, the app, its program file names, DOS
  extensions, Mac creator and type codes, and Amiga IFF types. It drives
  document matching (`documents.rs`), the wanted-apps list, and Import
  Apps Disc.
- **Save Wanted-Apps List…** writes the missing-files list format for
  every handler not in the library, plus `#gather: folder` (bring each
  matched file's folder) and `#forks: appledouble` (keep Mac forks as
  `._` files). **Import Apps Disc…** imports each handler found on the
  disc with its folder, once, nearest the root first, skipping ones
  already in the library.

## Asking Diskette (`request.rs`)

- While Diskette is running (bundle ID via `lsappinfo`, polled every
  5 s) and something is missing, a strip and a gear-menu item offer
  **Ask Diskette**. Floppy writes one request list
  (`library/run/requests/Floppy wants.txt`): the missing-files list,
  the wanted-apps lines under their `#gather:`/`#forks:` directives
  (which apply to the lines after them), and `#reply-to:
  com.ansiapps.floppy`. It's recorded like a saved list, then opened
  with Diskette (`open -b`).
- Diskette offers to Burn A CD and opens the finished ISO with Floppy.
  Files opened with Floppy arrive as `RunEvent::Opened` (`lib.rs`),
  queued in `request::Opened` until the window takes them. A Burn A CD
  disc, opened or dropped anywhere, goes to `import_disc`: setup files
  and apps from one mount.
- On a disc answering a known list, only files the manifest says
  matched a missing-files line can go on the ignore list
  (`discs::setup_files`), so apps never do.
- `tauri dev` has no bundle ID: Diskette's reply opens the installed
  `~/Applications/Floppy.app`, not the dev build.

## Handler verification (`verify.rs`)

- After each document session Floppy asks "Did <app> open <document>
  correctly?" (Worked / Didn't Work, optional note). Answers go in
  `library/verifications.json`, keyed by guest, app (or handler),
  program and file type, never the document's name.
- Totals rank "Open with": apps that worked with the type rise, and one
  that failed more than it worked goes last. The menu shows each app's
  record.
- **Export Test Report…** writes a `floppy-handler-tests` JSON report,
  only when asked (rule 4). `scripts/merge-handler-tests.py` merges
  reports into `docs/app-handlers.md`'s "Tested in Floppy" table, once
  per report ID, and suggests *believed* entries to promote.

## How DOS mode works

- `dos.rs`: 8.3 validity and naming, program ranking, and generating the
  per-launch DOSBox config (`[autoexec]`: mount the whole `library/dos` as
  C:, `cd` to the app, run it, `exit`).
- DOSBox Staging is spawned with `--noprimaryconf --nolocalconf --conf
  <file>`, so the user's own DOSBox settings never apply.
- DOSBox Staging comes from `scripts/fetch-dosbox.sh` (pinned version and
  SHA-256) into the gitignored `src-tauri/resources/dosbox/`, which
  tauri.conf.json bundles as a resource. Bump the version and hash there
  together, along with `SRC_URL`/`SRC_SHA256`, the matching source that
  every release ships (`scripts/fetch-sources.sh`).

## How classic Mac mode works

- `mac.rs`: a classic app's code is in its resource fork and it's only an
  app if its Finder info says `APPL`. Basilisk II's extfs on a macOS host
  reads both natively, so imports keep them: folder copies use `fs::copy`
  (which copies forks on macOS), zips made on a Mac get their AppleDouble
  files (`__MACOSX/…/._name`, `._name`) merged back, and MacBinary `.bin`
  files are decoded. StuffIt/BinHex/Compact Pro archives are copied and
  tagged with type/creator for expanding in the Mac. Disk images are
  mounted as extra disks at launch.
- Launch writes a Basilisk II prefs file (Quadra 900, 68040 + FPU,
  64 MB) and runs `BasiliskII --config <file>`. The library is the Mac's
  **Unix** volume. There's no auto-open yet: the user opens the app there.
- Basilisk II comes from `scripts/fetch-basilisk.sh` into the gitignored
  `src-tauri/resources/basilisk/`. Upstream publishes no binaries, so
  it's Floppy's own universal build of a pinned kanjitalk755/macemu
  commit: `.github/workflows/basilisk.yml` builds it and publishes a
  `basilisk-ii-<date>-<commit>` release of this repo with the exact
  source plus GMP and MPFR (statically linked, LGPL). A rebuild never
  gives the same bytes, so after one, re-pin every hash in the fetch
  script from the release's `SHA256SUMS`. A copy the user picks with
  **Locate Basilisk II…** in the gear menu wins over the bundled one.

## How Amiga mode works

- `amiga.rs`: programs are floppy images (`.adf`, `.adz`, `.dms`),
  hard-disk files (`.hdf`), and AmigaDOS executables (hunk header
  `000003F3`). The Kickstart version picks the default model (1.x A500,
  2.x A600, 3.x A1200).
- Launch writes an FS-UAE config: `base_dir` under `library/run/fs-uae`
  (never `~/Documents/FS-UAE`), the user's Kickstart, the app's disk in
  DF0 (its other floppies plus Workbench in the swap list), or Workbench
  for folder apps, and the library as the non-booting `Floppy:` drive.
- FS-UAE comes from `scripts/fetch-fs-uae.sh` (pinned version and per-arch
  SHA-256) into the gitignored `src-tauri/resources/fs-uae/`.

## Testing

- `cargo test --manifest-path src-tauri/Cargo.toml`: unit tests.
- `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`: the
  end-to-end test (`e2e.rs`) runs a real `.COM` in the bundled DOSBox,
  headless via `SDL_VIDEODRIVER=dummy`. It needs the fetch script first.
  FS-UAE can't run headless (it needs OpenGL and a window server), and
  Basilisk II can't boot without a user's ROM, so Mac and Amiga launches
  are covered by config-generation unit tests only. Fork handling tests
  run only on a macOS host.
- `npx tsc --noEmit` for the frontend.
- Run all three before calling a change done. A bug report becomes a
  failing test first, then the fix.
- In Claude Code's sandbox, `~/.cargo` and `~/.npm` aren't writable. Use
  `cargo … --offline` (its crates are already cached) and `npm install --cache "$TMPDIR/npm-cache"`. The sandbox
  has no display, so DOSBox only runs headless there.

## Dev conventions

- **Before any commit and push, check for security issues:** hardcoded
  dev-machine paths, anything that should be gitignored (especially
  `src-tauri/resources/dosbox/`, `src-tauri/resources/fs-uae/` and
  `src-tauri/resources/basilisk/`), and dependency licenses whenever
  `Cargo.lock` or `package-lock.json` changes (rule 1).
- **Build releases with `scripts/build-release.sh`**, never a bare
  `tauri build`. rustc bakes source paths (including every dependency's
  under `~/.cargo/registry`) into panic messages, which would publish the
  builder's home folder and user name. The script remaps them with
  `--remap-path-prefix` and fails if `$HOME` is still in the built app.
  Arguments pass through to `tauri build` (`--bundles app` skips the DMG,
  which needs `hdiutil`). It then downloads the bundled GPL emulators'
  source into `src-tauri/target/release/bundle/source/`. Attach those to
  the GitHub release with the DMG: shipping their binaries means offering
  their source.
  Last, it copies the new `Floppy.app` into `~/Applications/`, replacing
  the old one, so the installed app is always the latest release build.
  Outside the sandbox only: `~/Applications` isn't writable in it.
- **Track platform parity** in `docs/platform-parity.md` whenever a
  feature uses a macOS-specific mechanism. Don't implement the
  Windows/Linux side early.
- Dev server port is **1430**, kept in sync between `vite.config.ts` and
  `tauri.conf.json`'s `devUrl`.
- UI follows the shared ansiapps design system: same tokens, with
  Floppy's own `--accent` (phosphor green). Reuse existing components
  before adding a new UI element (`CLAUDE.local.md` says where the
  reference components live).
- **There is always a Gears button.** The gear icon at the header's
  top right (`data-testid="gear-button"`) is the one place for
  infrequent app-level actions: settings, lists and reports to save,
  imports from discs, and About Floppy. New actions of that kind go in
  its menu, grouped with `.menu-sep`, never as extra buttons in the main
  layout. Never remove it.
- **Two themes: modern (default) and ANSIapps**, an old-school DOS
  text-mode look toggled from the gear menu (`docs/ansiapps-theme.md`,
  `src/ansiapps-theme.css`, `src/lib/theme.ts`). New UI stays on the
  color tokens so the ANSIapps theme follows it, and gets checked in
  both themes. The theme's font (`public/fonts/ansiapps/`) is CC BY-SA
  4.0: ship it unmodified as its own file with its license and the
  About credit. Never subset, convert or inline it.

## Not built yet (next steps)

- Single-instance handling: a second `floppy import …` while Floppy is
  running currently opens a second window.
- Per-app DOSBox settings (cycles, machine type, sound).
- Code signing: Developer ID signing and notarization must also sign the
  nested `DOSBox Staging.app`, `FS-UAE.app` and `BasiliskII.app`. The fetch script's 7-Zip
  fallback path loses DOSBox's original signature, and FS-UAE's tarball
  signature doesn't verify as shipped. Tauri's resource copy also turns
  symlinks into plain files (BasiliskII.app's `SDL2.framework`), so
  `codesign --verify --deep` fails on all three nested apps in the built
  Floppy.app today. They still run unsigned-style (checked 2026-09-27),
  but signing must re-sign each one, or restore the framework symlinks
  first.
- Auto-opening a Mac app (alias in the startup disk's Startup Items) or
  an Amiga folder app (`user-startup`). Today the guest boots and the
  user opens the app.
- SheepShaver for PowerPC-only Mac apps (Mac OS 8.5 to 9.0.4).
- Windows/Linux: Mac fork handling, FS-UAE paths, Basilisk II detection
  (`docs/platform-parity.md`).
- A universal (arm64 + x86-64) build: the FS-UAE fetch is per-arch.
- More known-good hashes as users report copies that aren't listed (the
  IIci's single-file ROM dump, for one). `known_files.rs` is generated:
  edit `scripts/update-known-hashes.py` and run it, never the `.rs`.
