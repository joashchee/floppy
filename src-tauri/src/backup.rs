//! Backing up Floppy's system: the setup files the user supplied (ROMs,
//! startup disks, Workbench, an Amiga Forever `rom.key`) and the settings
//! that go with them, as one compressed disc image, for restoring on a
//! new Mac or PC, or after reinstalling.
//!
//! The disc is plain ISO 9660 (iso.rs), Application ID
//! `FLOPPY SYSTEM BACKUP`, holding:
//!
//! - `SYSTEM.ZIP`: the files, Deflate-compressed, each under its slot's
//!   name (`Mac ROM/…`, `Kickstart ROM/rom.key`, a Workbench folder whole),
//!   plus `floppy-backup.json`: each file's slot, name, size and SHA-256,
//!   the Amiga model and whether AROS stands in for a Kickstart.
//! - `README.TXT`: what it is and how to restore it, with or without
//!   Floppy.
//!
//! Restoring reads the disc directly (nothing is mounted), checks every
//! file against its SHA-256, and fills only the slots that are empty, so
//! it never replaces what the user has set up since.
//!
//! Apps and documents aren't included: this is the system that makes
//! Floppy work, not the library.
//!
//! Floppy offers to make one ("Burn A CD") once its system is fully
//! working: every guest has every setup file, which is every slot in
//! `cd::SLOTS` (today the Mac ROM and startup disk, the Kickstart and
//! Workbench; a guest added later joins by adding its slots there).
//! AROS doesn't count as a Kickstart. It asks once per set of files:
//! `library/backup.json` remembers the set last backed up or turned down.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::cd::{self, CdImport, Slot, SLOTS};
use crate::iso;
use crate::library::{GuestOs, Library};
#[cfg(test)]
use crate::library::SystemFile;

pub const APPLICATION_ID: &str = "FLOPPY SYSTEM BACKUP";
const FORMAT: &str = "floppy-system-backup";
const MANIFEST: &str = "floppy-backup.json";
const ZIP_NAME: &str = "SYSTEM.ZIP";

/// Whether `path` is a disc Floppy's Backup made.
pub fn is_backup(path: &Path) -> bool {
    path.is_file() && crate::discs::iso_application_id(path).as_deref() == Some(APPLICATION_ID)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    slot: String,
    /// Its name in `library/system/<os>/`, and its folder in the zip.
    name: String,
    /// Set for a Workbench folder.
    #[serde(default)]
    folder: bool,
    /// For a file: its size and SHA-256. A folder has neither.
    size: Option<u64>,
    sha256: Option<String>,
    /// For an encrypted Kickstart: its `rom.key`, backed up beside it.
    #[serde(default)]
    rom_key: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    format: String,
    version: u32,
    floppy_version: String,
    /// Unix seconds.
    created: u64,
    files: Vec<BackupFile>,
    amiga_model: Option<String>,
    #[serde(default)]
    aros: bool,
}

/// `library/backup.json`.
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct State {
    /// The set of files last backed up, and when.
    backed_up: Option<String>,
    last_backup: Option<u64>,
    /// A set the user said Not Now to.
    declined: Option<String>,
}

fn state_path(library: &Library) -> PathBuf {
    library.backup_path()
}

fn load_state(library: &Library) -> State {
    std::fs::read(state_path(library)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save_state(library: &Library, s: &State) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    std::fs::write(state_path(library), json).map_err(|e| format!("Couldn't note the backup: {e}"))
}

/// Whether to offer a backup, and what one would hold.
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Setup files a backup would hold ("Mac ROM", …).
    pub slots: Vec<String>,
    /// Every guest has every setup file (`cd::SLOTS`).
    pub complete: bool,
    /// Offer Burn A CD: complete, and this set wasn't backed up or turned down.
    pub offer: bool,
    /// Unix seconds of the last backup, if any.
    pub last_backup: Option<u64>,
}

/// The set files in each slot, with their paths.
fn present(library: &Library) -> Result<Vec<(Slot, PathBuf)>, String> {
    let mut out = Vec::new();
    for slot in SLOTS {
        if let Some(p) = library.system_file(slot.os(), slot.kind())? {
            if p.exists() {
                out.push((slot, p));
            }
        }
    }
    Ok(out)
}

