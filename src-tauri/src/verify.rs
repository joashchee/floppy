//! Handler verification: whether an app actually opened a file type,
//! from the user's answers after each document session. Results stay in
//! `library/verifications.json`. They rank the "Open with" choices, and
//! leave only when the user exports findings (findings.rs; rule 4: no
//! telemetry). `scripts/merge-findings.py` merges them into
//! `docs/app-handlers.md`'s "Tested in Floppy" table.
//!
//! A result records the guest, the app (and the handler and version it
//! is, when the user or a known fingerprint settled that), the program
//! and its size and SHA-256, the file type, the outcome, an optional
//! note, and the date. It never records the document's name or contents.
//! The fingerprint is what lets a merged report add the version to
//! `docs/app-handlers.md`'s "Known versions" table.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::library::{GuestOs, Library};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Worked,
    Failed,
}

/// What to ask about after a document session.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
    pub os: GuestOs,
    /// The library app. Its identity is read when the answer is
    /// recorded, so one confirmed after the session counts.
    #[serde(default)]
    pub app_id: String,
    pub app_name: String,
    /// The program's file name (`WP.EXE`).
    pub program: String,
    /// The handler this app is (handlers.rs), when it's known.
    pub handler: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    /// The program's fingerprint, when Floppy has it.
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    /// `.WP5`, a Mac type code, or `(none)`.
    pub file_type: String,
    /// The document's name, for the question only. Not stored.
    pub document: String,
}

/// One answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub os: GuestOs,
    pub app_name: String,
    pub program: String,
    pub handler: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub file_type: String,
    pub outcome: Outcome,
    pub note: Option<String>,
    /// Unix seconds.
    pub when: u64,
    pub floppy_version: String,
    /// Already in an Export Findings zip, so a later one leaves it out
    /// (merged counts add up).
    #[serde(default)]
    pub exported: bool,
}

/// Every answer for one guest + app + version + program + file type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub os: GuestOs,
    /// The handler's name when known, else the app's.
    pub app: String,
    /// Whether `app` is a handler the user or a fingerprint confirmed.
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub version: Option<String>,
    pub program: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub file_type: String,
    pub worked: u32,
    pub failed: u32,
    /// Unix seconds.
    pub last_tested: u64,
    /// The last outcome, which says most about the setup as it is now.
    pub last_outcome: Outcome,
    pub notes: Vec<String>,
}

/// The file type a result is filed under: `.WP5` for a DOS document.
pub fn dos_file_type(file: &str) -> String {
    match file.rsplit('/').next().unwrap_or(file).rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => format!(".{}", ext.to_ascii_uppercase()),
        _ => "(none)".into(),
    }
}

