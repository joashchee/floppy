//! The app library: one folder per guest OS under Floppy's app-data dir
//! (`library/dos/`, `library/mac/`, `library/amiga/`), each shared into
//! its emulator as a whole, plus a small JSON manifest
//! (`library/library.json`) naming what's in it and which guest system
//! files (ROMs, boot disks) the user has added (`library/system/<os>/`).
//!
//! An import copies the source (a folder, a `.zip`, or a single file)
//! into a staging folder first. It's then prepared for its guest (8.3
//! names for DOS, resource forks merged back for the Mac) and moved into
//! place only once it's known to contain something to launch, so a failed
//! import never leaves a half-copied app in the guest's drive.

use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{amiga, cd, documents, dos, handlers, mac};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum GuestOs {
    Dos,
    MacClassic,
    Amiga,
}

impl GuestOs {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "dos" => Some(GuestOs::Dos),
            "mac-classic" | "mac" => Some(GuestOs::MacClassic),
            "amiga" => Some(GuestOs::Amiga),
            _ => None,
        }
    }

    fn dir_name(self) -> &'static str {
        match self {
            GuestOs::Dos => "dos",
            GuestOs::MacClassic => "mac",
            GuestOs::Amiga => "amiga",
        }
    }

    /// Whether launches share a writable boot disk, so only one of this
    /// guest's apps may run at a time.
    pub fn single_instance(self) -> bool {
        self != GuestOs::Dos
    }

    /// Whether `dir` is a name this guest's import could have made: a
    /// single path component, so a damaged manifest can never point a
    /// delete somewhere else.
    fn is_own_dir_name(self, dir: &str) -> bool {
        match self {
            GuestOs::Dos => dos::is_valid_83(dir),
            GuestOs::MacClassic => mac::sanitize_name(dir, mac::MAX_NAME, "") == dir,
            GuestOs::Amiga => mac::sanitize_name(dir, amiga::MAX_NAME, "") == dir,
        }
    }
}

/// What the user calls a guest, as the app's tabs name it.
pub fn guest_label(os: GuestOs) -> &'static str {
    match os {
        GuestOs::Dos => "DOS",
        GuestOs::MacClassic => "Classic Mac",
        GuestOs::Amiga => "Amiga",
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryApp {
    pub id: String,
    pub os: GuestOs,
    /// Display name, editable; starts as the imported folder/zip/file name.
    pub name: String,
    /// The app's folder under the guest OS's library folder.
    pub dir: String,
    /// What to launch, relative to `dir`, `/`-separated.
    pub program: Option<String>,
    /// Everything launchable found at import, best guess first.
    pub programs: Vec<String>,
    /// File name (never the full path) of what was imported.
    pub source_name: String,
    /// Unix seconds.
    pub added: u64,
    /// Document extensions the user says this app opens (uppercase, no
    /// dot), on top of the well-known ones (documents.rs).
    #[serde(default)]
    pub opens: Vec<String>,
    /// Each program's size and SHA-256, keyed like `programs`. They tell
    /// versions apart, and apps whose programs share a name (DOS only).
    #[serde(default)]
    pub program_ids: BTreeMap<String, ProgramId>,
    /// Which handler app this is, and its version, once known
    /// (handlers.rs). `None` until a program matches a known version or
    /// the user says.
    #[serde(default)]
    pub identity: Option<Identity>,
    /// The version to open its handler's documents with, when the library
    /// has several. At most one app per handler has it.
    #[serde(default)]
    pub favorite: bool,
    /// Problems the user noted running it, for Export Findings.
    #[serde(default)]
    pub errors: String,
    /// The user renamed it, so saying what it is keeps their name.
    #[serde(default)]
    pub named_by_user: bool,
    /// The user chose to share its errors in Export Findings though it
    /// isn't a known app (so its name goes too). Known apps' always go.
    #[serde(default)]
    pub share_errors: bool,
}

impl LibraryApp {
    /// The handler this app is known to be. `None` when it isn't known
    /// yet, or is known not to be one.
    pub fn handler(&self) -> Option<&str> {
        self.identity.as_ref().and_then(|i| i.handler.as_deref())
    }

    /// Takes its identity, and the name that says it ("WordPerfect 5.1").
    /// An app that isn't a known one, or one the user named, keeps its
    /// name.
    fn identify_as(&mut self, identity: Option<Identity>) {
        if let Some(name) = identity.as_ref().and_then(Identity::name).filter(|_| !self.named_by_user) {
            self.name = name;
        }
        self.identity = identity;
    }
}

/// The longest Errors note kept.
const MAX_ERRORS: usize = 4000;

/// A program file's fingerprint.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ProgramId {
    pub size: u64,
    pub sha256: String,
}

/// What an app is: a handler (by its name in `HANDLERS`) and version,
/// or `handler: None` for an app that isn't one of them.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub handler: Option<String>,
    pub version: Option<String>,
    pub by: IdentifiedBy,
}

