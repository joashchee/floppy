//! Findings: what this Floppy has learned that could help every Floppy,
//! exported only when the user asks (rule 4: no telemetry), as one zip.
//! `scripts/merge-findings.py` merges findings into the living documents,
//! and Floppy reads those documents' tables when it's built, so each
//! release recognizes more:
//!
//! - **Handler tests** (verify.rs): which app, version and program opened
//!   which file type, and how often it worked. Into `docs/app-handlers.md`'s
//!   "Tested in Floppy".
//! - **Identities**: apps the user said are a known app and version, with
//!   their program's size and SHA-256. Into "Known versions", so the same
//!   program is recognized in every library.
//! - **File types**: extensions the user added to a known app's "Also
//!   opens". Into "Reported file types", which document matching uses.
//! - **System files**: ROMs and Workbench floppies the user set up that
//!   aren't among the known-good copies (`known_files.rs`). Into
//!   `docs/legal-setupfiles.md`'s "Reported by users", which the
//!   missing-files list asks for by content.
//! - **Errors**: what the user noted in an app's Errors field, with its
//!   app, version and program fingerprint. Into "Reported problems", for
//!   a person to read. A known app's always go. Another app's go only
//!   when the user ticks "Share", and then under its name in the library.
//! - **Drop choices** (drops.rs): where the user said a dropped item goes
//!   when Floppy had no clear winner, keyed by what kind of item it was
//!   (file, folder or zip, extension, what its contents look like), never
//!   its name. Into `docs/file-handling.md`'s "Drop choices", which Floppy
//!   follows before asking.
//! - **Setup reports**: what the user reported about getting a setup file:
//!   a source that stopped working, a file that didn't work once set up
//!   (with what Floppy recognized it as), or a better source. Into
//!   `docs/legal-setupfiles.md`'s "Reported setup notes", for a person to
//!   check before changing where the setup screen points.
//!
//! Findings are Floppy's training data. Merged into the living documents
//! they teach every later release; dropped on another Floppy (learned.rs)
//! they teach that one straight away, with no update.
//!
//! Each export holds only what earlier ones didn't: `library/findings.json`
//! keeps what was shared, and each test result is marked exported
//! (verify.rs), because merging adds counts up.
//!
//! Never included: documents or their names, the user's files, file names
//! of setup files, folder paths, or anything about the user.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cd::{self, Slot};
use crate::handlers;
use crate::library::{GuestOs, IdentifiedBy, Library, SystemFile};
use crate::drops;
use crate::verify::{self, Tally};
use crate::{amiga, sha1};

pub const FORMAT: &str = "floppy-findings";

/// The file inside the zip.
pub const JSON_NAME: &str = "floppy-findings.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Findings {
    pub format: String,
    pub version: u32,
    /// Unique per export, so merging the same findings twice counts once.
    pub id: String,
    pub floppy_version: String,
    /// Unix seconds.
    pub exported: u64,
    #[serde(default)]
    pub handler_tests: Vec<Tally>,
    #[serde(default)]
    pub identities: Vec<IdentityFinding>,
    #[serde(default)]
    pub file_types: Vec<FileTypeFinding>,
    #[serde(default)]
    pub system_files: Vec<SystemFileFinding>,
    #[serde(default)]
    pub app_errors: Vec<AppErrorsFinding>,
    #[serde(default)]
    pub setup_reports: Vec<SetupReport>,
    #[serde(default)]
    pub drop_choices: Vec<DropChoiceFinding>,
    /// The Floppy AI version (ai.rs) of the Floppy that exported them.
    #[serde(default)]
    pub ai_version: Option<u32>,
    /// Set on a knowledge pack (`scripts/make-ai-pack.py`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<PackInfo>,
    /// Files under `materials/` in the zip (learned.rs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<Material>,
    /// How many test results `handler_tests` was totalled from, to mark
    /// them exported. Not shared.
    #[serde(skip)]
    verifications_seen: usize,
}

/// A program the user said is a known app (and version).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityFinding {
    pub os: GuestOs,
    /// The handler's name.
    pub app: String,
    pub version: Option<String>,
    /// The program's file name.
    pub program: String,
    pub size: u64,
    pub sha256: String,
}

/// A document extension the user says a known app opens.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileTypeFinding {
    pub os: GuestOs,
    pub app: String,
    /// Uppercase, no dot.
    pub ext: String,
}