/// Names, sizes and modification times of the set files: changes when any
/// of them is replaced.
fn fingerprint(files: &[(Slot, PathBuf)]) -> String {
    let mut s = String::new();
    for (slot, p) in files {
        let m = std::fs::metadata(p).ok();
        let mtime = m.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
        s += &format!(
            "{}|{}|{}|{}\n",
            slot.label(),
            p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
            m.map(|m| m.len()).unwrap_or(0),
            mtime.map(|d| d.as_secs()).unwrap_or(0)
        );
    }
    crate::sha1::hex(s.as_bytes())
}

pub fn status(library: &Library) -> Result<Status, String> {
    let files = present(library)?;
    // Every setup file of every guest: `cd::SLOTS` lists them all, so a
    // guest added later counts as soon as its slots are there. AROS
    // doesn't stand in for a Kickstart here.
    let complete = cd::missing_slots(library)?.is_empty();
    let state = load_state(library);
    let print = fingerprint(&files);
    let offer = complete && state.backed_up.as_deref() != Some(&print) && state.declined.as_deref() != Some(&print);
    Ok(Status {
        slots: files.iter().map(|(s, _)| s.label().to_string()).collect(),
        complete,
        offer,
        last_backup: state.last_backup,
    })
}

/// Not Now: don't offer again until the set of files changes.
pub fn decline(library: &Library) -> Result<(), String> {
    let mut state = load_state(library);
    state.declined = Some(fingerprint(&present(library)?));
    save_state(library, &state)
}

/// What a backup held.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Made {
    pub slots: Vec<String>,
    /// The setup files' size, and the disc's.
    pub original_bytes: u64,
    pub disc_bytes: u64,
}

const README: &str = "Floppy system backup

This disc holds the setup files that make Floppy work, as you had them
set up: the Mac ROM and startup disk, the Amiga Kickstart (and its
rom.key) and Workbench, whichever you had, plus the Amiga model. Your
apps and documents aren't on it.

To restore: open this disc image with Floppy, or drop it on Floppy's
window. Floppy checks every file and fills in whatever isn't set up yet.
It never replaces files you've set up since.

Without Floppy: open SYSTEM.ZIP. Each setup file is in a folder named
after what it is. floppy-backup.json lists them with their SHA-256.

These are your own copies of copyrighted system software (except AROS,
which is free). Keep this disc for yourself: don't share it.
";

/// Makes a backup disc of the current system at `dest` (an `.iso`).
pub fn make(library: &Library, dest: &Path) -> Result<Made, String> {
    let files = present(library)?;
    if files.is_empty() {
        return Err("There's nothing to back up yet: add a Mac or Amiga setup file first.".into());
    }
    let scratch = library.run_dir().join(format!("backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    let result = (|| {
        let zip_path = scratch.join(ZIP_NAME);
        let readme = scratch.join("README.TXT");
        let original_bytes = write_zip(library, &files, &zip_path)?;
        std::fs::write(&readme, README).map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let volume = iso::Volume { id: "FLOPPY_BACKUP", application: APPLICATION_ID, publisher: "FLOPPY" };
        // Written beside `dest` first, so a failure never leaves half a disc.
        let tmp = dest.with_extension("iso.part");
        iso::write(&tmp, &volume, &[(ZIP_NAME, &zip_path), ("README.TXT", &readme)], now)
            .and_then(|_| std::fs::rename(&tmp, dest))
            .map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                format!("Couldn't write the backup disc: {e}")
            })?;
        let mut state = load_state(library);
        state.backed_up = Some(fingerprint(&files));
        state.last_backup = Some(now);
        save_state(library, &state)?;
        Ok(Made {
            slots: files.iter().map(|(s, _)| s.label().to_string()).collect(),
            original_bytes,
            disc_bytes: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0),
        })
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

