//! What this Floppy learned from findings dropped on it: another
//! Floppy's Export Findings zip (or its JSON), or a pack a maintainer put
//! together. Findings are Floppy's training data (findings.rs): merged
//! into the living documents they teach every later release, and dropped
//! here they teach this Floppy at once. Learning never reaches out: the
//! user brings the file (rule 4).
//!
//! What's applied, on top of what Floppy was built knowing:
//!
//! - **App versions** by program fingerprint (identities, and confirmed
//!   test results with one), so those programs are recognized without
//!   asking (`handlers::known_versions`).
//! - **File types** users said a known app opens (`handlers::opens_ext`).
//! - **Test results**, counted into "Open with"'s ranking.
//! - **Setup files** others used, so they're known copies
//!   (`cd::is_known`).
//! - **Drop choices**, followed when a drop has no clear winner and the
//!   user hasn't answered for that kind of item (drops.rs).
//!
//! A **knowledge pack** (`scripts/make-ai-pack.py`, published on
//! ansiapps.com with each Floppy AI version, `docs/floppy-ai.md`) is
//! findings whose materials are the living documents themselves: their
//! tables are read with the same code that reads them at build time, and
//! a pack newer than the build raises this Floppy's AI version (ai.rs).
//! Its setup sources replace the build's only then, and only for links to
//! sites the build already points to.
//!
//! Any findings zip may carry **materials**: `.md` or `.txt` files under
//! `materials/`, each listed with its licence and source. They're kept in
//! `library/learned/<id>/` for the user to read; the living documents
//! among them are learned from.
//!
//! Errors notes and setup reports need a person to read them, so they're
//! only counted. Floppy's own knowledge always wins: a fingerprint Floppy
//! already knows as another app or version is skipped and reported, as is
//! anything malformed. Each findings file is learned once (by its ID),
//! this Floppy's own exports not at all, and **Forget What Was Learned**
//! empties it. It's kept in `library/learned.json`.

use std::io::Read;
use std::path::Path;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::cd::{SetupSource, Slot};
use crate::drops::DropRule;
use crate::findings::{self, Findings, SystemFileFinding};
use crate::handlers::{self, KnownVersion, ReportedFileType};
use crate::library::{GuestOs, Library};
use crate::verify::Tally;

/// Findings bigger than this aren't findings.
const MAX_BYTES: u64 = 16 << 20;
/// At most this many entries of each kind are taken from one findings
/// file, and kept of each kind in all, so no file can stall or bloat
/// Floppy (docs/floppy-ai.md, "Safeguards").
const MAX_PER_FILE: usize = 5_000;
const MAX_KEPT: usize = 50_000;
/// Findings files learned from, at most (Forget What Was Learned… makes room).
const MAX_SOURCES: usize = 1_000;
/// Extensions of programs and system files: never learned as documents.
const PROGRAM_EXTS: &[&str] = &["EXE", "COM", "BAT", "SYS", "DLL", "OVL", "DRV", "PIF"];
/// The pack's signature, beside the JSON in the zip.
const SIG_NAME: &str = "floppy-findings.sig";

/// A findings file learned from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub floppy_version: String,
    /// Unix seconds: when it was exported, and when it was learned.
    pub exported: u64,
    pub learned: u64,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Learned {
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub versions: Vec<KnownVersion>,
    #[serde(default)]
    pub file_types: Vec<ReportedFileType>,
    #[serde(default)]
    pub tests: Vec<Tally>,
    #[serde(default)]
    pub system_files: Vec<SystemFileFinding>,
    #[serde(default)]
    pub drop_rules: Vec<DropRule>,
    /// The newest knowledge pack's AI version and date.
    #[serde(default)]
    pub ai_version: Option<u32>,
    #[serde(default)]
    pub ai_date: String,
    /// "Where Floppy points you", from that pack, when newer than the build.
    #[serde(default)]
    pub setup_sources: Vec<SetupSource>,
    /// Materials kept in `library/learned/<id>/`.
    #[serde(default)]
    pub materials: Vec<KeptMaterial>,
}

/// A material kept from a findings file.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeptMaterial {
    /// The findings file's ID, which names its folder.
    pub from: String,
    pub name: String,
    pub license: String,
    pub source: String,
}

/// What learning from one findings file did.
#[derive(Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LearnSummary {
    /// Learned from before: nothing changed.
    pub already: bool,
    /// This Floppy's own export: nothing to learn.
    pub own: bool,
    pub versions: usize,
    pub file_types: usize,
    pub tests: usize,
    pub system_files: usize,
    pub drop_choices: usize,
    /// Setup sources a newer pack updated.
    pub setup_sources: usize,
    /// Materials kept.
    pub materials: usize,
    /// A signed knowledge pack's AI version.
    pub ai_version: Option<u32>,
    /// It called itself a knowledge pack without a maintainers' signature:
    /// learned from as ordinary findings.
    pub unsigned_pack: bool,
    /// Errors notes and setup reports, for maintainers only.
    pub for_maintainers: usize,
    /// What was left out, and why.
    pub skipped: Vec<String>,
}

/// How much has been learned, for the gear menu.
#[derive(Serialize, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSummary {
    pub sources: usize,
    pub versions: usize,
    pub file_types: usize,
    pub tests: usize,
    pub system_files: usize,
    pub drop_rules: usize,
    pub materials: usize,
}

static CURRENT: RwLock<Option<Arc<Learned>>> = RwLock::new(None);

