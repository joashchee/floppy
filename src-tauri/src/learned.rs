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

use crate::cd::Slot;
use crate::drops::DropRule;
use crate::findings::{self, Findings, SystemFileFinding};
use crate::handlers::{self, KnownVersion, ReportedFileType};
use crate::library::Library;
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
    apply(&mut learned, &findings, &mut out);
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

fn apply(learned: &mut Learned, f: &Findings, out: &mut LearnSummary) {
    let add_version = |learned: &mut Learned, out: &mut LearnSummary, v: KnownVersion| {
        let same = |k: &&KnownVersion| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256) && k.size == v.size;
        let known = handlers::builtin_known_versions().iter().find(same).or_else(|| learned.versions.iter().find(same)).cloned();
        match known {
            Some(k) if (k.app.as_str(), k.version.as_str()) != (v.app.as_str(), v.version.as_str()) => {
                out.skipped.push(format!("{} {}…: Floppy knows it as {} {}, the findings say {} {}", v.program, &v.sha256[..12], k.app, k.version, v.app, v.version));
            }
            Some(_) => {
                if let Some(k) = learned.versions.iter_mut().find(|k| k.os == v.os && k.sha256.eq_ignore_ascii_case(&v.sha256)) {
                    k.worked += v.worked;
                    k.failed += v.failed;
                }
            }
            None => {
                learned.versions.push(v);
                out.versions += 1;
            }
        }
    };
    let version_of = |os, app: &str, version: Option<&str>, program: &str, size: u64, sha256: &str| -> Option<KnownVersion> {
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
    };
    for i in &f.identities {
        match version_of(i.os, &i.app, i.version.as_deref(), &i.program, i.size, &i.sha256) {
            Some(v) => add_version(learned, out, v),
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
                add_version(learned, out, KnownVersion { worked: t.worked, failed: t.failed, ..v });
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
        let ext = t.ext.trim_start_matches('.').to_ascii_uppercase();
        let Some(h) = handlers::find(t.os, &t.app).filter(|_| (1..=8).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric())) else {
            out.skipped.push(format!("a file type that isn't well formed ({} .{})", t.app, t.ext));
            continue;
        };
        if handlers::opens_ext(h, &ext) {
            if let Some(r) = learned.file_types.iter_mut().find(|r| r.os == t.os && r.app == h.name && r.ext == ext) {
                r.reports += 1;
            }
            continue;
        }
        learned.file_types.push(ReportedFileType { os: t.os, app: h.name.to_string(), ext, reports: 1 });
        out.file_types += 1;
    }
    for s in &f.system_files {
        let Some(slot) = Slot::from_label(&s.slot).filter(|_| is_hex(&s.sha1, 40) && s.size > 0) else {
            out.skipped.push("a setup file that isn't well formed".into());
            continue;
        };
        if crate::cd::is_known(slot, &s.sha1) || learned.system_files.iter().any(|x| x.sha1.eq_ignore_ascii_case(&s.sha1)) {
            continue;
        }
        let what = tidy(&s.what, 80).unwrap_or_else(|| slot.label().to_string());
        learned.system_files.push(SystemFileFinding { slot: slot.label().to_string(), what, size: s.size, sha1: s.sha1.to_ascii_lowercase() });
        out.system_files += 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drops::{Choice, Dest, Signature};
    use crate::findings::{DropChoiceFinding, FileTypeFinding, IdentityFinding};
    use crate::library::GuestOs;
    use crate::testutil::TempDir;
    use std::fs;

    fn findings(t: &TempDir, id: &str) -> Findings {
        let mut f = findings::collect(&Library::new(t.path().join(id))).unwrap();
        f.id = id.into();
        f
    }

    #[test]
    fn learns_from_findings_once_and_never_overrules_floppy() {
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