/// Writes the zip, returning how many bytes of setup files went in.
fn write_zip(library: &Library, files: &[(Slot, PathBuf)], out: &Path) -> Result<u64, String> {
    let err = |e: &dyn std::fmt::Display| format!("Couldn't write the backup: {e}");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(out).map_err(|e| err(&e))?);
    let mut manifest = Vec::new();
    let mut total = 0u64;
    let add = |zip: &mut zip::ZipWriter<std::fs::File>, name: &str, src: &Path| -> Result<u64, String> {
        let len = std::fs::metadata(src).map_err(|e| err(&e))?.len();
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(len >= u32::MAX as u64);
        zip.start_file(name, opts).map_err(|e| err(&e))?;
        std::io::copy(&mut std::fs::File::open(src).map_err(|e| err(&e))?, zip).map_err(|e| err(&e))?;
        Ok(len)
    };
    for (slot, path) in files {
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let base = format!("{}/{name}", slot.label());
        if path.is_dir() {
            for e in WalkDir::new(path).min_depth(1).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
                let rel = e.path().strip_prefix(path).map_err(|e| err(&e))?;
                let rel: Vec<String> = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
                total += add(&mut zip, &format!("{base}/{}", rel.join("/")), e.path())?;
            }
            manifest.push(BackupFile { slot: slot.label().into(), name, folder: true, size: None, sha256: None, rom_key: false });
            continue;
        }
        total += add(&mut zip, &base, path)?;
        let key = path.with_file_name("rom.key");
        let rom_key = *slot == Slot::Kickstart && key.is_file();
        if rom_key {
            total += add(&mut zip, &format!("{}/rom.key", slot.label()), &key)?;
        }
        let sha256 = cd::sha256_of(path).map_err(|e| err(&e))?;
        let size = std::fs::metadata(path).map_err(|e| err(&e))?.len();
        manifest.push(BackupFile { slot: slot.label().into(), name, folder: false, size: Some(size), sha256: Some(sha256), rom_key });
    }
    let amiga = library.system(GuestOs::Amiga)?;
    let m = Manifest {
        format: FORMAT.into(),
        version: 1,
        floppy_version: env!("CARGO_PKG_VERSION").into(),
        created: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        files: manifest,
        amiga_model: amiga.model,
        aros: amiga.aros,
    };
    zip.start_file(MANIFEST, zip::write::SimpleFileOptions::default()).map_err(|e| err(&e))?;
    zip.write_all(&serde_json::to_vec_pretty(&m).map_err(|e| err(&e))?).map_err(|e| err(&e))?;
    zip.finish().map_err(|e| err(&e))?;
    Ok(total)
}

