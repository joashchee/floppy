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
    /// A knowledge pack's AI version.
    pub ai_version: Option<u32>,
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

/// Findings read from a zip or a JSON file, or why not.
pub fn read_findings(path: &Path) -> Result<Findings, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let meta = std::fs::metadata(path).map_err(|e| format!("Couldn't open {name}: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(format!("{name} isn't Floppy findings."));
    }
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut magic = [0u8; 4];
    let bytes = if f.read_exact(&mut magic).is_ok() && magic == *b"PK\x03\x04" {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).map_err(|e| e.to_string())?).map_err(|_| format!("{name} isn't Floppy findings."))?;
        let inner = z.file_names().find(|n| n.rsplit('/').next() == Some(findings::JSON_NAME)).map(String::from);
        let inner = inner.ok_or(format!("{name} isn't Floppy findings."))?;
        let entry = z.by_name(&inner).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        entry.take(MAX_BYTES).read_to_end(&mut out).map_err(|e| e.to_string())?;
        out
    } else {
        std::fs::read(path).map_err(|e| e.to_string())?
    };
    let findings: Findings = serde_json::from_slice(&bytes).map_err(|_| format!("{name} isn't Floppy findings."))?;
    if findings.format != findings::FORMAT || findings.version < 1 || findings.id.is_empty() {
        return Err(format!("{name} isn't Floppy findings."));
    }
    Ok(findings)
}

/// Whether `path` is Floppy findings, to learn from rather than import.
pub fn is_findings(path: &Path) -> bool {
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "zip" | "json") && read_findings(path).is_ok()
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A short, printable name from someone else's file.
fn tidy(s: &str, max: usize) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && s.chars().count() <= max && !s.chars().any(|c| c.is_control() || c == '|' || c == '`')).then(|| s.to_string())
}

