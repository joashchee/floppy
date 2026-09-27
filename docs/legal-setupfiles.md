# Legal sources for setup files

A living list of the free, legal places to get the files Floppy's setup
asks for. Floppy never downloads or bundles any of them (rules 3 and 4
in CLAUDE.md). This list is for users, and it's what the setup screens
point to: Floppy reads the "Where Floppy points you" table below when
it's built, and shows each missing file's rows with an **Open** button
that opens the page in the user's browser. Floppy itself still makes no
network connection. When the user comes back from the browser, Floppy
looks through their Downloads folder for the file, by its contents.

**Last checked: 2026-09-27.** Re-check the links and terms whenever you
touch this file, and update the date. Add a row to the check log at the
bottom each time.

## Summary

| Setup file | Needed for | Free and legal? | Where |
|---|---|---|---|
| Nothing | DOS | n/a | DOSBox Staging provides DOS |
| Mac ROM (IIci, IIsi, Quadra, Centris…) | Classic Mac | **No** | Only a dump of a Mac you own |
| Mac startup disk (System 7.0.1 or 7.5.3) | Classic Mac | **Free from Apple**, now only on archives (see caveat) | [System 7.5.3 image](https://archive.org/download/AppleMacintoshSystem753/System7_5_3.img), [System 7.0.1 image](https://archive.org/download/AppleMacintoshSystem701/System7_0_1.img) |
| Mac startup disk (7.1, 7.6, 8.0, 8.1) | Classic Mac | **No**, these were paid releases | Your own install discs |
| Kickstart ROM | Amiga | **No** (cheapest licensed copy is paid) | [Amiga Forever](https://www.amigaforever.com/), or a dump of an Amiga you own. Meanwhile: the free AROS replacement, built into FS-UAE (see below) |
| Workbench | Amiga | **No** | Amiga Forever, or your own disks |
| Basilisk II (the emulator) | Classic Mac | **Yes** (GPL-2.0-or-later) | Bundled: Floppy's own build for macOS and Linux (`.github/workflows/basilisk.yml`); [E-Maculation builds](https://www.emaculation.com/forum/viewtopic.php?f=6&t=7361) |

## Where Floppy points you

Floppy's setup screen shows these rows for each file that's still
missing, in this order, and `cd.rs` reads them straight from this
table when Floppy is built (a unit test fails on a malformed row). Keep
to the columns:

- **Slot:** `Mac ROM`, `Mac startup disk`, `Kickstart ROM` or `Workbench disk`.
- **Kind:** `free` (costs nothing), `paid` (a licensed copy to buy) or
  `own` (how to copy it from hardware or disks you own).
- **Link:** `https://` only. Floppy opens it in the browser.
- **Note:** one or two sentences the user sees. Say plainly when a
  source is a mirror rather than the rights holder.

Change a row when merged findings report a source broken or a better
one (see "Reported setup notes" below), after checking it yourself.

<!-- sources:start -->
| Slot | Kind | Source | Link | Note |
|---|---|---|---|---|
| Mac ROM | own | Copy the ROM from a Mac you own (E-Maculation guide) | https://www.emaculation.com/doku.php/capturing_rom | Apple never released a Mac ROM for free. CopyROM saves one from a working 68k Mac: a IIci, IIsi, LC, Quadra, Centris or similar gives a 512 KB or 1 MB ROM that works here. |
| Mac startup disk | free | System 7.5.3, ready installed (Internet Archive) | https://archive.org/details/AppleMacintoshSystem753 | Apple gave System 7.5.3 away free to Mac owners. This is a third-party mirror of that release, not Apple's own download (Apple no longer offers one). Download System7_5_3.img: Floppy uses it as it is. |
| Mac startup disk | free | System 7.0.1, ready installed (Internet Archive) | https://archive.org/details/AppleMacintoshSystem701 | Also a free Apple release, mirrored the same way. Older than 7.5.3 and runs fewer apps, so pick 7.5.3 unless an app needs 7.0. |
| Kickstart ROM | paid | Amiga Forever Essentials for Android (about US$2) | https://www.amigaforever.com/android/ | The cheapest licensed copy: official Kickstart 1.2 to 3.1 ROM files. Copy the ROM files from the phone to this computer, then drop them on Floppy. |
| Kickstart ROM | paid | Amiga Forever (Windows) | https://www.amigaforever.com/ | Licensed ROMs for every Amiga model. They're encrypted: bring the rom.key file along with them and Floppy unlocks them. |
| Kickstart ROM | own | Copy the ROM from an Amiga you own (FS-UAE guide) | https://fs-uae.net/docs/kickstart-roms/ | Tools such as TransROM or GrabKick save the Kickstart of your own Amiga to a file. |
| Workbench disk | paid | Amiga Forever (Windows) | https://www.amigaforever.com/ | Includes Workbench floppies (1.3 in Value, every version in Plus). Only needed for apps that aren't bootable disks. |
<!-- sources:end -->

## Classic Mac

### ROM: no free legal source

Mac ROMs are Apple's copyrighted code, and Apple has never released
one. The only legal copy is a dump of a Mac you own (a 32-bit clean
512 KB or 1 MB ROM: IIci, IIsi, IIfx, LC, Quadra, Centris…). Floppy
refuses Plus, SE, Classic and Mac II ROMs because Basilisk II does
(`docs/emulators.md`). E-Maculation's
[Capturing a Mac ROM Image](https://www.emaculation.com/doku.php/capturing_rom)
explains how, with CopyROM. A ROM over 1 MB comes from a PowerPC Mac
(or a Quadra 660AV/840AV) and doesn't work in Basilisk II.

### Startup disk: System 7.0.1 or 7.5.3

Apple made System 6.0.8, System 7.0.1 and System 7.5.3 (plus the 7.5.5
update) free downloads, "free to Macintosh owners". It stopped hosting
them years ago ([Low End Mac](https://lowendmac.com/2013/classic-mac-os-downloads-and-updates/)).
System 7.1, 7.6 and Mac OS 8.x were paid products and are not free.

Ready-installed disk images of the free releases are on the Internet
Archive. Floppy takes these directly as the Mac startup disk: they are
raw HFS images with a blessed System Folder.

- [System 7.5.3, installed on a 25 MB disk image](https://archive.org/download/AppleMacintoshSystem753/System7_5_3.img) ([item page](https://archive.org/details/AppleMacintoshSystem753))
- [System 7.0.1, installed on a 10 MB disk image](https://archive.org/download/AppleMacintoshSystem701/System7_0_1.img) ([item page](https://archive.org/details/AppleMacintoshSystem701))

System 7.5.3 is the better choice: it's newer and runs more apps.
System 6.0.8 is free too, but Basilisk II can't run it.

**Caveat:** Apple's permission was for downloading from Apple. The
Internet Archive copies are third-party mirrors of that free release,
not an Apple-authorized distribution. That's why Floppy's UI shouldn't
link to them without a clear note.

## Amiga

### Kickstart ROM: no free legal source

Kickstart ROMs are still under copyright, and Cloanto is the only
licensed seller of ROM files
([FS-UAE: Kickstart ROMs](https://fs-uae.net/docs/kickstart-roms/)):

- **Amiga Forever Essentials** (Android, about US$2): Kickstart 1.2,
  1.3, 2.04, 3.0 and 3.1 as 512 KB ROM files
  ([product page](https://www.amigaforever.com/android/)). It's the
  cheapest licensed copy. Still listed on Google Play as of 2026-09-27.
- **Amiga Forever** Value, Plus and Premium (Windows, paid). The Plus
  edition covers every model. Its ROMs are encrypted and need the
  included `rom.key`, which Floppy handles.
- **Amiga Forever's Express Edition** is how Amiga Forever runs
  unregistered. I found no sign that it provides ROM files usable in
  other emulators, so it isn't a free source.
- **Your own Amiga:** dump the ROM with a tool such as TransROM or
  GrabKick.

**AROS replacement ROM (free, open source, offered as a fallback):**
AROS provides a free Kickstart replacement under the AROS Public
License, from [AROS nightly builds](https://aros.sourceforge.io/cgi-bin/files?type=nightly2&lang=en)
(`Boot/Amiga/`). FS-UAE's bundled `fs-uae.dat` contains a copy (the
2015-05-20 m68k build), so there's nothing to download. Floppy's setup
offers **Use AROS for Now** under the Kickstart sources, with its
caveats, and FS-UAE then boots that built-in copy:

- It runs some games and demos on bootable disks, but much Amiga
  software crashes or won't start on it.
- It doesn't boot Commodore's Workbench, so apps that need Workbench
  don't run.
- It's a stand-in only: the Kickstart stays on the missing-files list,
  and when Floppy finds a real Kickstart (dropped, in Downloads, on a
  files disc, or chosen) it switches to it and says so.

Floppy doesn't take AROS ROM files dropped by the user (a main plus an
extended ROM), only FS-UAE's built-in copy.

### Workbench: no free legal source

Workbench disks come with Amiga Forever (1.3 in Value, all versions in
Plus), with your own original disks, or with Hyperion's AmigaOS 3.2
(paid). The AROS m68k boot floppy is free, but it needs the AROS ROM
above.

## Known-good hashes Floppy looks for

Many users already own these files under a name no list can guess. Each
section of the missing-files list therefore ends with nameless lines
giving the size and SHA-1 of known-good copies, so a tool that hashes
files finds a copy whatever it's called (`cd.rs`). These are hashes
only: Floppy doesn't download or ship any of the files.

| Setup file | Copies listed | Hashes from |
|---|---|---|
| Mac ROM | 20 | [MAME](https://github.com/mamedev/mame/tree/master/src/mame/apple)'s Apple drivers. Only the 32-bit clean dumps Basilisk II runs (IIsi, IIfx, LC, LC III/520, Quadra, Centris, PowerBook 140–180 and Duo, Color Classic, TV). The IIci is missing: MAME lists it only as four separate chips. |
| Mac startup disk | 4 | Internet Archive file metadata: the installed System 7.5.3 and 7.0.1 disks above, and Mac OS 7.6's two bootable Utilities floppies |
| Kickstart ROM | 74 | FS-UAE's ROM table (`rommgr.cpp`) and TOSEC's "Commodore Amiga - Firmware" DAT. Includes 4 encrypted Amiga Forever ROMs, which also need their `rom.key`. |
| Workbench disk | 81 | TOSEC's "Commodore Amiga - Operating Systems - Workbench" DAT (boot disks only, v1.0 to 3.1) and the FS-UAE launcher's list |

Left out on purpose:
- **Bad dumps and overdumps** (`[b]`, `[o]`) and Kickstart betas.
- **Install CDs** (System 7.5.3, Mac OS 7.6, 8.0 and 8.1 are all on the
  Internet Archive). They're read-only, and the startup-disk slot needs
  a writable disk with the system already installed.
- **The Workbench sets' other disks** (Install, Extras, Fonts, Locale,
  Storage).

To refresh the list, run `scripts/update-known-hashes.py`, optionally
with `--tosec <TOSEC DAT pack zip>`. It regenerates
`src-tauri/src/known_files.rs` from these sources. Then add a row to the
check log. To add a Mac startup disk, add it to the script's
`MAC_BOOT_DISKS`. A user-reported ROM dump that's missing goes in the
same way, once its source publishes the hash.

### Reported by users

Copies Floppy users set up that none of the sources above list, from
merged findings (Floppy's **Export Findings…**, then
`scripts/merge-findings.py`). Floppy reads this table straight from this
file (`cd.rs`) and adds each row to the missing-files list like the
published ones, so every release looks for them too. Only ROMs and
Workbench floppies are reported: startup disks and hard-disk files
change as they're used, so their hashes say nothing.

A row means someone's Floppy recognized the file by its contents and
had it set up, not that anyone checked the dump. Floppy still checks
every file's contents on import, so a wrong row costs a wasted copy at
worst. Once a published source lists the same hash, the merge script
drops the row, since `known_files.rs` has it.

<!-- reported:start -->
| Slot | What | Size | SHA-1 | Reports | Last reported |
|---|---|---|---|---|---|
<!-- reported:end -->

## Reported setup notes

What Floppy users reported about getting their setup files, from merged
findings: a source that stopped working, a file that didn't work once
set up, or a better source. In Floppy they're under each missing file's
sources (**Report a Problem or Better Source…**), and they leave only in
an **Export Findings…** zip. For people to read: Floppy doesn't use this
table. Check a report yourself before changing "Where Floppy points
you", then delete its row.

<!-- setup-notes:start -->
| Slot | Kind | Source | Note | File | Reports | Last reported |
|---|---|---|---|---|---|---|
<!-- setup-notes:end -->

## Emulators

All three emulators are bundled with Floppy on macOS and Linux. Basilisk
II is Floppy's own build, and other builds are free under
GPL-2.0-or-later too:

- [E-Maculation: BasiliskII builds for Mac OS X](https://www.emaculation.com/forum/viewtopic.php?f=6&t=7361):
  universal SDL2 builds from kanjitalk755/macemu. The forum is behind a
  browser check, so download by hand.
- Floppy's own pinned build: the `basilisk-ii-…` releases on Floppy's
  GitHub repo, made by `.github/workflows/basilisk.yml` and bundled by
  `scripts/fetch-basilisk.sh`.

## Check log

| Date | What was checked | Result |
|---|---|---|
| 2026-09-27 | FS-UAE 3.2.35's built-in AROS (`fs-uae.dat`, its `configure_roms` in the source tarball) | `kickstart_file = internal` boots the AROS m68k ROM of 2015-05-20 (exec 51.3) without scanning for ROM files. Offered in setup as **Use AROS for Now**. |
| 2026-09-27 | Web searches for the System 7.5.3 Internet Archive item, Amiga Forever Essentials, and Mac ROM capture guides (the sandbox blocked direct fetches of archive.org, amigaforever.com and fs-uae.net) | System 7.5.3's installed-disk item is still listed. Amiga Forever Essentials is still on Google Play, with its own page at amigaforever.com/android. Added E-Maculation's ROM capture guide. Links not opened directly: re-check them from a browser. |
| 2026-09-27 | Floppy's Basilisk II build (macemu `892eeb7`, from `.github/workflows/basilisk.yml`) | Now bundled. Its source tarball matches GitHub's archive of the commit file for file, and the GMP 6.3.0 and MPFR 4.2.2 tarballs match GNU's SHA-256s. |
| 2026-09-26 | Known-good hashes: MAME Apple drivers, FS-UAE `rommgr.cpp` and launcher Workbench lists, TOSEC DAT pack 2025-03-13 (Amiga Firmware and Workbench DATs), Internet Archive metadata for System 7.0.1/7.5.3 and Mac OS 7.6 | 20 Mac ROMs, 4 Mac startup disks, 74 Kickstarts, 81 Workbench disks. TOSEC has no Mac OS DAT. The Internet Archive's Mac ROM collections are zipped, so their per-file hashes can't be read without downloading ROMs, which Floppy won't do. |
| 2026-09-26 | Apple free System releases, Internet Archive items (file lists via `archive.org/metadata/<id>`), Amiga Forever editions, FS-UAE Kickstart docs, AROS nightlies, E-Maculation builds | As above. The Wayback Machine was offline, so Apple's archived download pages and licence text weren't checked. |
