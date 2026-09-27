# Platform parity

Every feature that relies on a platform-specific mechanism gets a row
here when it's written: what it does, and the mechanism on each platform.
macOS and Linux are supported: build both sides where you can, and track
any Linux gap here. Windows comes later, so don't implement it early.
Just keep the gap tracked.

| Feature | macOS | Windows | Linux |
|---|---|---|---|
| Bundled DOSBox Staging | `scripts/fetch-dosbox.sh` unpacks the release DMG (hdiutil, or 7zz as a fallback). Binary at `DOSBox Staging.app/Contents/MacOS/dosbox` | Release `.zip`, `dosbox.exe` (`DOSBOX_BIN` in `emulator.rs` already set) | Done: the fetch script unpacks the x86-64 release `.tar.xz` whole (binary at `dosbox`, its own `lib/` beside it) |
| Installed-DOSBox fallback | `/Applications/DOSBox Staging.app` | `%ProgramFiles%\DOSBox Staging\dosbox.exe` (not implemented) | Done: `dosbox-staging` on `PATH`, or a `dosbox` whose `--version` says Staging (the original DOSBox lacks `--noprimaryconf`). Flatpak not looked for |
| Show in Finder | `revealItemInDir` (opener plugin) | Same API, opens Explorer | Same API, opens the file manager; labeled "Show in Folder" (`src/lib/platform.ts`) |
| `floppy import` handoff | Binary inside `Floppy.app/Contents/MacOS/` | `floppy.exe` on the install path | `floppy` binary / AppImage |
| Bundled FS-UAE | `scripts/fetch-fs-uae.sh` unpacks the release `.tar.xz` for the build machine's arch. Binary at `FS-UAE.app/Contents/MacOS/fs-uae` | Release `.zip` (`FSUAE_BIN` in `emulator.rs` is a guess, unverified) | Done: the fetch script unpacks the x86-64 release `.tar.xz` whole (binary at `Linux/x86-64/fs-uae`) |
| Installed FS-UAE fallback | `/Applications/FS-UAE.app` | Not implemented | Done: `fs-uae` on `PATH` |
| Bundled Basilisk II | `scripts/fetch-basilisk.sh` unpacks Floppy's own universal build (`.github/workflows/basilisk.yml`'s macOS job). Binary at `BasiliskII.app/Contents/MacOS/BasiliskII` | Not built (`BASILISK_BIN` is `None`) | Done: the workflow's Linux job (Unix build, static SDL2, Ubuntu 22.04); `fetch-basilisk.sh` unpacks `BasiliskII` and `COPYING` into `basilisk/` |
| Installed Basilisk II fallback | `BasiliskII.app` in `/Applications` or `~/Applications`, one folder deep, or one the user locates (an `.app` picked with **Locate Basilisk II…**, run from its `Contents/MacOS/`; `library/emulators.json`) | Not implemented (`BasiliskII.exe`) | Done: `BasiliskII` on `PATH`, or any program picked with **Locate Basilisk II…** (no `.app` filter there) |
| Classic Mac resource forks and Finder info | Written natively: `file/..namedfork/rsrc` and the `com.apple.FinderInfo` xattr (`mac.rs`, `libc`). Basilisk II's extfs reads them the same way. | Basilisk II's extfs keeps them in `.rsrc/` and `.finf/` folders. Not checked against a Windows build yet | Done: `.rsrc/<name>` and `.finf/<name>` beside each file (`mac.rs`), copied along with folder imports and skipped when finding programs |
| Universal (arm64 + x86-64) Floppy.app | The fetch script only fetches FS-UAE for the build machine's arch. A universal build needs both, plus picking the right one at runtime. | n/a | n/a |
| Release build (`scripts/build-release.sh`) | POSIX `sh`, leak check with `grep` over `bundle/macos` | Needs a PowerShell equivalent, with the same `RUSTFLAGS` remap of `%USERPROFILE%` | Done: the same script checks `bundle/deb` and `bundle/appimage` and skips the `~/Applications` install. `.deb` checked: emulators land in `/usr/lib/Floppy/` with exec bits |
| Import Files Disc | Mounts the ISO read-only with `/usr/bin/hdiutil attach` (`cd.rs`) | Not implemented: mount it and pick the folder (the command accepts a folder) | Done, untested on a desktop: `udisksctl loop-setup --read-only` then `udisksctl mount` (no root on desktop sessions), torn down after; uses the desktop's own mount if it got there first. Without udisks, pick the folder |
| Old media macOS can't mount (`media.rs`) | Polls `/dev` for `diskN`, judges with `diskutil list/info -plist`, reads through `/usr/libexec/authopen` | Not implemented (`\\.\PhysicalDriveN`, needs elevation) | Not implemented (`/dev/sdX` via udev, needs a polkit prompt) |
| Is Diskette running (Ask Diskette, `request.rs`) | `/usr/bin/lsappinfo info -only bundleid -app com.ansiapps.diskette`, polled every 5 s | Not implemented (process list, e.g. `sysinfo`, matched on Diskette's install path) | Same as Windows (`/proc/*/exe`) |
| Hand Diskette the request list | `open -b com.ansiapps.diskette <list>` | Not implemented (launch Diskette's `.exe` with the list) | Not implemented (its `.desktop` `Exec=` line) |
| Receive a disc Diskette sends back | Tauri's `RunEvent::Opened` (the "open documents" Apple event), `request::Opened` | Files arrive as arguments; a running Floppy needs the single-instance plugin to forward them | Same as Windows |
| Dropped setup files (`cd::import_dropped`) | A dropped disc image that isn't a system file itself is mounted read-only with `hdiutil`, like Import Files Disc. Files, folders and zips are portable. | Disc images are reported as skipped: drop the mounted folder instead | Done: mounted with udisks, as Import Files Disc |
| Picking up setup files from Downloads (`cd::import_from_downloads`) | Tauri's `download_dir()` (`~/Downloads`) | Same API (the Known Folder) | Done: `download_dir()` reads `user-dirs.dirs`; without it, `~/Downloads` if it exists |
| Dropdowns (`select`) | WebKit draws them; Floppy's CSS draws them itself anyway (`appearance: none`) | n/a | Done: WebKitGTK's native ones ignored the theme colors, hence the CSS |
