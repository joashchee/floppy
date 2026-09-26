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
use crate::verify::{self, Tally};
use crate::{amiga, sha1};

pub const FORMAT: &str = "floppy-findings";

/// The file inside the zip.
pub const JSON_NAME: &str = "floppy-findings.json";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Findings {
    pub format: String,
    pub version: u32,
    /// Unique per export, so merging the same findings twice counts once.
    pub id: String,
    pub floppy_version: String,
    /// Unix seconds.
    pub exported: u64,
    pub handler_tests: Vec<Tally>,
    pub identities: Vec<IdentityFinding>,
    pub file_types: Vec<FileTypeFinding>,
    pub system_files: Vec<SystemFileFinding>,
    #[serde(default)]
    pub app_errors: Vec<AppErrorsFinding>,
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

/// How much is new since the last export, for the gear menu.
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub handler_tests: usize,
    pub identities: usize,
    pub file_types: usize,
    pub system_files: usize,
    pub app_errors: usize,
    /// Unix seconds of the last export, if any.
    pub last_exported: Option<u64>,
}

impl Summary {
    fn total(&self) -> usize {
        self.handler_tests + self.identities + self.file_types + self.system_files + self.app_errors
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
and what you wrote in a known app's Errors field (or another app's,
with its name, when you ticked Share).

It holds no documents, no document names, no programs or ROMs, and no
file or folder names from your Mac. floppy-findings.json is all of it,
as plain text, so you can read exactly what's there. Each export holds
only what's new since the one before.

Floppy never sends this anywhere. To share it, send the zip to Floppy's
maintainers (see the GitHub repo). They merge it into Floppy's living
documents with scripts/merge-findings.py, and the next release
recognizes what you found.
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
    fn an_empty_library_has_nothing_to_share() {
        let t = TempDir::new();
        assert!(collect(&Library::new(t.path().join("lib"))).unwrap().is_empty());
    }
}
