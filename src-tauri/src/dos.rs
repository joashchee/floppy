//! DOS guest specifics: 8.3 file names, picking the program to run, and
//! the DOSBox config that boots it.
//!
//! DOSBox shows host files with long names under mangled 8.3 aliases
//! (`LONGFI~1.TXT`), and those aliases can't be predicted reliably from
//! outside. So an import renames anything DOS can't see as-is to a
//! generated 8.3 name (`normalize_tree`). A DOS program can only refer
//! to 8.3 names anyway, so a long name in a DOS program's folder is
//! almost always something added later, like a modern README.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;

use walkdir::WalkDir;

/// Punctuation DOS allows in a file name, besides letters and digits.
const SPECIAL: &str = "!#$%&'()-@^_`{}~";

fn valid_83_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || SPECIAL.contains(c)
}

/// Whether DOS can see `name` as-is. Case doesn't matter: DOSBox maps
/// host names case-insensitively.
pub fn is_valid_83(name: &str) -> bool {
    let (base, ext) = match name.rsplit_once('.') {
        Some((b, e)) => (b, Some(e)),
        None => (name, None),
    };
    if base.is_empty() || base.len() > 8 || !base.chars().all(valid_83_char) {
        return false;
    }
    match ext {
        None => true,
        Some(e) => !e.is_empty() && e.len() <= 3 && e.chars().all(valid_83_char),
    }
}

/// Uppercase, 8.3-legal characters of `s`, at most `max` of them.
fn clean(s: &str, max: usize) -> String {
    s.chars()
        .filter(|c| valid_83_char(*c))
        .map(|c| c.to_ascii_uppercase())
        .take(max)
        .collect()
}

/// An 8.3 name derived from `name` that isn't in `taken` (uppercase
/// names). A directory's whole name is its stem, so "Game v1.2" becomes
/// `GAMEV12`, not `GAMEV1.2`.
pub fn to_83(name: &str, taken: &HashSet<String>, is_dir: bool) -> String {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !is_dir && !s.is_empty() => (s, e),
        _ => (name, ""),
    };
    let mut base = clean(stem, 8);
    if base.is_empty() {
        base = if is_dir { "DIR" } else { "FILE" }.to_string();
    }
    let ext = clean(ext, 3);
    let join = |b: &str| {
        if ext.is_empty() {
            b.to_string()
        } else {
            format!("{b}.{ext}")
        }
    };
    let first = join(&base);
    if !taken.contains(&first) {
        return first;
    }
    (1u32..)
        .map(|n| {
            let suffix = format!("~{n}");
            let keep = 8usize.saturating_sub(suffix.len());
            join(&format!("{}{suffix}", base.chars().take(keep).collect::<String>()))
        })
        .find(|cand| !taken.contains(cand))
        .expect("unbounded range always finds a free name")
}

/// Renames every file and folder under `root` that DOS can't see as-is
/// to a unique 8.3 name. `root` itself is left alone.
pub fn normalize_tree(root: &Path) -> io::Result<()> {
    let mut entries: Vec<(String, bool)> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        entries.push((name, entry.file_type()?.is_dir()));
    }
    let mut taken: HashSet<String> = entries
        .iter()
        .filter(|(n, _)| is_valid_83(n))
        .map(|(n, _)| n.to_ascii_uppercase())
        .collect();
    for (name, is_dir) in entries {
        let name = if is_valid_83(&name) {
            name
        } else {
            let new = to_83(&name, &taken, is_dir);
            fs::rename(root.join(&name), root.join(&new))?;
            taken.insert(new.clone());
            new
        };
        if is_dir {
            normalize_tree(&root.join(name))?;
        }
    }
    Ok(())
}

/// Stems of programs that set a DOS app up rather than run it.
const NOT_THE_APP: &[&str] = &[
    "install", "setup", "config", "setsound", "sound", "sndsetup", "ipxsetup", "uninst", "uninstal",
    "deinstal", "readme", "register", "order", "catalog", "vendor", "update", "patch",
];