/// Restores a backup disc: fills every empty slot from it, after checking
/// each file's SHA-256. Slots already set up are kept as they are.
pub fn restore(library: &Library, disc: &Path) -> Result<CdImport, String> {
    let bad = |e: &dyn std::fmt::Display| format!("Couldn't read the backup disc: {e}");
    let mut zip = zip::ZipArchive::new(iso::open(disc, ZIP_NAME).map_err(|e| bad(&e))?).map_err(|e| bad(&e))?;
    let mut json = String::new();
    zip.by_name(MANIFEST).map_err(|e| bad(&e))?.read_to_string(&mut json).map_err(|e| bad(&e))?;
    let manifest: Manifest = serde_json::from_str(&json).map_err(|e| bad(&e))?;
    if manifest.format != FORMAT {
        return Err("That disc isn't a Floppy system backup.".into());
    }

    let scratch = library.run_dir().join(format!("restore-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let missing = cd::missing_slots(library)?;
    let mut report = CdImport::default();
    let mut restored_kickstart = false;
    let result = (|| {
        for f in &manifest.files {
            let Some(slot) = Slot::from_label(&f.slot) else { continue };
            if !missing.contains(&slot) {
                report.kept.push(f.slot.clone());
                continue;
            }
            // Only this slot's entries, unpacked under their zip names.
            let prefix = format!("{}/{}", f.slot, f.name);
            for i in 0..zip.len() {
                let mut entry = zip.by_index(i).map_err(|e| bad(&e))?;
                let Some(rel) = entry.enclosed_name() else { continue };
                let name = entry.name().to_string();
                let key = f.rom_key && name == format!("{}/rom.key", f.slot);
                if !(name == prefix || name.starts_with(&format!("{prefix}/")) || key) || entry.is_dir() {
                    continue;
                }
                let out = scratch.join(rel);
                std::fs::create_dir_all(out.parent().unwrap_or(&scratch)).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut std::fs::File::create(&out).map_err(|e| e.to_string())?).map_err(|e| bad(&e))?;
            }
            let path = scratch.join(&f.slot).join(&f.name);
            if !f.folder {
                let sha = cd::sha256_of(&path).map_err(|e| bad(&e))?;
                if Some(sha.as_str()) != f.sha256.as_deref() {
                    report.skipped.push(format!("{}: {} is damaged on the disc (its SHA-256 doesn't match)", f.slot, f.name));
                    continue;
                }
            }
            let key = f.rom_key.then(|| scratch.join(&f.slot).join("rom.key"));
            match library.set_system_file(slot.os(), slot.kind(), &path, key.as_deref()) {
                Ok(_) => {
                    report.added.push(format!("{}: {}", f.slot, f.name));
                    restored_kickstart |= slot == Slot::Kickstart;
                }
                Err(e) => report.skipped.push(format!("{}: {e}", f.slot)),
            }
        }
        Ok::<(), String>(())
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result?;

    // The Amiga's settings came with its Kickstart; AROS only matters without one.
    if restored_kickstart {
        if let Some(model) = &manifest.amiga_model {
            let _ = library.set_model(GuestOs::Amiga, model);
        }
    } else if manifest.aros && library.system(GuestOs::Amiga)?.rom.is_none() {
        library.set_aros(GuestOs::Amiga, true)?;
    }
    report.still_missing = cd::missing_slots(library)?.iter().map(|s| s.label().to_string()).collect();
    // When everything set up now came off this disc, it's already backed
    // up: don't offer to burn it again.
    let now = present(library)?;
    let all_restored = now.iter().all(|(s, _)| report.added.iter().any(|a| a.starts_with(&format!("{}: ", s.label()))));
    if !report.added.is_empty() && all_restored {
        let mut state = load_state(library);
        state.backed_up = Some(fingerprint(&now));
        state.last_backup = Some(manifest.created);
        save_state(library, &state)?;
    }
    Ok(report)
}

/// The kind of system file in each slot, for tests.
#[cfg(test)]
fn slot_file(library: &Library, slot: Slot) -> Option<PathBuf> {
    library.system_file(slot.os(), slot.kind()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    fn mac_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 524_288];
        rom[..4].copy_from_slice(&[0x42, 0x1E, 0xF4, 0x8B]);
        rom[8..10].copy_from_slice(&[0x06, 0x7C]);
        rom[1000] = 0x5A;
        rom
    }

    fn mac_disk() -> Vec<u8> {
        let mut img = vec![0u8; 800 * 1024];
        img[1024..1026].copy_from_slice(b"BD");
        img[1024 + 92..1024 + 96].copy_from_slice(&2u32.to_be_bytes());
        img
    }

    fn kick13() -> Vec<u8> {
        let mut ks = vec![0u8; 262_144];
        ks[..4].copy_from_slice(&[0x11, 0x11, 0x4E, 0xF9]);
        ks[12..16].copy_from_slice(&[0, 34, 0, 5]);
        ks
    }

    fn set(lib: &Library, t: &Path, os: GuestOs, kind: SystemFile, name: &str, bytes: &[u8]) {
        let p = t.join(name);
        fs::write(&p, bytes).unwrap();
        lib.set_system_file(os, kind, &p, None).unwrap();
    }

    #[test]
    fn offers_once_the_system_is_complete_and_once_per_set() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        assert_eq!(status(&lib).unwrap(), Status::default());
        // A whole Mac isn't a whole system while the Amiga lacks anything.
        set(&lib, t.path(), GuestOs::MacClassic, SystemFile::Rom, "q650.rom", &mac_rom());
        set(&lib, t.path(), GuestOs::MacClassic, SystemFile::Boot, "System 7.5.3.img", &mac_disk());
        assert!(!status(&lib).unwrap().complete);
        // Nor does AROS stand in for a Kickstart.
        lib.set_aros(GuestOs::Amiga, true).unwrap();
        assert!(!status(&lib).unwrap().complete);
        set(&lib, t.path(), GuestOs::Amiga, SystemFile::Rom, "kick13.rom", &kick13());
        assert!(!status(&lib).unwrap().complete, "Workbench is a setup file too");
        let wb = t.path().join("WB");
        fs::create_dir_all(&wb).unwrap();
        fs::write(wb.join("Disk.info"), b"x").unwrap();
        lib.set_system_file(GuestOs::Amiga, SystemFile::Boot, &wb, None).unwrap();
        let s = status(&lib).unwrap();
        assert!(s.complete && s.offer, "{s:?}");
        decline(&lib).unwrap();
        assert!(!status(&lib).unwrap().offer, "Not Now holds for this set");
        // A replaced file makes a new set.
        let mut other = kick13();
        other[9] = 1;
        set(&lib, t.path(), GuestOs::Amiga, SystemFile::Rom, "kick13b.rom", &other);
        assert!(status(&lib).unwrap().offer);
        make(&lib, &t.path().join("b.iso")).unwrap();
        let s = status(&lib).unwrap();
        assert!(!s.offer && s.last_backup.is_some());
    }

    #[test]
    fn a_backup_restores_into_a_fresh_library_and_keeps_whats_there() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        set(&lib, t.path(), GuestOs::MacClassic, SystemFile::Rom, "q650.rom", &mac_rom());
        set(&lib, t.path(), GuestOs::MacClassic, SystemFile::Boot, "System 7.5.3.img", &mac_disk());
        set(&lib, t.path(), GuestOs::Amiga, SystemFile::Rom, "kick13.rom", &kick13());
        lib.set_model(GuestOs::Amiga, "A600").unwrap();
        let wb = t.path().join("WB");
        fs::create_dir_all(wb.join("C")).unwrap();
        fs::write(wb.join("C/Dir"), b"hunk").unwrap();
        lib.set_system_file(GuestOs::Amiga, SystemFile::Boot, &wb, None).unwrap();

        let iso = t.path().join("Floppy backup.iso");
        let made = make(&lib, &iso).unwrap();
        assert_eq!(made.slots.len(), 4);
        assert!(made.disc_bytes < made.original_bytes, "compressed: {made:?}");
        assert!(is_backup(&iso));

        // A new install, with its own Kickstart already.
        let fresh = Library::new(t.path().join("fresh"));
        let mut other = kick13();
        other[500] = 1;
        set(&fresh, t.path(), GuestOs::Amiga, SystemFile::Rom, "mine.rom", &other);
        let r = restore(&fresh, &iso).unwrap();
        assert_eq!(r.added.len(), 3, "{r:?}");
        assert_eq!(r.kept, vec!["Kickstart ROM"]);
        assert!(r.skipped.is_empty() && r.still_missing.is_empty(), "{r:?}");
        let rom = slot_file(&fresh, Slot::MacRom).unwrap();
        assert_eq!(fs::read(rom).unwrap(), mac_rom());
        assert!(slot_file(&fresh, Slot::Workbench).unwrap().join("C/Dir").is_file());
        assert_eq!(fs::read(slot_file(&fresh, Slot::Kickstart).unwrap()).unwrap(), other, "kept the user's own");
        // Its own Kickstart isn't on any backup, so a backup is still worth offering.
        assert!(status(&fresh).unwrap().offer);
        // A library restored whole is backed up already.
        let whole = Library::new(t.path().join("whole"));
        restore(&whole, &iso).unwrap();
        let s = status(&whole).unwrap();
        assert!(s.complete && !s.offer && s.last_backup.is_some(), "{s:?}");
    }

    #[test]
    fn a_backup_in_downloads_restores_too() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        set(&lib, t.path(), GuestOs::Amiga, SystemFile::Rom, "kick13.rom", &kick13());
        let dl = t.path().join("Downloads");
        fs::create_dir_all(&dl).unwrap();
        make(&lib, &dl.join("Floppy System Backup.iso")).unwrap();
        let fresh = Library::new(t.path().join("fresh"));
        let r = cd::import_from_downloads(&fresh, &dl).unwrap();
        assert_eq!(r.added, vec!["Kickstart ROM: kick13.rom"]);
    }

    #[test]
    fn a_damaged_file_is_refused_not_restored() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        set(&lib, t.path(), GuestOs::Amiga, SystemFile::Rom, "kick13.rom", &kick13());
        let iso = t.path().join("b.iso");
        make(&lib, &iso).unwrap();
        // Flip a byte inside the stored zip: the zip's own CRC or the
        // SHA-256 check refuses it.
        let mut bytes = fs::read(&iso).unwrap();
        let at = 21 * 2048 + 200;
        bytes[at] ^= 0xFF;
        fs::write(&iso, bytes).unwrap();
        let fresh = Library::new(t.path().join("fresh"));
        let outcome = restore(&fresh, &iso);
        assert!(slot_file(&fresh, Slot::Kickstart).is_none(), "{outcome:?}");
    }
}
