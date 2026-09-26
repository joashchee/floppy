//! What Floppy remembers about files discs made from its missing-files
//! list, so the next list asks better (cd.rs has the list and disc
//! contract).
//!
//! Diskette's Burn A CD marks its discs two ways: the ISO 9660
//! Application Identifier `DISKETTE BURN A CD` (readable without
//! mounting), and a `diskette-burn.json` manifest at the disc's root.
//! From the manifest Floppy uses:
//!
//! - `id`, new for every disc, to recognize a disc it already imported;
//! - `list.sha1`, the SHA-1 of the list file the disc answers, which
//!   Floppy matches against the lists it saved;
//! - `lines[]` (by line number in that list): `matches` and `ignored` per
//!   line. When every line for a slot matched nothing usable, the user's
//!   drives can't fill that slot, and later lists stop asking for it
//!   until the user says Ask Again.
//!
//! Every file on such a disc that Floppy couldn't use (not a system file,
//! refused by its checks, or damaged) goes on the ignore list. Later
//! lists carry `#ignore: sha256 <hash> <reason>` lines, so the disc maker
//! leaves those copies off. The hash is of the file as it was on the disc
//! (an Amiga Forever ROM's encrypted bytes). Forget Ignored Files clears
//! the list.
//!
//! All of it lives in `library/files-discs.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cd::Slot;
use crate::library::Library;

/// The manifest's file name, at the disc's root.
pub const MANIFEST: &str = "diskette-burn.json";

/// The ISO 9660 Application Identifier Burn A CD writes.
pub const APPLICATION_ID: &str = "DISKETTE BURN A CD";

/// The ISO 9660 Application Identifier of an image file: 128 space-padded
/// bytes at 0x823E (the primary volume descriptor, sector 16, byte 574).
/// `None` when the file isn't an ISO 9660 image or the field is empty.
pub fn iso_application_id(path: &Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let mut pvd = [0u8; 6];
    f.seek(SeekFrom::Start(0x8000)).ok()?;
    f.read_exact(&mut pvd).ok()?;
    if &pvd != b"\x01CD001" {
        return None;
    }
    let mut id = [0u8; 128];
    f.seek(SeekFrom::Start(0x823E)).ok()?;
    f.read_exact(&mut id).ok()?;
    let id = String::from_utf8_lossy(&id).trim().to_string();
    (!id.is_empty()).then_some(id)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub producer: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub list: Option<ManifestList>,
    #[serde(default)]
    pub lines: Vec<ManifestLine>,
}

#[derive(Deserialize, Debug)]
pub struct ManifestList {
    #[serde(default)]
    pub sha1: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLine {
    pub line: usize,
    #[serde(default)]
    pub matches: u64,
    #[serde(default)]
    pub ignored: u64,
}

/// The manifest at `root`, if it has one. Its name is matched ignoring
/// case (some disc formats store names in capitals). A newer `version`
/// only adds fields, so it's read too.
pub fn read_manifest(root: &Path) -> Option<Manifest> {
    let path = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(MANIFEST)))?;
    let m: Manifest = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    (m.format == "diskette-burn" && m.version >= 1).then_some(m)
}

/// A file a files disc brought that Floppy couldn't use.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredFile {
    pub sha256: String,
    pub size: u64,
    /// Its name on the disc.
    pub name: String,
    pub reason: String,
    /// The disc's manifest `id`.
    pub disc: String,
    /// Unix time.
    pub recorded: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct ImportedDisc {
    id: String,
    producer: String,
    created_at: Option<String>,
    imported: u64,
}