/// How likely `rel` (a `/`-separated path under the app folder) is the
/// program to launch. Shallow paths win, as does a name matching the
/// app's, and installers and setup tools lose.
fn score(rel: &str, name_hint: &str) -> i32 {
    let depth = rel.matches('/').count() as i32;
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let (stem, ext) = file.rsplit_once('.').unwrap_or((file, ""));
    let stem = stem.to_ascii_lowercase();
    let hint = clean(name_hint, 8).to_ascii_lowercase();
    let mut s = -depth * 10;
    if !hint.is_empty() {
        if stem == hint {
            s += 50;
        } else if stem.len() >= 3 && hint.starts_with(&stem) {
            s += 25;
        }
    }
    if NOT_THE_APP.contains(&stem.as_str()) || stem.starts_with("install") || stem.starts_with("setup") {
        s -= 40;
    }
    if ext.eq_ignore_ascii_case("bat") {
        s -= 2;
    }
    s
}

/// DOS programs (`.exe`, `.com`, `.bat`) under `root`, best launch
/// candidate first, as `/`-separated paths relative to `root`.
pub fn find_programs(root: &Path, name_hint: &str) -> Vec<String> {
    let mut found: Vec<String> = WalkDir::new(root)
        .min_depth(1)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| ["exe", "com", "bat"].iter().any(|p| x.eq_ignore_ascii_case(p)))
        })
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?;
            let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
            Some(parts?.join("/"))
        })
        .collect();
    found.sort_by(|a, b| score(b, name_hint).cmp(&score(a, name_hint)).then_with(|| a.cmp(b)));
    found.truncate(200);
    found
}

/// What to boot: the whole guest library is drive C:, so apps can see
/// each other (a DOS word processor can reach PKZIP's folder).
pub struct Launch<'a> {
    pub mount_root: &'a Path,
    /// The app's folder under `mount_root` (an 8.3 name).
    pub app_dir: &'a str,
    /// Program to run, relative to `app_dir`; `None` stops at a prompt
    /// in the app's folder.
    pub program: Option<&'a str>,
    /// Quit DOSBox when the program exits.
    pub exit_after: bool,
    /// Passed to the program, as DOS sees it (a document's
    /// `C:\DOCS\LETTER.WP5`).
    pub args: Option<&'a str>,
}