/// What's been learned, as last loaded or learned.
pub fn current() -> Arc<Learned> {
    CURRENT.read().ok().and_then(|g| g.clone()).unwrap_or_default()
}

fn set(learned: Learned) {
    if let Ok(mut g) = CURRENT.write() {
        *g = Some(Arc::new(learned));
    }
}

fn read(library: &Library) -> Learned {
    std::fs::read(library.learned_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Loads what this library learned, when Floppy starts.
pub fn load(library: &Library) {
    set(read(library));
}

pub fn summary() -> KnowledgeSummary {
    let l = current();
    KnowledgeSummary {
        sources: l.sources.len(),
        versions: l.versions.len(),
        file_types: l.file_types.len(),
        tests: l.tests.len(),
        system_files: l.system_files.len(),
        drop_rules: l.drop_rules.len(),
        materials: l.materials.len(),
    }
}

/// Forgets everything learned from findings. The user's own answers and
/// test results stay.
pub fn forget(library: &Library) -> Result<(), String> {
    match std::fs::remove_file(library.learned_path()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Couldn't forget: {e}")),
    }
    match std::fs::remove_dir_all(library.learned_dir()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Couldn't forget the materials: {e}")),
    }
    set(Learned::default());
    Ok(())
}

/// Findings as read: the parsed JSON, its exact bytes (what a pack's
/// signature signs), and the signature, if the zip has one.
pub struct Loaded {
    pub findings: Findings,
    pub json: Vec<u8>,
    pub signature: Option<String>,
}

/// Findings read from a zip or a JSON file, or why not.
pub fn read_findings(path: &Path) -> Result<Findings, String> {
    read_all(path).map(|r| r.findings)
}

fn read_all(path: &Path) -> Result<Loaded, String> {
    let name = shown(&path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
    let not = || format!("{name} isn't Floppy findings.");
    let meta = std::fs::metadata(path).map_err(|e| format!("Couldn't open {name}: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(not());
    }
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut magic = [0u8; 4];
    let (json, signature) = if f.read_exact(&mut magic).is_ok() && magic == *b"PK\x03\x04" {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).map_err(|e| e.to_string())?).map_err(|_| not())?;
        let read = |z: &mut zip::ZipArchive<std::fs::File>, leaf: &str, max: u64| -> Option<Vec<u8>> {
            let inner = z.file_names().find(|n| n.rsplit('/').next() == Some(leaf))?.to_string();
            let mut out = Vec::new();
            z.by_name(&inner).ok()?.take(max).read_to_end(&mut out).ok()?;
            Some(out)
        };
        let json = read(&mut z, findings::JSON_NAME, MAX_BYTES).ok_or_else(not)?;
        let sig = read(&mut z, SIG_NAME, 1024).and_then(|b| String::from_utf8(b).ok()).map(|s| s.trim().to_string());
        (json, sig)
    } else {
        (std::fs::read(path).map_err(|e| e.to_string())?, None)
    };
    let findings: Findings = serde_json::from_slice(&json).map_err(|_| not())?;
    let id_ok = (1..=80).contains(&findings.id.len()) && findings.id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
    if findings.format != findings::FORMAT || findings.version < 1 || !id_ok {
        return Err(not());
    }
    Ok(Loaded { findings, json, signature })
}

/// Whether `path` is Floppy findings, to learn from rather than import.
pub fn is_findings(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "zip" | "json") && read_findings(path).is_ok()
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Characters that make text look like something it isn't: bidi
/// overrides and isolates, zero-width characters, soft hyphens.
fn is_sneaky(c: char) -> bool {
    c.is_control() || matches!(c, '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}')
}

/// A short, printable name from someone else's file, or `None`.
fn tidy(s: &str, max: usize) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && s.chars().count() <= max && !s.chars().any(|c| is_sneaky(c) || c == '|' || c == '`')).then(|| s.to_string())
}

/// A program's plain file name from someone else's file: no path, no
/// quotes or markup.
fn program_name(s: &str) -> Option<String> {
    tidy(s, 64).filter(|p| !p.starts_with('.') && !p.chars().any(|c| matches!(c, '/' | '\\' | ':' | '<' | '>' | '"' | '\'')))
}

/// Someone else's text for a message: sneaky characters dropped, at most
/// 64 characters.
fn shown(s: &str) -> String {
    let clean: String = s.chars().filter(|c| !is_sneaky(*c)).collect();
    if clean.chars().count() > 64 {
        format!("{}…", clean.chars().take(63).collect::<String>())
    } else {
        clean
    }
}

