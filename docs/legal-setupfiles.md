# Legal sources for setup files

A living list of the free, legal places to get the files Floppy's setup
asks for. Floppy never downloads or bundles any of them (rules 3 and 4
in CLAUDE.md). This list is for users, and for deciding what the setup
screens can point to.

**Last checked: 2026-09-26.** Re-check the links and terms whenever you
touch this file, and update the date. Add a row to the check log at the
bottom each time.

## Summary

| Setup file | Needed for | Free and legal? | Where |
|---|---|---|---|
| Nothing | DOS | n/a | DOSBox Staging provides DOS |
| Mac ROM (IIci, IIsi, Quadra, Centris…) | Classic Mac | **No** | Only a dump of a Mac you own |
| Mac startup disk (System 7.0.1 or 7.5.3) | Classic Mac | **Free from Apple**, now only on archives (see caveat) | [System 7.5.3 image](https://archive.org/download/AppleMacintoshSystem753/System7_5_3.img), [System 7.0.1 image](https://archive.org/download/AppleMacintoshSystem701/System7_0_1.img) |
| Mac startup disk (7.1, 7.6, 8.0, 8.1) | Classic Mac | **No**, these were paid releases | Your own install discs |
| Kickstart ROM | Amiga | **No** (cheapest licensed copy is paid) | [Amiga Forever](https://www.amigaforever.com/), or a dump of an Amiga you own |
| Workbench | Amiga | **No** | Amiga Forever, or your own disks |
| Basilisk II (the emulator) | Classic Mac | **Yes** (GPL-2.0-or-later) | [E-Maculation builds](https://www.emaculation.com/forum/viewtopic.php?f=6&t=7361); Floppy's own build once published (`.github/workflows/basilisk.yml`) |

## Classic Mac

### ROM: no free legal source

Mac ROMs are Apple's copyrighted code, and Apple has never released
one. The only legal copy is a dump of a Mac you own (a 32-bit clean
512 KB or 1 MB ROM: IIci, IIsi, IIfx, LC, Quadra, Centris…). Floppy
refuses Plus, SE, Classic and Mac II ROMs because Basilisk II does
(`docs/emulators.md`).

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
  1.3, 2.04, 3.0 and 3.1 as 512 KB ROM files. It's the cheapest licensed
  copy, but availability on Google Play has been inconsistent.
- **Amiga Forever** Value, Plus and Premium (Windows, paid). The Plus
  edition covers every model. Its ROMs are encrypted and need the
  included `rom.key`, which Floppy handles.
- **Amiga Forever's Express Edition** is how Amiga Forever runs
  unregistered. I found no sign that it provides ROM files usable in
  other emulators, so it isn't a free source.
- **Your own Amiga:** dump the ROM with a tool such as TransROM or
  GrabKick.

**AROS replacement ROM (free, open source, not usable yet):** AROS
provides a free Kickstart replacement under the AROS Public License,
from [AROS nightly builds](https://aros.sourceforge.io/cgi-bin/files?type=nightly2&lang=en)
(`Boot/Amiga/`). FS-UAE's bundled `fs-uae.dat` contains it too. Floppy
doesn't accept it today, for two reasons:

- AROS ships as a main ROM plus an extended ROM, but
  `amiga::identify_kickstart` only accepts a single 256 KB or 512 KB
  Kickstart.
- It runs much less Amiga software than a real Kickstart.

Supporting it would be a deliberate feature, not just a setup-file
source.

### Workbench: no free legal source

Workbench disks come with Amiga Forever (1.3 in Value, all versions in
Plus), with your own original disks, or with Hyperion's AmigaOS 3.2
(paid). The AROS m68k boot floppy is free, but it needs the AROS ROM
above.

## Emulators

DOSBox Staging and FS-UAE are bundled. Basilisk II isn't bundled yet,
and it's free under GPL-2.0-or-later:

- [E-Maculation: BasiliskII builds for Mac OS X](https://www.emaculation.com/forum/viewtopic.php?f=6&t=7361):
  universal SDL2 builds from kanjitalk755/macemu. The forum is behind a
  browser check, so download by hand.
- Floppy's own pinned build: the `basilisk-ii-…` releases on Floppy's
  GitHub repo, made by `.github/workflows/basilisk.yml`, once published.

## Check log

| Date | What was checked | Result |
|---|---|---|
| 2026-09-26 | Apple free System releases, Internet Archive items (file lists via `archive.org/metadata/<id>`), Amiga Forever editions, FS-UAE Kickstart docs, AROS nightlies, E-Maculation builds | As above. The Wayback Machine was offline, so Apple's archived download pages and licence text weren't checked. |