/// The config file Floppy passes to DOSBox Staging for one launch.
pub fn dosbox_conf(l: &Launch) -> Result<String, String> {
    let root = l.mount_root.to_str().ok_or("The library path isn't valid UTF-8.")?;
    if root.contains('"') {
        return Err("The library path contains a double quote, which DOSBox can't mount.".into());
    }
    let (dir, file) = match l.program {
        Some(p) => match p.rsplit_once('/') {
            Some((d, f)) => (format!("{}\\{}", l.app_dir, d.replace('/', "\\")), Some(f)),
            None => (l.app_dir.to_string(), Some(p)),
        },
        None => (l.app_dir.to_string(), None),
    };
    let mut s = String::from("# Written by Floppy for each launch. Changes here are overwritten.\n");
    s += "[autoexec]\n@echo off\n";
    s += &format!("mount c \"{root}\"\n");
    s += "c:\n";
    s += &format!("cd \\{dir}\n");
    if let Some(args) = l.args {
        // One autoexec line: a line break would run a second command.
        if args.contains(['\n', '\r']) {
            return Err("The program's arguments can't contain a line break.".into());
        }
    }
    if let Some(f) = file {
        let args = l.args.map(|a| format!(" {a}")).unwrap_or_default();
        // A batch file run without CALL never returns to the next line.
        if f.to_ascii_lowercase().ends_with(".bat") {
            s += &format!("call {f}{args}\n");
        } else {
            s += &format!("{f}{args}\n");
        }
        if l.exit_after {
            s += "exit\n";
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::path::PathBuf;

    fn set(names: &[&str]) -> HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn valid_83_names() {
        for ok in ["PKZIP.EXE", "wp.exe", "README", "A.B", "GO.BAT", "~$TMP", "12345678.123"] {
            assert!(is_valid_83(ok), "{ok}");
        }
        for bad in ["", ".hidden", "TOOLONGNAME.EXE", "A.BCDE", "two.dots.txt", "has space", "ÉTÉ.TXT", "X."] {
            assert!(!is_valid_83(bad), "{bad}");
        }
    }

    #[test]
    fn to_83_shortens_and_dedupes() {
        let none = HashSet::new();
        assert_eq!(to_83("Read Me First.txt", &none, false), "READMEFI.TXT");
        assert_eq!(to_83("Game v1.2", &none, true), "GAMEV12");
        assert_eq!(to_83("été", &none, true), "T");
        assert_eq!(to_83("...", &none, true), "DIR");
        let taken = set(&["WORDPERF", "WORDPE~1"]);
        assert_eq!(to_83("WordPerfect 5.1", &taken, true), "WORDPE~2");
        assert_eq!(to_83("notes.markdown", &set(&["NOTES.MAR"]), false), "NOTES~1.MAR");
    }

    #[test]
    fn normalize_renames_only_long_names() {
        let t = TempDir::new();
        let root = t.path();
        fs::create_dir_all(root.join("Sound Files")).unwrap();
        fs::write(root.join("Sound Files/Intro Theme.mod"), b"").unwrap();
        fs::write(root.join("GAME.EXE"), b"").unwrap();
        fs::write(root.join("Read Me.txt"), b"").unwrap();
        fs::write(root.join("README.TXT"), b"").unwrap();
        normalize_tree(root).unwrap();
        assert!(root.join("GAME.EXE").exists());
        assert!(root.join("README.TXT").exists());
        // "Read Me.txt" collides with the existing README.TXT.
        assert!(root.join("README~1.TXT").exists());
        assert!(root.join("SOUNDFIL/INTROTHE.MOD").exists());
    }

    #[test]
    fn programs_ranked_app_first() {
        let t = TempDir::new();
        let root = t.path();
        fs::create_dir_all(root.join("UTILS")).unwrap();
        for f in ["INSTALL.EXE", "SETUP.EXE", "WP.EXE", "GO.BAT", "UTILS/CONVERT.EXE", "WP.HLP"] {
            fs::write(root.join(f), b"").unwrap();
        }
        let found = find_programs(root, "WP");
        assert_eq!(found[0], "WP.EXE");
        assert_eq!(found.len(), 5);
        assert!(found.iter().position(|p| p == "INSTALL.EXE") > found.iter().position(|p| p == "UTILS/CONVERT.EXE"));
    }

    #[test]
    fn conf_runs_program_in_its_folder() {
        let root = PathBuf::from("/lib/dos");
        let conf = dosbox_conf(&Launch { mount_root: &root, app_dir: "WP51", program: Some("BIN/WP.EXE"), exit_after: true, args: None }).unwrap();
        assert!(conf.contains("mount c \"/lib/dos\"\n"));
        assert!(conf.contains("cd \\WP51\\BIN\nWP.EXE\nexit\n"));
        let conf = dosbox_conf(&Launch { mount_root: &root, app_dir: "GAME", program: Some("GO.BAT"), exit_after: false, args: None }).unwrap();
        assert!(conf.contains("call GO.BAT\n"));
        assert!(!conf.contains("exit"));
        let conf = dosbox_conf(&Launch { mount_root: &root, app_dir: "GAME", program: None, exit_after: true, args: None }).unwrap();
        assert!(conf.ends_with("cd \\GAME\n"));
    }

    #[test]
    fn conf_opens_a_document_with_the_program() {
        let root = PathBuf::from("/lib/dos");
        let launch = |program, args| Launch { mount_root: &root, app_dir: "WP51", program: Some(program), exit_after: true, args: Some(args) };
        let conf = dosbox_conf(&launch("WP.EXE", "C:\\DOCS\\LETTER.WP5")).unwrap();
        assert!(conf.contains("cd \\WP51\nWP.EXE C:\\DOCS\\LETTER.WP5\nexit\n"), "{conf}");
        assert!(dosbox_conf(&launch("GO.BAT", "C:\\DOCS\\A.TXT")).unwrap().contains("call GO.BAT C:\\DOCS\\A.TXT\n"));
        // A second autoexec command can't be smuggled in.
        assert!(dosbox_conf(&launch("WP.EXE", "A.TXT\ndel *.*")).is_err());
    }

    #[test]
    fn conf_rejects_quote_in_path() {
        let root = PathBuf::from("/odd\"path");
        assert!(dosbox_conf(&Launch { mount_root: &root, app_dir: "A", program: None, exit_after: true, args: None }).is_err());
    }
}
