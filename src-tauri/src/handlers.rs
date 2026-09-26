//! Old apps that open old file formats, per guest: the code copy of
//! `docs/app-handlers.md` (a test keeps the two in step). Floppy uses it
//! to match documents to apps (documents.rs), to write the wanted-apps
//! list a disc maker gathers the user's own copies from, and to import
//! those copies from the disc.
//!
//! The wanted-apps list is the missing-files list's format (cd.rs) with
//! two directives a disc maker needs for apps. Both are `#` lines, so an
//! older reader skips them:
//! - `#gather: folder`: gather each matched file's whole folder, keeping
//!   its layout, since an app needs its support files;
//! - `#forks: appledouble`: keep classic Mac resource forks and Finder
//!   info as AppleDouble `._name` files beside the files, since ISO 9660,
//!   Joliet and UDF can't hold them.
//!
//! Each directive applies to the lines after it, until one says
//! otherwise (`#gather: file`, `#forks: none`). So a request that asks
//! for system files and apps in one list (request.rs) gathers folders
//! only for the apps.
//!
//! A program's name only suggests which app it is: DOS apps keep the
//! same name across versions, and different apps share names
//! (`WORD.EXE`). What an app is gets settled by its program's SHA-256
//! matching a known version (the "Known versions" table in
//! `docs/app-handlers.md`, read straight from the document), or by the
//! user. Floppy keeps any number of versions of an app, and the user
//! picks a favorite to open documents with.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use walkdir::WalkDir;

use crate::library::{GuestOs, IdentifiedBy, Identity, Library, LibraryApp, ProgramId};

/// An app that opens old documents.
pub struct Handler {
    pub os: GuestOs,
    pub name: &'static str,
    /// What the app's program file is called: a DOS program (uppercase),
    /// a Mac application's file name, or an Amiga program. The
    /// wanted-apps list asks for these.
    pub programs: &'static [&'static str],
    /// DOS: document extensions (uppercase, no dot).
    pub exts: &'static [&'static str],
    /// Mac: the app's creator code.
    pub creator: Option<&'static str>,
    /// Mac document type codes, or Amiga IFF FORM types.
    pub types: &'static [&'static str],
}