/// Learns from the findings at `path`.
pub fn learn(library: &Library, path: &Path) -> Result<LearnSummary, String> {
    let Loaded { findings, json, signature } = read_all(path)?;
    let mut learned = read(library);
    let mut out = LearnSummary::default();
    if findings::is_own(library, &findings.id) {
        out.own = true;
        return Ok(out);
    }
    if learned.sources.iter().any(|s| s.id == findings.id) {
        out.already = true;
        return Ok(out);
    }
    if learned.sources.len() >= MAX_SOURCES {
        return Err(format!("Floppy has learned from {MAX_SOURCES} findings files already. Forget What Was Learned… (gear menu) makes room."));
    }
    // Only a pack the maintainers signed is official (ai.rs).
    let official = findings.pack.is_some() && signature.as_deref().is_some_and(|sig| crate::ai::verify(&crate::ai::trusted_keys(), &json, sig));
    out.unsigned_pack = findings.pack.is_some() && !official;
    let pack = if official { findings.pack.as_ref().map(|p| p.ai_version) } else { None };
    apply(&mut learned, &findings, &mut out);
    if !findings.materials.is_empty() {
        apply_materials(library, path, &findings, pack, &mut learned, &mut out)?;
    }
    if let Some(p) = findings.pack.as_ref().filter(|_| official) {
        out.ai_version = Some(p.ai_version);
        if learned.ai_version.is_none_or(|v| p.ai_version > v) {
            learned.ai_version = Some(p.ai_version);
            learned.ai_date = tidy(&p.date, 10).unwrap_or_default();
        }
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    learned.sources.push(Source {
        id: findings.id.clone(),
        floppy_version: tidy(&findings.floppy_version, 32).unwrap_or_default(),
        exported: findings.exported,
        learned: now,
    });
    let path = library.learned_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(&learned).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("Couldn't keep what was learned: {e}"))?;
    set(learned);
    Ok(out)
}

/// The findings with each list cut to `MAX_PER_FILE`.
fn capped(f: &Findings) -> Findings {
    let mut c = f.clone();
    c.identities.truncate(MAX_PER_FILE);
    c.handler_tests.truncate(MAX_PER_FILE);
    c.file_types.truncate(MAX_PER_FILE);
    c.system_files.truncate(MAX_PER_FILE);
    c.drop_choices.truncate(MAX_PER_FILE);
    c
}

/// Adds a version, unless Floppy (or earlier findings) knows its
/// fingerprint as another app or version. `add_counts` for findings'
/// test counts; a pack's tables are totals already.
fn add_version(learned: &mut Learned, out: &mut LearnSummary, v: KnownVersion, add_counts: bool) {
    let same = |k: &&KnownVersion| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256) && k.size == v.size;
    let known = handlers::builtin_known_versions().iter().find(same).or_else(|| learned.versions.iter().find(same)).cloned();
    match known {
        Some(k) if (k.app.as_str(), k.version.as_str()) != (v.app.as_str(), v.version.as_str()) => {
            out.skipped.push(format!("{} {}…: Floppy knows it as {} {}, the findings say {} {}", shown(&v.program), &v.sha256[..12], k.app, k.version, v.app, shown(&v.version)));
        }
        Some(_) => {
            if let Some(k) = learned.versions.iter_mut().find(|k| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256)) {
                if add_counts {
                    k.worked = k.worked.saturating_add(v.worked);
                    k.failed = k.failed.saturating_add(v.failed);
                } else {
                    k.worked = k.worked.max(v.worked);
                    k.failed = k.failed.max(v.failed);
                }
            }
        }
        None if learned.versions.len() >= MAX_KEPT => full(out, "app versions"),
        None => {
            learned.versions.push(v);
            out.versions += 1;
        }
    }
}

/// Notes, once, that a kind of learned knowledge is at `MAX_KEPT`.
fn full(out: &mut LearnSummary, kind: &str) {
    let note = format!("{kind}: Floppy keeps at most {MAX_KEPT} learned; Forget What Was Learned… makes room");
    if !out.skipped.contains(&note) {
        out.skipped.push(note);
    }
}

/// A version from someone else's file, if it's well formed and its app
/// is one Floppy knows.
fn version_of(os: GuestOs, app: &str, version: Option<&str>, program: &str, size: u64, sha256: &str) -> Option<KnownVersion> {
    let h = handlers::find(os, app)?;
    Some(KnownVersion {
        os,
        app: h.name.to_string(),
        version: tidy(version?, 32)?,
        program: program_name(program)?,
        size: Some(size).filter(|s| *s > 0)?,
        sha256: Some(sha256.to_ascii_lowercase()).filter(|s| is_hex(s, 64))?,
        worked: 0,
        failed: 0,
    })
}

/// Adds a file type a known app opens. `reports` from a pack's table are
/// totals: kept as the higher count, not added.
fn add_file_type(learned: &mut Learned, out: &mut LearnSummary, os: GuestOs, app: &str, ext: &str, reports: u32, from_pack: bool) {
    let ext = ext.trim_start_matches('.').to_ascii_uppercase();
    let Some(h) = handlers::find(os, app).filter(|_| (1..=8).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric())) else {
        out.skipped.push(format!("a file type that isn't well formed ({} .{})", shown(app), shown(&ext)));
        return;
    };
    if PROGRAM_EXTS.contains(&ext.as_str()) {
        out.skipped.push(format!("{} .{ext}: a program's extension, never a document's", h.name));
        return;
    }
    if let Some(r) = learned.file_types.iter_mut().find(|r| r.os == os && r.app == h.name && r.ext == ext) {
        r.reports = if from_pack { r.reports.max(reports) } else { r.reports.saturating_add(reports) };
        return;
    }
    if handlers::opens_ext(h, &ext) {
        return;
    }
    if learned.file_types.len() >= MAX_KEPT {
        return full(out, "file types");
    }
    learned.file_types.push(ReportedFileType { os, app: h.name.to_string(), ext, reports });
    out.file_types += 1;
}

