//! Documents: old files opened in the app that made them. A document is
//! imported into the guest's documents folder (`C:\DOCS` for DOS), matched
//! to apps already in the library that can open it, and launched with the
//! app and the document together. What the app saved is listed when the
//! emulator quits.
//!
//! Matching uses only the user's own apps: the public table of well-known
//! programs and the file types they open (handlers.rs,
//! `docs/app-handlers.md`), plus extensions the user says an app opens.
//! Nothing is looked up or downloaded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::handlers::HANDLERS;
use crate::verify::{self, Tally};
use crate::library::{GuestOs, LibraryApp, LibraryDoc};

/// The documents folder in the DOS guest's drive C:.
pub const DOS_DOCS_DIR: &str = "DOCS";

/// An app in the library that can open a document, and the program to run.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Opener {
    pub app_id: String,
    pub app_name: String,
    /// Relative to the app's folder, `/`-separated.
    pub program: String,
    /// Why it's offered: "WordPerfect opens .WP5 files", or the user said so.
    pub why: String,
    /// How often it opened this file type correctly, and how often not,
    /// from the user's answers (verify.rs).
    pub worked: u32,
    pub failed: u32,
}

fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_uppercase()).unwrap_or_default()
}

fn base_of(program: &str) -> String {
    program.rsplit('/').next().unwrap_or(program).to_ascii_uppercase()
}

/// The library's DOS apps that can open `file`, the one the document last
/// opened with first, then ones the user said open its type, then the
/// well-known table's. Within each, apps that opened this type correctly
/// come first, and an app that has failed with it more often than it
/// worked goes last whatever its rank.
pub fn dos_openers(file: &str, remembered: Option<&str>, apps: &[LibraryApp], tests: &[Tally]) -> Vec<Opener> {
    let ext = ext_of(file);
    if ext.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(u8, Opener)> = Vec::new();
    for app in apps.iter().filter(|a| a.os == GuestOs::Dos) {
        let said = app.opens.iter().any(|e| e.eq_ignore_ascii_case(&ext));
        let known = app.programs.iter().find_map(|p| {
            HANDLERS
                .iter()
                .filter(|h| h.os == GuestOs::Dos)
                .find(|h| h.programs.contains(&base_of(p).as_str()) && h.exts.contains(&ext.as_str()))
                .map(|h| (p, h))
        });
        let opener = match (said, known) {
            (true, _) => app.program.clone().or_else(|| app.programs.first().cloned()).map(|program| {
                (1, Opener { app_id: app.id.clone(), app_name: app.name.clone(), program, why: format!("You said {} opens .{ext} files.", app.name), worked: 0, failed: 0 })
            }),
            (false, Some((p, k))) => {
                Some((2, Opener { app_id: app.id.clone(), app_name: app.name.clone(), program: p.clone(), why: format!("{} opens .{ext} files.", k.name), worked: 0, failed: 0 }))
            }
            _ => None,
        };
        if let Some((rank, mut o)) = opener {
            (o.worked, o.failed) = verify::record_for(tests, GuestOs::Dos, base_of(&o.program).as_str(), &format!(".{ext}"));
            let rank = if o.failed > o.worked {
                3
            } else if remembered == Some(o.app_id.as_str()) {
                0
            } else {
                rank
            };
            out.push((rank, o));
        }
    }
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.1.worked.cmp(&a.1.worked))
            .then_with(|| a.1.app_name.to_lowercase().cmp(&b.1.app_name.to_lowercase()))
    });
    out.into_iter().map(|(_, o)| o).collect()
}

/// Whether a dropped or picked path is a DOS app (a folder, a zip or a
/// program) rather than a document to open in one.
pub fn is_dos_app_source(path: &Path) -> bool {
    path.is_dir() || matches!(ext_of(&path.to_string_lossy()).as_str(), "ZIP" | "EXE" | "COM" | "BAT")
}

/// A document's path inside DOS: `C:\DOCS\LETTER.WP5`.
pub fn dos_path(doc: &LibraryDoc) -> String {
    format!("C:\\{}", doc.file.replace('/', "\\"))
}

/// Every file under `root` with its size and modification time, to tell
/// afterwards what a session changed.
pub type Snapshot = HashMap<PathBuf, (u64, u128)>;

pub fn snapshot(root: &Path) -> Snapshot {
    WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let mtime = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
            Some((e.path().strip_prefix(root).ok()?.to_path_buf(), (m.len(), mtime)))
        })
        .collect()
}

/// A file a session created or changed.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// Relative to the guest's library folder, `/`-separated.
    pub path: String,
    pub new: bool,
}