/// A setup file in use that isn't a known-good copy.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemFileFinding {
    /// cd.rs slot label: "Mac ROM", "Kickstart ROM", "Workbench disk".
    pub slot: String,
    /// What Floppy recognized it as ("Kickstart 3.1 (40.68)").
    pub what: String,
    pub size: u64,
    pub sha1: String,
}

/// What the user noted going wrong with an app.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorsFinding {
    pub os: GuestOs,
    /// The handler's name, or for an app that isn't a known one, its name
    /// in the user's library.
    pub app: String,
    /// Whether `app` is a known app (handlers.rs).
    pub known: bool,
    pub version: Option<String>,
    /// The program's file name, and its fingerprint when Floppy has one.
    pub program: Option<String>,
    pub size: Option<u64>,
    pub sha256: Option<String>,
    pub errors: String,
}

/// What a knowledge pack is: the Floppy AI version its materials make.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackInfo {
    pub ai_version: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
    /// What changed in this version.
    #[serde(default)]
    pub what: String,
}

/// A file a findings zip carries under `materials/`: plain text that
/// adds to what Floppy knows, legal to share on ansiapps.com.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Material {
    /// Its file name under `materials/`: `.md` or `.txt`.
    pub name: String,
    /// The licence it's shared under ("GPL-2.0-or-later", "CC0-1.0").
    pub license: String,
    /// Where it came from: a repo path or a link.
    pub source: String,
    #[serde(default)]
    pub note: String,
    /// Its SHA-256, checked when it's read. A signed pack's materials must
    /// have one: the signature covers the JSON, the hashes the files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Where the user said a dropped item goes (drops.rs).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DropChoiceFinding {
    pub signature: drops::Signature,
    pub choice: drops::Choice,
    /// What the dialog offered.
    pub offered: Vec<drops::Choice>,
    /// Unix seconds.
    pub answered: u64,
}

impl DropChoiceFinding {
    fn key(&self) -> String {
        let s = &self.signature;
        format!("drop {} {} {} {:?} {}", s.kind, s.ext, s.content, self.choice, self.answered)
    }
}

/// What a setup report is about.
pub const SETUP_REPORT_KINDS: [&str; 3] = ["source-broken", "didnt-work", "better-source"];

/// Something the user reported about getting a setup file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SetupReport {
    /// cd.rs slot label.
    pub slot: String,
    /// One of `SETUP_REPORT_KINDS`.
    pub kind: String,
    /// The source it's about, or the better one: a link or a name.
    pub source: Option<String>,
    pub note: String,
    /// For `didnt-work`: what Floppy recognized the file in the slot as,
    /// with its size and, for a ROM or floppy, its SHA-1.
    pub file: Option<String>,
    /// Unix seconds.
    pub reported: u64,
}

impl SetupReport {
    fn key(&self) -> String {
        format!("setup {} {} {} {} {}", self.slot, self.kind, self.source.as_deref().unwrap_or(""), self.note, self.reported)
    }
}

const MAX_REPORT_NOTE: usize = 1000;
const MAX_REPORT_SOURCE: usize = 300;

/// Keeps a setup report for the next export. For "didn't work", notes
/// what the file in that slot is, so a maintainer can tell which copy.
pub fn add_setup_report(library: &Library, slot: &str, kind: &str, source: Option<&str>, note: &str) -> Result<(), String> {
    let slot = Slot::from_label(slot).ok_or(format!("Unknown setup file {slot:?}."))?;
    if !SETUP_REPORT_KINDS.contains(&kind) {
        return Err(format!("Unknown kind of report {kind:?}."));
    }
    let note = scrub(note.trim());
    let source = source.map(str::trim).filter(|s| !s.is_empty()).map(scrub);
    if note.is_empty() && source.is_none() {
        return Err("Say what happened, or where the better copy is.".into());
    }
    if note.chars().count() > MAX_REPORT_NOTE || source.as_ref().is_some_and(|s| s.chars().count() > MAX_REPORT_SOURCE) {
        return Err("That's too long to send: keep it to a short paragraph.".into());
    }
    let file = if kind == "didnt-work" { slot_file(library, slot)? } else { None };
    let reported = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    let mut all = load_setup_reports(library);
    all.push(SetupReport { slot: slot.label().to_string(), kind: kind.to_string(), source, note, file, reported });
    let path = library.setup_reports_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Couldn't keep the report: {e}"))
}