/// Adds a setup file others used, unless it's known already.
fn add_system_file(learned: &mut Learned, out: &mut LearnSummary, slot: Slot, what: &str, size: u64, sha1: &str) {
    if !is_hex(sha1, 40) || size == 0 {
        out.skipped.push("a setup file that isn't well formed".into());
        return;
    }
    if crate::cd::is_known(slot, sha1) || learned.system_files.iter().any(|x| x.sha1.eq_ignore_ascii_case(sha1)) {
        return;
    }
    if learned.system_files.len() >= MAX_KEPT {
        return full(out, "setup files");
    }
    let what = tidy(what, 80).unwrap_or_else(|| slot.label().to_string());
    learned.system_files.push(SystemFileFinding { slot: slot.label().to_string(), what, size, sha1: sha1.to_ascii_lowercase() });
    out.system_files += 1;
}

fn apply(learned: &mut Learned, f: &Findings, out: &mut LearnSummary) {
    for (kind, n) in [
        ("app versions", f.identities.len()),
        ("test results", f.handler_tests.len()),
        ("file types", f.file_types.len()),
        ("setup files", f.system_files.len()),
        ("drop choices", f.drop_choices.len()),
    ] {
        if n > MAX_PER_FILE {
            out.skipped.push(format!("{kind}: only the first {MAX_PER_FILE} of {n}"));
        }
    }
    let f = &capped(f);
    for i in &f.identities {
        match version_of(i.os, &i.app, i.version.as_deref(), &i.program, i.size, &i.sha256) {
            Some(v) => add_version(learned, out, v, true),
            None => out.skipped.push(format!("an app version that isn't well formed ({})", i.app)),
        }
    }
    for t in &f.handler_tests {
        let Some(h) = handlers::find(t.os, &t.app).filter(|_| t.confirmed) else {
            // Tests of apps Floppy doesn't know rank nothing here.
            continue;
        };
        let (Some(program), Some(file_type)) = (program_name(&t.program), tidy(&t.file_type, 16)) else {
            out.skipped.push(format!("a test result that isn't well formed ({})", shown(&t.app)));
            continue;
        };
        if t.worked > 10_000 || t.failed > 10_000 {
            out.skipped.push(format!("a test result with implausible counts ({})", shown(&t.app)));
            continue;
        }
        // Only checked fields, and never someone else's notes.
        let tally = Tally {
            app: h.name.to_string(),
            program,
            file_type,
            notes: Vec::new(),
            version: t.version.as_deref().and_then(|v| tidy(v, 32)),
            sha256: t.sha256.as_deref().filter(|x| is_hex(x, 64)).map(str::to_ascii_lowercase),
            ..t.clone()
        };
        if let (Some(size), Some(sha)) = (t.size, t.sha256.as_deref()) {
            if let Some(v) = version_of(t.os, h.name, t.version.as_deref(), &tally.program, size, sha) {
                add_version(learned, out, KnownVersion { worked: t.worked, failed: t.failed, ..v }, true);
            }
        }
        let same = |x: &&mut Tally| {
            x.os == tally.os && x.app == tally.app && x.version == tally.version && x.program.eq_ignore_ascii_case(&tally.program)
                && x.sha256 == tally.sha256 && x.file_type.eq_ignore_ascii_case(&tally.file_type)
        };
        let kept = learned.tests.len();
        match learned.tests.iter_mut().find(same) {
            Some(x) => {
                x.worked = x.worked.saturating_add(tally.worked);
                x.failed = x.failed.saturating_add(tally.failed);
                if tally.last_tested > x.last_tested {
                    x.last_tested = tally.last_tested;
                    x.last_outcome = tally.last_outcome;
                }
            }
            None if kept >= MAX_KEPT => full(out, "test results"),
            None => learned.tests.push(tally),
        }
        out.tests += 1;
    }
    for t in &f.file_types {
        add_file_type(learned, out, t.os, &t.app, &t.ext, 1, false);
    }
    for sf in &f.system_files {
        match Slot::from_label(&sf.slot) {
            Some(slot) => add_system_file(learned, out, slot, &sf.what, sf.size, &sf.sha1),
            None => out.skipped.push("a setup file that isn't well formed".into()),
        }
    }
    for d in &f.drop_choices {
        if !d.signature.is_valid() || !d.offered.contains(&d.choice) {
            out.skipped.push("a drop choice that isn't well formed".into());
            continue;
        }
        let kept = learned.drop_rules.len();
        match learned.drop_rules.iter_mut().find(|r| r.signature == d.signature && r.choice == d.choice) {
            Some(r) => r.answers = r.answers.saturating_add(1),
            None if kept >= MAX_KEPT => full(out, "drop choices"),
            None => learned.drop_rules.push(DropRule { signature: d.signature.clone(), choice: d.choice, answers: 1 }),
        }
        out.drop_choices += 1;
    }
    out.for_maintainers = f.app_errors.len() + f.setup_reports.len();
}

/// One material at most this big, all of a findings file's together at
/// most `MATERIALS_MAX`.
const MATERIAL_MAX: u64 = 1 << 20;
const MATERIALS_MAX: u64 = 4 << 20;
/// Materials taken from one findings file, at most.
const MATERIALS_COUNT: usize = 32;

/// A material's file name: short, plain, `.md` or `.txt`.
fn material_name_ok(name: &str) -> bool {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    (1..=64).contains(&name.len())
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b' '))
        && matches!(ext.as_str(), "md" | "txt")
}

/// The folder a findings file's materials are kept in, from its ID.
fn materials_dir(library: &Library, id: &str) -> std::path::PathBuf {
    let safe: String = id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).take(64).collect();
    library.learned_dir().join(safe)
}