impl Identity {
    /// The app's name for it: the handler and version, the version before
    /// a trailing qualifier ("Microsoft Word 5.5 (DOS)"). `None` for an
    /// app that isn't a known one.
    pub fn name(&self) -> Option<String> {
        let handler = self.handler.as_deref()?;
        let Some(version) = self.version.as_deref().filter(|v| !v.is_empty()) else { return Some(handler.to_string()) };
        Some(match handler.rsplit_once(" (") {
            Some((base, qualifier)) if handler.ends_with(')') => format!("{base} {version} ({qualifier}"),
            _ => format!("{handler} {version}"),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum IdentifiedBy {
    /// A program matched a known version's SHA-256 (`docs/app-handlers.md`).
    Hash,
    /// The user said, after a session or in the app's details.
    User,
}

/// An old file in the library, to open in an app that made it
/// (documents.rs).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDoc {
    pub id: String,
    pub os: GuestOs,
    /// Its original name, shown and used on export.
    pub name: String,
    /// Where it is, relative to the guest's library folder, `/`-separated
    /// (`DOCS/LETTER.WP5`: an 8.3 name for DOS).
    pub file: String,
    /// The app it last opened with, offered first next time.
    pub opens_with: Option<String>,
    /// Unix seconds.
    pub added: u64,
}

/// The guest system files the user supplied for one guest OS, as file
/// names under `library/system/<os>/`. Floppy never ships these.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestSystem {
    /// Mac ROM, or Amiga Kickstart ROM.
    pub rom: Option<String>,
    /// Mac startup disk image, or Amiga Workbench (a floppy image, a
    /// hard-disk file, or a folder).
    pub boot: Option<String>,
    /// FS-UAE Amiga model. Set from the Kickstart version, changeable.
    pub model: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum SystemFile {
    Rom,
    Boot,
}

#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    apps: Vec<LibraryApp>,
    #[serde(default)]
    documents: Vec<LibraryDoc>,
    #[serde(default)]
    systems: BTreeMap<GuestOs, GuestSystem>,
}

impl Default for Manifest {
    fn default() -> Self {
        Manifest { version: 1, apps: Vec::new(), documents: Vec::new(), systems: BTreeMap::new() }
    }
}

/// Uncompressed size cap for one imported zip — far beyond any old
/// program, low enough to stop a zip bomb filling the disk.
const MAX_ZIP_BYTES: u64 = 4 << 30;

pub struct Library {
    root: PathBuf,
    /// Serializes manifest read-modify-write and imports.
    lock: Mutex<()>,
}

impl Library {
    pub fn new(root: PathBuf) -> Self {
        Library { root, lock: Mutex::new(()) }
    }

    #[cfg(test)]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn os_root(&self, os: GuestOs) -> PathBuf {
        self.root.join(os.dir_name())
    }

    /// Where the user's system files for `os` are kept. Outside the
    /// guest's shared folder, so the guest never sees its own ROM.
    pub fn system_dir(&self, os: GuestOs) -> PathBuf {
        self.root.join("system").join(os.dir_name())
    }

    /// What Floppy remembers about files discs (discs.rs).
    pub fn files_discs_path(&self) -> PathBuf {
        self.root.join("files-discs.json")
    }

    /// Whether apps opened file types correctly, from the user (verify.rs).
    pub fn verifications_path(&self) -> PathBuf {
        self.root.join("verifications.json")
    }

    /// What Export Findings has already shared (findings.rs).
    pub fn findings_path(&self) -> PathBuf {
        self.root.join("findings.json")
    }

    /// What the user reported about getting setup files (findings.rs).
    pub fn setup_reports_path(&self) -> PathBuf {
        self.root.join("setup-reports.json")
    }

    /// Emulators the user located themselves (emulator.rs).
    pub fn emulators_path(&self) -> PathBuf {
        self.root.join("emulators.json")
    }

    /// Per-launch config files and emulator scratch space.
    pub fn run_dir(&self) -> PathBuf {
        self.root.join("run")
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("library.json")
    }

    fn load(&self) -> Result<Manifest, String> {
        match fs::read(self.manifest_path()) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("The library list is damaged: {e}")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Manifest::default()),
            Err(e) => Err(format!("Couldn't read the library list: {e}")),
        }
    }

    fn save(&self, m: &Manifest) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let tmp = self.root.join("library.json.tmp");
        let json = serde_json::to_vec_pretty(m).map_err(|e| e.to_string())?;
        fs::write(&tmp, json).and_then(|_| fs::rename(&tmp, self.manifest_path()))
            .map_err(|e| format!("Couldn't save the library list: {e}"))
    }

    fn guard(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn list(&self) -> Result<Vec<LibraryApp>, String> {
        let _g = self.guard();
        Ok(self.load()?.apps)
    }

    pub fn get(&self, id: &str) -> Result<LibraryApp, String> {
        self.list()?.into_iter().find(|a| a.id == id).ok_or_else(|| "That app is no longer in the library.".into())
    }

    /// Changes one app's manifest entry and saves.
    fn update(&self, id: &str, f: impl FnOnce(&mut LibraryApp) -> Result<(), String>) -> Result<LibraryApp, String> {
        let _g = self.guard();
        let mut m = self.load()?;
        let app = m.apps.iter_mut().find(|a| a.id == id).ok_or("That app is no longer in the library.")?;
        f(app)?;
        let out = app.clone();
        self.save(&m)?;
        Ok(out)
    }

    pub fn set_program(&self, id: &str, program: Option<String>) -> Result<LibraryApp, String> {
        self.update(id, |a| {
            if let Some(p) = &program {
                if !a.programs.contains(p) {
                    return Err(format!("{p} isn't one of this app's programs."));
                }
            }
            a.program = program;
            Ok(())
        })
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<LibraryApp, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A name can't be empty.".into());
        }
        self.update(id, |a| {
            a.name = name.to_string();
            a.named_by_user = true;
            Ok(())
        })
    }

    /// Deletes the app's folder from the guest drive and drops it from
    /// the manifest.
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let _g = self.guard();
        let mut m = self.load()?;
        let idx = m.apps.iter().position(|a| a.id == id).ok_or("That app is no longer in the library.")?;
        let app = &m.apps[idx];
        if !app.os.is_own_dir_name(&app.dir) {
            return Err(format!("Refusing to delete an unexpected folder name: {}", app.dir));
        }
        let dir = self.os_root(app.os).join(&app.dir);
        match fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Couldn't delete {}: {e}", app.dir)),
        }
        m.apps.remove(idx);
        self.save(&m)
    }

    /// Absolute path of an app's folder (for Reveal in Finder).
    pub fn app_path(&self, id: &str) -> Result<PathBuf, String> {
        let app = self.get(id)?;
        Ok(self.os_root(app.os).join(app.dir))
    }

    pub fn import(&self, os: GuestOs, src: &Path) -> Result<LibraryApp, String> {
        let _g = self.guard();
        let src_name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or("That path has no file name.")?;
        let meta = fs::metadata(src).map_err(|e| format!("Couldn't open {src_name}: {e}"))?;
        let keep_forks = os == GuestOs::MacClassic;

        let staging = Staging::new(&self.root)?;
        let name = if meta.is_dir() {
            copy_tree(src, staging.path(), keep_forks).map_err(|e| format!("Couldn't copy {src_name}: {e}"))?;
            src_name.clone()
        } else {
            let (stem, ext) = match src_name.rsplit_once('.') {
                Some((s, e)) if !s.is_empty() => (s.to_string(), e.to_ascii_lowercase()),
                _ => (src_name.clone(), String::new()),
            };
            if ext == "zip" {
                extract_zip(src, staging.path(), keep_forks).map_err(|e| format!("Couldn't unpack {src_name}: {e}"))?;
                stem
            } else {
                import_file(os, src, &src_name, &ext, staging.path())?.unwrap_or(stem)
            }
        };
        if keep_forks {
            mac::absorb_appledouble(staging.path()).map_err(|e| format!("Couldn't restore Mac resource forks: {e}"))?;
        }

        let content = single_subdir(staging.path()).map_err(|e| e.to_string())?;
        let programs = match os {
            GuestOs::Dos => {
                dos::normalize_tree(&content).map_err(|e| format!("Couldn't rename files for DOS: {e}"))?;
                dos::find_programs(&content, &name)
            }
            GuestOs::MacClassic => mac::find_programs(&content, &name),
            GuestOs::Amiga => amiga::find_programs(&content, &name),
        };
        if programs.is_empty() {
            return Err(match os {
                GuestOs::Dos => format!("No DOS program (.exe, .com, .bat) found in {src_name}."),
                GuestOs::MacClassic => format!(
                    "No classic Mac app, disk image or StuffIt archive found in {src_name}. \
                     If it was zipped or copied on a non-Mac system, its resource forks may have been lost."
                ),
                GuestOs::Amiga => format!("No Amiga program or disk image (.adf, .adz, .dms, .hdf) found in {src_name}."),
            });
        }

        let program_ids = if os == GuestOs::Dos { program_ids(&content, &programs) } else { BTreeMap::new() };
        let identity = handlers::identify(os, &program_ids, handlers::known_versions());

        let os_root = self.os_root(os);
        fs::create_dir_all(&os_root).map_err(|e| e.to_string())?;
        let mut taken: HashSet<String> = fs::read_dir(&os_root)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().to_ascii_uppercase())
            .collect();
        // The documents folder is never an app's.
        taken.insert(documents::DOS_DOCS_DIR.to_string());
        let dir = match os {
            GuestOs::Dos => dos::to_83(&name, &taken, true),
            GuestOs::MacClassic => unique_name(&mac::sanitize_name(&name, mac::MAX_NAME, "App"), &taken, mac::MAX_NAME),
            GuestOs::Amiga => unique_name(&mac::sanitize_name(&name, amiga::MAX_NAME, "App"), &taken, amiga::MAX_NAME),
        };
        fs::rename(&content, os_root.join(&dir)).map_err(|e| format!("Couldn't move {src_name} into the library: {e}"))?;

        let mut m = self.load()?;
        // The folder name was free on disk, so an entry still naming it is
        // stale (its folder was deleted outside Floppy).
        m.apps.retain(|a| !(a.os == os && a.dir == dir));
        let mut app = LibraryApp {
            id: unique_id(os, &dir, &m.apps),
            os,
            name,
            dir,
            program: programs.first().cloned(),
            programs,
            source_name: src_name,
            added: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            opens: Vec::new(),
            program_ids,
            identity: None,
            favorite: false,
            errors: String::new(),
            named_by_user: false,
            share_errors: false,
        };
        app.identify_as(identity);
        m.apps.push(app.clone());
        self.save(&m)?;
        Ok(app)
    }

    /// Fingerprints the programs of DOS apps imported before Floppy kept
    /// fingerprints, and recognizes any that match a known version.
    /// Cheap once done: only apps without fingerprints are read.
    pub fn backfill_program_ids(&self) -> Result<(), String> {
        let _g = self.guard();
        let mut m = self.load()?;
        let mut changed = false;
        for app in m.apps.iter_mut().filter(|a| a.os == GuestOs::Dos && a.program_ids.is_empty() && !a.programs.is_empty()) {
            if !app.os.is_own_dir_name(&app.dir) {
                continue;
            }
            app.program_ids = program_ids(&self.os_root(app.os).join(&app.dir), &app.programs);
            if app.identity.is_none() {
                app.identify_as(handlers::identify(app.os, &app.program_ids, handlers::known_versions()));
            }
            changed |= !app.program_ids.is_empty();
        }
        if changed {
            self.save(&m)?;
        }
        Ok(())
    }

    /// Says what an app is (`None`: not known yet), as the user confirmed
    /// it, and names it for that ("WordPerfect 5.1"). A handler must be
    /// one of its guest's in `HANDLERS`. Changing the handler drops the
    /// app's favorite mark.
    pub fn set_identity(&self, id: &str, identity: Option<(Option<String>, Option<String>)>) -> Result<LibraryApp, String> {
        self.update(id, |a| {
            let identity = match identity {
                None => None,
                Some((handler, version)) => {
                    let handler = match handler.as_deref().map(str::trim).filter(|h| !h.is_empty()) {
                        Some(h) => Some(handlers::find(a.os, h).ok_or_else(|| format!("{h} isn't a known {} app.", guest_label(a.os)))?.name.to_string()),
                        None => None,
                    };
                    let version = version.map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
                    Some(Identity { version: version.filter(|_| handler.is_some()), handler, by: IdentifiedBy::User })
                }
            };
            if a.handler() != identity.as_ref().and_then(|i| i.handler.as_deref()) {
                a.favorite = false;
            }
            a.identify_as(identity);
            Ok(())
        })
    }

    /// Whether an app that isn't a known one shares its errors.
    pub fn set_share_errors(&self, id: &str, share: bool) -> Result<LibraryApp, String> {
        self.update(id, |a| {
            a.share_errors = share;
            Ok(())
        })
    }

    /// Keeps the user's note of what went wrong running an app.
    pub fn set_errors(&self, id: &str, text: &str) -> Result<LibraryApp, String> {
        let text = text.trim();
        if text.chars().count() > MAX_ERRORS {
            return Err(format!("Keep it under {MAX_ERRORS} characters."));
        }
        self.update(id, |a| {
            a.errors = text.to_string();
            Ok(())
        })
    }

    /// Makes an app the version its handler's documents open with, in
    /// place of any other.
    pub fn set_favorite(&self, id: &str) -> Result<LibraryApp, String> {
        let _g = self.guard();
        let mut m = self.load()?;
        let app = m.apps.iter().find(|a| a.id == id).ok_or("That app is no longer in the library.")?;
        let (os, handler) = (app.os, app.handler().ok_or("Say which app this is first.")?.to_string());
        for a in m.apps.iter_mut().filter(|a| a.os == os && a.handler() == Some(handler.as_str())) {
            a.favorite = a.id == id;
        }
        let out = m.apps.iter().find(|a| a.id == id).cloned().expect("found above");
        self.save(&m)?;
        Ok(out)
    }

    /// Says which document extensions an app opens, on top of the
    /// well-known ones. Stored uppercase, without dots or duplicates.
    pub fn set_opens(&self, id: &str, exts: &[String]) -> Result<LibraryApp, String> {
        let mut clean: Vec<String> = Vec::new();
        for e in exts {
            let e = e.trim().trim_start_matches('.').to_ascii_uppercase();
            if e.is_empty() {
                continue;
            }
            if e.len() > 3 || !e.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(format!(".{e} isn't a DOS extension: up to 3 letters or digits."));
            }
            if !clean.contains(&e) {
                clean.push(e);
            }
        }
        self.update(id, |a| {
            a.opens = clean;
            Ok(())
        })
    }

    pub fn documents(&self) -> Result<Vec<LibraryDoc>, String> {
        let _g = self.guard();
        Ok(self.load()?.documents)
    }

    pub fn document(&self, id: &str) -> Result<LibraryDoc, String> {
        self.documents()?.into_iter().find(|d| d.id == id).ok_or_else(|| "That document is no longer in the library.".into())
    }

    /// Absolute path of a document.
    pub fn document_path(&self, doc: &LibraryDoc) -> Result<PathBuf, String> {
        let rel = Path::new(&doc.file);
        if rel.is_absolute() || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("Refusing an unexpected document path: {}", doc.file));
        }
        Ok(self.os_root(doc.os).join(rel))
    }

    /// Copies one file into the guest's documents folder (`C:\DOCS` for
    /// DOS, under an 8.3 name) and lists it. Only DOS takes documents so
    /// far.
    pub fn import_document(&self, os: GuestOs, src: &Path) -> Result<LibraryDoc, String> {
        let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("That path has no file name.")?;
        if os != GuestOs::Dos {
            return Err("Only DOS documents can be opened in their app so far.".into());
        }
        if !src.is_file() {
            return Err(format!("{name} isn't a file."));
        }
        let _g = self.guard();
        let dir = self.os_root(os).join(documents::DOS_DOCS_DIR);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let taken: HashSet<String> =
            fs::read_dir(&dir).map_err(|e| e.to_string())?.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().to_ascii_uppercase()).collect();
        let file = dos::to_83(&name, &taken, false);
        fs::copy(src, dir.join(&file)).map_err(|e| format!("Couldn't copy {name}: {e}"))?;
        let mut m = self.load()?;
        let doc = self.new_document(&mut m, os, name, format!("{}/{file}", documents::DOS_DOCS_DIR));
        self.save(&m)?;
        Ok(doc)
    }

    fn new_document(&self, m: &mut Manifest, os: GuestOs, name: String, file: String) -> LibraryDoc {
        m.documents.retain(|d| !(d.os == os && d.file.eq_ignore_ascii_case(&file)));
        let base = format!("doc-{}-{}", os.dir_name(), file.rsplit('/').next().unwrap_or(&file).to_ascii_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "-"));
        let used = |id: &str| m.documents.iter().any(|d| d.id == id);
        let id = if used(&base) { (2u32..).map(|n| format!("{base}-{n}")).find(|id| !used(id)).expect("unbounded") } else { base };
        let doc = LibraryDoc { id, os, name, file, opens_with: None, added: SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) };
        m.documents.push(doc.clone());
        doc
    }

    /// Lists files an app saved into the documents folder that aren't in
    /// the library yet, under their DOS names. Returns the new ones.
    pub fn adopt_documents(&self, os: GuestOs) -> Result<Vec<LibraryDoc>, String> {
        if os != GuestOs::Dos {
            return Ok(Vec::new());
        }
        let _g = self.guard();
        let dir = self.os_root(os).join(documents::DOS_DOCS_DIR);
        let Ok(entries) = fs::read_dir(&dir) else { return Ok(Vec::new()) };
        let mut m = self.load()?;
        let mut added = Vec::new();
        let mut names: Vec<String> = entries.filter_map(Result::ok).filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        for name in names {
            let file = format!("{}/{name}", documents::DOS_DOCS_DIR);
            if name.starts_with('.') || m.documents.iter().any(|d| d.os == os && d.file.eq_ignore_ascii_case(&file)) {
                continue;
            }
            added.push(self.new_document(&mut m, os, name, file));
        }
        if !added.is_empty() {
            self.save(&m)?;
        }
        Ok(added)
    }

    /// Remembers the app a document last opened with.
    pub fn set_opens_with(&self, doc_id: &str, app_id: &str) -> Result<(), String> {
        let _g = self.guard();
        let mut m = self.load()?;
        let doc = m.documents.iter_mut().find(|d| d.id == doc_id).ok_or("That document is no longer in the library.")?;
        doc.opens_with = Some(app_id.to_string());
        self.save(&m)
    }

    /// Deletes a document's file from the guest drive and drops it.
    pub fn remove_document(&self, id: &str) -> Result<(), String> {
        let doc = self.document(id)?;
        let path = self.document_path(&doc)?;
        let _g = self.guard();
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("Couldn't delete {}: {e}", doc.name)),
        }
        let mut m = self.load()?;
        m.documents.retain(|d| d.id != id);
        self.save(&m)
    }

    pub fn system(&self, os: GuestOs) -> Result<GuestSystem, String> {
        let _g = self.guard();
        Ok(self.load()?.systems.get(&os).cloned().unwrap_or_default())
    }

    /// Absolute path of one of `os`'s system files, if it's set.
    pub fn system_file(&self, os: GuestOs, kind: SystemFile) -> Result<Option<PathBuf>, String> {
        let sys = self.system(os)?;
        let name = match kind {
            SystemFile::Rom => sys.rom,
            SystemFile::Boot => sys.boot,
        };
        Ok(name.map(|n| self.system_dir(os).join(n)))
    }

    /// Copies a user-supplied ROM or boot disk into `system/<os>/`,
    /// replacing the previous one, after checking it's the right kind of
    /// file. An encrypted Amiga Forever ROM is unlocked with `rom_key`, or
    /// else the `rom.key` next to it, and the key is copied too.
    pub fn set_system_file(&self, os: GuestOs, kind: SystemFile, src: &Path, rom_key: Option<&Path>) -> Result<GuestSystem, String> {
        let _g = self.guard();
        let src_name = src.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("That path has no file name.")?;
        let meta = fs::metadata(src).map_err(|e| format!("Couldn't open {src_name}: {e}"))?;
        let mut model = None;
        let mut extra: Option<PathBuf> = None;
        match (os, kind) {
            (GuestOs::Dos, _) => return Err("DOS needs no system files: DOSBox provides DOS.".into()),
            (GuestOs::MacClassic, SystemFile::Rom) => {
                let head = if meta.is_file() { read_head(src, 16).unwrap_or_default() } else { Vec::new() };
                if !mac::is_basilisk_rom(&head, meta.len()) {
                    return Err(format!(
                        "{src_name} isn't a Mac ROM Basilisk II can use. It needs a 32-bit clean 512 KB or 1 MB ROM, from a Mac IIci, IIsi, IIfx, LC, Quadra, Centris or similar."
                    ));
                }
            }
            (GuestOs::MacClassic, SystemFile::Boot) => {
                if !meta.is_file() || meta.len() < 400 * 1024 {
                    return Err(format!("{src_name} doesn't look like a Mac disk image with a System Folder on it."));
                }
            }
            (GuestOs::Amiga, SystemFile::Rom) => {
                let head = read_head(src, 16).map_err(|e| format!("Couldn't read {src_name}: {e}"))?;
                match amiga::identify_kickstart(&head, meta.len()) {
                    Some(amiga::Kickstart::Plain { version, .. }) => model = Some(amiga::model_for(version).to_string()),
                    Some(amiga::Kickstart::Encrypted) => {
                        let key = rom_key.map(Path::to_path_buf).unwrap_or_else(|| src.with_file_name("rom.key"));
                        if !key.is_file() {
                            return Err(format!("{src_name} is an encrypted Amiga Forever ROM. Its rom.key file needs to be in the same folder."));
                        }
                        let rom = fs::read(src).map_err(|e| format!("Couldn't read {src_name}: {e}"))?;
                        let key_bytes = fs::read(&key).map_err(|e| format!("Couldn't read rom.key: {e}"))?;
                        let plain = amiga::decrypt_kickstart(&rom, &key_bytes).ok_or_else(|| {
                            format!("That rom.key doesn't unlock {src_name}. It needs the rom.key from the same Amiga Forever installation.")
                        })?;
                        if let Some(amiga::Kickstart::Plain { version, .. }) = amiga::identify_kickstart(&plain, plain.len() as u64) {
                            model = Some(amiga::model_for(version).to_string());
                        }
                        extra = Some(key);
                    }
                    None => return Err(format!("{src_name} doesn't look like a Kickstart ROM.")),
                }
            }
            (GuestOs::Amiga, SystemFile::Boot) => {
                if !(meta.is_dir() || amiga::is_disk_image(&src_name)) {
                    return Err("Workbench can be a floppy image (.adf, .adz, .dms), a hard-disk file (.hdf), or a folder with Workbench installed.".into());
                }
            }
        }

        let dir = self.system_dir(os);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let name = mac::sanitize_name(&src_name, 64, "System");
        let tmp = dir.join(format!(".incoming-{}", std::process::id()));
        let _ = remove_any(&tmp);
        let copied = if meta.is_dir() { copy_tree(src, &tmp, false) } else { fs::copy(src, &tmp).map(|_| ()) };
        if let Err(e) = copied {
            let _ = remove_any(&tmp);
            return Err(format!("Couldn't copy {src_name}: {e}"));
        }
        if let Some(key) = &extra {
            fs::copy(key, dir.join("rom.key")).map_err(|e| format!("Couldn't copy rom.key: {e}"))?;
        }

        let mut m = self.load()?;
        let sys = m.systems.entry(os).or_default();
        let slot = match kind {
            SystemFile::Rom => &mut sys.rom,
            SystemFile::Boot => &mut sys.boot,
        };
        // Replace the previous file, and anything already using this name.
        for old in slot.iter().chain(std::iter::once(&name)) {
            if !old.is_empty() && mac::sanitize_name(old, 64, "") == *old {
                let _ = remove_any(&dir.join(old));
            }
        }
        fs::rename(&tmp, dir.join(&name)).map_err(|e| format!("Couldn't move {src_name} into place: {e}"))?;
        *slot = Some(name);
        if model.is_some() {
            sys.model = model;
        }
        let out = sys.clone();
        self.save(&m)?;
        Ok(out)
    }

    /// Chooses the Amiga model FS-UAE emulates.
    pub fn set_model(&self, os: GuestOs, model: &str) -> Result<GuestSystem, String> {
        if os != GuestOs::Amiga || !amiga::MODELS.contains(&model) {
            return Err(format!("{model} isn't a model Floppy can emulate."));
        }
        let _g = self.guard();
        let mut m = self.load()?;
        let sys = m.systems.entry(os).or_default();
        sys.model = Some(model.to_string());
        let out = sys.clone();
        self.save(&m)?;
        Ok(out)
    }
}