/// A list Floppy saved: its SHA-1, and which slot each line asks for.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct SavedList {
    sha1: String,
    saved: u64,
    lines: Vec<(usize, String)>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct NotOnDrives {
    slot: String,
    disc: String,
    checked: u64,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
struct State {
    #[serde(default)]
    discs: Vec<ImportedDisc>,
    #[serde(default)]
    lists: Vec<SavedList>,
    #[serde(default)]
    ignored: Vec<IgnoredFile>,
    #[serde(default)]
    not_on_drives: Vec<NotOnDrives>,
}

/// Saved lists kept for matching discs to. A disc answers a recent list.
const MAX_LISTS: usize = 20;

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn load(library: &Library) -> State {
    std::fs::read(library.files_discs_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save(library: &Library, state: &State) -> Result<(), String> {
    let path = library.files_discs_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("Couldn't save what Floppy knows about files discs: {e}"))
}

pub fn ignored_files(library: &Library) -> Vec<IgnoredFile> {
    load(library).ignored
}

/// Empties the ignore list, so the next disc may bring those files again.
pub fn forget_ignored(library: &Library) -> Result<(), String> {
    let mut state = load(library);
    state.ignored.clear();
    save(library, &state)
}

/// Slots the user's drives couldn't fill, per the last disc that checked.
pub fn not_on_drives(library: &Library) -> Vec<Slot> {
    load(library).not_on_drives.iter().filter_map(|n| Slot::from_label(&n.slot)).collect()
}

/// Puts `slot` back on the missing-files list.
pub fn ask_again(library: &Library, slot: Slot) -> Result<(), String> {
    let mut state = load(library);
    state.not_on_drives.retain(|n| n.slot != slot.label());
    save(library, &state)
}

/// Remembers a list Floppy saved: `text`'s SHA-1 and the slot each line
/// (1-based) asks for, so a disc answering it can be read line by line.
pub fn record_list(library: &Library, text: &str, lines: &[(usize, Slot)]) -> Result<(), String> {
    let mut state = load(library);
    let sha1 = crate::sha1::hex(text.as_bytes());
    state.lists.retain(|l| l.sha1 != sha1);
    state.lists.push(SavedList { sha1, saved: now(), lines: lines.iter().map(|(n, s)| (*n, s.label().to_string())).collect() });
    let excess = state.lists.len().saturating_sub(MAX_LISTS);
    state.lists.drain(..excess);
    save(library, &state)
}

/// What an import learned from a disc's manifest.
#[derive(Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscReport {
    pub producer: String,
    /// This disc was imported before.
    pub already_imported: bool,
    /// Slots the user's drives had nothing usable for, now left off the list.
    pub not_on_drives: Vec<String>,
}

/// Records an import from a disc with a manifest: the disc, the files
/// Floppy couldn't use (onto the ignore list), and the still-missing
/// slots whose every line in the answered list found nothing usable.
pub fn record_import(library: &Library, manifest: &Manifest, unusable: Vec<IgnoredFile>, still_missing: &[Slot]) -> Result<DiscReport, String> {
    let mut state = load(library);
    let already_imported = !manifest.id.is_empty() && state.discs.iter().any(|d| d.id == manifest.id);
    if !already_imported {
        state.discs.push(ImportedDisc {
            id: manifest.id.clone(),
            producer: manifest.producer.clone(),
            created_at: manifest.created_at.clone(),
            imported: now(),
        });
    }
    for f in unusable {
        state.ignored.retain(|o| !(o.sha256 == f.sha256 && o.size == f.size));
        state.ignored.push(f);
    }

    let mut newly = Vec::new();
    let list = manifest.list.as_ref().and_then(|l| state.lists.iter().find(|s| s.sha1.eq_ignore_ascii_case(&l.sha1)));
    if let Some(list) = list {
        for slot in still_missing {
            let lines: Vec<usize> = list.lines.iter().filter(|(_, s)| s == slot.label()).map(|(n, _)| *n).collect();
            // Every line for the slot is in the manifest, and none of them
            // found a copy that wasn't ignored.
            let reported: Vec<&ManifestLine> = lines.iter().filter_map(|n| manifest.lines.iter().find(|l| l.line == *n)).collect();
            let none = !lines.is_empty() && reported.len() == lines.len() && reported.iter().all(|l| l.matches <= l.ignored);
            if none && !state.not_on_drives.iter().any(|n| n.slot == slot.label()) {
                state.not_on_drives.push(NotOnDrives { slot: slot.label().to_string(), disc: manifest.id.clone(), checked: now() });
                newly.push(slot.label().to_string());
            }
        }
    }
    save(library, &state)?;
    Ok(DiscReport { producer: manifest.producer.clone(), already_imported, not_on_drives: newly })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn reads_the_iso_application_id() {
        let t = TempDir::new();
        let iso = t.path().join("disc.iso");
        let mut img = vec![0u8; 0x8800];
        img[0x8000..0x8006].copy_from_slice(b"\x01CD001");
        img[0x823E..0x823E + 128].fill(b' ');
        img[0x823E..0x823E + APPLICATION_ID.len()].copy_from_slice(APPLICATION_ID.as_bytes());
        std::fs::write(&iso, &img).unwrap();
        assert_eq!(iso_application_id(&iso).as_deref(), Some(APPLICATION_ID));
        // Not an ISO: no descriptor.
        img[0x8001] = b'X';
        std::fs::write(&iso, &img).unwrap();
        assert_eq!(iso_application_id(&iso), None);
    }

    #[test]
    fn ask_again_and_forget_undo_what_a_disc_taught() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        record_list(&lib, "list", &[(3, Slot::Kickstart), (4, Slot::Kickstart), (6, Slot::MacRom)]).unwrap();
        let manifest: Manifest = serde_json::from_str(&format!(
            r#"{{"format": "diskette-burn", "version": 2, "producer": "D", "id": "disc-1", "newField": true,
                "list": {{"sha1": "{}"}},
                "lines": [{{"line": 3, "matches": 0}}, {{"line": 4, "matches": 2, "ignored": 2}}, {{"line": 6, "matches": 1}}]}}"#,
            crate::sha1::hex(b"list")
        ))
        .unwrap();
        let ignored = IgnoredFile { sha256: "ab".repeat(32), size: 5, name: "x".into(), reason: "r".into(), disc: "disc-1".into(), recorded: 1 };
        let report = record_import(&lib, &manifest, vec![ignored.clone()], &[Slot::Kickstart, Slot::MacRom]).unwrap();
        // Kickstart: nothing usable on any line. Mac ROM: line 6 found one.
        assert_eq!(report.not_on_drives, vec!["Kickstart ROM".to_string()]);
        assert!(!report.already_imported);
        assert_eq!(not_on_drives(&lib), vec![Slot::Kickstart]);
        assert_eq!(ignored_files(&lib), vec![ignored.clone()]);

        // The same disc again: recognized, nothing duplicated.
        let again = record_import(&lib, &manifest, vec![ignored], &[Slot::Kickstart]).unwrap();
        assert!(again.already_imported && again.not_on_drives.is_empty());
        assert_eq!(ignored_files(&lib).len(), 1);

        ask_again(&lib, Slot::Kickstart).unwrap();
        assert!(not_on_drives(&lib).is_empty());
        forget_ignored(&lib).unwrap();
        assert!(ignored_files(&lib).is_empty());
    }
}
