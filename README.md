# Floppy

Run the old apps your files need, in the OS they were made for.

Floppy is an emulation frontend and launcher for original-era desktop
software. Import a DOS program (a folder, a zip, or a single `.EXE`), and
Floppy keeps it in a library and boots it in DOSBox Staging with one click.
It also runs Amiga software in FS-UAE and classic Mac OS (System 7 to
Mac OS 8.1) apps in Basilisk II, using ROMs and system disks you supply.

## Get Floppy

Floppy is free, and always will be. Download it from
[ansiapps.com](https://ansiapps.com), the only place it's published.

Floppy is the heart of ansiapps. Diskette (an offline catalog of your
drives that finds the old apps and setup files you already own) and
Crunchy (duplicate finding across those drives) are paid extensions to
it. They're separate programs, and Floppy works fully without them:
anything can hand Floppy an app or its files through the plain
interfaces below ([Handoff](#handoff), [Finding missing
files](#finding-missing-files)).

## Status

Early development, on macOS and Linux, with an experimental Windows build
path. DOS and Amiga can use the bundled Windows emulators; classic Mac
needs a separately installed or located Windows Basilisk II build. Windows
old-media pickup and Diskette handoff are not implemented yet. DOS is the
most complete. Classic Mac and Amiga boot the guest with your library
shared into it, but don't open a folder app for you yet.

## Building (macOS)

Requires Node 20+, Rust, and the [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/).

```sh
npm install
scripts/fetch-dosbox.sh   # downloads the pinned DOSBox Staging into src-tauri/resources/dosbox/
scripts/fetch-fs-uae.sh   # downloads the pinned FS-UAE into src-tauri/resources/fs-uae/
scripts/fetch-basilisk.sh # downloads Floppy's pinned Basilisk II build into src-tauri/resources/basilisk/
npm run tauri dev         # develop
scripts/build-release.sh  # release build: tauri build, with build-machine paths stripped
```

## Building (Windows)

Requires Node 20+, Rust, Python 3, and the
[Tauri Windows prerequisites](https://tauri.app/start/prerequisites/).
From PowerShell:

```powershell
npm ci
.\scripts\build-windows.ps1
```

The script downloads the hash-pinned DOSBox Staging and FS-UAE Windows
builds, generates the bundled library licenses, and builds the MSI and
NSIS installers with build-machine paths remapped. The Windows Basilisk II
executable is not bundled; install or locate a compatible build before
using classic Mac mode. The remaining Windows gaps are tracked in
[platform parity](docs/platform-parity.md). Build packages locally; Floppy
is distributed only from ansiapps.com.

Basilisk II (classic Mac) has no official binary release, so Floppy
bundles its own build of a pinned kanjitalk755/macemu commit, made by
`.github/workflows/basilisk.yml`. To use a different build, install it
as `BasiliskII.app` and pick it with **Locate Basilisk II…** in the gear
menu, or point `FLOPPY_BASILISK` at its executable:

```sh
FLOPPY_BASILISK=/path/to/BasiliskII.app/Contents/MacOS/BasiliskII npm run tauri dev
```

## How the DOS library works

- Each imported app gets its own folder under Floppy's app-data directory
  (`~/Library/Application Support/com.ansiapps.floppy/library/dos/`).
  That whole folder is drive C: in DOSBox, so apps can reach each other's
  files.
- File names DOS can't see as-is (longer than 8.3, spaces, non-ASCII) are
  renamed to 8.3 names at import. DOS programs can only refer to 8.3 names,
  so this doesn't break them.
- Floppy guesses which program launches the app. Installers and setup
  tools rank low. You can pick a different one.
- DOSBox Staging starts with `--noprimaryconf` and a config Floppy writes
  for each launch, so your own DOSBox settings don't affect Floppy.

## Classic Mac and Amiga

Each needs system files you own. Add them in the guest's setup box, and
Floppy copies them into its library (`library/system/`):

- **Classic Mac:** a Mac ROM (Mac II, IIci, Quadra and similar, 512 KB or
  1 MB) and a startup disk image with System 7 to Mac OS 8.1. Floppy
  emulates a Quadra 900. Your apps appear on the Mac's **Unix** volume.
  Imports keep resource forks: copy apps from a Mac disk, a zip made on a
  Mac, or a MacBinary `.bin`. StuffIt and BinHex archives are copied to
  expand in the Mac with your own StuffIt Expander.
- **Amiga:** a Kickstart ROM (from your Amiga, or Amiga Forever with its
  `rom.key`) and, for apps that aren't bootable disks, Workbench (a floppy
  image, a hard-disk file, or a folder). A disk-image app boots straight
  from its disk. Your apps appear on the Amiga's `Floppy:` drive.

Only one Mac and one Amiga run at a time, because each guest's apps
share its startup disk. See [docs/emulators.md](docs/emulators.md) for
why these emulators were chosen.

## Handoff

```sh
Floppy.app/Contents/MacOS/floppy import [--os dos|mac-classic|amiga] <path>
```

This imports `<path>` (a folder, a `.zip`, or a program or disk image for
that guest) and opens Floppy with the app selected. `--os` defaults to
`dos`. Other apps (such as Diskette) can use this command to send
Floppy an app.

## Finding missing files

Already have your ROMs and system disks somewhere on your drives, but
don't know where? In the Mac or Amiga setup box:

1. **Save Missing-Files List…** writes a `.txt` naming the files still
   needed, one per line, under the names they're commonly stored as
   (`#` lines are comments).
2. Gather copies of those files into a disc image or a folder. Any layout
   works, and duplicates are fine. Diskette's **Burn A CD** does this
   from its catalog of your drives, or you can copy them by hand.
3. **Import Files Disc…** (a disc image) or **Import Folder of Files…**
   (a folder) fills in whatever's still missing.

Floppy recognizes each file by its contents, not its name, and picks an
undamaged copy when there are several. The list format is specified in
[`src-tauri/src/cd.rs`](src-tauri/src/cd.rs).

## What Floppy never includes

Floppy never bundles or downloads operating systems, ROMs, or the
applications you run in it, abandonware included. You supply them.
DOS needs no OS files, since DOSBox provides its own DOS. (FS-UAE's own
download contains AROS, a free, open-source replacement Kickstart.
Floppy uses it only if you choose **Use AROS for Now** in the Amiga
setup, and switches to a real Kickstart as soon as you add one.)

## License

Floppy is free software: GPL-2.0-or-later (see `LICENSE`). It bundles
[DOSBox Staging](https://www.dosbox-staging.org) (GPL-2.0-or-later),
[FS-UAE](https://fs-uae.net) (GPL-2.0), and the
IBM Plex fonts (SIL Open Font License 1.1, see `public/fonts/OFL.txt`).
