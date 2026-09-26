//! The missing-files list and files disc. Floppy can't fetch guest system
//! files (rule 3), but the user may already own copies somewhere on their
//! drives. This is a plain-file contract any tool can take part in
//! (Diskette's Burn A CD is one):
//!
//! 1. **Missing-files list** (`missing_list`): a `.txt` of the system files
//!    Floppy still needs, one file name per line, `#` comment lines,
//!    meant to be matched ignoring case. Names are the ones these files
//!    are commonly stored under, so a renamed dump won't be found.
//! 2. **Files disc**: a disc image (ISO and similar) or a folder holding
//!    copies of those files, laid out any way at all, at any depth, with
//!    duplicates allowed. A tool may place each name independently, so a
//!    ROM and its `rom.key` can end up in different folders.
//! 3. `import_cd` reads the disc, recognizes each file by its contents
//!    rather than its name, and fills every empty slot with the best
//!    copy, trying the next copy if one is damaged.
//!
//! Only these two files cross between programs (rule 2).

use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::library::{GuestOs, Library, SystemFile};
use crate::{amiga, mac};

/// A system file a guest needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    MacRom,
    MacBoot,
    Kickstart,
    Workbench,
}

pub const SLOTS: [Slot; 4] = [Slot::MacRom, Slot::MacBoot, Slot::Kickstart, Slot::Workbench];

impl Slot {
    pub fn os(self) -> GuestOs {
        match self {
            Slot::MacRom | Slot::MacBoot => GuestOs::MacClassic,
            Slot::Kickstart | Slot::Workbench => GuestOs::Amiga,
        }
    }

    pub fn kind(self) -> SystemFile {
        match self {
            Slot::MacRom | Slot::Kickstart => SystemFile::Rom,
            Slot::MacBoot | Slot::Workbench => SystemFile::Boot,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Slot::MacRom => "Mac ROM",
            Slot::MacBoot => "Mac startup disk",
            Slot::Kickstart => "Kickstart ROM",
            Slot::Workbench => "Workbench disk",
        }
    }

    /// What the `.txt` says this group of names is for.
    fn comment(self) -> &'static str {
        match self {
            Slot::MacRom => "Mac ROM for Basilisk II: a 512 KB or 1 MB ROM from a Mac IIci, Quadra, Centris or similar.",
            Slot::MacBoot => "Mac startup disk: a disk image with System 7 to Mac OS 8.1 installed.",
            Slot::Kickstart => "Amiga Kickstart ROM (plus rom.key for encrypted Amiga Forever ROMs).",
            Slot::Workbench => "Amiga Workbench boot disk.",
        }
    }

    /// Names these files are commonly stored under: emulator and Amiga
    /// Forever conventions, checksum-named Mac ROM dumps, and TOSEC names.
    fn names(self) -> &'static [&'static str] {
        match self {
            Slot::MacRom => &[
                "Quadra650.ROM", "Quadra 650.ROM", "Q650.ROM", "Quadra800.ROM", "Quadra 800.ROM", "Quadra900.ROM",
                "Quadra 900.ROM", "Quadra950.ROM", "Quadra 950.ROM", "Quadra610.ROM", "Quadra700.ROM",
                "Centris650.ROM", "Centris 650.ROM", "Centris610.ROM", "LC475.ROM", "LC 475.ROM", "IIci.ROM",
                "Mac IIci.ROM", "MacIIci.ROM", "IIsi.ROM", "IIfx.ROM", "Mac.ROM", "MacROM", "Mac ROM", "ROM",
                // Mac ROM dumps are often named by the checksum in their first long word.
                "F1ACAD13.ROM", "F1A6F343.ROM", "3DC27823.ROM", "420DBFF3.ROM", "368CADFE.ROM", "36B7FB6C.ROM",
                "4147DD77.ROM", "350EACF0.ROM", "35C28F5F.ROM", "FF7439EE.ROM", "5BF10FD1.ROM",
            ],
            Slot::MacBoot => &[
                "Macintosh HD.dsk", "Macintosh HD.img", "Macintosh HD.hfv", "Macintosh HD.hda", "System 7.dsk",
                "System 7.img", "System 7.hfv", "System7.dsk", "System 7.5.dsk", "System 7.5.3.dsk", "System 7.5.3.img",
                "System 7.5.5.dsk", "System 7.5.5.img", "System 7.6.dsk", "System 7.6.img", "System 7.6.1.dsk",
                "Mac OS 8.dsk", "Mac OS 8.img", "Mac OS 8.1.dsk", "Mac OS 8.1.img", "MacOS8.dsk", "boot.dsk",
                "boot.img", "hd.dsk", "HD.hfv", "Disk Tools.image", "Disk Tools.img",
            ],
            Slot::Kickstart => &[
                "kick.rom", "kickstart.rom", "kick13.rom", "kick204.rom", "kick205.rom", "kick30.rom", "kick31.rom",
                "kick34005.A500", "kick37175.A500", "kick37350.A600", "kick40063.A600", "kick40068.A1200",
                "kick40068.A4000", "amiga-os-130.rom", "amiga-os-204.rom", "amiga-os-205-a600.rom",
                "amiga-os-300-a1200.rom", "amiga-os-310-a600.rom", "amiga-os-310-a1200.rom", "amiga-os-310-a4000.rom",
                "Kickstart v1.3 rev 34.5 (1987)(Commodore)(A500-A1000-A2000-CDTV).rom",
                "Kickstart v2.04 rev 37.175 (1991)(Commodore)(A500+).rom",
                "Kickstart v2.05 rev 37.350 (1992)(Commodore)(A600HD).rom",
                "Kickstart v3.1 rev 40.68 (1993)(Commodore)(A1200).rom",
                "Kickstart v3.1 rev 40.68 (1993)(Commodore)(A4000).rom", "rom.key",
            ],
            Slot::Workbench => &[
                "Workbench.adf", "Workbench1.3.adf", "Workbench 1.3.adf", "wb13.adf", "Workbench2.04.adf",
                "Workbench 2.04.adf", "wb204.adf", "Workbench3.0.adf", "Workbench3.1.adf", "Workbench 3.1.adf",
                "wb31.adf", "amiga-os-130-workbench.adf", "amiga-os-204-workbench.adf", "amiga-os-300-workbench.adf",
                "amiga-os-310-workbench.adf", "Workbench.hdf", "wb31.hdf", "System.hdf",
            ],
        }
    }
}

