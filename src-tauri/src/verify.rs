//! Handler verification: whether an app actually opened a file type,
//! from the user's answers after each document session. Results stay in
//! `library/verifications.json`. They rank the "Open with" choices, and
//! leave only when the user exports a report (rule 4: no telemetry).
//! `scripts/merge-handler-tests.py` merges reports into
//! `docs/app-handlers.md`'s "Tested in Floppy" table.
//!
//! A result records the guest, the app (and the handler it is, when
//! known), the program, the file type, the outcome, an optional note, and
//! the date. It never records the document's name or contents.

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
    pub app_name: String,
    /// The program's file name (`WP.EXE`).
    pub program: String,
    /// The handler this app is (handlers.rs), when it's a known one.
    pub handler: Option<String>,
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
    pub file_type: String,
    pub outcome: Outcome,
    pub note: Option<String>,
    /// Unix seconds.
    pub when: u64,
    pub floppy_version: String,
}

/// Every answer for one guest + app + file type.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub os: GuestOs,
    /// The handler's name when known, else the app's.
    pub app: String,
    pub program: String,
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
        file_type: pending.file_type.clone(),
        outcome,
        note,
        when: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
        floppy_version: env!("CARGO_PKG_VERSION").to_string(),
    });
    save(library, &all)
}

/// Every answer, totalled per guest + app + program + file type, sorted.
pub fn tallies(library: &Library) -> Vec<Tally> {
    tally(&load(library))
}

pub fn tally(all: &[Verification]) -> Vec<Tally> {
    let mut map: BTreeMap<(String, String, String, String), Tally> = BTreeMap::new();
    let mut sorted: Vec<&Verification> = all.iter().collect();
    sorted.sort_by_key(|v| v.when);
    for v in sorted {
        let app = v.handler.clone().unwrap_or_else(|| v.app_name.clone());
        let key = (format!("{:?}", v.os), app.to_lowercase(), v.program.to_ascii_uppercase(), v.file_type.to_ascii_uppercase());
        let t = map.entry(key).or_insert_with(|| Tally {
            os: v.os,
            app,
            program: v.program.clone(),
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

/// A program's record for a file type: (worked, failed).
pub fn record_for(tallies: &[Tally], os: GuestOs, program: &str, file_type: &str) -> (u32, u32) {
    tallies
        .iter()
        .filter(|t| t.os == os && t.program.eq_ignore_ascii_case(program) && t.file_type.eq_ignore_ascii_case(file_type))
        .fold((0, 0), |(w, f), t| (w + t.worked, f + t.failed))
}

/// Forgets every answer.
pub fn forget_all(library: &Library) -> Result<(), String> {
    match std::fs::remove_file(library.verifications_path()) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// A report the user chose to export, for `scripts/merge-handler-tests.py`.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub format: String,
    pub version: u32,
    /// Unique per export, so merging a report twice doesn't count twice.
    pub id: String,
    pub floppy_version: String,
    /// Unix seconds.
    pub exported: u64,
    pub tallies: Vec<Tally>,
}

pub fn report(library: &Library) -> Report {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    Report {
        format: "floppy-handler-tests".into(),
        version: 1,
        id: format!("{:x}-{:x}", now.as_nanos(), std::process::id()),
        floppy_version: env!("CARGO_PKG_VERSION").to_string(),
        exported: now.as_secs(),
        tallies: tallies(library),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn pending(program: &str, file_type: &str) -> Pending {
        Pending {
            os: GuestOs::Dos,
            app_name: "WordPerfect 5.1".into(),
            program: program.into(),
            handler: Some("WordPerfect".into()),
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
        assert_eq!(record_for(&all, GuestOs::Dos, "wp.exe", ".doc"), (0, 1));
        // The document's name is never stored.
        let raw = std::fs::read_to_string(lib.verifications_path()).unwrap();
        assert!(!raw.contains("Letter to Bank"));
        assert_eq!(report(&lib).tallies, all);
        forget_all(&lib).unwrap();
        assert!(tallies(&lib).is_empty());
    }

    #[test]
    fn files_dos_documents_by_extension() {
        assert_eq!(dos_file_type("DOCS/LETTER.wp5"), ".WP5");
        assert_eq!(dos_file_type("DOCS/README"), "(none)");
        assert_eq!(dos_file_type("DOCS/.HIDDEN"), "(none)");
    }
}
