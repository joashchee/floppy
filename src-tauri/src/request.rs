//! Asking a running disc maker for what Floppy still needs, and getting
//! the disc back without the user carrying files between the apps. It
//! builds on the missing-files list and files disc (cd.rs) and the
//! wanted-apps list (handlers.rs):
//!
//! 1. **Request.** Floppy writes one list: the missing-files list, then
//!    the wanted-apps list's lines (whose `#gather: folder` and
//!    `#forks: appledouble` apply only to the apps after them), then
//!
//!    ```text
//!    #reply-to: com.ansiapps.floppy
//!    ```
//!
//!    the bundle ID of the app the finished disc should be opened with.
//! 2. Floppy opens the list with the disc maker, by its bundle ID
//!    (`open -b`), as if the user had opened it there. Today that's
//!    Diskette (`com.ansiapps.diskette`), whose Burn A CD reads it. The
//!    offer only shows while the disc maker is running, which Floppy
//!    checks by bundle ID (`lsappinfo`), never by reading its data.
//! 3. **Reply.** The disc maker makes the disc and opens it with the app
//!    `#reply-to:` names. Floppy receives it as an opened file (lib.rs,
//!    `RunEvent::Opened`) and imports it like a dropped disc: setup files
//!    and apps both (`commands::import_disc`).
//!
//! Only the list and the disc cross between the programs (rule 2).

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cd::{self, Slot};
use crate::discs;
use crate::handlers;
use crate::library::Library;

/// The disc maker Floppy asks: Diskette.
pub const DISKETTE: &str = "com.ansiapps.diskette";
/// Floppy's own bundle ID, for `#reply-to:` (CLAUDE.md rule 5).
pub const FLOPPY: &str = "com.ansiapps.floppy";

/// How much a request would ask for.
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestSummary {
    /// Missing system files (slots), leaving off ones the drives couldn't fill.
    pub setup: usize,
    /// Old apps that open old files, not in the library yet.
    pub apps: usize,
}

pub struct Request {
    pub text: String,
    /// The slot each missing-files line (1-based) asks for.
    pub slot_lines: Vec<(usize, Slot)>,
    pub summary: RequestSummary,
}

/// The request list for what's still missing, or `None` when nothing is.
pub fn build(library: &Library) -> Result<Option<Request>, String> {
    let unfindable = discs::not_on_drives(library);
    let slots: Vec<Slot> = cd::missing_slots(library)?.into_iter().filter(|s| !unfindable.contains(s)).collect();
    let setup = cd::missing_list(&slots, &discs::ignored_files(library));
    let apps = handlers::wanted_lines(&library.list()?);
    let summary = RequestSummary { setup: slots.len(), apps: apps.as_ref().map_or(0, |(_, n)| *n) };
    let (mut text, slot_lines) = match setup {
        Some(list) => (list.text, list.lines),
        None if apps.is_some() => ("#columns: name size sha1\n".to_string(), Vec::new()),
        None => return Ok(None),
    };
    if let Some((lines, _)) = apps {
        text.push('\n');
        text.push_str(&lines.join("\n"));
        text.push('\n');
    }
    text.push_str(&format!("\n# Open the finished disc with Floppy.\n#reply-to: {FLOPPY}\n"));
    Ok(Some(Request { text, slot_lines, summary }))
}

/// What a request would ask for right now.
pub fn summary(library: &Library) -> Result<RequestSummary, String> {
    Ok(build(library)?.map(|r| r.summary).unwrap_or_default())
}

/// Writes the request list into `library/run/requests/`, remembers it for
/// matching the disc that answers it (discs.rs), and opens it with the
/// disc maker. Returns what it asked for.
pub fn send(library: &Library) -> Result<RequestSummary, String> {
    let request = build(library)?.ok_or("Floppy has everything it can ask for.")?;
    let dir = library.run_dir().join("requests");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("Floppy wants.txt");
    std::fs::write(&path, &request.text).map_err(|e| format!("Couldn't save the request: {e}"))?;
    discs::record_list(library, &request.text, &request.slot_lines)?;
    open_with(DISKETTE, &path)?;
    Ok(request.summary)
}

/// Whether the app with `bundle_id` is running.
#[cfg(target_os = "macos")]
pub fn is_running(bundle_id: &str) -> bool {
    std::process::Command::new("/usr/bin/lsappinfo")
        .args(["info", "-only", "bundleid", "-app", bundle_id])
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{bundle_id}\"")))
}

#[cfg(not(target_os = "macos"))]
pub fn is_running(_bundle_id: &str) -> bool {
    false
}

/// Opens `path` with the app that has `bundle_id`.
#[cfg(target_os = "macos")]
fn open_with(bundle_id: &str, path: &Path) -> Result<(), String> {
    let ok = std::process::Command::new("/usr/bin/open")
        .args(["-b", bundle_id])
        .arg(path)
        .status()
        .map_err(|e| format!("Couldn't open the request: {e}"))?
        .success();
    ok.then_some(()).ok_or_else(|| "Couldn't hand the request to Diskette. Is it installed?".to_string())
}

#[cfg(not(target_os = "macos"))]
fn open_with(_bundle_id: &str, _path: &Path) -> Result<(), String> {
    Err("Asking Diskette is macOS-only for now.".to_string())
}

/// Files the system asked Floppy to open (`RunEvent::Opened`), waiting
/// for the window to take them.
#[derive(Default)]
pub struct Opened(pub std::sync::Mutex<Vec<PathBuf>>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn a_request_asks_for_setup_files_then_apps_and_names_floppy() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let r = build(&lib).unwrap().unwrap();
        assert!(r.text.starts_with("#columns: name size sha1\n"));
        assert_eq!(r.summary.setup, cd::missing_slots(&lib).unwrap().len());
        assert!(r.summary.apps > 0);
        // Folders are gathered only for the apps: every slot line comes
        // before the directive.
        let lines: Vec<&str> = r.text.lines().collect();
        let gather = lines.iter().position(|l| *l == "#gather: folder").unwrap();
        assert!(r.slot_lines.iter().all(|(n, _)| *n < gather + 1));
        assert!(lines.contains(&"WP.EXE"));
        assert!(lines.iter().position(|l| *l == "WP.EXE").unwrap() > gather);
        assert_eq!(lines.last(), Some(&"#reply-to: com.ansiapps.floppy"));
        // Line numbers still point at the list's name and content lines.
        for (n, _) in &r.slot_lines {
            let line = lines[n - 1];
            assert!(!line.is_empty() && !line.starts_with('#'), "line {n}: {line}");
        }
    }
}