/// Slots the user hasn't filled yet.
pub fn missing_slots(library: &Library) -> Result<Vec<Slot>, String> {
    let mut out = Vec::new();
    for slot in SLOTS {
        if library.system_file(slot.os(), slot.kind())?.is_none() {
            out.push(slot);
        }
    }
    Ok(out)
}

/// The missing-files list, or `None` when nothing is missing.
pub fn missing_list(slots: &[Slot]) -> Option<String> {
    if slots.is_empty() {
        return None;
    }
    let mut s = String::from(
        "# Files Floppy's emulators still need. Gather copies into a disc\n\
         # image or folder (Diskette's Burn A CD can do this from its\n\
         # catalog), then use Import Files Disc in Floppy.\n\
         # Floppy checks each file's contents, so extra matches do no harm.\n",
    );
    for slot in slots {
        s += &format!("\n# {}\n", slot.comment());
        for name in slot.names() {
            s += name;
            s += "\n";
        }
    }
    Some(s)
}

/// A file on the CD that looks like it fills a slot, and how good a copy
/// it seems.
#[derive(Debug, PartialEq)]
pub struct Candidate {
    pub slot: Slot,
    pub path: PathBuf,
    pub score: i64,
    /// For a Kickstart, its version; for Workbench, its volume or file name.
    pub detail: String,
    /// The `rom.key` that unlocks an encrypted Amiga Forever ROM.
    pub key: Option<PathBuf>,
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// What `path` is, judged by its contents. Checksums that pass beat
/// ones that fail, so an undamaged copy wins over a damaged one. `keys`
/// are every `rom.key` on the CD, for encrypted Amiga Forever ROMs.
fn classify(path: &Path, len: u64, keys: &[PathBuf]) -> Option<Candidate> {
    let name = file_name(path);
    let lower = name.to_lowercase();
    let head = crate::library::read_head(path, 16).ok()?;
    let cand = |slot, score, detail: String| Some(Candidate { slot, path: path.to_path_buf(), score, detail, key: None });

    if mac::is_basilisk_rom(&head, len) {
        let ok = std::fs::read(path).is_ok_and(|b| mac::rom_checksum_ok(&b));
        return cand(Slot::MacRom, 100 + if ok { 50 } else { 0 }, String::new());
    }
    match amiga::identify_kickstart(&head, len) {
        Some(amiga::Kickstart::Plain { version, .. }) => {
            let ok = std::fs::read(path).is_ok_and(|b| amiga::carry_sum_ok(&b));
            // Newer Kickstarts run more software.
            return cand(Slot::Kickstart, 100 + if ok { 50 } else { 0 } + i64::from(version), version.to_string());
        }
        Some(amiga::Kickstart::Encrypted) => {
            // A files disc may place each name independently, so the ROM
            // and its rom.key can land in different folders. Try the one next
            // to it first, then every other, until one unlocks it.
            let rom = std::fs::read(path).ok()?;
            let sibling = path.with_file_name("rom.key");
            let ordered = keys.iter().filter(|k| **k == sibling).chain(keys.iter().filter(|k| **k != sibling));
            return ordered.into_iter().find_map(|key| {
                let plain = amiga::decrypt_kickstart(&rom, &std::fs::read(key).ok()?)?;
                let version = match amiga::identify_kickstart(&plain, plain.len() as u64) {
                    Some(amiga::Kickstart::Plain { version, .. }) => version,
                    _ => 0,
                };
                let ok = amiga::carry_sum_ok(&plain);
                Some(Candidate {
                    slot: Slot::Kickstart,
                    path: path.to_path_buf(),
                    score: 100 + if ok { 50 } else { 0 } + i64::from(version),
                    detail: version.to_string(),
                    key: Some(key.clone()),
                })
            });
        }
        None => {}
    }
    if let Some((volume, bootable)) = amiga::adf_volume(path) {
        return (volume.to_lowercase().starts_with("workbench") || lower.contains("workbench"))
            .then(|| Candidate {
                slot: Slot::Workbench,
                path: path.to_path_buf(),
                score: 100 + if bootable { 50 } else { 0 },
                detail: volume,
                key: None,
            });
    }
    if amiga::is_disk_image(&name) && (lower.contains("workbench") || lower.starts_with("wb")) {
        return cand(Slot::Workbench, 60, name);
    }
    // Mac startup disks: at least a floppy's size, with a blessed System
    // Folder. Hard-disk images (with room for apps) beat floppies.
    if len >= 400 * 1024 && !amiga::is_disk_image(&name) {
        if let Some(_volume) = mac::bootable_volume(path) {
            return cand(Slot::MacBoot, 100 + if len >= 10 << 20 { 50 } else { 0 }, String::new());
        }
    }
    None
}

/// Every file under `root` that could fill a slot, best first. The walk
/// has no depth limit: two copies from the same volume keep their whole
/// volume-relative path under that volume's folder, however deep, and
/// the CD holds only matched files, so a full walk is cheap.
pub fn find_candidates(root: &Path) -> Vec<Candidate> {
    let files: Vec<(PathBuf, u64)> = WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| Some((e.path().to_path_buf(), e.metadata().ok()?.len())))
        .collect();
    let keys: Vec<PathBuf> = files.iter().map(|(p, _)| p).filter(|p| file_name(p).eq_ignore_ascii_case("rom.key")).cloned().collect();
    let mut out: Vec<Candidate> = files.iter().filter_map(|(p, len)| classify(p, *len, &keys)).collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    out
}