/// Copies one file imported on its own into `staging`. Returns a better
/// name for the app than the file's stem, when the file carries one.
fn import_file(os: GuestOs, src: &Path, src_name: &str, ext: &str, staging: &Path) -> Result<Option<String>, String> {
    let copy = || fs::copy(src, staging.join(src_name)).map(|_| ()).map_err(|e| format!("Couldn't copy {src_name}: {e}"));
    match os {
        GuestOs::Dos => match ext {
            "exe" | "com" | "bat" => copy().map(|_| None),
            _ => Err("Floppy imports a folder, a .zip, or a DOS program (.exe, .com, .bat).".into()),
        },
        GuestOs::MacClassic => {
            if mac::MACBINARY_EXTS.contains(&ext) {
                let stem = src_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(src_name);
                return mac::decode_macbinary(src, staging, stem)
                    .map(Some)
                    .map_err(|e| format!("Couldn't decode {src_name}: {e}"));
            }
            // Anything else is copied with its forks: a disk image, an
            // archive to expand in the Mac, or a bare app from a Mac volume.
            copy()?;
            mac::tag_archive(&staging.join(src_name)).map_err(|e| format!("Couldn't set {src_name}'s file type: {e}"))?;
            Ok(None)
        }
        GuestOs::Amiga => {
            let exe = read_head(src, 4).is_ok_and(|h| h == [0x00, 0x00, 0x03, 0xF3]);
            if amiga::is_disk_image(src_name) || exe {
                copy().map(|_| None)
            } else {
                Err("Floppy imports a folder, a .zip, a disk image (.adf, .adz, .dms, .hdf), or an Amiga program.".into())
            }
        }
    }
}

