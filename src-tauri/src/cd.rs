//! The missing-files list and files disc. Floppy can't fetch guest system
//! files (rule 3), but the user may already own copies somewhere on their
//! drives. This is a plain-file contract any tool can take part in
//! (Diskette's Burn A CD is one):
//!
//! 1. **Missing-files list** (`missing_list`): a `.txt` of the system files
//!    Floppy still needs. Its first line is a column header, then one
//!    file per line, `#` comment lines, blank lines ignored:
//!
//!    ```text
//!    #columns: name size sha1
//!    Quadra650.ROM
//!    # Quadra 950 (3DC27823)
//!    <tab>1048576<tab>d61dba4a2d2cf9048244b713eaa294100063658d
//!    ```
//!
//!    - A **name line** is just a file name, the one a file is commonly
//!      stored under, matched ignoring case. It has no tabs.
//!    - A **content line** has an empty name, then the size in bytes and
//!      SHA-1 of a known-good copy (`known_files.rs`), tab-separated. It
//!      finds that copy whatever it's called: a renamed file can't be
//!      found by name. Every slot has them, from published hash lists
//!      (`scripts/update-known-hashes.py`).
//!
//!    The header starts with `#`, so a reader that only knows plain name
//!    lists treats it as a comment. Name lines still work there, and the
//!    content lines read as odd names that match nothing.
//! 2. **Files disc**: a disc image (ISO and similar) or a folder holding
//!    copies of those files, laid out any way at all, at any depth, with
//!    duplicates allowed. A tool may place each name independently, so a
//!    ROM and its `rom.key` can end up in different folders.
//!    A disc can carry a manifest saying which list it answers and how
//!    each line went (Diskette's Burn A CD writes `diskette-burn.json`;
//!    discs.rs reads it).
//! 3. `import_cd` reads the disc, recognizes each file by its contents
//!    rather than its name, and fills every empty slot with the best
//!    copy, trying the next copy if one is damaged.
//!
//! 4. **Ignore lines.** From a disc with a manifest, Floppy remembers
//!    every file it couldn't use: not a system file, refused by its
//!    checks, or damaged (a failing checksum). Good copies that just
//!    weren't needed don't count. Later lists carry one line per file,
//!    so the disc maker leaves that content off whichever line it
//!    matches. Old readers see a comment:
//!
//!    ```text
//!    #ignore: sha256 <hash> <reason>
//!    ```
//!
//!    A slot whose every line found nothing usable on the user's drives
//!    is left off later lists until the user says Ask Again (discs.rs).
//!
//! Only these files (the list, the disc and its manifest) cross between
//! programs (rule 2).