fn load(library: &Library) -> Vec<Verification> {
    std::fs::read(library.verifications_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn save(library: &Library, all: &[Verification]) -> Result<(), String> {
    let path = library.verifications_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(all).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, json).and_then(|_| std::fs::rename(&tmp, &path)).map_err(|e| format!("Couldn't save the test result: {e}"))
}

/// Records the user's answer about a session.
pub fn record(library: &Library, pending: &Pending, outcome: Outcome, note: Option<&str>) -> Result<(), String> {
    let note = note.map(|n| n.trim().replace(['\n', '\r', '\t', '|'], " ")).filter(|n| !n.is_empty());
    let mut all = load(library);
    all.push(Verification {
        os: pending.os,
        app_name: pending.app_name.clone(),
        program: pending.program.clone(),
        handler: pending.handler.clone(),
        version: pending.version.clone(),
        size: pending.size,
        sha256: pending.sha256.clone(),
        file_type: pending.file_type.clone(),
        outcome,
        note,
        when: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        floppy_version: env!("CARGO_PKG_VERSION").to_string(),
        exported: false,
    });
    save(library, &all)
}

/// Every answer, totalled per guest + app + version + program + file
/// type, sorted.
pub fn tallies(library: &Library) -> Vec<Tally> {
    tally(&load(library))
}

/// The answers no Export Findings has shared yet, totalled, and how many
/// answers there were in all (for `mark_exported`).
pub fn unexported(library: &Library) -> (Vec<Tally>, usize) {
    let all = load(library);
    let new: Vec<Verification> = all.iter().filter(|v| !v.exported).cloned().collect();
    (tally(&new), all.len())
}

/// Marks the first `count` answers as exported: the ones `unexported`
/// saw, answers being only ever added at the end.
pub fn mark_exported(library: &Library, count: usize) -> Result<(), String> {
    let mut all = load(library);
    if all.len() < count {
        return Ok(()); // Forgotten since.
    }
    let mut changed = false;
    for v in &mut all[..count] {
        changed |= !std::mem::replace(&mut v.exported, true);
    }
    if changed { save(library, &all) } else { Ok(()) }
}

pub fn tally(all: &[Verification]) -> Vec<Tally> {
    type Key = (String, String, String, String, String, String);
    let mut map: BTreeMap<Key, Tally> = BTreeMap::new();
    let mut sorted: Vec<&Verification> = all.iter().collect();
    sorted.sort_by_key(|v| v.when);
    for v in sorted {
        let app = v.handler.clone().unwrap_or_else(|| v.app_name.clone());
        let key = (
            format!("{:?}", v.os),
            app.to_lowercase(),
            v.version.clone().unwrap_or_default(),
            v.program.to_ascii_uppercase(),
            v.sha256.clone().unwrap_or_default(),
            v.file_type.to_ascii_uppercase(),
        );
        let t = map.entry(key).or_insert_with(|| Tally {
            os: v.os,
            app,
            confirmed: v.handler.is_some(),
            version: v.version.clone(),
            program: v.program.clone(),
            size: v.size,
            sha256: v.sha256.clone(),
            file_type: v.file_type.clone(),
            worked: 0,
            failed: 0,
            last_tested: 0,
            last_outcome: v.outcome,
            notes: Vec::new(),
        });
        match v.outcome {
            Outcome::Worked => t.worked += 1,
            Outcome::Failed => t.failed += 1,
        }
        t.last_tested = v.when;
        t.last_outcome = v.outcome;
        if let Some(n) = &v.note {
            if !t.notes.contains(n) {
                t.notes.push(n.clone());
            }
        }
    }
    map.into_values().collect()
}

/// A program's record for a file type: (worked, failed). With its
/// SHA-256, only answers about that exact program count, plus older ones
/// recorded without a fingerprint under its name. Two apps that share a
/// program name (`WORD.EXE`) keep separate records that way.
pub fn record_for(tallies: &[Tally], os: GuestOs, program: &str, sha256: Option<&str>, file_type: &str) -> (u32, u32) {
    tallies
        .iter()
        .filter(|t| t.os == os && t.program.eq_ignore_ascii_case(program) && t.file_type.eq_ignore_ascii_case(file_type))
        .filter(|t| match (sha256, t.sha256.as_deref()) {
            (Some(want), Some(had)) => want.eq_ignore_ascii_case(had),
            _ => true,
        })
        .fold((0, 0), |(w, f), t| (w + t.worked, f + t.failed))
}

/// Forgets every answer.
pub fn forget_all(library: &Library) -> Result<(), String> {
    match std::fs::remove_file(library.verifications_path()) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn pending(program: &str, file_type: &str) -> Pending {
        Pending {
            os: GuestOs::Dos,
            app_id: "dos-wp51".into(),
            app_name: "WordPerfect 5.1".into(),
            program: program.into(),
            handler: Some("WordPerfect".into()),
            version: Some("5.1".into()),
            size: Some(4),
            sha256: Some("aa".repeat(32)),
            file_type: file_type.into(),
            document: "Letter to Bank.wp5".into(),
        }
    }

    #[test]
    fn tallies_answers_per_app_and_file_type() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        record(&lib, &pending("WP.EXE", ".WP5"), Outcome::Worked, None).unwrap();
        record(&lib, &pending("wp.exe", ".wp5"), Outcome::Worked, Some("  fine\n")).unwrap();
        record(&lib, &pending("WP.EXE", ".WP5"), Outcome::Failed, Some("garbled | tables")).unwrap();
        record(&lib, &pending("WP.EXE", ".DOC"), Outcome::Failed, None).unwrap();
        let all = tallies(&lib);
        assert_eq!(all.len(), 2);
        let wp5 = all.iter().find(|t| t.file_type == ".WP5").unwrap();
        assert_eq!((wp5.app.as_str(), wp5.worked, wp5.failed, wp5.last_outcome), ("WordPerfect", 2, 1, Outcome::Failed));
        // Notes are trimmed, and kept to one table-safe line.
        assert_eq!(wp5.notes, ["fine", "garbled   tables"]);
        assert_eq!(record_for(&all, GuestOs::Dos, "wp.exe", None, ".doc"), (0, 1));
        assert_eq!((wp5.version.as_deref(), wp5.confirmed, wp5.size), (Some("5.1"), true, Some(4)));
        // The document's name is never stored.
        let raw = std::fs::read_to_string(lib.verifications_path()).unwrap();
        assert!(!raw.contains("Letter to Bank"));
        forget_all(&lib).unwrap();
        assert!(tallies(&lib).is_empty());
    }

    #[test]
    fn exported_answers_stay_counted_but_leave_once() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        record(&lib, &pending("WP.EXE", ".WP5"), Outcome::Worked, None).unwrap();
        record(&lib, &pending("WP.EXE", ".WP5"), Outcome::Worked, None).unwrap();
        let (new, seen) = unexported(&lib);
        assert_eq!((new[0].worked, seen), (2, 2));
        // One more arrives while the zip is written: it isn't marked.
        record(&lib, &pending("WP.EXE", ".WP5"), Outcome::Failed, None).unwrap();
        mark_exported(&lib, seen).unwrap();
        let (new, _) = unexported(&lib);
        assert_eq!((new.len(), new[0].worked, new[0].failed), (1, 0, 1));
        assert_eq!((tallies(&lib)[0].worked, tallies(&lib)[0].failed), (2, 1), "Open with still counts them all");
    }

    #[test]
    fn same_named_programs_keep_separate_records() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let word = |sha: &str, handler: Option<&str>| Pending {
            app_id: format!("dos-{sha}"),
            app_name: "WORD".into(),
            handler: handler.map(String::from),
            version: None,
            sha256: Some(sha.repeat(64)),
            ..pending("WORD.EXE", ".DOC")
        };
        record(&lib, &word("a", Some("Microsoft Word (DOS)")), Outcome::Worked, None).unwrap();
        record(&lib, &word("b", None), Outcome::Failed, None).unwrap();
        let all = tallies(&lib);
        assert_eq!(all.len(), 2);
        assert_eq!(record_for(&all, GuestOs::Dos, "WORD.EXE", Some(&"a".repeat(64)), ".DOC"), (1, 0));
        assert_eq!(record_for(&all, GuestOs::Dos, "WORD.EXE", Some(&"b".repeat(64)), ".DOC"), (0, 1));
        let other = all.iter().find(|t| !t.confirmed).unwrap();
        assert_eq!(other.app, "WORD", "an app nobody identified is filed under its own name");
    }

    #[test]
    fn files_dos_documents_by_extension() {
        assert_eq!(dos_file_type("DOCS/LETTER.wp5"), ".WP5");
        assert_eq!(dos_file_type("DOCS/README"), "(none)");
        assert_eq!(dos_file_type("DOCS/.HIDDEN"), "(none)");
    }
}