pub(crate) fn read_head(path: &Path, n: usize) -> io::Result<Vec<u8>> {
    use std::io::Read;
    let mut buf = Vec::with_capacity(n);
    File::open(path)?.take(n as u64).read_to_end(&mut buf)?;
    Ok(buf)
}

fn remove_any(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// `name`, or `name 2`, `name 3`… (kept within `max` characters), whichever
/// isn't in `taken` (uppercase names).
/// Fingerprints of `programs` (relative to `dir`). A program that can't be
/// read is left out.
fn program_ids(dir: &Path, programs: &[String]) -> BTreeMap<String, ProgramId> {
    programs
        .iter()
        .filter_map(|p| {
            let path = dir.join(p);
            let size = fs::metadata(&path).ok()?.len();
            Some((p.clone(), ProgramId { size, sha256: cd::sha256_of(&path).ok()? }))
        })
        .collect()
}

fn unique_name(name: &str, taken: &HashSet<String>, max: usize) -> String {
    if !taken.contains(&name.to_uppercase()) {
        return name.to_string();
    }
    (2u32..)
        .map(|n| {
            let suffix = format!(" {n}");
            let keep = max.saturating_sub(suffix.chars().count());
            format!("{}{suffix}", name.chars().take(keep).collect::<String>().trim_end())
        })
        .find(|cand| !taken.contains(&cand.to_uppercase()))
        .expect("unbounded range always finds a free name")
}

/// A manifest id for an app in `dir`: `<os>-<slug>`, made unique. Ids are
/// also used as file names for per-launch configs, so they're kept to
/// lowercase ASCII letters, digits and `-_~`.
fn unique_id(os: GuestOs, dir: &str, apps: &[LibraryApp]) -> String {
    let slug: String = dir
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '~') { c } else { '-' })
        .collect();
    let base = format!("{}-{slug}", os.dir_name());
    let used = |id: &str| apps.iter().any(|a| a.id == id);
    if !used(&base) {
        return base;
    }
    (2u32..).map(|n| format!("{base}-{n}")).find(|id| !used(id)).expect("unbounded range always finds a free id")
}