fn load_setup_reports(library: &Library) -> Vec<SetupReport> {
    std::fs::read(library.setup_reports_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// What the file in `slot` is, for a "didn't work" report: never its
/// name or where it is.
fn slot_file(library: &Library, slot: Slot) -> Result<Option<String>, String> {
    let Some(path) = library.system_file(slot.os(), slot.kind())? else {
        // What ran instead, when that's the AROS fallback.
        let aros = slot == Slot::Kickstart && library.system(GuestOs::Amiga)?.aros;
        return Ok(aros.then(|| "AROS replacement Kickstart (FS-UAE's built-in copy)".to_string()));
    };
    let Ok(meta) = std::fs::metadata(&path) else { return Ok(None) };
    if !meta.is_file() {
        return Ok(Some("a folder".into()));
    }
    if slot == Slot::MacBoot {
        let volume = crate::mac::volume_name(&path).filter(|v| !v.is_empty()).map(|v| format!(" \"{v}\"")).unwrap_or_default();
        return Ok(Some(format!("startup disk{volume}, {} bytes", meta.len())));
    }
    if meta.len() > MAX_SYSTEM_FILE {
        return Ok(Some(format!("{} bytes", meta.len())));
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let what = describe(slot, &path, &bytes).unwrap_or_else(|| slot.label().to_string());
    Ok(Some(format!("{what}, {} bytes, SHA-1 {}", bytes.len(), sha1::hex(&bytes))))
}

/// How much is new since the last export, for the gear menu.
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub handler_tests: usize,
    pub identities: usize,
    pub file_types: usize,
    pub system_files: usize,
    pub app_errors: usize,
    pub setup_reports: usize,
    pub drop_choices: usize,
    /// Unix seconds of the last export, if any.
    pub last_exported: Option<u64>,
}

impl Summary {
    fn total(&self) -> usize {
        self.handler_tests + self.identities + self.file_types + self.system_files + self.app_errors + self.setup_reports + self.drop_choices
    }
}

impl Findings {
    pub fn summary(&self) -> Summary {
        Summary {
            handler_tests: self.handler_tests.len(),
            identities: self.identities.len(),
            file_types: self.file_types.len(),
            system_files: self.system_files.len(),
            app_errors: self.app_errors.len(),
            setup_reports: self.setup_reports.len(),
            drop_choices: self.drop_choices.len(),
            last_exported: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.summary().total() == 0
    }
}

impl IdentityFinding {
    fn key(&self) -> String {
        format!("identity {:?} {} {} {}", self.os, self.sha256, self.app, self.version.as_deref().unwrap_or(""))
    }
}

impl FileTypeFinding {
    fn key(&self) -> String {
        format!("type {:?} {} {}", self.os, self.app, self.ext)
    }
}

impl SystemFileFinding {
    fn key(&self) -> String {
        format!("system {}", self.sha1)
    }
}

impl AppErrorsFinding {
    fn key(&self) -> String {
        let version = self.version.as_deref().unwrap_or("");
        let program = self.sha256.as_deref().or(self.program.as_deref()).unwrap_or("");
        format!("errors {:?} {} {} {version} {program} {}", self.os, self.known, self.app, self.errors)
    }
}

/// What earlier exports shared (`library/findings.json`).
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Shared {
    /// Unix seconds.
    last_exported: Option<u64>,
    /// Every identity, file type, system file and errors note exported.
    keys: BTreeSet<String>,
    /// The IDs of this Floppy's own exports, so dropping one back on it
    /// teaches it nothing twice (learned.rs).
    #[serde(default)]
    exported_ids: BTreeSet<String>,
}

/// Whether findings `id` came from this Floppy.
pub fn is_own(library: &Library, id: &str) -> bool {
    load_shared(library).exported_ids.contains(id)
}

fn load_shared(library: &Library) -> Shared {
    std::fs::read(library.findings_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// The gear menu's count: what an export would hold now.
pub fn summary(library: &Library) -> Result<Summary, String> {
    Ok(Summary { last_exported: load_shared(library).last_exported, ..collect(library)?.summary() })
}

/// Remembers that `findings` left in a zip, so the next export doesn't
/// repeat them.
pub fn mark_exported(library: &Library, findings: &Findings) -> Result<(), String> {
    verify::mark_exported(library, findings.verifications_seen)?;
    let mut shared = load_shared(library);
    shared.last_exported = Some(findings.exported);
    shared.keys.extend(findings.identities.iter().map(IdentityFinding::key));
    shared.keys.extend(findings.file_types.iter().map(FileTypeFinding::key));
    shared.keys.extend(findings.system_files.iter().map(SystemFileFinding::key));
    shared.keys.extend(findings.app_errors.iter().map(AppErrorsFinding::key));
    shared.keys.extend(findings.setup_reports.iter().map(SetupReport::key));
    shared.keys.extend(findings.drop_choices.iter().map(DropChoiceFinding::key));
    shared.exported_ids.insert(findings.id.clone());
    let path = library.findings_path();
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(&shared).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("Couldn't note what was exported: {e}"))
}

/// An Errors note as shared: the user's home folder, should they paste a
/// path, reads `~`.
fn scrub(errors: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if home.len() > 1 => errors.replace(home.trim_end_matches('/'), "~"),
        _ => errors.to_string(),
    }
}

/// What this library has learned since the last export. Reads the ROM
/// and Workbench floppy (a few hundred KB each) to fingerprint them.
pub fn collect(library: &Library) -> Result<Findings, String> {
    library.backfill_program_ids()?;
    let apps = library.list()?;
    let shared = load_shared(library);
    let mut identities = Vec::new();
    let mut file_types = Vec::new();
    let mut app_errors = Vec::new();
    let file_name = |p: &String| p.rsplit('/').next().unwrap_or(p).to_string();
    for app in &apps {
        let known = app.identity.as_ref().and_then(|i| i.handler.as_deref()).and_then(|n| handlers::find(app.os, n));
        let Some((identity, h)) = app.identity.as_ref().zip(known) else {
            // Not a known app: its errors go only if the user said so.
            if app.share_errors && !app.errors.is_empty() {
                let program_id = app.program.as_ref().and_then(|p| app.program_ids.get(p));
                let f = AppErrorsFinding {
                    os: app.os,
                    app: app.name.clone(),
                    known: false,
                    version: None,
                    program: app.program.as_ref().map(file_name),
                    size: program_id.map(|id| id.size),
                    sha256: program_id.map(|id| id.sha256.clone()),
                    errors: scrub(&app.errors),
                };
                if !shared.keys.contains(&f.key()) && !app_errors.contains(&f) {
                    app_errors.push(f);
                }
            }
            continue;
        };
        let program = handlers::handler_program(app, h);
        let program_id = program.and_then(|p| app.program_ids.get(p));
        if !app.errors.is_empty() {
            let f = AppErrorsFinding {
                os: app.os,
                app: h.name.to_string(),
                known: true,
                version: identity.version.clone(),
                program: program.map(file_name),
                size: program_id.map(|id| id.size),
                sha256: program_id.map(|id| id.sha256.clone()),
                errors: scrub(&app.errors),
            };
            if !shared.keys.contains(&f.key()) && !app_errors.contains(&f) {
                app_errors.push(f);
            }
        }
        // A fingerprint that identified it is already known.
        if identity.by == IdentifiedBy::User {
            if let Some(program) = program {
                if let Some(id) = program_id {
                    let f = IdentityFinding {
                        os: app.os,
                        app: h.name.to_string(),
                        version: identity.version.clone(),
                        program: file_name(program),
                        size: id.size,
                        sha256: id.sha256.clone(),
                    };
                    if !shared.keys.contains(&f.key()) && !identities.contains(&f) {
                        identities.push(f);
                    }
                }
            }
        }
        for ext in &app.opens {
            if h.exts.contains(&ext.as_str()) || handlers::reported_ext(app.os, h.name, ext) {
                continue;
            }
            let f = FileTypeFinding { os: app.os, app: h.name.to_string(), ext: ext.clone() };
            if !shared.keys.contains(&f.key()) && !file_types.contains(&f) {
                file_types.push(f);
            }
        }
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let (handler_tests, verifications_seen) = verify::unexported(library);
    let mut system_files = system_files(library)?;
    system_files.retain(|f| !shared.keys.contains(&f.key()));
    let mut setup_reports = load_setup_reports(library);
    setup_reports.retain(|r| !shared.keys.contains(&r.key()));
    let drop_choices: Vec<DropChoiceFinding> = drops::load_answers(library)
        .into_iter()
        .map(|a| DropChoiceFinding { signature: a.signature, choice: a.choice, offered: a.offered, answered: a.answered })
        .filter(|f| !shared.keys.contains(&f.key()))
        .collect();
    Ok(Findings {
        format: FORMAT.into(),
        version: 1,
        id: format!("f-{:x}-{:x}", now.as_nanos(), std::process::id()),
        floppy_version: env!("CARGO_PKG_VERSION").to_string(),
        exported: now.as_secs(),
        handler_tests,
        identities,
        file_types,
        system_files,
        app_errors,
        setup_reports,
        drop_choices,
        ai_version: Some(crate::ai::info().version),
        pack: None,
        materials: Vec::new(),
        verifications_seen,
    })
}

/// Setup files larger than this aren't fingerprinted: hard-disk files and
/// startup disks change as they're used, so their hash says nothing.
const MAX_SYSTEM_FILE: u64 = 2 << 20;

fn system_files(library: &Library) -> Result<Vec<SystemFileFinding>, String> {
    let mut out = Vec::new();
    for (slot, os, kind) in [
        (Slot::MacRom, GuestOs::MacClassic, SystemFile::Rom),
        (Slot::Kickstart, GuestOs::Amiga, SystemFile::Rom),
        (Slot::Workbench, GuestOs::Amiga, SystemFile::Boot),
    ] {
        let Some(path) = library.system_file(os, kind)? else { continue };
        if !path.is_file() || std::fs::metadata(&path).map(|m| m.len() > MAX_SYSTEM_FILE).unwrap_or(true) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let sha1 = sha1::hex(&bytes);
        if cd::is_known(slot, &sha1) {
            continue;
        }
        let Some(what) = describe(slot, &path, &bytes) else { continue };
        out.push(SystemFileFinding { slot: slot.label().to_string(), what, size: bytes.len() as u64, sha1 });
    }
    Ok(out)
}

/// What a setup file is, from its contents. `None` for a Workbench that
/// isn't a floppy image.
fn describe(slot: Slot, path: &Path, bytes: &[u8]) -> Option<String> {
    match slot {
        Slot::MacRom => {
            let checksum: String = bytes.get(..4)?.iter().map(|b| format!("{b:02X}")).collect();
            Some(format!("Mac ROM, checksum {checksum}"))
        }
        Slot::Kickstart => Some(match amiga::identify_kickstart(bytes, bytes.len() as u64)? {
            amiga::Kickstart::Plain { version, revision } => {
                format!("Kickstart {} ({version}.{revision})", amiga::kickstart_release(version))
            }
            amiga::Kickstart::Encrypted => "Amiga Forever ROM (encrypted)".into(),
        }),
        Slot::Workbench => amiga::adf_volume(path).map(|(volume, _)| format!("Workbench floppy \"{volume}\"")),
        Slot::MacBoot => None,
    }
}

const README: &str = "Floppy findings

What this Floppy learned that can help every Floppy: which old apps
opened which file types, which app and version each program is (by its
size and SHA-256), file types you said an app opens, and setup files
(ROMs, Workbench floppies) it didn't already know, by size and SHA-1,
what you wrote in a known app's Errors field (or another app's,
with its name, when you ticked Share), and what you reported about
getting setup files: a source that stopped working, a file that didn't
work (with what Floppy recognized it as), or a better source, and where
you said dropped files go when Floppy couldn't tell (by the kind of file:
its extension and what its contents look like, never its name).

It holds no documents, no document names, no programs or ROMs, and no
file or folder names from your Mac. floppy-findings.json is all of it,
as plain text, so you can read exactly what's there. Each export holds
only what's new since the one before.

These are Floppy's training data. Floppy never sends them anywhere;
you choose who gets them:

- Drop this zip on anyone's Floppy window (or use Learn from Findings…
  in its gear menu), and that Floppy learns what's in it straight away:
  the app versions, file types, test results, setup files and drop
  choices. Nothing it already knows is overruled, and it can forget
  what it learned at any time.
- Send it to Floppy's maintainers (see the GitHub repo). They merge it
  into Floppy's living documents with scripts/merge-findings.py, and
  every Floppy knows it from the next release on.
";

/// Writes `findings` as a zip: the JSON and a README saying what's in it.
pub fn write_zip(findings: &Findings, path: &Path) -> Result<(), String> {
    let json = serde_json::to_vec_pretty(findings).map_err(|e| e.to_string())?;
    let file = std::fs::File::create(path).map_err(|e| format!("Couldn't save the findings: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut add = |name: &str, data: &[u8]| -> zip::result::ZipResult<()> {
        zip.start_file(name, options)?;
        zip.write_all(data)?;
        Ok(())
    };
    add(JSON_NAME, &json).and_then(|_| add("README.txt", README.as_bytes())).map_err(|e| format!("Couldn't save the findings: {e}"))?;
    zip.finish().map_err(|e| format!("Couldn't save the findings: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;
    use std::io::Read;

    #[test]
    fn collects_confirmed_apps_file_types_and_unknown_roms_and_zips_them() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("WP51");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("WP.EXE"), "MZ five-one").unwrap();
        fs::write(src.join("WPINFO.EXE"), "MZ info").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        lib.set_identity(&app.id, Some((Some("WordPerfect".into()), Some("5.1".into())))).unwrap();
        // WP5 is in the table already; Q9Z is news.
        lib.set_opens(&app.id, &["WP5".into(), "Q9Z".into()]).unwrap();
        // An app the user said isn't a known one tells nothing.
        let other = t.path().join("GAME");
        fs::create_dir_all(&other).unwrap();
        fs::write(other.join("WORD.EXE"), "MZ game").unwrap();
        let game = lib.import(GuestOs::Dos, &other).unwrap();
        lib.set_identity(&game.id, Some((None, None))).unwrap();
        lib.set_opens(&game.id, &["SAV".into()]).unwrap();

        let mut rom = vec![0u8; 524_288];
        rom[..4].copy_from_slice(&[0x42, 0x1E, 0xF4, 0x8B]);
        rom[8..10].copy_from_slice(&[0x06, 0x7C]);
        let rom_path = t.path().join("my.rom");
        fs::write(&rom_path, &rom).unwrap();
        lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &rom_path, None).unwrap();

        let f = collect(&lib).unwrap();
        assert_eq!(f.identities.len(), 1);
        let id = &f.identities[0];
        assert_eq!((id.app.as_str(), id.version.as_deref(), id.program.as_str(), id.size), ("WordPerfect", Some("5.1"), "WP.EXE", 11));
        assert_eq!(f.file_types, [FileTypeFinding { os: GuestOs::Dos, app: "WordPerfect".into(), ext: "Q9Z".into() }]);
        assert_eq!(f.system_files.len(), 1);
        assert_eq!((f.system_files[0].slot.as_str(), f.system_files[0].what.as_str()), ("Mac ROM", "Mac ROM, checksum 421EF48B"));
        assert_eq!(f.system_files[0].sha1, sha1::hex(&rom));
        assert_eq!(f.summary(), Summary { identities: 1, file_types: 1, system_files: 1, ..Summary::default() });

        let zip_path = t.path().join("findings.zip");
        write_zip(&f, &zip_path).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(&zip_path).unwrap()).unwrap();
        let mut json = String::new();
        zip.by_name(JSON_NAME).unwrap().read_to_string(&mut json).unwrap();
        let back: Findings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, f);
        assert!(zip.by_name("README.txt").is_ok());
        // Nothing names the user's files or folders.
        for private in ["my.rom", "WP51", "GAME", t.path().to_str().unwrap()] {
            assert!(!json.contains(private), "{private} leaked into the findings");
        }
    }

    #[test]
    fn each_export_holds_only_whats_new() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("WP51");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("WP.EXE"), "MZ five-one").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        lib.set_identity(&app.id, Some((Some("WordPerfect".into()), Some("5.1".into())))).unwrap();
        lib.set_opens(&app.id, &["Q9Z".into()]).unwrap();
        let home = std::env::var("HOME").unwrap();
        lib.set_errors(&app.id, &format!("Can't find {home}/fonts")).unwrap();
        let pending = verify::Pending {
            os: GuestOs::Dos,
            app_id: app.id.clone(),
            app_name: app.name.clone(),
            program: "WP.EXE".into(),
            handler: Some("WordPerfect".into()),
            version: Some("5.1".into()),
            size: None,
            sha256: None,
            file_type: ".WP5".into(),
            document: "x.wp5".into(),
        };
        verify::record(&lib, &pending, verify::Outcome::Worked, None).unwrap();

        let first = collect(&lib).unwrap();
        assert_eq!(first.summary(), Summary { handler_tests: 1, identities: 1, file_types: 1, app_errors: 1, ..Summary::default() });
        let errors = &first.app_errors[0];
        assert_eq!((errors.app.as_str(), errors.program.as_deref(), errors.errors.as_str()), ("WordPerfect", Some("WP.EXE"), "Can't find ~/fonts"));
        assert!(errors.sha256.is_some());
        mark_exported(&lib, &first).unwrap();
        let s = summary(&lib).unwrap();
        assert!(s.total() == 0 && s.last_exported == Some(first.exported), "nothing new: {s:?}");

        // New answers, and an edited note, are new.
        verify::record(&lib, &pending, verify::Outcome::Failed, None).unwrap();
        lib.set_errors(&app.id, "Printer driver missing").unwrap();
        let second = collect(&lib).unwrap();
        assert_eq!(second.summary(), Summary { handler_tests: 1, app_errors: 1, ..Summary::default() });
        assert_eq!((second.handler_tests[0].worked, second.handler_tests[0].failed), (0, 1), "only the new answer");
        assert_ne!(second.id, first.id);
    }

    #[test]
    fn errors_of_apps_that_arent_known_stay_home_unless_shared() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("MYGAME");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("GAME.EXE"), "MZ").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        lib.set_errors(&app.id, "Sound stutters").unwrap();
        assert!(collect(&lib).unwrap().is_empty());
        lib.set_identity(&app.id, Some((None, None))).unwrap();
        assert!(collect(&lib).unwrap().is_empty());
        // Unless the user ticks Share: then they go, under its name.
        lib.set_share_errors(&app.id, true).unwrap();
        let f = collect(&lib).unwrap();
        let e = &f.app_errors[0];
        assert_eq!((e.app.as_str(), e.known, e.program.as_deref(), e.errors.as_str()), ("MYGAME", false, Some("GAME.EXE"), "Sound stutters"));
        assert!(e.sha256.is_some());
        mark_exported(&lib, &f).unwrap();
        assert!(collect(&lib).unwrap().is_empty());
    }

    #[test]
    fn setup_reports_go_once_and_say_what_the_file_is() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let home = std::env::var("HOME").unwrap();
        add_setup_report(&lib, "Mac startup disk", "source-broken", Some("https://example.org/753"), "Gone (404)").unwrap();
        add_setup_report(&lib, "Kickstart ROM", "better-source", Some("https://example.org/roms"), "").unwrap();
        let mut rom = vec![0u8; 524_288];
        rom[..4].copy_from_slice(&[0x42, 0x1E, 0xF4, 0x8B]);
        rom[8..10].copy_from_slice(&[0x06, 0x7C]);
        let rom_path = t.path().join("secret-name.rom");
        fs::write(&rom_path, &rom).unwrap();
        lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &rom_path, None).unwrap();
        add_setup_report(&lib, "Mac ROM", "didnt-work", None, &format!("Black screen, see {home}/log")).unwrap();
        // Nothing to say, an unknown slot or kind: refused.
        assert!(add_setup_report(&lib, "Mac ROM", "didnt-work", Some(" "), " ").is_err());
        assert!(add_setup_report(&lib, "Toaster", "didnt-work", None, "x").is_err());
        assert!(add_setup_report(&lib, "Mac ROM", "meh", None, "x").is_err());

        let f = collect(&lib).unwrap();
        assert_eq!(f.summary().setup_reports, 3);
        let didnt = f.setup_reports.iter().find(|r| r.kind == "didnt-work").unwrap();
        assert_eq!(didnt.note, "Black screen, see ~/log");
        let file = didnt.file.as_deref().unwrap();
        assert!(file.starts_with("Mac ROM, checksum 421EF48B, 524288 bytes, SHA-1 "), "{file}");
        let json = serde_json::to_string(&f).unwrap();
        assert!(!json.contains("secret-name") && !json.contains(&home));
        mark_exported(&lib, &f).unwrap();
        assert_eq!(collect(&lib).unwrap().summary().setup_reports, 0);
    }

    #[test]
    fn a_kickstart_report_says_when_aros_was_running() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        lib.set_aros(GuestOs::Amiga, true).unwrap();
        add_setup_report(&lib, "Kickstart ROM", "didnt-work", None, "Guru meditation in my game").unwrap();
        let f = collect(&lib).unwrap();
        assert_eq!(f.setup_reports[0].file.as_deref(), Some("AROS replacement Kickstart (FS-UAE's built-in copy)"));
    }

    #[test]
    fn an_empty_library_has_nothing_to_share() {
        let t = TempDir::new();
        assert!(collect(&Library::new(t.path().join("lib"))).unwrap().is_empty());
    }
}