#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CdImport {
    /// "Kickstart ROM: kick40068.A1200".
    pub added: Vec<String>,
    /// Labels of slots the CD had nothing usable for.
    pub still_missing: Vec<String>,
}

/// Fills every empty slot from the files under `root`.
pub fn import_from_dir(library: &Library, root: &Path) -> Result<CdImport, String> {
    let candidates = find_candidates(root);
    let mut report = CdImport::default();
    for slot in missing_slots(library)? {
        let mut these: Vec<&Candidate> = candidates.iter().filter(|c| c.slot == slot).collect();
        if slot == Slot::Workbench {
            // Prefer the Workbench that matches the Kickstart in use.
            let release = library
                .system_file(GuestOs::Amiga, SystemFile::Rom)?
                .and_then(|p| crate::library::read_head(&p, 16).ok().zip(std::fs::metadata(&p).ok()))
                .and_then(|(h, m)| match amiga::identify_kickstart(&h, m.len()) {
                    Some(amiga::Kickstart::Plain { version, .. }) => amiga::workbench_release(version),
                    _ => None,
                });
            if let Some(r) = release {
                these.sort_by_key(|c| (!c.detail.contains(r) && !file_name(&c.path).contains(r), -c.score));
            }
        }
        // A copy that fails the library's own checks (or won't copy) is
        // skipped for the next one.
        let added = these.iter().find(|c| library.set_system_file(slot.os(), slot.kind(), &c.path, c.key.as_deref()).is_ok());
        match added {
            Some(c) => report.added.push(format!("{}: {}", slot.label(), file_name(&c.path))),
            None => report.still_missing.push(slot.label().to_string()),
        }
    }
    Ok(report)
}