/// A scratch folder under `<library>/.staging/`, deleted on drop.
struct Staging(PathBuf);

impl Staging {
    fn new(root: &Path) -> Result<Self, String> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let dir = root.join(".staging").join(format!("{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create a staging folder: {e}"))?;
        Ok(Staging(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// macOS clutter that never belongs on a DOS or Amiga drive: Finder
/// metadata and the AppleDouble files zips made on a Mac carry. (A Mac
/// import keeps the AppleDouble files until `mac::absorb_appledouble`
/// merges them back.)
fn is_host_clutter(name: &str) -> bool {
    name == ".DS_Store" || name == "__MACOSX" || name.starts_with("._")
}

/// Copies a folder. `fs::copy` carries resource forks and Finder info on
/// macOS, which a classic Mac import (`keep_forks`) depends on. Elsewhere
/// they're the `.rsrc`/`.finf` folders beside each file, copied like any
/// other folder.
fn copy_tree(src: &Path, dest: &Path, keep_forks: bool) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    let walker = WalkDir::new(src).min_depth(1).follow_links(false).into_iter();
    for entry in walker.filter_entry(|e| keep_forks || !is_host_clutter(&e.file_name().to_string_lossy())) {
        let entry = entry.map_err(io::Error::other)?;
        let rel = entry.path().strip_prefix(src).map_err(io::Error::other)?;
        let out = dest.join(rel);
        let ft = entry.file_type();
        if ft.is_dir() {
            fs::create_dir_all(&out)?;
        } else if ft.is_file() {
            fs::copy(entry.path(), &out)?;
        }
        // Symlinks are skipped: the guests have no equivalent, and
        // following one could copy far more than the folder imported.
    }
    Ok(())
}

/// Unpacks a zip into `dest`, refusing entries that would land outside it
/// and stopping past MAX_ZIP_BYTES. Without `keep_forks`, macOS clutter
/// (`__MACOSX/`, `._` files, `.DS_Store`) is left out.
pub(crate) fn extract_zip(src: &Path, dest: &Path, keep_forks: bool) -> io::Result<()> {
    let mut zip = zip::ZipArchive::new(File::open(src)?).map_err(io::Error::other)?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(io::Error::other)?;
        // enclosed_name rejects absolute paths and `..` (zip-slip).
        let Some(rel) = entry.enclosed_name() else { continue };
        if !keep_forks && rel.components().any(|c| is_host_clutter(&c.as_os_str().to_string_lossy())) {
            continue;
        }
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        total += entry.size();
        if total > MAX_ZIP_BYTES {
            return Err(io::Error::other("it unpacks to more than 4 GB"));
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(&out)?)?;
    }
    Ok(())
}

/// A zip or folder holding just one folder (`WP51/…`) imports as that
/// folder, not as a wrapper around it.
fn single_subdir(dir: &Path) -> io::Result<PathBuf> {
    let entries: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    match entries.as_slice() {
        [only] if only.file_type()?.is_dir() => Ok(only.path()),
        _ => Ok(dir.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::io::Write;

    fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, data) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn dos_apps_are_fingerprinted_identified_and_given_a_favorite() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let mut ids = Vec::new();
        for (dir, exe) in [("WP50", "MZ five"), ("WP51", "MZ five-one")] {
            let src = t.path().join(dir);
            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("WP.EXE"), exe).unwrap();
            let app = lib.import(GuestOs::Dos, &src).unwrap();
            let id = &app.program_ids["WP.EXE"];
            assert_eq!(id.size, exe.len() as u64);
            assert_eq!(id.sha256.len(), 64);
            assert!(app.identity.is_none(), "its fingerprint isn't a known version");
            ids.push(app.id);
        }
        // Say what they are.
        let wp50 = lib.set_identity(&ids[0], Some((Some("wordperfect".into()), Some(" 5.0 ".into())))).unwrap();
        assert_eq!(wp50.name, "WordPerfect 5.0", "the name follows Is and Version");
        let identity = wp50.identity.unwrap();
        assert_eq!((identity.handler.as_deref(), identity.version.as_deref(), identity.by), (Some("WordPerfect"), Some("5.0"), IdentifiedBy::User));
        lib.set_identity(&ids[1], Some((Some("WordPerfect".into()), Some("5.1".into())))).unwrap();
        assert!(lib.set_identity(&ids[1], Some((Some("WordBlaster".into()), None))).is_err());

        // One favorite per handler.
        lib.set_favorite(&ids[0]).unwrap();
        lib.set_favorite(&ids[1]).unwrap();
        let favs: Vec<bool> = lib.list().unwrap().iter().map(|a| a.favorite).collect();
        assert_eq!(favs, [false, true]);
        // Saying it's another app drops its favorite mark; an app that
        // isn't a handler can't be one.
        let other = lib.set_identity(&ids[1], Some((None, Some("7".into())))).unwrap();
        assert!(!other.favorite);
        assert_eq!(other.name, "WordPerfect 5.1", "an app that isn't a known one keeps its name");
        // A name the user chose stays through a new version.
        lib.rename(&ids[0], "Office WP").unwrap();
        let renamed = lib.set_identity(&ids[0], Some((Some("WordPerfect".into()), Some("5.1".into())))).unwrap();
        assert_eq!(renamed.name, "Office WP");
        assert_eq!(other.identity.as_ref().map(|i| (i.handler.is_none(), i.version.is_none())), Some((true, true)));
        assert!(lib.set_favorite(&ids[1]).is_err());
        assert!(lib.set_identity(&ids[1], None).unwrap().identity.is_none());
    }

