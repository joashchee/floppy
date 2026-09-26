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

use crate::handlers;
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
    /// The app's version, when known.
    pub version: Option<String>,
    /// The favorite version of its handler (library.rs).
    pub favorite: bool,
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

/// The handler an app is that opens `.ext`, and the program to run. An
/// app whose identity is settled uses just that (none, if it isn't a
/// handler). Otherwise its program names are the only clue, which the
/// reason says.
fn known_opener(app: &LibraryApp, ext: &str) -> Option<(String, String)> {
    if let Some(identity) = &app.identity {
        let h = handlers::find(GuestOs::Dos, identity.handler.as_deref()?)?;
        let program = handlers::handler_program(app, h)?;
        return h.exts.contains(&ext).then(|| (program.clone(), format!("{} opens .{ext} files.", h.name)));
    }
    app.programs.iter().find_map(|p| {
        let h = handlers::candidates(GuestOs::Dos, p).into_iter().find(|h| h.exts.contains(&ext))?;
        Some((p.clone(), format!("{} opens .{ext} files. Floppy is going by the name {}, so say which app this is in its details.", h.name, base_of(p))))
    })
}

/// The library's DOS apps that can open `file`, best first:
/// 1. the app the document last opened with, or its handler's favorite
///    version when the user has picked another since;
/// 2. apps the user said open its type;
/// 3. apps the well-known table says do.
///
/// Within each, the favorite version of a handler comes before its other
/// versions, then apps that opened this type correctly. An app that has
/// failed with it more often than it worked goes last whatever its rank.
pub fn dos_openers(file: &str, remembered: Option<&str>, apps: &[LibraryApp], tests: &[Tally]) -> Vec<Opener> {
    let ext = ext_of(file);
    if ext.is_empty() {
        return Vec::new();
    }
    let dos: Vec<&LibraryApp> = apps.iter().filter(|a| a.os == GuestOs::Dos).collect();
    // The favorite of the remembered app's handler stands in for it.
    let first = remembered.map(|id| {
        dos.iter()
            .find(|a| a.id == id)
            .and_then(|r| r.handler())
            .and_then(|h| dos.iter().find(|a| a.favorite && a.handler() == Some(h)))
            .map_or(id, |f| f.id.as_str())
    });
    let mut out: Vec<(u8, Opener)> = Vec::new();
    for app in dos {
        let said = app.opens.iter().any(|e| e.eq_ignore_ascii_case(&ext));
        let chosen = if said {
            app.program.clone().or_else(|| app.programs.first().cloned()).map(|p| (1, p, format!("You said {} opens .{ext} files.", app.name)))
        } else {
            known_opener(app, &ext).map(|(p, why)| (2, p, why))
        };
        let Some((rank, program, why)) = chosen else { continue };
        let sha256 = app.program_ids.get(&program).map(|id| id.sha256.as_str());
        let (worked, failed) = verify::record_for(tests, GuestOs::Dos, &base_of(&program), sha256, &format!(".{ext}"));
        let rank = if failed > worked {
            3
        } else if first == Some(app.id.as_str()) {
            0
        } else {
            rank
        };
        let version = app.identity.as_ref().and_then(|i| i.version.clone());
        out.push((rank, Opener { app_id: app.id.clone(), app_name: app.name.clone(), program, version, favorite: app.favorite, why, worked, failed }));
    }
    out.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| b.1.favorite.cmp(&a.1.favorite))
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
    use crate::library::{IdentifiedBy, Identity};
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
            program_ids: Default::default(),
            identity: None,
            favorite: false,
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
        assert_eq!(wp.program, "WP.EXE");
        assert!(wp.why.starts_with("WordPerfect opens .WP5 files. Floppy is going by the name WP.EXE"), "{}", wp.why);
        assert_eq!(dos_openers("A.TXT", None, &apps, &[])[0].program, "BIN/ED.COM");

        // Test results: an app that worked with .DOC moves up; one that
        // failed more than it worked goes last, even if remembered.
        let tally = |program: &str, worked, failed| Tally {
            os: GuestOs::Dos,
            app: program.into(),
            confirmed: false,
            version: None,
            program: program.into(),
            size: None,
            sha256: None,
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

    fn known(mut a: LibraryApp, handler: Option<&str>, version: &str, favorite: bool) -> LibraryApp {
        a.identity = Some(Identity { handler: handler.map(String::from), version: Some(version.into()), by: IdentifiedBy::User });
        a.favorite = favorite;
        a
    }

    #[test]
    fn several_versions_open_with_the_favorite_first() {
        let apps = vec![
            known(app("wp50", "WordPerfect 5.0", &["WP.EXE"], &[]), Some("WordPerfect"), "5.0", false),
            known(app("wp51", "WordPerfect 5.1", &["WP.EXE"], &[]), Some("WordPerfect"), "5.1", true),
            known(app("wp60", "WordPerfect 6.0", &["WPWIN/WP.EXE"], &[]), Some("WordPerfect"), "6.0", false),
        ];
        let ids = |remembered| dos_openers("A.WP5", remembered, &apps, &[]).into_iter().map(|o| o.app_id).collect::<Vec<_>>();
        assert_eq!(ids(None), ["wp51", "wp50", "wp60"]);
        // A document last opened in 5.0 opens in the favorite now.
        assert_eq!(ids(Some("wp50")), ["wp51", "wp50", "wp60"]);
        let o = &dos_openers("A.WP5", None, &apps, &[])[0];
        assert_eq!((o.version.as_deref(), o.favorite, o.why.as_str()), (Some("5.1"), true, "WordPerfect opens .WP5 files."));
        assert_eq!(dos_openers("A.WP5", None, &apps, &[])[2].program, "WPWIN/WP.EXE");
    }

    #[test]
    fn a_program_named_like_a_handler_that_isnt_one_opens_nothing() {
        let apps = vec![
            known(app("word", "WORD (a game)", &["WORD.EXE"], &[]), None, "", false),
            known(app("msword", "Word 5.5", &["WORD.EXE"], &[]), Some("Microsoft Word (DOS)"), "5.5", false),
        ];
        let ids: Vec<String> = dos_openers("LETTER.DOC", None, &apps, &[]).into_iter().map(|o| o.app_id).collect();
        assert_eq!(ids, ["msword"]);
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