/// Keeps the materials a findings zip lists, and learns from the living
/// documents among them.
fn apply_materials(library: &Library, path: &Path, f: &Findings, pack: Option<u32>, learned: &mut Learned, out: &mut LearnSummary) -> Result<(), String> {
    let Ok(file) = std::fs::File::open(path) else { return Ok(()) };
    let Ok(mut z) = zip::ZipArchive::new(file) else {
        out.skipped.push("materials: they only come in a zip".into());
        return Ok(());
    };
    let dir = materials_dir(library, &f.id);
    let mut total = 0u64;
    if f.materials.len() > MATERIALS_COUNT {
        out.skipped.push(format!("materials: only the first {MATERIALS_COUNT} of {}", f.materials.len()));
    }
    let mut names: Vec<&str> = Vec::new();
    for m in f.materials.iter().take(MATERIALS_COUNT) {
        let name = shown(&m.name);
        let (Some(license), Some(source)) = (tidy(&m.license, 64), tidy(&m.source, 200)) else {
            out.skipped.push(format!("material {name}: it needs a licence and a source"));
            continue;
        };
        if !material_name_ok(&m.name) || names.iter().any(|n| n.eq_ignore_ascii_case(&m.name)) {
            out.skipped.push(format!("material {name}: only plain .md and .txt files, each once, are kept"));
            continue;
        }
        // A signed pack's materials must match the hashes it signed.
        let want = m.sha256.as_deref().map(str::to_ascii_lowercase);
        if want.as_deref().is_some_and(|h| !is_hex(h, 64)) || (pack.is_some() && want.is_none()) {
            out.skipped.push(format!("material {name}: its SHA-256 is missing or malformed"));
            continue;
        }
        let Ok(entry) = z.by_name(&format!("materials/{}", m.name)) else {
            out.skipped.push(format!("material {name}: listed, but not in the zip"));
            continue;
        };
        if entry.size() > MATERIAL_MAX || total + entry.size() > MATERIALS_MAX {
            out.skipped.push(format!("material {name}: too big"));
            continue;
        }
        let mut bytes = Vec::new();
        entry.take(MATERIAL_MAX).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        total += bytes.len() as u64;
        if let Some(want) = &want {
            use sha2::{Digest, Sha256};
            let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
            if &got != want {
                out.skipped.push(format!("material {name}: it isn't the file that was listed (SHA-256 differs)"));
                continue;
            }
        }
        let Some(text) = String::from_utf8(bytes).ok().filter(|t| !t.contains('\0')) else {
            out.skipped.push(format!("material {name}: not plain text"));
            continue;
        };
        names.push(&m.name);
        std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't keep the materials: {e}"))?;
        std::fs::write(dir.join(&m.name), &text).map_err(|e| format!("Couldn't keep {name}: {e}"))?;
        learned.materials.retain(|k| !(k.from == f.id && k.name == m.name));
        learned.materials.push(KeptMaterial { from: f.id.clone(), name: m.name.clone(), license, source });
        out.materials += 1;
        learn_from_document(learned, out, &m.name, &text, pack);
    }
    Ok(())
}

/// Learns from a living document a findings zip carried, with the same
/// code that reads it when Floppy is built.
fn learn_from_document(learned: &mut Learned, out: &mut LearnSummary, name: &str, text: &str, pack: Option<u32>) {
    let mut errors: Vec<String> = Vec::new();
    match name {
        "app-handlers.md" => {
            match handlers::parse_known_versions(text) {
                Ok(versions) => {
                    for v in versions {
                        match version_of(v.os, &v.app, Some(&v.version), &v.program, v.size, &v.sha256) {
                            Some(k) => add_version(learned, out, KnownVersion { worked: v.worked, failed: v.failed, ..k }, false),
                            None => errors.push(format!("a version of {} that isn't well formed", v.app)),
                        }
                    }
                }
                Err(e) => errors.push(e),
            }
            match handlers::parse_reported_file_types(text) {
                Ok(types) => {
                    for t in types {
                        add_file_type(learned, out, t.os, &t.app, &t.ext, t.reports, true);
                    }
                }
                Err(e) => errors.push(e),
            }
        }
        "legal-setupfiles.md" => {
            match crate::cd::parse_reported_files(text) {
                Ok(files) => {
                    for r in files {
                        add_system_file(learned, out, r.slot, &r.what, r.size, &r.sha1);
                    }
                }
                Err(e) => errors.push(e),
            }
            if let Some(v) = pack {
                match crate::cd::parse_setup_sources(text) {
                    Ok(rows) => apply_sources(learned, out, rows, v),
                    Err(e) => errors.push(e),
                }
            }
        }
        "file-handling.md" => match crate::drops::parse_rules(text) {
            Ok(rules) => {
                for r in rules {
                    match learned.drop_rules.iter_mut().find(|x| x.signature == r.signature && x.choice == r.choice) {
                        Some(x) => x.answers = x.answers.max(r.answers),
                        None => {
                            learned.drop_rules.push(r);
                            out.drop_choices += 1;
                        }
                    }
                }
            }
            Err(e) => errors.push(e),
        },
        "floppy-ai.md" => {
            if let Err(e) = crate::ai::parse_versions(text) {
                errors.push(e);
            }
        }
        _ => {}
    }
    out.skipped.extend(errors.into_iter().map(|e| format!("material {name}: {e}")));
}

/// A link's host: `https://www.example.org/x` is `www.example.org`.
fn host(url: &str) -> &str {
    url.strip_prefix("https://").unwrap_or("").split(['/', '?', '#']).next().unwrap_or("")
}