    #[test]
    fn names_say_the_app_and_version() {
        let id = |h: Option<&str>, v: Option<&str>| Identity { handler: h.map(String::from), version: v.map(String::from), by: IdentifiedBy::User };
        assert_eq!(id(Some("WordPerfect"), Some("5.1")).name().as_deref(), Some("WordPerfect 5.1"));
        assert_eq!(id(Some("Microsoft Word (DOS)"), Some("5.5")).name().as_deref(), Some("Microsoft Word 5.5 (DOS)"));
        assert_eq!(id(Some("Lotus 1-2-3"), None).name().as_deref(), Some("Lotus 1-2-3"));
        assert_eq!(id(None, Some("2")).name(), None);
    }

    #[test]
    fn errors_notes_are_kept_trimmed_and_bounded() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("WP51");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("WP.EXE"), "MZ").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        assert_eq!(lib.set_errors(&app.id, "  Printer driver missing\n").unwrap().errors, "Printer driver missing");
        assert!(lib.set_errors(&app.id, &"x".repeat(MAX_ERRORS + 1)).is_err());
    }

    #[test]
    fn older_apps_get_fingerprints_once() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("WP51");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("WP.EXE"), "MZ").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        // As a library from before fingerprints had it.
        lib.update(&app.id, |a| {
            a.program_ids.clear();
            Ok(())
        })
        .unwrap();
        lib.backfill_program_ids().unwrap();
        assert_eq!(lib.get(&app.id).unwrap().program_ids["WP.EXE"].size, 2);
    }

    #[test]
    fn imports_zip_with_wrapper_folder() {
        let t = TempDir::new();
        let zip = t.path().join("wp51.zip");
        write_zip(&zip, &[
            ("WordPerfect/WP.EXE", b"MZ"),
            ("WordPerfect/INSTALL.EXE", b"MZ"),
            ("WordPerfect/Read Me.txt", b"hi"),
            ("__MACOSX/WordPerfect/._WP.EXE", b"x"),
        ]);
        let lib = Library::new(t.path().join("lib"));
        let app = lib.import(GuestOs::Dos, &zip).unwrap();
        assert_eq!(app.name, "wp51");
        assert_eq!(app.dir, "WP51");
        assert_eq!(app.program.as_deref(), Some("WP.EXE"));
        let dir = lib.os_root(GuestOs::Dos).join("WP51");
        assert!(dir.join("README.TXT").exists());
        assert!(!lib.root().join("dos/__MACOSX").exists());
        // Staging is cleaned up.
        assert_eq!(fs::read_dir(lib.root().join(".staging")).unwrap().count(), 0);
        assert_eq!(lib.list().unwrap().len(), 1);
    }

    #[test]
    fn zip_slip_entries_are_skipped() {
        let t = TempDir::new();
        let zip = t.path().join("evil.zip");
        write_zip(&zip, &[("../../escape.exe", b"MZ"), ("GAME.EXE", b"MZ")]);
        let lib = Library::new(t.path().join("lib"));
        lib.import(GuestOs::Dos, &zip).unwrap();
        assert!(!t.path().join("escape.exe").exists());
        assert!(!lib.root().join("escape.exe").exists());
    }

    #[test]
    fn same_name_twice_gets_distinct_folders() {
        let t = TempDir::new();
        let src = t.path().join("Commander Keen");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("KEEN1.EXE"), b"MZ").unwrap();
        let lib = Library::new(t.path().join("lib"));
        let a = lib.import(GuestOs::Dos, &src).unwrap();
        let b = lib.import(GuestOs::Dos, &src).unwrap();
        assert_eq!(a.dir, "COMMANDE");
        assert_eq!(b.dir, "COMMAN~1");
        assert_ne!(a.id, b.id);
        assert_eq!(lib.list().unwrap().len(), 2);
    }

    #[test]
    fn single_program_and_rejects() {
        let t = TempDir::new();
        let exe = t.path().join("PKZIP.EXE");
        fs::write(&exe, b"MZ").unwrap();
        let lib = Library::new(t.path().join("lib"));
        let app = lib.import(GuestOs::Dos, &exe).unwrap();
        assert_eq!((app.name.as_str(), app.program.as_deref()), ("PKZIP", Some("PKZIP.EXE")));

        let txt = t.path().join("notes.txt");
        fs::write(&txt, b"").unwrap();
        assert!(lib.import(GuestOs::Dos, &txt).is_err());

        let empty = t.path().join("Empty");
        fs::create_dir_all(&empty).unwrap();
        fs::write(empty.join("DATA.DAT"), b"").unwrap();
        assert!(lib.import(GuestOs::Dos, &empty).unwrap_err().contains("No DOS program"));
        assert!(!lib.os_root(GuestOs::Dos).join("EMPTY").exists());
    }

    #[test]
    fn set_program_rename_remove() {
        let t = TempDir::new();
        let src = t.path().join("GAME");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("GAME.EXE"), b"MZ").unwrap();
        fs::write(src.join("SETUP.EXE"), b"MZ").unwrap();
        let lib = Library::new(t.path().join("lib"));
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        assert!(lib.set_program(&app.id, Some("NOPE.EXE".into())).is_err());
        assert_eq!(lib.set_program(&app.id, Some("SETUP.EXE".into())).unwrap().program.as_deref(), Some("SETUP.EXE"));
        assert_eq!(lib.rename(&app.id, "  My Game ").unwrap().name, "My Game");
        lib.remove(&app.id).unwrap();
        assert!(!lib.os_root(GuestOs::Dos).join("GAME").exists());
        assert!(lib.list().unwrap().is_empty());
    }

    #[test]
    fn stale_entry_is_replaced_and_ids_stay_unique() {
        let t = TempDir::new();
        let src = t.path().join("GAME");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("GAME.EXE"), b"MZ").unwrap();
        let lib = Library::new(t.path().join("lib"));
        lib.import(GuestOs::Dos, &src).unwrap();
        // The folder vanishes outside Floppy; the next import reuses its name.
        fs::remove_dir_all(lib.os_root(GuestOs::Dos).join("GAME")).unwrap();
        lib.import(GuestOs::Dos, &src).unwrap();
        assert_eq!(lib.list().unwrap().len(), 1);

        let apps = vec![LibraryApp {
            id: "mac-my-app".into(),
            os: GuestOs::MacClassic,
            name: "x".into(),
            dir: "My-App".into(),
            program: None,
            programs: vec![],
            source_name: "x".into(),
            added: 0,
            opens: vec![],
            program_ids: BTreeMap::new(),
            identity: None,
            favorite: false,
            errors: String::new(),
            named_by_user: false,
            share_errors: false,
        }];
        assert_eq!(unique_id(GuestOs::MacClassic, "My App", &apps), "mac-my-app-2");
        assert_eq!(unique_id(GuestOs::MacClassic, "Café", &apps), "mac-caf-");
    }

    #[test]
    fn unique_names_fit_the_limit() {
        let taken: HashSet<String> = ["PROTRACKER".to_string(), "PROTRACKER 2".to_string()].into();
        assert_eq!(unique_name("ProTracker", &taken, 30), "ProTracker 3");
        let long = "x".repeat(30);
        let taken: HashSet<String> = [long.to_uppercase()].into();
        let got = unique_name(&long, &taken, 30);
        assert_eq!(got.chars().count(), 30);
        assert!(got.ends_with(" 2"));
    }

    #[test]
    fn amiga_imports_disk_images_and_program_folders() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let adf = t.path().join("Lemmings.adf");
        fs::write(&adf, vec![0u8; 1024]).unwrap();
        let app = lib.import(GuestOs::Amiga, &adf).unwrap();
        assert_eq!((app.name.as_str(), app.dir.as_str(), app.program.as_deref()), ("Lemmings", "Lemmings", Some("Lemmings.adf")));
        assert!(app.id.starts_with("amiga-"));

        let folder = t.path().join("ProTracker 2.3d: the tracker for everyone");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("ProTracker"), [0, 0, 3, 0xF3]).unwrap();
        fs::write(folder.join("._ProTracker"), b"junk").unwrap();
        let app = lib.import(GuestOs::Amiga, &folder).unwrap();
        assert_eq!(app.dir, "ProTracker 2.3d the tracker fo");
        assert!(!lib.os_root(GuestOs::Amiga).join(&app.dir).join("._ProTracker").exists());

        let txt = t.path().join("notes.txt");
        fs::write(&txt, b"hi").unwrap();
        assert!(lib.import(GuestOs::Amiga, &txt).is_err());
        assert!(lib.import(GuestOs::Dos, &adf).is_err());
    }

    #[test]
    fn mac_imports_disk_images_and_archives() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let dsk = t.path().join("Game Disk.dsk");
        fs::write(&dsk, vec![0u8; 1024]).unwrap();
        let app = lib.import(GuestOs::MacClassic, &dsk).unwrap();
        assert_eq!((app.dir.as_str(), app.program.as_deref()), ("Game Disk", Some("Game Disk.dsk")));
        assert!(lib.remove(&app.id).is_ok());

        let empty = t.path().join("Docs");
        fs::create_dir_all(&empty).unwrap();
        fs::write(empty.join("Letter"), b"no forks").unwrap();
        assert!(lib.import(GuestOs::MacClassic, &empty).unwrap_err().contains("resource forks"));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn mac_folder_keeps_basilisk_fork_folders() {
        let t = TempDir::new();
        let src = t.path().join("MacPaint");
        fs::create_dir_all(src.join(".rsrc")).unwrap();
        fs::create_dir_all(src.join(".finf")).unwrap();
        fs::write(src.join("MacPaint"), b"").unwrap();
        fs::write(src.join(".rsrc/MacPaint"), b"CODE").unwrap();
        let mut info = [0u8; 32];
        info[..8].copy_from_slice(b"APPLMPNT");
        fs::write(src.join(".finf/MacPaint"), info).unwrap();
        let lib = Library::new(t.path().join("lib"));
        let app = lib.import(GuestOs::MacClassic, &src).unwrap();
        assert_eq!(app.program.as_deref(), Some("MacPaint"));
        let file = lib.os_root(GuestOs::MacClassic).join("MacPaint/MacPaint");
        assert_eq!(crate::mac::resource_fork_len(&file), 4);
    }

    #[test]
    fn mac_zip_with_macosx_folder_keeps_the_app() {
        let t = TempDir::new();
        let mut ad = Vec::new();
        ad.extend_from_slice(&0x0005_1607u32.to_be_bytes());
        ad.extend_from_slice(&0x0002_0000u32.to_be_bytes());
        ad.extend_from_slice(&[0; 16]);
        ad.extend_from_slice(&2u16.to_be_bytes());
        for (id, off, len) in [(9u32, 50u32, 32u32), (2, 82, 4)] {
            ad.extend_from_slice(&id.to_be_bytes());
            ad.extend_from_slice(&off.to_be_bytes());
            ad.extend_from_slice(&len.to_be_bytes());
        }
        let mut info = [0u8; 32];
        info[..8].copy_from_slice(b"APPLMPNT");
        ad.extend_from_slice(&info);
        ad.extend_from_slice(b"CODE");
        let zip = t.path().join("MacPaint.zip");
        write_zip(&zip, &[("MacPaint/MacPaint", b""), ("__MACOSX/MacPaint/._MacPaint", &ad)]);
        let lib = Library::new(t.path().join("lib"));
        let app = lib.import(GuestOs::MacClassic, &zip).unwrap();
        assert_eq!((app.dir.as_str(), app.program.as_deref()), ("MacPaint", Some("MacPaint")));
        let file = lib.os_root(GuestOs::MacClassic).join("MacPaint/MacPaint");
        assert_eq!(crate::mac::resource_fork_len(&file), 4);
        assert!(!lib.os_root(GuestOs::MacClassic).join("MacPaint/__MACOSX").exists());
    }

    #[test]
    fn system_files_are_checked_copied_and_replaced() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let bogus = t.path().join("bogus.rom");
        fs::write(&bogus, b"nope").unwrap();
        assert!(lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &bogus, None).is_err());
        assert!(lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &bogus, None).is_err());
        assert!(lib.set_system_file(GuestOs::Dos, SystemFile::Rom, &bogus, None).is_err());

        let mac_rom = |len: usize| {
            let mut r = vec![0u8; len];
            r[8..10].copy_from_slice(&[0x06, 0x7C]);
            r
        };
        let plus = t.path().join("Mac Plus.ROM");
        fs::write(&plus, vec![0u8; 131_072]).unwrap();
        assert!(lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &plus, None).is_err());
        let rom = t.path().join("Quadra 650.ROM");
        fs::write(&rom, mac_rom(1_048_576)).unwrap();
        let sys = lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &rom, None).unwrap();
        assert_eq!(sys.rom.as_deref(), Some("Quadra 650.ROM"));
        assert!(lib.system_dir(GuestOs::MacClassic).join("Quadra 650.ROM").is_file());
        let rom2 = t.path().join("IIci.ROM");
        fs::write(&rom2, mac_rom(524_288)).unwrap();
        lib.set_system_file(GuestOs::MacClassic, SystemFile::Rom, &rom2, None).unwrap();
        assert!(!lib.system_dir(GuestOs::MacClassic).join("Quadra 650.ROM").exists());
        assert_eq!(
            lib.system_file(GuestOs::MacClassic, SystemFile::Rom).unwrap(),
            Some(lib.system_dir(GuestOs::MacClassic).join("IIci.ROM"))
        );

        let mut ks = vec![0u8; 262_144];
        ks[..4].copy_from_slice(&[0x11, 0x11, 0x4E, 0xF9]);
        ks[12..16].copy_from_slice(&[0, 34, 0, 5]);
        let kick = t.path().join("kick13.rom");
        fs::write(&kick, ks).unwrap();
        let sys = lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &kick, None).unwrap();
        assert_eq!(sys.model.as_deref(), Some("A500"));
        assert_eq!(lib.set_model(GuestOs::Amiga, "A1200").unwrap().model.as_deref(), Some("A1200"));
        assert!(lib.set_model(GuestOs::Amiga, "C64").is_err());

        // An encrypted Amiga Forever ROM: needs a key that unlocks it.
        let mut plain = vec![0u8; 524_288];
        plain[..4].copy_from_slice(&[0x11, 0x14, 0x4E, 0xF9]);
        plain[12..16].copy_from_slice(&[0, 40, 0, 68]);
        let key = b"secret key".to_vec();
        let mut enc = b"AMIROMTYPE1".to_vec();
        enc.extend(plain.iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]));
        let af = t.path().join("amiga-os-310-a1200.rom");
        fs::write(&af, &enc).unwrap();
        assert!(lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &af, None).unwrap_err().contains("rom.key"));
        let keys = t.path().join("elsewhere");
        fs::create_dir_all(&keys).unwrap();
        fs::write(keys.join("rom.key"), b"wrong key!").unwrap();
        assert!(lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &af, Some(&keys.join("rom.key"))).unwrap_err().contains("doesn't unlock"));
        fs::write(keys.join("rom.key"), &key).unwrap();
        let sys = lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &af, Some(&keys.join("rom.key"))).unwrap();
        assert_eq!(sys.model.as_deref(), Some("A1200"));
        assert_eq!(fs::read(lib.system_dir(GuestOs::Amiga).join("rom.key")).unwrap(), key);
        lib.set_system_file(GuestOs::Amiga, SystemFile::Rom, &kick, None).unwrap();

        let wb = t.path().join("Workbench");
        fs::create_dir_all(wb.join("S")).unwrap();
        fs::write(wb.join("S/Startup-Sequence"), b"LoadWB\n").unwrap();
        lib.set_system_file(GuestOs::Amiga, SystemFile::Boot, &wb, None).unwrap();
        assert!(lib.system_dir(GuestOs::Amiga).join("Workbench/S/Startup-Sequence").is_file());
        // The library list still loads, and apps and systems coexist.
        assert!(lib.list().unwrap().is_empty());
        assert_eq!(lib.system(GuestOs::Amiga).unwrap().boot.as_deref(), Some("Workbench"));
    }

    #[test]
    fn documents_live_in_c_docs_under_8_3_names() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("Letter to Bank.wp5");
        fs::write(&src, b"letter").unwrap();
        let doc = lib.import_document(GuestOs::Dos, &src).unwrap();
        assert_eq!((doc.name.as_str(), doc.file.as_str()), ("Letter to Bank.wp5", "DOCS/LETTERTO.WP5"));
        assert_eq!(fs::read(lib.document_path(&doc).unwrap()).unwrap(), b"letter");
        // A second file with the same short name gets its own.
        let src2 = t.path().join("Letter to Tom.wp5");
        fs::write(&src2, b"tom").unwrap();
        assert_eq!(lib.import_document(GuestOs::Dos, &src2).unwrap().file, "DOCS/LETTER~1.WP5");
        // Only DOS takes documents so far, and only files.
        assert!(lib.import_document(GuestOs::MacClassic, &src).is_err());
        assert!(lib.import_document(GuestOs::Dos, t.path()).is_err());

        // An app called "Docs" never takes the documents folder.
        let app_src = t.path().join("Docs");
        fs::create_dir_all(&app_src).unwrap();
        fs::write(app_src.join("DOCS.EXE"), b"MZ").unwrap();
        assert_ne!(lib.import(GuestOs::Dos, &app_src).unwrap().dir, "DOCS");

        // Files an app saved there are picked up, once.
        fs::write(lib.os_root(GuestOs::Dos).join("DOCS/REPLY.WP5"), b"reply").unwrap();
        let adopted = lib.adopt_documents(GuestOs::Dos).unwrap();
        assert_eq!(adopted.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["REPLY.WP5"]);
        assert!(lib.adopt_documents(GuestOs::Dos).unwrap().is_empty());

        lib.set_opens_with(&doc.id, "dos-wp51").unwrap();
        assert_eq!(lib.document(&doc.id).unwrap().opens_with.as_deref(), Some("dos-wp51"));
        lib.remove_document(&doc.id).unwrap();
        assert!(!lib.os_root(GuestOs::Dos).join("DOCS/LETTERTO.WP5").exists());
        assert_eq!(lib.documents().unwrap().len(), 2);
    }

    #[test]
    fn apps_say_which_extensions_they_open() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let src = t.path().join("Editor");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("ED.COM"), b"").unwrap();
        let app = lib.import(GuestOs::Dos, &src).unwrap();
        let set = |v: &[&str]| lib.set_opens(&app.id, &v.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(set(&[".txt", "Doc", " TXT ", ""]).unwrap().opens, ["TXT", "DOC"]);
        assert!(set(&["LONGER"]).is_err());
        assert!(set(&["A*B"]).is_err());
    }
}