const fn dos(name: &'static str, programs: &'static [&'static str], exts: &'static [&'static str]) -> Handler {
    Handler { os: GuestOs::Dos, name, programs, exts, creator: None, types: &[] }
}

const fn mac(name: &'static str, programs: &'static [&'static str], creator: &'static str, types: &'static [&'static str]) -> Handler {
    Handler { os: GuestOs::MacClassic, name, programs, exts: &[], creator: Some(creator), types }
}

const fn amiga(name: &'static str, programs: &'static [&'static str], types: &'static [&'static str]) -> Handler {
    Handler { os: GuestOs::Amiga, name, programs, exts: &[], creator: None, types }
}

/// Keep in step with `docs/app-handlers.md`, which has each entry's
/// confidence and source.
pub const HANDLERS: &[Handler] = &[
    dos("WordPerfect", &["WP.EXE"], &["WP", "WP5", "WP6", "WPD", "DOC"]),
    dos("Microsoft Word (DOS)", &["WORD.EXE"], &["DOC"]),
    dos("WordStar", &["WS.EXE"], &["WS", "WS4", "WS5", "WS7", "WSD"]),
    dos("Microsoft Works (DOS)", &["WORKS.EXE"], &["WPS", "WKS", "WDB"]),
    dos("Lotus 1-2-3", &["123.EXE", "LOTUS.COM"], &["WKS", "WK1", "WK3"]),
    dos("Quattro Pro", &["Q.EXE"], &["WQ1", "WQ2", "WKQ"]),
    dos("dBASE", &["DBASE.EXE"], &["DBF"]),
    dos("Paradox", &["PARADOX.EXE"], &["DB"]),
    dos("Harvard Graphics", &["HG.EXE"], &["CHT", "SHW"]),
    dos("Deluxe Paint II", &["DPAINT.EXE"], &["LBM", "IFF", "BBM"]),
    dos("PC Paintbrush", &["PBRUSH.EXE"], &["PCX"]),
    dos("AutoCAD", &["ACAD.EXE"], &["DWG"]),
    dos("Turbo Pascal", &["TURBO.EXE"], &["PAS"]),
    mac("MacWrite", &["MacWrite"], "MACA", &["WORD"]),
    mac("MacWrite II", &["MacWrite II"], "MWII", &["MW2D"]),
    mac("MacPaint", &["MacPaint"], "MPNT", &["PNTG"]),
    mac("MacDraw", &["MacDraw"], "MDRW", &["DRWG"]),
    mac("MacDraw II", &["MacDraw II"], "MDPL", &["DRWG"]),
    mac("ClarisWorks", &["ClarisWorks", "AppleWorks"], "BOBO", &["CWWP", "CWDB", "CWSS", "CWGR"]),
    mac("Microsoft Word", &["Microsoft Word"], "MSWD", &["WDBN", "W6BN"]),
    mac("Microsoft Excel", &["Microsoft Excel"], "XCEL", &["XLS ", "XLS4", "XLW4"]),
    mac("Microsoft Works", &["Microsoft Works"], "MSWK", &["AWWP", "AWDB", "AWSS", "AWDR"]),
    mac("PowerPoint", &["Microsoft PowerPoint"], "PPNT", &["SLDS"]),
    mac("FileMaker Pro", &["FileMaker Pro"], "FMPR", &["FMPR"]),
    mac("HyperCard", &["HyperCard"], "WILD", &["STAK"]),
    mac("PageMaker", &["PageMaker", "Aldus PageMaker"], "ALD5", &["ALB3", "ALB4", "ALB5"]),
    mac("QuarkXPress", &["QuarkXPress"], "XPR3", &["XDOC"]),
    mac("WriteNow", &["WriteNow"], "nX^n", &["*WNW"]),
    mac("Nisus Writer", &["Nisus Writer", "Nisus"], "NISI", &["TEXT"]),
    mac("WordPerfect (Mac)", &["WordPerfect"], "WPC2", &["WPD0", "WPD1", "WPD2"]),
    mac("MORE", &["MORE"], "MOR2", &["MOR3"]),
    mac("FullWrite", &["FullWrite"], "FWRT", &[]),
    mac("SuperPaint", &["SuperPaint"], "SPNT", &["SPTG"]),
    mac("Canvas", &["Canvas"], "DAD2", &["drw2"]),
    mac("Photoshop", &["Adobe Photoshop"], "8BIM", &["8BIM"]),
    mac("FreeHand", &["Aldus FreeHand", "FreeHand"], "FHA3", &["FHD3"]),
    mac("Persuasion", &["Aldus Persuasion", "Persuasion"], "PLP2", &["PRS1", "PRS2"]),
    mac("MacProject II", &["MacProject II"], "MPRX", &["MPRD"]),
    mac("TeachText / SimpleText", &["TeachText", "SimpleText"], "ttxt", &["TEXT", "ttro"]),
    mac("Compact Pro", &["Compact Pro"], "CPCT", &["PACT"]),
    mac("Disk Copy", &["Disk Copy"], "dCpy", &["dImg"]),
    mac("StuffIt Expander", &["StuffIt Expander"], "SITx", &["SIT!"]),
    amiga("Deluxe Paint", &["DPaint"], &["ILBM", "ANIM", "PRSP"]),
    amiga("Personal Paint", &["PPaint"], &["ILBM"]),
    amiga("Brilliance", &["Brilliance"], &["ILBM", "ANIM"]),
    amiga("ProWrite", &["ProWrite"], &["WORD"]),
    amiga("Flow", &["Flow"], &["HEAD"]),
    amiga("Deluxe Music Construction Set", &["DMCS"], &["SMUS"]),
    amiga("AudioMaster", &["AudioMaster"], &["8SVX"]),
    amiga("ProTracker", &["ProTracker"], &[]),
    amiga("OctaMED", &["OctaMED"], &[]),
    amiga("Imagine / Turbo Silver", &["Imagine", "TurboSilver"], &["TDDD"]),
    amiga("Deluxe Video", &["DVideo"], &["ANBM"]),
    amiga("Final Writer", &["FinalWriter"], &[]),
    amiga("Wordworth", &["Wordworth"], &[]),
    amiga("PageStream", &["PageStream"], &[]),
    amiga("MultiView", &["MultiView"], &["ILBM", "8SVX", "FTXT"]),
];

/// The handler called `name`.
pub fn find(os: GuestOs, name: &str) -> Option<&'static Handler> {
    HANDLERS.iter().find(|h| h.os == os && h.name.eq_ignore_ascii_case(name))
}

/// Every handler with a program named like `program` (a path or file
/// name): what it might be, going by its name alone.
pub fn candidates(os: GuestOs, program: &str) -> Vec<&'static Handler> {
    let name = base_name(program);
    HANDLERS.iter().filter(|h| h.os == os && h.programs.iter().any(|p| p.eq_ignore_ascii_case(name))).collect()
}

/// A version of a handler, known by its program's fingerprint. From the
/// "Known versions" table in `docs/app-handlers.md`, which test reports
/// fill (`scripts/merge-handler-tests.py`).
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KnownVersion {
    pub os: GuestOs,
    /// The handler's name.
    pub app: String,
    pub version: String,
    /// The program's file name.
    pub program: String,
    pub size: u64,
    pub sha256: String,
    pub worked: u32,
    pub failed: u32,
}

/// The known versions, read once from `docs/app-handlers.md`.
pub fn known_versions() -> &'static [KnownVersion] {
    static KNOWN: OnceLock<Vec<KnownVersion>> = OnceLock::new();
    KNOWN.get_or_init(|| parse_known_versions(include_str!("../../docs/app-handlers.md")).unwrap_or_default())
}

/// The rows between `<!-- versions:start -->` and `<!-- versions:end -->`:
/// `| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed |
/// Last tested |`. A row that doesn't parse is an error, so a bad merge
/// fails the tests rather than quietly dropping a version.
fn parse_known_versions(doc: &str) -> Result<Vec<KnownVersion>, String> {
    let start = doc.find("<!-- versions:start -->").ok_or("no versions:start marker")?;
    let end = doc[start..].find("<!-- versions:end -->").ok_or("no versions:end marker")? + start;
    let mut out = Vec::new();
    for line in doc[start..end].lines().skip(1).filter(|l| l.trim_start().starts_with('|')).skip(2) {
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').map(|c| c.trim().trim_matches('`')).collect();
        let [guest, app, version, program, size, sha256, worked, failed, _last] = cells[..] else {
            return Err(format!("a known-versions row needs 9 cells: {line}"));
        };
        let os = match guest {
            "DOS" => GuestOs::Dos,
            "Classic Mac" => GuestOs::MacClassic,
            "Amiga" => GuestOs::Amiga,
            _ => return Err(format!("unknown guest {guest:?}: {line}")),
        };
        if sha256.len() != 64 || !sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("not a SHA-256: {line}"));
        }
        let num = |s: &str| s.replace(',', "").parse::<u64>().map_err(|_| format!("not a number {s:?}: {line}"));
        out.push(KnownVersion {
            os,
            app: app.to_string(),
            version: version.to_string(),
            program: program.to_string(),
            size: num(size)?,
            sha256: sha256.to_ascii_lowercase(),
            worked: num(worked)? as u32,
            failed: num(failed)? as u32,
        });
    }
    Ok(out)
}

/// What an app is, when one of its programs matches a known version.
pub fn identify(os: GuestOs, ids: &BTreeMap<String, ProgramId>, known: &[KnownVersion]) -> Option<Identity> {
    ids.values().find_map(|id| {
        known.iter().find(|k| k.os == os && k.size == id.size && k.sha256.eq_ignore_ascii_case(&id.sha256)).map(|k| Identity {
            handler: Some(k.app.clone()),
            version: Some(k.version.clone()),
            by: IdentifiedBy::Hash,
        })
    })
}

/// The program that makes `app` its handler: one named like the
/// handler's programs, preferring the one it runs. For a renamed one,
/// what it runs.
pub fn handler_program<'a>(app: &'a LibraryApp, h: &Handler) -> Option<&'a String> {
    let named = |p: &&String| h.programs.iter().any(|q| base_name(p).eq_ignore_ascii_case(q));
    app.program.iter().find(named).or_else(|| app.programs.iter().find(named)).or(app.program.as_ref())
}

/// A handler, for the frontend's "Is" menu and after-session question.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HandlerInfo {
    pub name: &'static str,
    pub programs: &'static [&'static str],
    /// Document extensions (DOS).
    pub exts: &'static [&'static str],
    /// Versions known by fingerprint.
    pub versions: Vec<String>,
}

/// Every handler for `os`, with the versions known for it.
pub fn catalog(os: GuestOs) -> Vec<HandlerInfo> {
    HANDLERS
        .iter()
        .filter(|h| h.os == os)
        .map(|h| {
            let mut versions: Vec<String> =
                known_versions().iter().filter(|k| k.os == os && k.app == h.name).map(|k| k.version.clone()).collect();
            versions.dedup();
            HandlerInfo { name: h.name, programs: h.programs, exts: h.exts, versions }
        })
        .collect()
}

fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Whether the library already has `h`: an app of its guest with one of
/// its program files.
fn in_library(h: &Handler, apps: &[LibraryApp]) -> bool {
    apps.iter().filter(|a| a.os == h.os).any(|a| a.programs.iter().any(|p| h.programs.iter().any(|q| base_name(p).eq_ignore_ascii_case(q))))
}

/// The wanted-apps list (see the module doc) for every handler not in
/// the library yet, or `None` when the library has them all.
pub fn wanted_list(apps: &[LibraryApp]) -> Option<String> {
    let (lines, _) = wanted_lines(apps)?;
    let mut out = vec!["#columns: name size sha1".to_string()];
    out.extend(lines);
    Some(out.join("\n") + "\n")
}

/// The wanted-apps list without its `#columns:` header, starting with
/// its `#gather:` and `#forks:` directives, which apply to the lines
/// after them. So it can follow a missing-files list in one request
/// (request.rs). Also returns how many apps it asks for.
pub fn wanted_lines(apps: &[LibraryApp]) -> Option<(Vec<String>, usize)> {
    let wanted: Vec<&Handler> = HANDLERS.iter().filter(|h| !in_library(h, apps)).collect();
    if wanted.is_empty() {
        return None;
    }
    let mut out = vec![
        "#gather: folder".to_string(),
        "#forks: appledouble".to_string(),
        "# Old apps that open old files, for Floppy. Gather each one with its".to_string(),
        "# folder, keeping Mac resource forks as AppleDouble ._ files, then".to_string(),
        "# use Import Apps Disc in Floppy.".to_string(),
    ];
    let mut seen = HashSet::new();
    for os in [GuestOs::Dos, GuestOs::MacClassic, GuestOs::Amiga] {
        let group: Vec<&&Handler> = wanted.iter().filter(|h| h.os == os).collect();
        if group.is_empty() {
            continue;
        }
        out.push(String::new());
        out.push(format!("# {}", crate::library::guest_label(os)));
        for h in group {
            let opens = if !h.exts.is_empty() {
                h.exts.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(" ")
            } else {
                h.types.iter().map(|t| t.trim()).collect::<Vec<_>>().join(" ")
            };
            let name = match h.creator {
                Some(c) => format!("{} (creator {c})", h.name),
                None => h.name.to_string(),
            };
            out.push(if opens.is_empty() { format!("# {name}") } else { format!("# {name}: {opens}") });
            for p in h.programs {
                if seen.insert(p.to_ascii_lowercase()) {
                    out.push(p.to_string());
                }
            }
        }
    }
    Some((out, wanted.len()))
}

/// What Import Apps Disc did.
#[derive(Serialize, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppsImport {
    /// "WordPerfect (DOS)".
    pub imported: Vec<String>,
    /// Handlers found that were already in the library.
    pub already: Vec<String>,
    /// Imports that failed, with why.
    pub failed: Vec<String>,
}

/// Imports every handler app found under `root` (an apps disc's root,
/// see `docs/app-handlers.md`): each file named like a handler's program
/// brings in its folder, or itself when it's at the root. A Mac app at
/// the root is imported with its AppleDouble `._` file, so its forks
/// survive.
///
/// A DOS program is imported once per version: a copy whose program is
/// byte for byte one already in the library, or already imported from
/// this disc, is skipped, and a different one comes in beside it. Mac and
/// Amiga apps have no fingerprints yet, so each is imported once and not
/// when the library already has it.
pub fn import_apps_from_dir(library: &Library, root: &Path) -> Result<AppsImport, String> {
    let mut report = AppsImport::default();
    library.backfill_program_ids()?;
    let apps = library.list()?;
    // DOS program fingerprints in the library before this import, and
    // since: each version comes in once.
    let had: HashSet<(u64, String)> = apps
        .iter()
        .filter(|a| a.os == GuestOs::Dos)
        .flat_map(|a| a.program_ids.values().map(|id| (id.size, id.sha256.clone())))
        .collect();
    let mut have = had.clone();
    let mut done: HashSet<&'static str> = HashSet::new();
    let scratch = library.run_dir().join(format!("apps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter(|e| e.file_type().is_file() || (e.file_type().is_dir() && e.path().extension().is_some_and(|x| x == "app")))
        .map(|e| e.into_path())
        .collect();
    // Nearest the root first: a copy gathered once sits there, and
    // copies from other drives sit in a folder per drive.
    files.sort_by(|a, b| a.components().count().cmp(&b.components().count()).then_with(|| a.cmp(b)));
    for file in files {
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(h) = HANDLERS.iter().find(|h| h.programs.iter().any(|p| p.eq_ignore_ascii_case(&name))) else { continue };
        let label = format!("{} ({})", h.name, crate::library::guest_label(h.os));
        if h.os == GuestOs::Dos {
            let (Ok(meta), Ok(sha256)) = (std::fs::metadata(&file), crate::cd::sha256_of(&file)) else { continue };
            let id = (meta.len(), sha256);
            if have.contains(&id) {
                if had.contains(&id) && !report.already.contains(&label) {
                    report.already.push(label);
                }
                continue;
            }
        } else {
            if done.contains(h.name) {
                continue;
            }
            if in_library(h, &apps) {
                done.insert(h.name);
                report.already.push(label);
                continue;
            }
        }
        let parent = file.parent().unwrap_or(root);
        let source = if parent != root {
            parent.to_path_buf()
        } else if h.os == GuestOs::MacClassic {
            // A folder for the app and its AppleDouble file, so the import
            // merges the forks back.
            let dir = scratch.join(&name);
            let copied = std::fs::create_dir_all(&dir).and_then(|_| std::fs::copy(&file, dir.join(&name))).and_then(|_| {
                let ad = root.join(format!("._{name}"));
                if ad.is_file() {
                    std::fs::copy(&ad, dir.join(format!("._{name}")))?;
                }
                Ok(())
            });
            if let Err(e) = copied {
                report.failed.push(format!("{label}: {e}"));
                continue;
            }
            dir
        } else {
            file.clone()
        };
        match library.import(h.os, &source) {
            Ok(app) => {
                // Its folder may hold other handler programs: they came too.
                have.extend(app.program_ids.values().map(|id| (id.size, id.sha256.clone())));
                let another = !done.insert(h.name) || in_library(h, &apps);
                report.imported.push(if another { format!("{label}, another version ({})", app.name) } else { label });
            }
            Err(e) => report.failed.push(format!("{label}: {e}")),
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    #[test]
    fn every_handler_is_in_the_living_document() {
        let doc = include_str!("../../docs/app-handlers.md");
        for h in HANDLERS {
            let first = h.name.split(" (").next().unwrap();
            assert!(doc.contains(first), "{} is missing from docs/app-handlers.md", h.name);
            for p in h.programs {
                assert!(doc.contains(&format!("`{p}`")), "{p} ({}) is missing from docs/app-handlers.md", h.name);
            }
            if let Some(c) = h.creator {
                assert!(doc.contains(&format!("`{c}`")), "{}'s creator {c} is missing", h.name);
            }
        }
    }

    #[test]
    fn the_known_versions_table_parses_and_names_real_handlers() {
        let doc = include_str!("../../docs/app-handlers.md");
        let known = parse_known_versions(doc).expect("docs/app-handlers.md's Known versions table");
        for k in &known {
            assert!(find(k.os, &k.app).is_some(), "{} {} isn't a handler above", k.app, k.version);
            assert!(!k.version.is_empty(), "{}: a known version needs its version", k.app);
        }
        let mut shas: Vec<&str> = known.iter().map(|k| k.sha256.as_str()).collect();
        shas.sort();
        let n = shas.len();
        shas.dedup();
        assert_eq!(shas.len(), n, "a fingerprint is listed twice");
    }

    const SAMPLE: &str = "<!-- versions:start -->
| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |
|---|---|---|---|---|---|---|---|---|
| DOS | WordPerfect | 5.1 | `WP.EXE` | 1,234 | `ABABABABABABABABABABABABABABABABABABABABABABABABABABABABABABABAB` | 3 | 1 | 2026-09-27 |
<!-- versions:end -->";

    #[test]
    fn a_program_matching_a_known_version_identifies_the_app() {
        let known = parse_known_versions(SAMPLE).unwrap();
        assert_eq!(known.len(), 1);
        assert_eq!((known[0].size, known[0].sha256.as_str(), known[0].worked), (1234, "ab".repeat(32).as_str(), 3));
        let ids = |size, sha: &str| BTreeMap::from([("WP.EXE".to_string(), ProgramId { size, sha256: sha.into() })]);
        let id = identify(GuestOs::Dos, &ids(1234, &"ab".repeat(32)), &known).unwrap();
        assert_eq!((id.handler.as_deref(), id.version.as_deref(), id.by), (Some("WordPerfect"), Some("5.1"), IdentifiedBy::Hash));
        // Same name, different bytes: not identified.
        assert!(identify(GuestOs::Dos, &ids(1234, &"cd".repeat(32)), &known).is_none());
        // A bad row fails loudly.
        assert!(parse_known_versions(&SAMPLE.replace("| 3 |", "| x |")).is_err());
        assert!(parse_known_versions(&SAMPLE.replace("ABAB", "ZZ")).is_err());
    }

    #[test]
    fn candidates_go_by_program_name_only() {
        let names = |p| candidates(GuestOs::Dos, p).iter().map(|h| h.name).collect::<Vec<_>>();
        assert_eq!(names("WORD5/word.exe"), ["Microsoft Word (DOS)"]);
        assert_eq!(names("LOTUS.COM"), ["Lotus 1-2-3"]);
        assert!(names("GAME.EXE").is_empty());
        assert!(catalog(GuestOs::Dos).iter().any(|h| h.name == "WordPerfect" && h.exts.contains(&"WP5")));
    }

    #[test]
    fn an_apps_disc_brings_in_each_version_once() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let disc = t.path().join("disc");
        // Two WordPerfect versions, one of them twice (from two drives).
        for (dir, exe) in [("WP51", "MZ 5.1"), ("WP60", "MZ 6.0"), ("Backup 2/WP51", "MZ 5.1")] {
            fs::create_dir_all(disc.join(dir)).unwrap();
            fs::write(disc.join(dir).join("WP.EXE"), exe).unwrap();
        }
        let report = import_apps_from_dir(&lib, &disc).unwrap();
        assert_eq!(report.imported.len(), 2, "{report:?}");
        assert_eq!(report.imported[0], "WordPerfect (DOS)");
        assert!(report.imported[1].starts_with("WordPerfect (DOS), another version"), "{report:?}");
        assert!(report.already.is_empty(), "{report:?}");
        let apps = lib.list().unwrap();
        assert_eq!(apps.len(), 2);
        assert!(apps.iter().all(|a| a.program_ids["WP.EXE"].size == 6));

        // A third version later comes in beside them; the others are already there.
        fs::create_dir_all(disc.join("WP42")).unwrap();
        fs::write(disc.join("WP42/WP.EXE"), "MZ 4.2").unwrap();
        let again = import_apps_from_dir(&lib, &disc).unwrap();
        assert_eq!(again.imported.len(), 1, "{again:?}");
        assert!(again.imported[0].contains("another version"));
        assert_eq!(again.already, ["WordPerfect (DOS)"]);
    }

    #[test]
    fn the_wanted_list_asks_for_apps_not_in_the_library() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let wp = t.path().join("WP51");
        fs::create_dir_all(&wp).unwrap();
        fs::write(wp.join("WP.EXE"), b"MZ").unwrap();
        lib.import(GuestOs::Dos, &wp).unwrap();

        let list = wanted_list(&lib.list().unwrap()).unwrap();
        let lines: Vec<&str> = list.lines().collect();
        assert_eq!(&lines[..3], ["#columns: name size sha1", "#gather: folder", "#forks: appledouble"]);
        assert!(!lines.contains(&"WP.EXE"), "WordPerfect is already in the library");
        for wanted in ["123.EXE", "MacWrite II", "ClarisWorks", "DPaint"] {
            assert!(lines.contains(&wanted), "{wanted}");
        }
        // Name lines are plain names (no tabs), each once.
        let names: Vec<&&str> = lines.iter().filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        assert!(names.iter().all(|l| !l.contains('\t')));
        assert_eq!(names.len(), names.iter().map(|l| l.to_lowercase()).collect::<HashSet<_>>().len());
    }

    #[test]
    fn an_apps_disc_brings_in_each_app_once_with_its_folder() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let disc = t.path().join("disc");
        // A gathered DOS app folder, the same app from a second drive, an
        // Amiga app folder, and a Mac app at the root with its forks.
        fs::create_dir_all(disc.join("WP51")).unwrap();
        fs::write(disc.join("WP51/WP.EXE"), b"MZ").unwrap();
        fs::write(disc.join("WP51/WP.FIL"), b"support").unwrap();
        fs::create_dir_all(disc.join("Backup 2/WP51")).unwrap();
        fs::write(disc.join("Backup 2/WP51/WP.EXE"), b"MZ").unwrap();
        fs::create_dir_all(disc.join("DPaintIV")).unwrap();
        fs::write(disc.join("DPaintIV/DPaint"), [0x00, 0x00, 0x03, 0xF3, 0, 0, 0, 0]).unwrap();
        fs::write(disc.join("notes.txt"), b"not an app").unwrap();
        fs::write(disc.join("MacWrite II"), b"").unwrap();

        let report = import_apps_from_dir(&lib, &disc).unwrap();
        assert_eq!(report.imported, ["Deluxe Paint (Amiga)", "WordPerfect (DOS)"], "{report:?}");
        let apps = lib.list().unwrap();
        let wp = apps.iter().find(|a| a.os == GuestOs::Dos).unwrap();
        assert!(lib.os_root(GuestOs::Dos).join(&wp.dir).join("WP.FIL").exists(), "support files came along");
        // The Mac app has no resource fork here, so it isn't an app Floppy
        // can find, and says so.
        assert!(report.failed.iter().any(|f| f.starts_with("MacWrite II (Classic Mac)")), "{report:?}");

        // Again: everything found is already in the library.
        let again = import_apps_from_dir(&lib, &disc).unwrap();
        assert!(again.imported.is_empty());
        assert_eq!(again.already, ["Deluxe Paint (Amiga)", "WordPerfect (DOS)"]);
    }
}