/// A newer pack's "Where Floppy points you", for the links it can follow:
/// sites this build already points to. A pack no newer than the build, or
/// than one learned before, changes nothing.
fn apply_sources(learned: &mut Learned, out: &mut LearnSummary, rows: Vec<SetupSource>, pack: u32) {
    if pack <= crate::ai::builtin().version || learned.ai_version.is_some_and(|v| v > pack) {
        return;
    }
    let hosts: std::collections::HashSet<&str> = crate::cd::setup_sources().iter().map(|s| host(&s.url)).collect();
    let (ok, other): (Vec<SetupSource>, Vec<SetupSource>) = rows.into_iter().partition(|r| hosts.contains(host(&r.url)));
    for r in &other {
        out.skipped.push(format!("setup source {}: a link to {} waits for a Floppy release", r.name, host(&r.url)));
    }
    out.setup_sources = ok.len();
    learned.setup_sources = ok;
}

/// Where to get each setup file: a newer pack's rows for the slots it
/// has, the build's for the rest.
pub fn setup_sources() -> Vec<SetupSource> {
    let l = current();
    let builtin = crate::cd::setup_sources();
    if l.setup_sources.is_empty() || l.ai_version.is_none_or(|v| v <= crate::ai::builtin().version) {
        return builtin.to_vec();
    }
    let mut out: Vec<SetupSource> = Vec::new();
    for slot in crate::cd::SLOTS {
        let learned: Vec<&SetupSource> = l.setup_sources.iter().filter(|s| s.slot == slot.label()).collect();
        if learned.is_empty() {
            out.extend(builtin.iter().filter(|s| s.slot == slot.label()).cloned());
        } else {
            out.extend(learned.into_iter().cloned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drops::{Choice, Dest, Signature};
    use crate::findings::{DropChoiceFinding, FileTypeFinding, IdentityFinding};
    use crate::testutil::TempDir;
    use std::fs;

    /// What's learned is one store for the whole app: tests that learn
    /// take turns.
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn findings(t: &TempDir, id: &str) -> Findings {
        let mut f = findings::collect(&Library::new(t.path().join(id))).unwrap();
        f.id = id.into();
        f
    }

    /// A pack zip: the findings JSON, a signature by `key` when given, and
    /// materials, each listed with its SHA-256 unless `hashes` is false.
    fn pack_zip(path: &Path, f: Findings, materials: &[(&str, &str)], key: Option<&ed25519_dalek::SigningKey>, hashes: bool) {
        pack_zip_with(path, f, materials, key, hashes, &[]);
    }

    /// `swapped`: files written under `materials/` in place of what the
    /// JSON lists, as if changed after signing.
    fn pack_zip_with(path: &Path, mut f: Findings, materials: &[(&str, &str)], key: Option<&ed25519_dalek::SigningKey>, hashes: bool, swapped: &[(&str, &str)]) {
        use sha2::{Digest, Sha256};
        use std::io::Write;
        for (name, text) in materials {
            let sha256 = hashes.then(|| Sha256::digest(text.as_bytes()).iter().map(|b| format!("{b:02x}")).collect());
            f.materials.push(findings::Material { name: name.to_string(), license: "GPL-2.0-or-later".into(), source: "test".into(), note: String::new(), sha256 });
        }
        let json = serde_json::to_vec(&f).unwrap();
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file(findings::JSON_NAME, opts).unwrap();
        z.write_all(&json).unwrap();
        if let Some(key) = key {
            use ed25519_dalek::Signer;
            z.start_file(SIG_NAME, opts).unwrap();
            z.write_all(key.sign(&json).to_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>().as_bytes()).unwrap();
        }
        for (name, text) in materials {
            let text = swapped.iter().find(|(n, _)| n == name).map_or(*text, |(_, t)| *t);
            z.start_file(format!("materials/{name}"), opts).unwrap();
            z.write_all(text.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    fn pack_docs() -> (String, String) {
        let known_host = crate::cd::setup_sources()[0].url.clone();
        let handlers_doc = format!(
            "<!-- versions:start -->\n| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |\n|---|---|---|---|---|---|---|---|---|\n\
             | DOS | WordPerfect | 5.1 pack | `WP.EXE` | 777 | `{}` | 4 | 0 | 2026-10-01 |\n<!-- versions:end -->\n\
             <!-- filetypes:start -->\n| Guest | App | Extension | Reports | Last reported |\n|---|---|---|---|---|\n| DOS | WordPerfect | .Q6P | 3 | 2026-10-01 |\n\
             | DOS | WordPerfect | .EXE | 9 | 2026-10-01 |\n<!-- filetypes:end -->\n",
            "ef".repeat(32)
        );
        let setup_doc = format!(
            "<!-- reported:start -->\n| Slot | What | Size | SHA-1 | Reports | Last reported |\n|---|---|---|---|---|---|\n<!-- reported:end -->\n\
             <!-- sources:start -->\n| Slot | Kind | Source | Link | Note |\n|---|---|---|---|---|\n\
             | Mac ROM | own | A better guide | {known_host} | Moved here. |\n\
             | Mac ROM | free | Somewhere new | https://new.example.org/rom | Unknown site. |\n<!-- sources:end -->\n"
        );
        (handlers_doc, setup_doc)
    }

    fn pack(t: &TempDir, id: &str) -> Findings {
        let mut f = findings(t, id);
        f.pack = Some(findings::PackInfo { ai_version: crate::ai::builtin().version + 1, date: "2026-10-01".into(), what: "test".into() });
        f
    }

    #[test]
    fn a_signed_knowledge_pack_teaches_from_its_documents() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let next = crate::ai::builtin().version + 1;
        let (handlers_doc, setup_doc) = pack_docs();
        let path = t.path().join("Floppy AI.zip");
        let materials = [("app-handlers.md", handlers_doc.as_str()), ("legal-setupfiles.md", setup_doc.as_str()), ("notes.txt", "Thanks!"), ("tool.exe", "MZ")];
        pack_zip(&path, pack(&t, "floppy-ai-test-pack"), &materials, Some(&crate::ai::test_key()), true);

        let s = learn(&lib, &path).unwrap();
        assert_eq!((s.ai_version, s.unsigned_pack, s.materials, s.versions, s.file_types, s.setup_sources), (Some(next), false, 3, 1, 1, 1), "{s:?}");
        assert!(s.skipped.iter().any(|x| x.contains("tool.exe")), "{:?}", s.skipped);
        assert!(s.skipped.iter().any(|x| x.contains("new.example.org")), "{:?}", s.skipped);
        assert!(s.skipped.iter().any(|x| x.contains(".EXE: a program's extension")), "{:?}", s.skipped);
        assert!(lib.learned_dir().join("floppy-ai-test-pack/notes.txt").is_file());
        assert!(!lib.learned_dir().join("floppy-ai-test-pack/tool.exe").exists());
        let info = crate::ai::info();
        assert!(info.version >= next && info.from_pack);
        // The pack's Mac ROM rows replace the build's; other slots keep theirs.
        let sources = setup_sources();
        assert_eq!(sources.iter().filter(|x| x.slot == "Mac ROM").map(|x| x.name.as_str()).collect::<Vec<_>>(), ["A better guide"]);
        assert!(sources.iter().any(|x| x.slot == "Kickstart ROM"));
        forget(&lib).unwrap();
        assert!(!lib.learned_dir().exists());
    }

    #[test]
    fn a_pack_without_a_trusted_signature_is_only_findings() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let (handlers_doc, setup_doc) = pack_docs();
        let materials = [("app-handlers.md", handlers_doc.as_str()), ("legal-setupfiles.md", setup_doc.as_str())];
        // Unsigned, and signed with a key Floppy doesn't trust: the tables
        // still teach, but no AI version and no setup links.
        let stranger = ed25519_dalek::SigningKey::from_bytes(&[9u8; 32]);
        for (id, key) in [("fake-pack-unsigned", None), ("fake-pack-stranger", Some(&stranger))] {
            let path = t.path().join(format!("{id}.zip"));
            pack_zip(&path, pack(&t, id), &materials, key, true);
            let s = learn(&lib, &path).unwrap();
            assert!(s.unsigned_pack, "{id}");
            assert_eq!((s.ai_version, s.setup_sources), (None, 0), "{id}: {s:?}");
            assert!(!crate::ai::info().from_pack, "{id}");
            assert!(setup_sources().iter().all(|x| x.name != "A better guide"), "{id}");
        }
        // A signed pack whose materials don't carry their hashes, or were
        // swapped after signing, loses those materials.
        let path = t.path().join("nohash.zip");
        pack_zip(&path, pack(&t, "pack-nohash"), &[("notes.txt", "hi")], Some(&crate::ai::test_key()), false);
        let s = learn(&lib, &path).unwrap();
        assert_eq!(s.materials, 0);
        assert!(s.skipped.iter().any(|x| x.contains("SHA-256 is missing")), "{:?}", s.skipped);
        let path = t.path().join("swapped.zip");
        pack_zip_with(&path, pack(&t, "pack-swapped"), &[("notes.txt", "hi")], Some(&crate::ai::test_key()), true, &[("notes.txt", "evil")]);
        let s = learn(&lib, &path).unwrap();
        assert_eq!(s.materials, 0, "{s:?}");
        assert!(s.skipped.iter().any(|x| x.contains("SHA-256 differs")), "{:?}", s.skipped);
        forget(&lib).unwrap();
    }

    #[test]
    fn hostile_text_and_floods_are_refused() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let mut f = findings(&t, "f-hostile");
        // A bidi override to make "EXE.5.1" read as "1.5.EXE", and a flood.
        f.identities.push(IdentityFinding { os: GuestOs::Dos, app: "WordPerfect".into(), version: Some("\u{202E}1.5".into()), program: "WP.EXE".into(), size: 5, sha256: "aa".repeat(32) });
        for i in 0..(MAX_PER_FILE + 10) {
            f.file_types.push(FileTypeFinding { os: GuestOs::Dos, app: "WordPerfect".into(), ext: format!("Z{:X}", i % 4096) });
        }
        let path = t.path().join("hostile.json");
        fs::write(&path, serde_json::to_vec(&f).unwrap()).unwrap();
        let s = learn(&lib, &path).unwrap();
        assert_eq!(s.versions, 0, "the spoofed version is refused");
        assert!(s.skipped.iter().any(|x| x.contains("only the first 5000")), "{:?}", &s.skipped[..3.min(s.skipped.len())]);
        assert!(s.file_types <= 4096);
        // Messages never carry the sneaky characters.
        assert!(s.skipped.iter().all(|x| !x.contains('\u{202E}')));
        // An ID that could name a path or break a document is not findings.
        let mut g = findings(&t, "ok");
        g.id = "../../evil".into();
        let bad = t.path().join("bad.json");
        fs::write(&bad, serde_json::to_vec(&g).unwrap()).unwrap();
        assert!(learn(&lib, &bad).is_err());
        forget(&lib).unwrap();
    }

    /// The pack `scripts/make-ai-pack.py` makes is one Floppy learns from.
    /// Ignored like the e2e test: it runs python3.
    #[test]
    #[ignore]
    fn the_pack_script_makes_a_pack_floppy_learns() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let v = crate::ai::builtin().version;
        let make = |out: &Path, key: Option<&Path>| {
            let mut cmd = std::process::Command::new("python3");
            cmd.arg(root.join("scripts/make-ai-pack.py")).arg("--out").arg(out);
            if let Some(key) = key {
                cmd.arg("--sign").arg(key);
            }
            assert!(cmd.status().expect("python3").success());
            out.join(format!("Floppy AI {v}.zip"))
        };
        // Unsigned: learned from, but not official.
        let lib = Library::new(t.path().join("lib"));
        let s = learn(&lib, &make(&t.path().join("unsigned"), None)).unwrap();
        assert_eq!((s.ai_version, s.unsigned_pack, s.materials), (None, true, 4), "{s:?}");
        assert!(s.skipped.is_empty(), "{:?}", s.skipped);
        forget(&lib).unwrap();
        // Signed with the test key (the seed of `ai::test_key`): official.
        let key = t.path().join("test.key");
        fs::write(&key, "2a".repeat(32)).unwrap();
        let s = learn(&lib, &make(&t.path().join("signed"), Some(&key))).unwrap();
        assert_eq!((s.ai_version, s.unsigned_pack, s.materials), (Some(v), false, 4), "{s:?}");
        assert!(s.skipped.is_empty(), "{:?}", s.skipped);
        forget(&lib).unwrap();
    }

    #[test]
    fn learns_from_findings_once_and_never_overrules_floppy() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let mut f = findings(&t, "f-learn-test-1");
        let sha = "ab".repeat(32);
        f.identities.push(IdentityFinding { os: GuestOs::Dos, app: "WordPerfect".into(), version: Some("5.1+".into()), program: "WP.EXE".into(), size: 424_242, sha256: sha.clone() });
        // Not a known app, and a malformed fingerprint: skipped.
        f.identities.push(IdentityFinding { os: GuestOs::Dos, app: "Nonesuch".into(), version: Some("1".into()), program: "N.EXE".into(), size: 1, sha256: "cd".repeat(32) });
        f.identities.push(IdentityFinding { os: GuestOs::Dos, app: "WordPerfect".into(), version: Some("9".into()), program: "WP.EXE".into(), size: 1, sha256: "zz".into() });
        f.file_types.push(FileTypeFinding { os: GuestOs::Dos, app: "WordPerfect".into(), ext: "Q7Z".into() });
        let sig = Signature { kind: "file".into(), ext: "q7y".into(), content: "binary".into() };
        let dos_doc = Choice { to: Dest::Document, os: GuestOs::Dos };
        let amiga_doc = Choice { to: Dest::Document, os: GuestOs::Amiga };
        f.drop_choices.push(DropChoiceFinding { signature: sig.clone(), choice: dos_doc, offered: vec![dos_doc, amiga_doc], answered: 1 });
        let path = t.path().join("findings.zip");
        findings::write_zip(&f, &path).unwrap();
        assert!(is_findings(&path));
        assert!(!is_findings(&t.path().join("nope.zip")));

        let s = learn(&lib, &path).unwrap();
        assert_eq!((s.versions, s.file_types, s.drop_choices, s.skipped.len()), (1, 1, 1, 2), "{s:?}");
        assert!(handlers::known_versions().iter().any(|k| k.sha256 == sha && k.version == "5.1+"));
        assert!(handlers::opens_ext(handlers::find(GuestOs::Dos, "WordPerfect").unwrap(), "Q7Z"));
        assert!(current().drop_rules.iter().any(|r| r.signature == sig && r.choice == dos_doc));
        // Once only.
        assert!(learn(&lib, &path).unwrap().already);

        // Another Floppy saying the same program is another version is
        // reported and left out.
        let mut g = findings(&t, "f-learn-test-2");
        g.identities.push(IdentityFinding { os: GuestOs::Dos, app: "WordPerfect".into(), version: Some("6.0".into()), program: "WP.EXE".into(), size: 424_242, sha256: sha.clone() });
        let path2 = t.path().join("findings2.json");
        fs::write(&path2, serde_json::to_vec(&g).unwrap()).unwrap();
        let s = learn(&lib, &path2).unwrap();
        assert_eq!(s.versions, 0);
        assert!(s.skipped[0].contains("knows it as WordPerfect 5.1+"), "{:?}", s.skipped);
        assert_eq!(summary().sources >= 2, true);

        // This Floppy's own findings teach it nothing.
        let own = findings::collect(&lib).unwrap();
        findings::mark_exported(&lib, &own).unwrap();
        let path3 = t.path().join("own.zip");
        findings::write_zip(&own, &path3).unwrap();
        assert!(learn(&lib, &path3).unwrap().own);

        forget(&lib).unwrap();
        assert!(!handlers::known_versions().iter().any(|k| k.sha256 == sha));
        load(&lib);
        assert_eq!(current().versions.len(), 0);
    }
}
