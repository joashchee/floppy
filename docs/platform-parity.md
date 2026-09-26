# Platform parity

Every feature that relies on a macOS-specific mechanism gets a row here
when it's written: what it does, the macOS mechanism, and the
Windows/Linux equivalent. Don't implement the other platforms early. Just
keep the gap tracked.

| Feature | macOS | Windows | Linux |
|---|---|---|---|
| Bundled DOSBox Staging | `scripts/fetch-dosbox.sh` unpacks the release DMG (hdiutil, or 7zz as a fallback). Binary at `DOSBox Staging.app/Contents/MacOS/dosbox` | Release `.zip`, `dosbox.exe` (`BUNDLED_BIN` in `dosbox.rs` already set) | Release `.tar.xz`, `dosbox`. Or rely on a system/Flatpak DOSBox Staging |
| Installed-DOSBox fallback | `/Applications/DOSBox Staging.app` | `%ProgramFiles%\DOSBox Staging\dosbox.exe` (not implemented) | `dosbox` on `PATH` (not implemented) |
| Show in Finder | `revealItemInDir` (opener plugin) | Same API, opens Explorer | Same API, opens the file manager |
| `floppy import` handoff | Binary inside `Floppy.app/Contents/MacOS/` | `floppy.exe` on the install path | `floppy` binary / AppImage |
| Bundled FS-UAE | `scripts/fetch-fs-uae.sh` unpacks the release `.tar.xz` for the build machine's arch. Binary at `FS-UAE.app/Contents/MacOS/fs-uae` | Release `.zip` (`FSUAE_BIN` in `emulator.rs` is a guess, unverified) | Release `.tar.xz` (same) |
| Installed FS-UAE fallback | `/Applications/FS-UAE.app` | Not implemented | `fs-uae` on `PATH` (not implemented) |
| Basilisk II (not bundled) | `BasiliskII.app` in `/Applications` or `~/Applications`, one folder deep | Not implemented (`BasiliskII.exe`) | Not implemented (`BasiliskII` on `PATH`) |
| Classic Mac resource forks and Finder info | Written natively: `file/..namedfork/rsrc` and the `com.apple.FinderInfo` xattr (`mac.rs`, `libc`). Basilisk II's extfs reads them the same way. | Basilisk II's extfs keeps them in `.rsrc/` and `.finf/` folders. Mac imports return an error for now. | Same as Windows |
| Universal (arm64 + x86-64) Floppy.app | The fetch script only fetches FS-UAE for the build machine's arch. A universal build needs both, plus picking the right one at runtime. | n/a | n/a |
| Release build (`scripts/build-release.sh`) | POSIX `sh`, leak check with `grep` over `bundle/macos` | Needs a PowerShell equivalent, with the same `RUSTFLAGS` remap of `%USERPROFILE%` | Same script works; check `bundle/appimage`, `bundle/deb` instead |
| Import Files Disc | Mounts the ISO read-only with `/usr/bin/hdiutil attach` (`cd.rs`) | Not implemented: mount it and pick the folder (the command accepts a folder) | Same as Windows (or `mount -o loop,ro`) |
| Dropped setup files (`cd::import_dropped`) | A dropped disc image that isn't a system file itself is mounted read-only with `hdiutil`, like Import Files Disc. Files, folders and zips are portable. | Disc images are reported as skipped: drop the mounted folder instead | Same as Windows |