/// What changed between two snapshots of the same folder, sorted by path.
/// Deleted files aren't listed: there's nothing to hand back.
pub fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
    let mut out: Vec<Change> = after
        .iter()
        .filter_map(|(p, v)| match before.get(p) {
            None => Some(true),
            Some(old) if old != v => Some(false),
            Some(_) => None,
        }
        .map(|new| Change { path: p.to_string_lossy().replace('\\', "/"), new }))
        .collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn app(id: &str, name: &str, programs: &[&str], opens: &[&str]) -> LibraryApp {
        LibraryApp {
            id: id.into(),
            os: GuestOs::Dos,
            name: name.into(),
            dir: id.to_uppercase(),
            program: programs.first().map(|p| p.to_string()),
            programs: programs.iter().map(|p| p.to_string()).collect(),
            source_name: name.into(),
            added: 0,
            opens: opens.iter().map(|e| e.to_string()).collect(),
        }
    }

    #[test]
    fn offers_the_apps_that_open_a_document() {
        let apps = vec![
            app("wp", "WordPerfect 5.1", &["WP.EXE", "WPINFO.EXE"], &[]),
            app("word", "Word 5.5", &["WORD.EXE"], &[]),
            app("ed", "My Editor", &["BIN/ED.COM"], &["TXT", "doc"]),
            app("123", "1-2-3", &["123.EXE"], &[]),
        ];
        let names = |file, remembered| dos_openers(file, remembered, &apps, &[]).into_iter().map(|o| o.app_id).collect::<Vec<_>>();
        // DOC: the user's own say-so first, then the table's, by name.
        assert_eq!(names("LETTER.DOC", None), ["ed", "word", "wp"]);
        // The app it last opened with comes first.
        assert_eq!(names("letter.doc", Some("wp")), ["wp", "ed", "word"]);
        assert_eq!(names("BUDGET.WK1", None), ["123"]);
        assert!(names("PHOTO.JPG", None).is_empty());
        assert!(names("README", None).is_empty());
        let wp = &dos_openers("A.WP5", None, &apps, &[])[0];
        assert_eq!((wp.program.as_str(), wp.why.as_str()), ("WP.EXE", "WordPerfect opens .WP5 files."));
        assert_eq!(dos_openers("A.TXT", None, &apps, &[])[0].program, "BIN/ED.COM");

        // Test results: an app that worked with .DOC moves up; one that
        // failed more than it worked goes last, even if remembered.
        let tally = |program: &str, worked, failed| Tally {
            os: GuestOs::Dos,
            app: program.into(),
            program: program.into(),
            file_type: ".DOC".into(),
            worked,
            failed,
            last_tested: 1,
            last_outcome: verify::Outcome::Worked,
            notes: vec![],
        };
        let tests = [tally("WP.EXE", 2, 0), tally("ED.COM", 0, 1)];
        let ranked = dos_openers("LETTER.DOC", Some("ed"), &apps, &tests);
        assert_eq!(ranked.iter().map(|o| o.app_id.as_str()).collect::<Vec<_>>(), ["wp", "word", "ed"]);
        assert_eq!((ranked[0].worked, ranked[0].failed), (2, 0));
    }

    #[test]
    fn tells_apps_from_documents() {
        let t = TempDir::new();
        assert!(is_dos_app_source(t.path()));
        for app in ["WP51.ZIP", "game.exe", "X.COM", "go.bat"] {
            assert!(is_dos_app_source(Path::new(app)), "{app}");
        }
        for doc in ["LETTER.WP5", "budget.wk1", "README"] {
            assert!(!is_dos_app_source(Path::new(doc)), "{doc}");
        }
    }

    #[test]
    fn lists_what_a_session_saved() {
        let t = TempDir::new();
        let root = t.path();
        std::fs::create_dir_all(root.join("DOCS")).unwrap();
        std::fs::write(root.join("DOCS/LETTER.WP5"), b"v1").unwrap();
        std::fs::write(root.join("DOCS/OLD.WP5"), b"same").unwrap();
        let before = snapshot(root);
        std::fs::write(root.join("DOCS/LETTER.WP5"), b"version 2").unwrap();
        std::fs::write(root.join("DOCS/LETTER.BK!"), b"backup").unwrap();
        let after = snapshot(root);
        assert_eq!(
            changes(&before, &after),
            vec![Change { path: "DOCS/LETTER.BK!".into(), new: true }, Change { path: "DOCS/LETTER.WP5".into(), new: false }]
        );
    }
}