/// Learns from the findings at `path`.
pub fn learn(library: &Library, path: &Path) -> Result<LearnSummary, String> {
    let findings = read_findings(path)?;
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
    let pack = findings.pack.as_ref().map(|p| p.ai_version);
    apply(&mut learned, &findings, &mut out);
    if !findings.materials.is_empty() {
        apply_materials(library, path, &findings, pack, &mut learned, &mut out)?;
    }
    if let Some(p) = &findings.pack {
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

/// Adds a version, unless Floppy (or earlier findings) knows its
/// fingerprint as another app or version. `add_counts` for findings'
/// test counts; a pack's tables are totals already.
fn add_version(learned: &mut Learned, out: &mut LearnSummary, v: KnownVersion, add_counts: bool) {
    let same = |k: &&KnownVersion| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256) && k.size == v.size;
    let known = handlers::builtin_known_versions().iter().find(same).or_else(|| learned.versions.iter().find(same)).cloned();
    match known {
        Some(k) if (k.app.as_str(), k.version.as_str()) != (v.app.as_str(), v.version.as_str()) => {
            out.skipped.push(format!("{} {}…: Floppy knows it as {} {}, the findings say {} {}", v.program, &v.sha256[..12], k.app, k.version, v.app, v.version));
        }
        Some(_) => {
            if let Some(k) = learned.versions.iter_mut().find(|k| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256)) {
                if add_counts {
                    k.worked += v.worked;
                    k.failed += v.failed;
                } else {
                    k.worked = k.worked.max(v.worked);
                    k.failed = k.failed.max(v.failed);
                }
            }
        }
        None => {
            learned.versions.push(v);
            out.versions += 1;
        }
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
        program: tidy(program, 64)?,
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
        out.skipped.push(format!("a file type that isn't well formed ({app} .{ext})"));
        return;
    };
    if let Some(r) = learned.file_types.iter_mut().find(|r| r.os == os && r.app == h.name && r.ext == ext) {
        r.reports = if from_pack { r.reports.max(reports) } else { r.reports + reports };
        return;
    }
    if handlers::opens_ext(h, &ext) {
        return;
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
    let what = tidy(what, 80).unwrap_or_else(|| slot.label().to_string());
    learned.system_files.push(SystemFileFinding { slot: slot.label().to_string(), what, size, sha1: sha1.to_ascii_lowercase() });
    out.system_files += 1;
}

fn apply(learned: &mut Learned, f: &Findings, out: &mut LearnSummary) {
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
        let (Some(program), Some(file_type)) = (tidy(&t.program, 64), tidy(&t.file_type, 16)) else {
            out.skipped.push(format!("a test result that isn't well formed ({})", t.app));
            continue;
        };
        if t.worked > 10_000 || t.failed > 10_000 {
            out.skipped.push(format!("a test result with implausible counts ({})", t.app));
            continue;
        }
        let tally = Tally { app: h.name.to_string(), program, file_type, notes: Vec::new(), ..t.clone() };
        if let (Some(size), Some(sha)) = (t.size, t.sha256.as_deref()) {
            if let Some(v) = version_of(t.os, h.name, t.version.as_deref(), &tally.program, size, sha) {
                add_version(learned, out, KnownVersion { worked: t.worked, failed: t.failed, ..v }, true);
            }
        }
        let same = |x: &&mut Tally| {
            x.os == tally.os && x.app == tally.app && x.version == tally.version && x.program.eq_ignore_ascii_case(&tally.program)
                && x.sha256 == tally.sha256 && x.file_type.eq_ignore_ascii_case(&tally.file_type)
        };
        match learned.tests.iter_mut().find(same) {
            Some(x) => {
                x.worked += tally.worked;
                x.failed += tally.failed;
                if tally.last_tested > x.last_tested {
                    x.last_tested = tally.last_tested;
                    x.last_outcome = tally.last_outcome;
                }
            }
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
        match learned.drop_rules.iter_mut().find(|r| r.signature == d.signature && r.choice == d.choice) {
            Some(r) => r.answers += 1,
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
    for m in &f.materials {
        let (Some(license), Some(source)) = (tidy(&m.license, 64), tidy(&m.source, 200)) else {
            out.skipped.push(format!("material {}: it needs a licence and a source", m.name));
            continue;
        };
        if !material_name_ok(&m.name) {
            out.skipped.push(format!("material {}: only plain .md and .txt files are kept", m.name));
            continue;
        }
        let Ok(entry) = z.by_name(&format!("materials/{}", m.name)) else {
            out.skipped.push(format!("material {}: listed, but not in the zip", m.name));
            continue;
        };
        if entry.size() > MATERIAL_MAX || total + entry.size() > MATERIALS_MAX {
            out.skipped.push(format!("material {}: too big", m.name));
            continue;
        }
        let mut bytes = Vec::new();
        entry.take(MATERIAL_MAX).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        total += bytes.len() as u64;
        let Some(text) = String::from_utf8(bytes).ok().filter(|t| !t.contains('\0')) else {
            out.skipped.push(format!("material {}: not plain text", m.name));
            continue;
        };
        std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't keep the materials: {e}"))?;
        std::fs::write(dir.join(&m.name), &text).map_err(|e| format!("Couldn't keep {}: {e}", m.name))?;
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

    #[test]
    fn a_knowledge_pack_teaches_from_its_documents() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        use std::io::Write;
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let next = crate::ai::builtin().version + 1;
        let mut f = findings(&t, "floppy-ai-test-pack");
        f.pack = Some(findings::PackInfo { ai_version: next, date: "2026-10-01".into(), what: "test".into() });
        let known_host = crate::cd::setup_sources()[0].url.clone();
        let handlers_doc = format!(
            "<!-- versions:start -->\n| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |\n|---|---|---|---|---|---|---|---|---|\n\
             | DOS | WordPerfect | 5.1 pack | `WP.EXE` | 777 | `{}` | 4 | 0 | 2026-10-01 |\n<!-- versions:end -->\n\
             <!-- filetypes:start -->\n| Guest | App | Extension | Reports | Last reported |\n|---|---|---|---|---|\n| DOS | WordPerfect | .Q6P | 3 | 2026-10-01 |\n<!-- filetypes:end -->\n",
            "ef".repeat(32)
        );
        let setup_doc = format!(
            "<!-- reported:start -->\n| Slot | What | Size | SHA-1 | Reports | Last reported |\n|---|---|---|---|---|---|\n<!-- reported:end -->\n\
             <!-- sources:start -->\n| Slot | Kind | Source | Link | Note |\n|---|---|---|---|---|\n\
             | Mac ROM | own | A better guide | {known_host} | Moved here. |\n\
             | Mac ROM | free | Somewhere new | https://new.example.org/rom | Unknown site. |\n<!-- sources:end -->\n"
        );
        let mut materials: Vec<(&str, String)> = vec![("app-handlers.md", handlers_doc), ("legal-setupfiles.md", setup_doc), ("notes.txt", "Thanks!".into())];
        for (name, _) in &materials {
            f.materials.push(findings::Material { name: name.to_string(), license: "GPL-2.0-or-later".into(), source: "test".into(), note: String::new() });
        }
        f.materials.push(findings::Material { name: "tool.exe".into(), license: "MIT".into(), source: "test".into(), note: String::new() });
        materials.push(("tool.exe", "MZ".into()));
        let path = t.path().join("Floppy AI.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file(findings::JSON_NAME, opts).unwrap();
        z.write_all(&serde_json::to_vec(&f).unwrap()).unwrap();
        for (name, text) in &materials {
            z.start_file(format!("materials/{name}"), opts).unwrap();
            z.write_all(text.as_bytes()).unwrap();
        }
        z.finish().unwrap();

        let s = learn(&lib, &path).unwrap();
        assert_eq!((s.ai_version, s.materials, s.versions, s.file_types, s.setup_sources), (Some(next), 3, 1, 1, 1), "{s:?}");
        assert!(s.skipped.iter().any(|x| x.contains("tool.exe")), "{:?}", s.skipped);
        assert!(s.skipped.iter().any(|x| x.contains("new.example.org")), "{:?}", s.skipped);
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

    /// The pack `scripts/make-ai-pack.py` makes is one Floppy learns from.
    /// Ignored like the e2e test: it runs python3.
    #[test]
    #[ignore]
    fn the_pack_script_makes_a_pack_floppy_learns() {
        let _turn = TURN.lock().unwrap_or_else(|p| p.into_inner());
        let t = TempDir::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        let status = std::process::Command::new("python3")
            .arg(root.join("scripts/make-ai-pack.py"))
            .arg("--out")
            .arg(t.path())
            .status()
            .expect("python3");
        assert!(status.success());
        let v = crate::ai::builtin().version;
        let lib = Library::new(t.path().join("lib"));
        let s = learn(&lib, &t.path().join(format!("Floppy AI {v}.zip"))).unwrap();
        assert_eq!((s.ai_version, s.materials), (Some(v), 4), "{s:?}");
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