use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::discs::{self, DiscReport, IgnoredFile};
use crate::known_files::{self, KnownFile};
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

    pub fn from_label(label: &str) -> Option<Slot> {
        SLOTS.into_iter().find(|s| s.label() == label)
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

    /// Known-good copies this slot can be found by, for the content lines.
    /// A startup disk or Workbench changes once it's used, but the user's
    /// original download or disk image still matches.
    fn known(self) -> &'static [KnownFile] {
        match self {
            Slot::MacRom => known_files::MAC_ROMS,
            Slot::MacBoot => known_files::MAC_BOOT_DISKS,
            Slot::Kickstart => known_files::KICKSTARTS,
            Slot::Workbench => known_files::WORKBENCH_DISKS,
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

/// A missing-files list, and which slot each of its lines (1-based) asks
/// for.
pub struct MissingList {
    pub text: String,
    pub lines: Vec<(usize, Slot)>,
}

/// The missing-files list (see the module doc) for `slots`, ending with
/// an `#ignore:` line per file in `ignored`. `None` when `slots` is empty.
pub fn missing_list(slots: &[Slot], ignored: &[IgnoredFile]) -> Option<MissingList> {
    if slots.is_empty() {
        return None;
    }
    // The header must be the first line.
    let mut out: Vec<String> = vec![
        "#columns: name size sha1".into(),
        "# Files Floppy's emulators still need. Gather copies into a disc".into(),
        "# image or folder (Diskette's Burn A CD can do this from its".into(),
        "# catalog), then use Import Files Disc in Floppy.".into(),
        "# Floppy checks each file's contents, so extra matches do no harm.".into(),
        "# Lines starting with a tab have no name: they find a known-good".into(),
        "# copy by its size and SHA-1, whatever the file is called.".into(),
    ];
    let mut lines = Vec::new();
    for slot in slots {
        out.push(String::new());
        out.push(format!("# {}", slot.comment()));
        for name in slot.names() {
            out.push(name.to_string());
            lines.push((out.len(), *slot));
        }
        out.push("# Known-good copies, found by content under any name:".into());
        for f in slot.known() {
            out.push(format!("# {}", f.label));
            out.push(format!("\t{}\t{}", f.size, f.sha1));
            lines.push((out.len(), *slot));
        }
    }
    if !ignored.is_empty() {
        out.push(String::new());
        out.push("# Copies an earlier files disc brought that Floppy couldn't use.".into());
        out.push("# Leave that content off the next disc, whatever it's called.".into());
        for f in ignored {
            out.push(format!("#ignore: sha256 {} {}", f.sha256, f.reason.replace(['\n', '\r', '\t'], " ")));
        }
    }
    let mut text = out.join("\n");
    text.push('\n');
    Some(MissingList { text, lines })
}

/// Writes the missing-files list to `path`, leaving off slots the user's
/// drives couldn't fill (discs.rs), and remembers it for matching discs.
/// Returns how many slots it asks for: 0 when nothing is missing.
pub fn write_list(library: &Library, path: &Path) -> Result<usize, String> {
    let missing = missing_slots(library)?;
    let unfindable = discs::not_on_drives(library);
    let slots: Vec<Slot> = missing.iter().copied().filter(|s| !unfindable.contains(s)).collect();
    if slots.is_empty() && !missing.is_empty() {
        let labels: Vec<&str> = missing.iter().map(|s| s.label()).collect();
        return Err(format!(
            "Your drives had no usable copy of what's still missing ({}). Add it another way, or use Ask Again to put it back on the list.",
            labels.join(", ")
        ));
    }
    let Some(list) = missing_list(&slots, &discs::ignored_files(library)) else { return Ok(0) };
    std::fs::write(path, &list.text).map_err(|e| format!("Couldn't save the list: {e}"))?;
    discs::record_list(library, &list.text, &list.lines)?;
    Ok(slots.len())
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
    /// Its checksum fails. (Only where the checksum is known to be right:
    /// Kickstarts, not Mac ROMs.)
    pub damaged: bool,
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
    let cand = |slot, score, detail: String| Some(Candidate { slot, path: path.to_path_buf(), score, detail, key: None, damaged: false });

    if mac::is_basilisk_rom(&head, len) {
        let ok = std::fs::read(path).is_ok_and(|b| mac::rom_checksum_ok(&b));
        return cand(Slot::MacRom, 100 + if ok { 50 } else { 0 }, String::new());
    }
    match amiga::identify_kickstart(&head, len) {
        Some(amiga::Kickstart::Plain { version, .. }) => {
            let ok = std::fs::read(path).is_ok_and(|b| amiga::carry_sum_ok(&b));
            // Newer Kickstarts run more software.
            return Some(Candidate {
                damaged: !ok,
                ..cand(Slot::Kickstart, 100 + if ok { 50 } else { 0 } + i64::from(version), version.to_string())?
            });
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
                    damaged: !ok,
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
                damaged: false,
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

/// Every file under `root`, with its size. No depth limit: two copies
/// from the same volume keep their whole volume-relative path under that
/// volume's folder, however deep, and a files disc holds only matched
/// files, so a full walk is cheap. Hidden files (AppleDouble `._` files
/// included) are skipped.
fn files_under(root: &Path) -> Vec<(PathBuf, u64)> {
    WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && !e.file_name().to_string_lossy().starts_with('.'))
        .filter(|e| !e.file_name().to_string_lossy().eq_ignore_ascii_case(discs::MANIFEST))
        .filter_map(|e| Some((e.path().to_path_buf(), e.metadata().ok()?.len())))
        .collect()
}

/// The files that could fill a slot, best first. Every `rom.key` among
/// them is tried against every encrypted ROM.
fn candidates_in(files: &[(PathBuf, u64)]) -> Vec<Candidate> {
    let keys: Vec<PathBuf> = files.iter().map(|(p, _)| p).filter(|p| file_name(p).eq_ignore_ascii_case("rom.key")).cloned().collect();
    let mut out: Vec<Candidate> = files.iter().filter_map(|(p, len)| classify(p, *len, &keys)).collect();
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    out
}

/// Every file under `root` that could fill a slot, best first.
#[cfg(test)]
pub fn find_candidates(root: &Path) -> Vec<Candidate> {
    candidates_in(&files_under(root))
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CdImport {
    /// "Kickstart ROM: kick40068.A1200".
    pub added: Vec<String>,
    /// Labels of slots the CD had nothing usable for.
    pub still_missing: Vec<String>,
    /// Dropped zips and disc images that couldn't be opened, with why.
    pub skipped: Vec<String>,
    /// Files on a disc with a manifest that Floppy couldn't use ("name:
    /// reason"), now on the ignore list.
    pub unusable: Vec<String>,
    /// What the disc's manifest told Floppy, when it had one.
    pub disc: Option<DiscReport>,
}

/// Fills every empty slot from the files under `root`. When `root` has a
/// disc manifest, what couldn't be used goes on the skip list.
pub fn import_from_dir(library: &Library, root: &Path) -> Result<CdImport, String> {
    let files = files_under(root);
    let candidates = candidates_in(&files);
    let filled = fill_slots(library, &candidates)?;
    let mut report = filled.report.clone();
    if let Some(manifest) = discs::read_manifest(root) {
        track_disc(library, root, &manifest, &files, &candidates, &filled, &mut report)?;
    }
    Ok(report)
}

/// What filling the slots did with each candidate.
#[derive(Default)]
struct Filled {
    report: CdImport,
    /// The copies that filled a slot.
    used: Vec<PathBuf>,
    /// Copies the library refused, with why.
    failed: Vec<(PathBuf, String)>,
}

/// Fills every empty slot with the best of `candidates`.
fn fill_slots(library: &Library, candidates: &[Candidate]) -> Result<Filled, String> {
    let mut out = Filled::default();
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
        let mut added = None;
        for c in these {
            match library.set_system_file(slot.os(), slot.kind(), &c.path, c.key.as_deref()) {
                Ok(_) => {
                    added = Some(c);
                    break;
                }
                Err(e) => out.failed.push((c.path.clone(), e)),
            }
        }
        match added {
            Some(c) => {
                out.report.added.push(format!("{}: {}", slot.label(), file_name(&c.path)));
                out.used.push(c.path.clone());
            }
            None => out.report.still_missing.push(slot.label().to_string()),
        }
    }
    Ok(out)
}


fn sha256_of(path: &Path) -> std::io::Result<String> {
    use sha2::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Every file under `root` that couldn't be used, hashed for the ignore
/// list. A file counts when it isn't a system file, the library refused
/// it, or it's damaged. A good copy that just wasn't needed doesn't, and
/// neither do `rom.key` files, which are only ever companions.
fn unusable_under(root: &Path, disc: &str, files: &[(PathBuf, u64)], candidates: &[Candidate], filled: &Filled) -> Vec<IgnoredFile> {
    let recorded = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let mut out = Vec::new();
    for (path, size) in files.iter().filter(|(p, _)| p.starts_with(root)) {
        let name = file_name(path);
        if name.eq_ignore_ascii_case("rom.key") || filled.used.contains(path) {
            continue;
        }
        let reason = if let Some((_, e)) = filled.failed.iter().find(|(p, _)| p == path) {
            e.clone()
        } else {
            match candidates.iter().find(|c| c.path == *path) {
                None => "Not a system file Floppy recognizes.".to_string(),
                Some(c) if c.damaged => format!("A damaged {}: its checksum doesn't match.", c.slot.label()),
                Some(_) => continue,
            }
        };
        let Ok(sha256) = sha256_of(path) else { continue };
        out.push(IgnoredFile { sha256, size: *size, name, reason, disc: disc.to_string(), recorded });
    }
    out
}

/// Records what a disc with a manifest taught (discs.rs) into `report`.
fn track_disc(
    library: &Library,
    root: &Path,
    manifest: &discs::Manifest,
    files: &[(PathBuf, u64)],
    candidates: &[Candidate],
    filled: &Filled,
    report: &mut CdImport,
) -> Result<(), String> {
    let unusable = unusable_under(root, &manifest.id, files, candidates, filled);
    report.unusable.extend(unusable.iter().map(|f| format!("{}: {}", f.name, f.reason)));
    let still_missing: Vec<Slot> = report.still_missing.iter().filter_map(|l| Slot::from_label(l)).collect();
    report.disc = Some(discs::record_import(library, manifest, unusable, &still_missing)?);
    Ok(())
}

/// Disc images a dropped file may be, when it isn't a system file itself.
const DISC_IMAGE_EXTS: [&str; 4] = ["iso", "cdr", "dmg", "toast"];

/// Removes a temporary folder, and everything in it, when dropped.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Fills empty slots from whatever the user dropped or picked, in any
/// mix: system files themselves (under any name), folders of them at any
/// depth, zips, and disc images. Each file is judged by its contents
/// first, so a Mac startup disk saved as `.dmg` or `.iso` is used as it
/// is. A disc image that isn't a system file itself is opened read-only
/// and searched. Zips are unpacked into a temporary folder under
/// `library/run/`, removed afterwards. A dropped encrypted Amiga Forever
/// ROM brings the `rom.key` beside it.
pub fn import_dropped(library: &Library, paths: &[PathBuf]) -> Result<CdImport, String> {
    let scratch = Scratch(library.run_dir().join(format!("setup-{}", std::process::id())));
    let _ = std::fs::remove_dir_all(&scratch.0);
    // Declared after `scratch`, so disc images detach before it's removed.
    let mut mounts: Vec<Mount> = Vec::new();
    let mut files: Vec<(PathBuf, u64)> = Vec::new();
    let mut skipped = Vec::new();
    // Folders and discs that came with a manifest (discs.rs).
    let mut tracked: Vec<(PathBuf, discs::Manifest)> = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        if path.is_dir() {
            files.extend(files_under(path));
            if let Some(m) = discs::read_manifest(path) {
                tracked.push((path.clone(), m));
            }
            continue;
        }
        let Ok(meta) = std::fs::metadata(path) else { continue };
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext == "zip" {
            let dest = scratch.0.join(format!("zip-{i}"));
            match crate::library::extract_zip(path, &dest, false) {
                Ok(()) => files.extend(files_under(&dest)),
                Err(e) => skipped.push(format!("Couldn't unpack {}: {e}", file_name(path))),
            }
            continue;
        }
        let this = (path.clone(), meta.len());
        // A Burn A CD disc goes straight to being opened: it's never a
        // system file itself.
        let files_disc = discs::iso_application_id(path).as_deref() == Some(discs::APPLICATION_ID);
        if DISC_IMAGE_EXTS.contains(&ext.as_str()) && (files_disc || candidates_in(std::slice::from_ref(&this)).is_empty()) {
            match Mount::attach(path, &scratch.0.join(format!("disc-{i}"))) {
                Ok(m) => {
                    files.extend(files_under(&m.0));
                    if let Some(manifest) = discs::read_manifest(&m.0) {
                        tracked.push((m.0.clone(), manifest));
                    }
                    mounts.push(m);
                }
                Err(e) => skipped.push(e),
            }
            continue;
        }
        files.push(this);
        let key = path.with_file_name("rom.key");
        if !file_name(path).eq_ignore_ascii_case("rom.key") && key.is_file() && !files.iter().any(|(p, _)| *p == key) {
            let len = std::fs::metadata(&key).map(|m| m.len()).unwrap_or(0);
            files.push((key, len));
        }
    }
    let candidates = candidates_in(&files);
    let filled = fill_slots(library, &candidates)?;
    let mut report = filled.report.clone();
    report.skipped = skipped;
    for (root, manifest) in &tracked {
        track_disc(library, root, manifest, &files, &candidates, &filled, &mut report)?;
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

/// Runs `f` on a disc's root: a folder as it is, or a disc image
/// attached read-only for the duration.
pub fn with_disc<T>(library: &Library, path: &Path, f: impl FnOnce(&Path) -> Result<T, String>) -> Result<T, String> {
    if path.is_dir() {
        return f(path);
    }
    let mount = Mount::attach(path, &library.run_dir().join(format!("disc-{}", std::process::id())))?;
    f(&mount.0)
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
        assert!(missing_list(&[], &[]).is_none());
        let list = missing_list(&[Slot::Kickstart], &[]).unwrap().text;
        assert!(list.starts_with("#columns: name size sha1\n"));
        assert!(list.contains("\nkick40068.A1200\n"));
        assert!(list.contains("\nrom.key\n"));
        assert!(!list.contains("Quadra"));
        assert!(list.contains("\n# Kickstart v3.1 (A1200)\n\t524288\te21545723fe8374e91342617604f1b3d703094f1\n"));
        // Name lines have no tabs, so name-only readers still match them.
        // Content lines are an empty name, a size and a 40-digit SHA-1.
        let mut content = 0;
        for line in list.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let cells: Vec<&str> = line.split('\t').collect();
            match cells[..] {
                [name] => assert!(!name.trim().is_empty() && name.trim() == name, "{line:?}"),
                ["", size, sha1] => {
                    assert!(matches!(size, "262144" | "524288" | "262155" | "524299"), "{line:?}");
                    assert!(sha1.len() == 40 && sha1.bytes().all(|b| b.is_ascii_hexdigit()), "{line:?}");
                    content += 1;
                }
                _ => panic!("bad line {line:?}"),
            }
        }
        assert_eq!(content, known_files::KICKSTARTS.len());
        let mac = missing_list(&[Slot::MacRom, Slot::MacBoot], &[]).unwrap().text;
        assert!(mac.contains("\n\t1048576\tf2a9ce387019bf272c6e3459d961b30f28942ac5\n"));
        assert_eq!(mac.matches("\n\t").count(), known_files::MAC_ROMS.len() + known_files::MAC_BOOT_DISKS.len());
        // System 7.5.3's installed disk, and a Workbench boot disk, are found by content too.
        assert!(mac.contains("\n# System 7.5.3, installed (25 MB)\n\t26214400\tda2239b83e572d7f594d1b7af050f5bc3f6fae84\n"));
        let wb = missing_list(&[Slot::Workbench], &[]).unwrap().text;
        assert!(wb.contains("\t901120\t486ea9520e60051c20ec01329d5a0fe3bb10b4fc\n"), "Workbench 3.1 [!]");
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
                skipped: vec![],
                unusable: vec![],
                disc: None,
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

    fn mac_rom() -> Vec<u8> {
        let mut rom = vec![0u8; 524_288];
        rom[8..10].copy_from_slice(&[0x06, 0x7C]);
        rom
    }

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in files {
            z.start_file(*name, opts).unwrap();
            std::io::Write::write_all(&mut z, data).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn dropped_files_folders_and_zips_fill_slots_together() {
        let t = TempDir::new();
        let src = t.path().join("src");
        fs::create_dir_all(src.join("Amiga stuff/deep")).unwrap();
        // A single file with a name no list would guess.
        fs::write(src.join("my quadra (copy).bin"), mac_rom()).unwrap();
        // A folder holding a Kickstart a few levels down.
        fs::write(src.join("Amiga stuff/deep/kick.rom"), kickstart(40, true)).unwrap();
        // A zip holding a Mac startup disk and Workbench, plus Mac clutter.
        let zip = src.join("system.zip");
        write_zip(&zip, &[
            ("disks/System 7.5.3.img", &mac_boot_disk()),
            ("disks/wb31.adf", &workbench_adf("Workbench3.1")),
            ("__MACOSX/disks/._wb31.adf", b"junk"),
        ]);

        let lib = Library::new(t.path().join("lib"));
        let paths = [src.join("my quadra (copy).bin"), src.join("Amiga stuff"), zip.clone()];
        let report = import_dropped(&lib, &paths).unwrap();
        assert_eq!(
            report.added,
            vec![
                "Mac ROM: my quadra (copy).bin".to_string(),
                "Mac startup disk: System 7.5.3.img".to_string(),
                "Kickstart ROM: kick.rom".to_string(),
                "Workbench disk: wb31.adf".to_string(),
            ]
        );
        assert!(report.still_missing.is_empty() && report.skipped.is_empty(), "{report:?}");
        assert!(missing_slots(&lib).unwrap().is_empty());
        // The unpacked zip is cleaned up, and the sources are untouched.
        assert!(fs::read_dir(lib.run_dir()).map_or(true, |mut d| d.next().is_none()));
        assert!(zip.exists() && src.join("my quadra (copy).bin").exists());
    }

    #[test]
    fn a_dropped_encrypted_rom_finds_the_rom_key_beside_it() {
        let t = TempDir::new();
        let key = b"the right key".to_vec();
        let mut enc = b"AMIROMTYPE1".to_vec();
        enc.extend(kickstart(40, true).iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]));
        fs::write(t.path().join("amiga-os-310-a1200.rom"), &enc).unwrap();
        fs::write(t.path().join("rom.key"), &key).unwrap();

        let lib = Library::new(t.path().join("lib"));
        let report = import_dropped(&lib, &[t.path().join("amiga-os-310-a1200.rom")]).unwrap();
        assert_eq!(report.added, vec!["Kickstart ROM: amiga-os-310-a1200.rom".to_string()]);
    }

    #[test]
    fn dropping_nothing_useful_changes_nothing() {
        let t = TempDir::new();
        let notes = t.path().join("notes.txt");
        fs::write(&notes, b"hi").unwrap();
        let bad_zip = t.path().join("broken.zip");
        fs::write(&bad_zip, b"not a zip").unwrap();
        let lib = Library::new(t.path().join("lib"));
        let report = import_dropped(&lib, &[notes, bad_zip]).unwrap();
        assert!(report.added.is_empty());
        assert_eq!(report.still_missing.len(), SLOTS.len());
        assert_eq!(report.skipped.len(), 1, "{report:?}");
        assert!(report.skipped[0].contains("broken.zip"));
    }

    fn sha256_hex(data: &[u8]) -> String {
        use sha2::Digest;
        sha2::Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn a_burn_disc_teaches_the_next_list() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        // The list the disc answers, as Floppy saved it.
        let list_path = t.path().join("Floppy needs.txt");
        assert_eq!(write_list(&lib, &list_path).unwrap(), SLOTS.len());
        let saved = fs::read_to_string(&list_path).unwrap();
        let line_of = |needle: &str| saved.lines().position(|l| l == needle).unwrap() + 1;
        let kick_lines: Vec<usize> = missing_list(&[Slot::Kickstart], &[]).unwrap().lines.iter().map(|(n, _)| *n).collect();
        assert!(!kick_lines.is_empty());

        let cd = t.path().join("cd");
        fs::create_dir_all(cd.join("Backup")).unwrap();
        let junk = b"not a ROM at all".to_vec();
        fs::write(cd.join("Quadra650.ROM"), &junk).unwrap(); // matched by name only
        fs::write(cd.join("wb31.adf"), workbench_adf("Workbench3.1")).unwrap(); // used
        fs::write(cd.join("Backup/wb31.adf"), workbench_adf("Workbench3.1")).unwrap(); // fine, not needed
        // A damaged copy beside a good one: the good one is used. (A damaged
        // copy that's the only one is still used, and isn't ignored.)
        let damaged = kickstart(40, false);
        fs::write(cd.join("Backup/kick40068.A1200"), &damaged).unwrap();
        fs::write(cd.join("kick40068.A1200"), kickstart(40, true)).unwrap();
        // Every Kickstart line matched nothing, except the damaged copy's
        // name line, which Floppy then can't use either.
        let kick_name_line = line_of("kick40068.A1200");
        let lines: Vec<String> = saved
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.starts_with('#') && !l.is_empty())
            .map(|(i, _)| {
                let n = i + 1;
                let matches = if n == kick_name_line || n == line_of("Quadra650.ROM") || n == line_of("wb31.adf") { 1 } else { 0 };
                format!(r#"{{"line": {n}, "matches": {matches}, "onDisc": {matches}, "ignored": 0}}"#)
            })
            .collect();
        fs::write(
            cd.join("DISKETTE-BURN.JSON"), // some disc formats store names in capitals
            format!(
                r#"{{"format": "diskette-burn", "version": 1, "producer": "Diskette 0.10.0", "id": "disc-1",
                    "list": {{"name": "Floppy needs.txt", "sha1": "{}"}}, "lines": [{}], "files": [], "leftOut": []}}"#,
                crate::sha1::hex(saved.as_bytes()),
                lines.join(",")
            ),
        )
        .unwrap();

        let report = import_cd(&lib, &cd).unwrap();
        assert_eq!(report.added, vec!["Kickstart ROM: kick40068.A1200".to_string(), "Workbench disk: wb31.adf".to_string()]);
        // The manifest itself isn't a file Floppy "couldn't use".
        assert_eq!(report.unusable.len(), 2, "{report:?}");
        let disc = report.disc.clone().unwrap();
        assert_eq!(disc.producer, "Diskette 0.10.0");
        assert!(!disc.already_imported);
        // Mac ROM: its name line matched (junk), so the drives had something:
        // the ignore list's job now. Startup disk: nothing on any line.
        assert_eq!(disc.not_on_drives, vec!["Mac startup disk".to_string()], "{disc:?}");

        // The next list ignores both bad copies by content, and leaves off
        // the startup disk the drives don't have.
        let next = t.path().join("next.txt");
        write_list(&lib, &next).unwrap();
        let next = fs::read_to_string(&next).unwrap();
        assert!(next.starts_with("#columns: name size sha1\n"));
        assert!(next.contains(&format!("\n#ignore: sha256 {} Not a system file Floppy recognizes.\n", sha256_hex(&junk))), "{next}");
        assert!(next.contains(&format!("\n#ignore: sha256 {} A damaged Kickstart ROM", sha256_hex(&damaged))));
        assert!(!next.contains("System 7.5.3.img"));
        assert!(next.contains("Quadra650.ROM"));

        // Importing the same disc again is recognized, and adds nothing.
        let again = import_cd(&lib, &cd).unwrap();
        assert!(again.disc.unwrap().already_imported);
        assert_eq!(discs::ignored_files(&lib).len(), 2);

        // Ask Again puts the startup disk back.
        discs::ask_again(&lib, Slot::MacBoot).unwrap();
        write_list(&lib, &t.path().join("again.txt")).unwrap();
        assert!(fs::read_to_string(t.path().join("again.txt")).unwrap().contains("System 7.5.3.img"));
    }

    #[test]
    fn a_disc_without_a_manifest_isnt_tracked() {
        let t = TempDir::new();
        let cd = t.path().join("cd");
        fs::create_dir_all(&cd).unwrap();
        fs::write(cd.join("Quadra650.ROM"), b"junk").unwrap();
        let lib = Library::new(t.path().join("lib"));
        let report = import_cd(&lib, &cd).unwrap();
        assert!(report.unusable.is_empty() && report.disc.is_none());
        // A manifest in some other format doesn't count either.
        fs::write(cd.join(discs::MANIFEST), r#"{"format": "something-else", "version": 1}"#).unwrap();
        assert!(import_cd(&lib, &cd).unwrap().disc.is_none());
        assert!(discs::ignored_files(&lib).is_empty());
    }

    #[test]
    fn a_list_the_drives_cant_fill_at_all_says_so() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let list = t.path().join("l.txt");
        write_list(&lib, &list).unwrap();
        let saved = fs::read_to_string(&list).unwrap();
        let lines: Vec<String> = saved
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.starts_with('#') && !l.is_empty())
            .map(|(i, _)| format!(r#"{{"line": {}, "matches": 0}}"#, i + 1))
            .collect();
        let cd = t.path().join("cd");
        fs::create_dir_all(&cd).unwrap();
        fs::write(
            cd.join(discs::MANIFEST),
            format!(r#"{{"format": "diskette-burn", "version": 1, "id": "d", "list": {{"sha1": "{}"}}, "lines": [{}]}}"#, crate::sha1::hex(saved.as_bytes()), lines.join(",")),
        )
        .unwrap();
        assert_eq!(import_cd(&lib, &cd).unwrap().disc.unwrap().not_on_drives.len(), SLOTS.len());
        let err = write_list(&lib, &t.path().join("m.txt")).unwrap_err();
        assert!(err.contains("Ask Again"), "{err}");
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