/// Reads a files disc: a disc image (mounted read-only for the
/// duration), or a folder (a mounted disc, or its copied contents).
pub fn import_cd(library: &Library, path: &Path) -> Result<CdImport, String> {
    if path.is_dir() {
        return import_from_dir(library, path);
    }
    let mount = Mount::attach(path, &library.run_dir().join(format!("cd-{}", std::process::id())))?;
    import_from_dir(library, &mount.0)
}

/// A disc image attached read-only with `hdiutil`, detached on drop.
struct Mount(PathBuf);

impl Mount {
    #[cfg(target_os = "macos")]
    fn attach(image: &Path, at: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(at).map_err(|e| e.to_string())?;
        let out = std::process::Command::new("/usr/bin/hdiutil")
            .args(["attach", "-readonly", "-nobrowse", "-noverify", "-mountpoint"])
            .arg(at)
            .arg(image)
            .output()
            .map_err(|e| format!("Couldn't open the disc image: {e}"))?;
        if !out.status.success() {
            let _ = std::fs::remove_dir(at);
            return Err(format!(
                "Couldn't open {} as a disc image: {}",
                file_name(image),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(Mount(at.to_path_buf()))
    }

    #[cfg(not(target_os = "macos"))]
    fn attach(_image: &Path, _at: &Path) -> Result<Self, String> {
        Err("Opening a disc image needs macOS for now. Mount it and choose its folder instead.".into())
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("/usr/bin/hdiutil").args(["detach", "-force"]).arg(&self.0).output();
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    fn kickstart(version: u16, valid: bool) -> Vec<u8> {
        let mut r = vec![0u8; 524_288];
        r[..4].copy_from_slice(&[0x11, 0x14, 0x4E, 0xF9]);
        r[12..14].copy_from_slice(&version.to_be_bytes());
        // The checksum long word sits 24 bytes from the end.
        let at = r.len() - 24;
        let mut sum: u32 = 0;
        for w in r.chunks_exact(4) {
            let (s, c) = sum.overflowing_add(u32::from_be_bytes([w[0], w[1], w[2], w[3]]));
            sum = s + c as u32;
        }
        r[at..at + 4].copy_from_slice(&(!sum).to_be_bytes());
        if !valid {
            r[1000] ^= 0xFF;
        }
        r
    }

    fn workbench_adf(volume: &str) -> Vec<u8> {
        let mut adf = vec![0u8; 901_120];
        adf[0..4].copy_from_slice(b"DOS\0");
        let root = 880 * 512;
        adf[root + 432] = volume.len() as u8;
        adf[root + 433..root + 433 + volume.len()].copy_from_slice(volume.as_bytes());
        adf
    }

    fn mac_boot_disk() -> Vec<u8> {
        let mut img = vec![0u8; 800 * 1024];
        img[1024..1026].copy_from_slice(b"BD");
        img[1024 + 92..1024 + 96].copy_from_slice(&2u32.to_be_bytes());
        img
    }

    #[test]
    fn list_names_only_missing_slots() {
        assert_eq!(missing_list(&[]), None);
        let list = missing_list(&[Slot::Kickstart]).unwrap();
        assert!(list.lines().all(|l| l.is_empty() || l.starts_with('#') || !l.contains('\t')));
        assert!(list.contains("\nkick40068.A1200\n"));
        assert!(list.contains("\nrom.key\n"));
        assert!(!list.contains("Quadra"));
        // Every slot has names, and none repeat within a slot.
        for slot in SLOTS {
            let mut names = slot.names().to_vec();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), slot.names().len(), "{slot:?}");
        }

        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        assert_eq!(missing_slots(&lib).unwrap(), SLOTS.to_vec());
    }

    #[test]
    fn cd_fills_empty_slots_with_the_best_copies() {
        let t = TempDir::new();
        let cd = t.path().join("cd");
        // A common layout: unique files at the root, duplicates in a
        // folder per source volume.
        fs::create_dir_all(cd.join("Backup")).unwrap();
        fs::create_dir_all(cd.join("Backup 2")).unwrap();
        fs::write(cd.join("Backup/kick40068.A1200"), kickstart(40, false)).unwrap();
        fs::write(cd.join("Backup 2/kick40068.A1200"), kickstart(40, true)).unwrap();
        fs::write(cd.join("kick34005.A500"), kickstart(34, true)).unwrap();
        fs::write(cd.join("Workbench1.3.adf"), workbench_adf("Workbench1.3")).unwrap();
        fs::write(cd.join("wb31.adf"), workbench_adf("Workbench3.1")).unwrap();
        fs::write(cd.join("Lemmings.adf"), workbench_adf("Lemmings")).unwrap();
        fs::write(cd.join("System 7.dsk"), mac_boot_disk()).unwrap();
        fs::write(cd.join("Quadra650.ROM"), vec![0u8; 1_048_576]).unwrap(); // not a ROM
        fs::write(cd.join("notes.txt"), b"hi").unwrap();

        let lib = Library::new(t.path().join("lib"));
        let report = import_cd(&lib, &cd).unwrap();
        assert_eq!(
            report,
            CdImport {
                added: vec![
                    "Mac startup disk: System 7.dsk".into(),
                    "Kickstart ROM: kick40068.A1200".into(),
                    "Workbench disk: wb31.adf".into(),
                ],
                still_missing: vec!["Mac ROM".into()],
            }
        );
        // The undamaged 3.1 copy won, and set the matching model.
        let kick = lib.system_file(GuestOs::Amiga, SystemFile::Rom).unwrap().unwrap();
        assert!(amiga::carry_sum_ok(&fs::read(kick).unwrap()));
        assert_eq!(lib.system(GuestOs::Amiga).unwrap().model.as_deref(), Some("A1200"));

        // A second import leaves filled slots alone.
        let again = import_cd(&lib, &cd).unwrap();
        assert!(again.added.is_empty());
        assert_eq!(again.still_missing, vec!["Mac ROM".to_string()]);
    }

    #[test]
    fn finds_a_kickstart_eight_levels_deep() {
        let t = TempDir::new();
        let cd = t.path().join("cd");
        // Two copies on one volume keep their volume-relative paths.
        let deep = cd.join("Backup/Users/x/Documents/Amiga Forever/Shared/rom");
        let other = cd.join("Backup/Users/x/Old/Amiga Forever/Shared/rom");
        fs::create_dir_all(&deep).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::write(deep.join("kick.rom"), kickstart(40, true)).unwrap();
        fs::write(other.join("kick.rom"), kickstart(40, false)).unwrap();
        assert_eq!(deep.join("kick.rom").strip_prefix(&cd).unwrap().components().count(), 8);

        let lib = Library::new(t.path().join("lib"));
        let report = import_cd(&lib, &cd).unwrap();
        assert!(report.added.contains(&"Kickstart ROM: kick.rom".to_string()));
        let kick = lib.system_file(GuestOs::Amiga, SystemFile::Rom).unwrap().unwrap();
        assert!(amiga::carry_sum_ok(&fs::read(kick).unwrap()), "the undamaged copy won");
    }

    #[test]
    fn pairs_an_encrypted_rom_with_a_rom_key_in_another_folder() {
        let t = TempDir::new();
        let cd = t.path().join("cd");
        fs::create_dir_all(cd.join("Backup")).unwrap();
        fs::create_dir_all(cd.join("Backup 2")).unwrap();
        let key = b"the right key".to_vec();
        let mut enc = b"AMIROMTYPE1".to_vec();
        enc.extend(kickstart(40, true).iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]));
        // The ROM was found once (root); rom.key on two volumes, one of
        // them from a different Amiga Forever install.
        fs::write(cd.join("amiga-os-310-a1200.rom"), &enc).unwrap();
        fs::write(cd.join("Backup/rom.key"), b"some other key").unwrap();
        fs::write(cd.join("Backup 2/rom.key"), &key).unwrap();

        let lib = Library::new(t.path().join("lib"));
        let report = import_cd(&lib, &cd).unwrap();
        assert!(report.added.contains(&"Kickstart ROM: amiga-os-310-a1200.rom".to_string()), "{report:?}");
        assert_eq!(fs::read(lib.system_dir(GuestOs::Amiga).join("rom.key")).unwrap(), key);
        assert_eq!(lib.system(GuestOs::Amiga).unwrap().model.as_deref(), Some("A1200"));

        // With no key that unlocks it, the ROM isn't offered at all.
        let t2 = TempDir::new();
        fs::write(t2.path().join("amiga-os-310-a1200.rom"), &enc).unwrap();
        fs::write(t2.path().join("rom.key"), b"some other key").unwrap();
        assert!(find_candidates(t2.path()).is_empty());
    }

    #[test]
    fn non_images_are_refused() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let f = t.path().join("not-a-disc.iso");
        fs::write(&f, b"nope").unwrap();
        assert!(import_cd(&lib, &f).is_err());
        assert!(!lib.run_dir().join(format!("cd-{}", std::process::id())).exists());
    }
}
