//! Tauri commands the frontend calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::emulator::{self, Emulator, EmulatorSource};
use crate::documents::{self, Change, Opener};
use crate::library::{GuestOs, GuestSystem, Library, LibraryApp, LibraryDoc, SystemFile};
use crate::cd::{self, CdImport};
use crate::discs;
use crate::backup;
use crate::findings;
use crate::handlers;
use crate::request;
use crate::verify;
use crate::media::{self, MediaWatch, OldMedia};
use crate::{amiga, dos, mac};

pub struct AppState {
    pub library: Library,
    /// Apps with an emulator window open, and their guest OS.
    pub running: Arc<Mutex<HashMap<String, GuestOs>>>,
    /// Result of a `floppy import …` launch, taken once by the frontend.
    pub startup: Mutex<Option<StartupImport>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StartupImport {
    pub app: Option<LibraryApp>,
    /// From `floppy open <file>`.
    pub document: Option<LibraryDoc>,
    pub error: Option<String>,
}

/// One guest OS's readiness: its emulator, and the system files the user
/// has supplied.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestStatus {
    pub os: GuestOs,
    pub emulator: &'static str,
    pub found: bool,
    pub source: Option<EmulatorSource>,
    pub system: GuestSystem,
    /// What the ROM is, when Floppy can tell (`Kickstart 3.1 (40.68)`).
    pub rom_note: Option<String>,
    /// Why apps can't launch yet, or `None` when they can.
    pub blocker: Option<String>,
}

fn running_map(state: &AppState) -> HashMap<String, GuestOs> {
    state.running.lock().unwrap_or_else(|p| p.into_inner()).clone()
}

/// Why `id` (a `os` app) can't start next to what's already running.
fn launch_conflict(id: &str, os: GuestOs, running: &HashMap<String, GuestOs>) -> Option<String> {
    if running.contains_key(id) {
        return Some("This app is already running.".into());
    }
    if os.single_instance() && running.values().any(|o| *o == os) {
        return Some(format!(
            "{} is already running another app. Quit it first: both would share the same startup disk.",
            Emulator::for_os(os).name()
        ));
    }
    None
}

fn guest_status(app: &AppHandle, library: &Library, os: GuestOs) -> Result<GuestStatus, String> {
    let emu = Emulator::for_os(os);
    let found = emu.locate(app.path().resource_dir().ok().as_deref(), emulator::chosen(library, emu).as_deref());
    let system = library.system(os)?;
    let rom_note = match (os, library.system_file(os, SystemFile::Rom)?) {
        (GuestOs::Amiga, Some(p)) => kickstart_note(&p),
        _ => None,
    };
    let blocker = if found.is_none() {
        Some(emu.missing_message())
    } else {
        match os {
            GuestOs::Dos => None,
            GuestOs::MacClassic if system.rom.is_none() => Some("Add a Mac ROM file to start the Mac.".into()),
            GuestOs::MacClassic if system.boot.is_none() => {
                Some("Add a startup disk image (System 7 to Mac OS 8.1) to start the Mac.".into())
            }
            GuestOs::Amiga if system.rom.is_none() && !system.aros => {
                Some("Add a Kickstart ROM to start the Amiga, or use the free AROS replacement for now.".into())
            }
            _ => None,
        }
    };
    Ok(GuestStatus { os, emulator: emu.name(), found: found.is_some(), source: found.map(|(_, s)| s), system, rom_note, blocker })
}

fn kickstart_note(path: &Path) -> Option<String> {
    let len = std::fs::metadata(path).ok()?.len();
    let mut head = std::fs::read(path).ok()?;
    head.truncate(16);
    match amiga::identify_kickstart(&head, len)? {
        amiga::Kickstart::Plain { version, revision } => {
            Some(format!("Kickstart {} ({version}.{revision})", amiga::kickstart_release(version)))
        }
        amiga::Kickstart::Encrypted => Some("Amiga Forever ROM (encrypted)".into()),
    }
}

/// The library's apps. DOS apps imported before Floppy fingerprinted
/// programs get their fingerprints here, once.
#[tauri::command]
pub async fn list_apps(app: AppHandle) -> Result<Vec<LibraryApp>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let library = &app.state::<AppState>().library;
        library.backfill_program_ids()?;
        library.list()
    })
    .await
    .map_err(|e| e.to_string())?
}

/// What the user says an app is.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityInput {
    /// A handler's name, or `None` for an app that isn't one.
    handler: Option<String>,
    version: Option<String>,
}

/// Says which handler an app is and its version (`None`: not known yet).
#[tauri::command]
pub fn set_app_identity(state: State<AppState>, id: String, identity: Option<IdentityInput>) -> Result<LibraryApp, String> {
    state.library.set_identity(&id, identity.map(|i| (i.handler, i.version)))
}

/// Makes an app the version its handler's documents open with.
#[tauri::command]
pub fn set_favorite_app(state: State<AppState>, id: String) -> Result<LibraryApp, String> {
    state.library.set_favorite(&id)
}

/// Every handler for a guest, with the versions known by fingerprint.
#[tauri::command]
pub fn handler_catalog(os: String) -> Result<Vec<handlers::HandlerInfo>, String> {
    Ok(handlers::catalog(GuestOs::parse(&os).ok_or("Unknown guest OS.")?))
}

/// Copying a big folder takes a while, so this runs off the main thread.
#[tauri::command]
pub async fn import_app(app: AppHandle, os: String, path: String) -> Result<LibraryApp, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().library.import(os, &PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn remove_app(state: State<AppState>, id: String) -> Result<(), String> {
    let entry = state.library.get(&id)?;
    let running = running_map(&state);
    // A running Mac or Amiga can see every app in its library, not just
    // the one it was launched with.
    if running.contains_key(&id) || (entry.os.single_instance() && running.values().any(|o| *o == entry.os)) {
        return Err(format!("Quit {} before removing this app.", Emulator::for_os(entry.os).name()));
    }
    state.library.remove(&id)
}

#[tauri::command]
pub fn set_program(state: State<AppState>, id: String, program: Option<String>) -> Result<LibraryApp, String> {
    state.library.set_program(&id, program)
}

#[tauri::command]
pub fn rename_app(state: State<AppState>, id: String, name: String) -> Result<LibraryApp, String> {
    state.library.rename(&id, &name)
}

#[tauri::command]
pub fn app_folder(state: State<AppState>, id: String) -> Result<String, String> {
    Ok(state.library.app_path(&id)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn guest_statuses(app: AppHandle, state: State<AppState>) -> Result<Vec<GuestStatus>, String> {
    [GuestOs::Dos, GuestOs::MacClassic, GuestOs::Amiga].into_iter().map(|os| guest_status(&app, &state.library, os)).collect()
}

/// Copies a user-supplied ROM or boot disk into the library. A boot disk
/// can be large, so this runs off the main thread.
#[tauri::command]
pub async fn set_system_file(app: AppHandle, os: String, kind: SystemFile, path: String) -> Result<GuestSystem, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| *o == os) {
        return Err(format!("Quit {} before changing its system files.", Emulator::for_os(os).name()));
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle.state::<AppState>().library.set_system_file(os, kind, &PathBuf::from(path), None)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Remembers `path` (an app bundle or executable) as the emulator for
/// `os`'s guest, for one that isn't where Floppy looks.
#[tauri::command]
pub fn locate_emulator(app: AppHandle, state: State<AppState>, os: String, path: String) -> Result<GuestStatus, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    emulator::set_chosen(&state.library, Emulator::for_os(os), Path::new(&path))?;
    guest_status(&app, &state.library, os)
}

/// Turns the Amiga's built-in AROS replacement Kickstart on or off
/// (`Library::set_aros`).
#[tauri::command]
pub fn set_aros(app: AppHandle, state: State<AppState>, on: bool) -> Result<GuestStatus, String> {
    state.library.set_aros(GuestOs::Amiga, on)?;
    guest_status(&app, &state.library, GuestOs::Amiga)
}

#[tauri::command]
pub fn set_guest_model(state: State<AppState>, os: String, model: String) -> Result<GuestSystem, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    state.library.set_model(os, &model)
}

/// Writes the missing-files list (see cd.rs) to `path`. Returns how many
/// system files it asks for (0 writes nothing).
#[tauri::command]
pub fn write_missing_list(state: State<AppState>, path: String) -> Result<usize, String> {
    cd::write_list(&state.library, Path::new(&path))
}

/// Writes the wanted-apps list (handlers.rs, `docs/app-handlers.md`) to
/// `path`: the old apps that open old files and aren't in the library
/// yet. Returns how many it asks for (0 writes nothing).
#[tauri::command]
pub fn write_wanted_apps(state: State<AppState>, path: String) -> Result<usize, String> {
    let apps = state.library.list()?;
    let Some(list) = handlers::wanted_list(&apps) else { return Ok(0) };
    std::fs::write(&path, &list).map_err(|e| format!("Couldn't save the list: {e}"))?;
    Ok(list.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).count())
}

/// Imports the old apps on an apps disc (a disc image or folder) made
/// from the wanted-apps list.
#[tauri::command]
pub async fn import_apps_disc(app: AppHandle, path: String) -> Result<handlers::AppsImport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let library = &app.state::<AppState>().library;
        cd::with_disc(library, &PathBuf::from(path), |root| handlers::import_apps_from_dir(library, root))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Whether Diskette is running, so Floppy can offer to ask it (request.rs).
#[tauri::command]
pub async fn diskette_running() -> bool {
    tauri::async_runtime::spawn_blocking(|| request::is_running(request::DISKETTE)).await.unwrap_or(false)
}

/// How many setup files and apps a request would ask for.
#[tauri::command]
pub fn request_summary(state: State<AppState>) -> Result<request::RequestSummary, String> {
    request::summary(&state.library)
}

/// Asks Diskette for every setup file and app Floppy still needs
/// (request.rs). Its disc comes back as an opened file.
#[tauri::command]
pub fn ask_diskette(state: State<AppState>) -> Result<request::RequestSummary, String> {
    request::send(&state.library)
}

/// Whether to offer a system backup, and what it would hold (backup.rs).
#[tauri::command]
pub fn backup_status(state: State<AppState>) -> Result<backup::Status, String> {
    backup::status(&state.library)
}

/// Not Now on the backup offer, until the setup files change.
#[tauri::command]
pub fn decline_backup(state: State<AppState>) -> Result<(), String> {
    backup::decline(&state.library)
}

/// Burns the system backup disc image to `path` (backup.rs).
#[tauri::command]
pub async fn make_backup(app: AppHandle, path: String) -> Result<backup::Made, String> {
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| o.single_instance()) {
        return Err("Quit Basilisk II and FS-UAE before backing up, so their disks aren't changing.".into());
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || backup::make(&handle.state::<AppState>().library, Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
}

/// Whether `path` is a system backup disc Floppy made.
#[tauri::command]
pub fn is_backup_disc(path: String) -> bool {
    backup::is_backup(Path::new(&path))
}

/// Whether `path` is a disc Diskette's Burn A CD made (discs.rs).
#[tauri::command]
pub fn is_burn_disc(path: String) -> bool {
    let path = Path::new(&path);
    path.is_file() && discs::iso_application_id(path).as_deref() == Some(discs::APPLICATION_ID)
}

/// What a disc brought: setup files and apps.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscImport {
    setup: CdImport,
    apps: handlers::AppsImport,
}

/// Imports everything on a disc that answers a request or a list: the
/// system files it fills slots with (cd.rs), then the old apps on it
/// (handlers.rs). One mount for both.
#[tauri::command]
pub async fn import_disc(app: AppHandle, path: String) -> Result<DiscImport, String> {
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| o.single_instance()) {
        return Err("Quit Basilisk II and FS-UAE before importing a disc.".into());
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let library = &handle.state::<AppState>().library;
        cd::with_disc(library, &PathBuf::from(path), |root| {
            let setup = cd::import_from_dir(library, root)?;
            let apps = handlers::import_apps_from_dir(library, root)?;
            Ok(DiscImport { setup, apps })
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Files the system asked Floppy to open (Diskette sending a disc back,
/// or Open With in Finder), taken once each.
#[tauri::command]
pub fn take_opened_files(opened: State<request::Opened>) -> Vec<String> {
    std::mem::take(&mut *opened.0.lock().unwrap_or_else(|p| p.into_inner())).into_iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

/// Records whether an app opened a document's file type correctly. The
/// app's identity is read now, so one the user confirmed after the
/// session is what's recorded.
#[tauri::command]
pub fn record_verification(state: State<AppState>, pending: verify::Pending, worked: bool, note: Option<String>) -> Result<(), String> {
    let outcome = if worked { verify::Outcome::Worked } else { verify::Outcome::Failed };
    let pending = match state.library.get(&pending.app_id) {
        Ok(entry) => {
            let program = entry.programs.iter().find(|p| p.rsplit('/').next() == Some(pending.program.as_str())).cloned();
            match program {
                Some(p) => verify_pending(&entry, &p, pending.file_type, pending.document),
                None => pending,
            }
        }
        Err(_) => pending,
    };
    verify::record(&state.library, &pending, outcome, note.as_deref())
}

/// Every test result, totalled per app and file type.
#[tauri::command]
pub fn handler_tests(state: State<AppState>) -> Vec<verify::Tally> {
    verify::tallies(&state.library)
}

/// How much Export Findings would share (findings.rs), for the gear menu.
#[tauri::command]
pub async fn findings_summary(app: AppHandle) -> Result<findings::Summary, String> {
    tauri::async_runtime::spawn_blocking(move || findings::summary(&app.state::<AppState>().library))
        .await
        .map_err(|e| e.to_string())?
}

/// Writes what this Floppy has learned since the last export to a zip at
/// `path`, for `scripts/merge-findings.py`, and remembers it went. Only
/// when the user asks: nothing leaves the machine otherwise.
#[tauri::command]
pub async fn export_findings(app: AppHandle, path: String) -> Result<findings::Summary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let library = &app.state::<AppState>().library;
        let found = findings::collect(library)?;
        if found.is_empty() {
            return Err("Nothing new to share since the last export.".to_string());
        }
        findings::write_zip(&found, Path::new(&path))?;
        findings::mark_exported(library, &found)?;
        Ok(found.summary())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn forget_handler_tests(state: State<AppState>) -> Result<(), String> {
    verify::forget_all(&state.library)
}

/// What earlier files discs taught Floppy (discs.rs), for the setup panel.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupTracking {
    /// Copies on the ignore list.
    ignored: usize,
    /// Labels of slots the user's drives couldn't fill, left off the list.
    not_on_drives: Vec<String>,
}

#[tauri::command]
pub fn setup_tracking(state: State<AppState>) -> SetupTracking {
    SetupTracking {
        ignored: discs::ignored_files(&state.library).len(),
        not_on_drives: discs::not_on_drives(&state.library).iter().map(|s| s.label().to_string()).collect(),
    }
}

/// Empties the ignore list, so the next files disc may bring those copies again.
#[tauri::command]
pub fn forget_ignored_files(state: State<AppState>) -> Result<(), String> {
    discs::forget_ignored(&state.library)
}

/// Puts a slot the drives couldn't fill back on the missing-files list.
#[tauri::command]
pub fn ask_again(state: State<AppState>, slot: String) -> Result<(), String> {
    discs::ask_again(&state.library, cd::Slot::from_label(&slot).ok_or("Unknown system file.")?)
}

/// Sets up every guest it can from a files disc (a disc image, or a
/// folder). Copying a startup disk takes a while, so this runs off the
/// main thread.
#[tauri::command]
pub async fn import_files_disc(app: AppHandle, path: String) -> Result<CdImport, String> {
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| o.single_instance()) {
        return Err("Quit Basilisk II and FS-UAE before importing system files.".into());
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || cd::import_cd(&handle.state::<AppState>().library, &PathBuf::from(path)))
        .await
        .map_err(|e| e.to_string())?
}

/// Adds system files from whatever the user dropped or picked: the files
/// themselves, folders, zips or disc images, in any mix (`cd::import_dropped`).
#[tauri::command]
pub async fn import_setup_files(app: AppHandle, paths: Vec<String>) -> Result<CdImport, String> {
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| o.single_instance()) {
        return Err("Quit Basilisk II and FS-UAE before adding system files.".into());
    }
    let handle = app.clone();
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || cd::import_dropped(&handle.state::<AppState>().library, &paths))
        .await
        .map_err(|e| e.to_string())?
}

/// Keeps what the user reported about getting a setup file for the next
/// Export Findings (`findings::add_setup_report`).
#[tauri::command]
pub fn add_setup_report(
    state: State<AppState>,
    slot: String,
    kind: String,
    source: Option<String>,
    note: String,
) -> Result<(), String> {
    findings::add_setup_report(&state.library, &slot, &kind, source.as_deref(), &note)
}

/// Where to get each setup file (`cd::setup_sources`), for the setup screen.
#[tauri::command]
pub fn setup_sources() -> Vec<cd::SetupSource> {
    cd::setup_sources().to_vec()
}

/// Fills missing setup files from the user's Downloads folder, after they
/// fetched one in the browser (`cd::import_from_downloads`).
#[tauri::command]
pub async fn import_from_downloads(app: AppHandle) -> Result<CdImport, String> {
    let state = app.state::<AppState>();
    if running_map(&state).values().any(|o| o.single_instance()) {
        return Err("Quit Basilisk II and FS-UAE before adding system files.".into());
    }
    // Linux desktops name it in user-dirs.dirs, which minimal setups lack.
    let dir = app
        .path()
        .download_dir()
        .ok()
        .or_else(|| app.path().home_dir().ok().map(|h| h.join("Downloads")).filter(|d| d.is_dir()))
        .ok_or("Floppy couldn't find your Downloads folder.")?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || cd::import_from_downloads(&handle.state::<AppState>().library, &dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn running_apps(state: State<AppState>) -> Vec<String> {
    running_map(&state).into_keys().collect()
}

#[tauri::command]
pub fn take_startup_import(state: State<AppState>) -> Option<StartupImport> {
    state.startup.lock().unwrap_or_else(|p| p.into_inner()).take()
}

/// Writes the per-launch config for `entry` and returns its path.
/// `program: None` boots the guest without the app: a DOS prompt in the
/// app's folder, or plain Mac OS / Workbench. `args` go to a DOS program.
fn write_launch_config(library: &Library, entry: &LibraryApp, program: Option<&str>, args: Option<&str>) -> Result<PathBuf, String> {
    let app_dir = library.os_root(entry.os).join(&entry.dir);
    let run = library.run_dir();
    let (conf, sub, ext) = match entry.os {
        GuestOs::Dos => {
            let mount_root = library.os_root(entry.os);
            let conf = dos::dosbox_conf(&dos::Launch { mount_root: &mount_root, app_dir: &entry.dir, program, exit_after: true, args })?;
            (conf, "dosbox", "conf")
        }
        GuestOs::MacClassic => {
            let rom = library.system_file(entry.os, SystemFile::Rom)?.ok_or("Add a Mac ROM file first.")?;
            let boot_disk = library.system_file(entry.os, SystemFile::Boot)?.ok_or("Add a Mac startup disk first.")?;
            let disks = program.filter(|p| mac::is_disk_image(p)).map(|p| app_dir.join(p)).into_iter().collect();
            let shared = library.os_root(entry.os);
            let conf = mac::basilisk_prefs(&mac::Launch { rom: &rom, boot_disk: &boot_disk, shared: &shared, disks, ram_mb: 64 })?;
            (conf, "basilisk", "prefs")
        }
        GuestOs::Amiga => {
            let sys = library.system(entry.os)?;
            // A real Kickstart always wins over the AROS fallback.
            let kickstart = library.system_file(entry.os, SystemFile::Rom)?;
            if kickstart.is_none() && !sys.aros {
                return Err("Add a Kickstart ROM first, or use the free AROS replacement.".into());
            }
            let workbench = library.system_file(entry.os, SystemFile::Boot)?;
            // The chosen disk first, then the app's other floppies for the
            // swap list. A chosen program file boots Workbench instead.
            let app_disks: Vec<PathBuf> = match program.filter(|p| amiga::is_disk_image(p)) {
                Some(p) => std::iter::once(p)
                    .chain(entry.programs.iter().map(String::as_str).filter(|q| *q != p && amiga::is_floppy_image(q)))
                    .map(|q| app_dir.join(q))
                    .collect(),
                None => Vec::new(),
            };
            let base_dir = run.join("fs-uae");
            let shared = library.os_root(entry.os);
            let conf = amiga::fsuae_conf(&amiga::Launch {
                base_dir: &base_dir,
                model: sys.model.as_deref().unwrap_or("A1200"),
                kickstart: kickstart.as_deref(),
                workbench: workbench.as_deref(),
                shared: &shared,
                app_disks,
            })?;
            std::fs::create_dir_all(&base_dir).map_err(|e| e.to_string())?;
            (conf, "fs-uae", "fs-uae")
        }
    };
    let dir = run.join(sub);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.{ext}", entry.id));
    std::fs::write(&path, conf).map_err(|e| format!("Couldn't write the emulator config: {e}"))?;
    Ok(path)
}

/// Boots the app's emulator with the app, or with just the guest when
/// `prompt_only`. Emits `running-changed` when it starts and again when
/// it quits.
#[tauri::command]
pub fn launch_app(app: AppHandle, state: State<AppState>, id: String, prompt_only: bool) -> Result<(), String> {
    let entry = state.library.get(&id)?;
    let program = if prompt_only { None } else { entry.program.clone() };
    start(app, &state, entry, program, None, None)
}

/// Opens a document in one of the apps that can open it (documents.rs):
/// the app's `program` runs with the document's DOS path.
#[tauri::command]
pub fn open_document(app: AppHandle, state: State<AppState>, id: String, app_id: String, program: String) -> Result<(), String> {
    let doc = state.library.document(&id)?;
    let entry = state.library.get(&app_id)?;
    if doc.os != GuestOs::Dos || entry.os != GuestOs::Dos {
        return Err("Only DOS documents can be opened in their app so far.".into());
    }
    if !entry.programs.contains(&program) {
        return Err(format!("{program} isn't one of {}'s programs.", entry.name));
    }
    state.library.set_opens_with(&id, &app_id)?;
    let args = documents::dos_path(&doc);
    let pending = verify_pending(&entry, &program, verify::dos_file_type(&doc.file), doc.name.clone());
    start(app, &state, entry, Some(program), Some(args), Some((doc, pending)))
}

/// What to ask about a session: the app as it's known now, with its
/// program's fingerprint. The handler is only ever a confirmed one.
fn verify_pending(entry: &LibraryApp, program: &str, file_type: String, document: String) -> verify::Pending {
    let identity = entry.identity.as_ref();
    let id = entry.program_ids.get(program);
    verify::Pending {
        os: entry.os,
        app_id: entry.id.clone(),
        app_name: entry.name.clone(),
        program: program.rsplit('/').next().unwrap_or(program).to_string(),
        handler: identity.and_then(|i| i.handler.clone()),
        version: identity.and_then(|i| i.version.clone()),
        size: id.map(|i| i.size),
        sha256: id.map(|i| i.sha256.clone()),
        file_type,
        document,
    }
}

/// After a session, when the app isn't identified yet but the program
/// that ran is named like a known handler's: what to ask the user it was.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct IdentifyAsk {
    app_id: String,
    app_name: String,
    /// The program's file name (`WORD.EXE`).
    program: String,
    /// Handlers with a program of that name, the likeliest first.
    candidates: Vec<String>,
}

fn identify_ask(entry: &LibraryApp, program: Option<&str>) -> Option<IdentifyAsk> {
    let program = program?;
    if entry.os != GuestOs::Dos || entry.identity.is_some() {
        return None;
    }
    let candidates: Vec<String> = handlers::candidates(entry.os, program).iter().map(|h| h.name.to_string()).collect();
    (!candidates.is_empty()).then(|| IdentifyAsk {
        app_id: entry.id.clone(),
        app_name: entry.name.clone(),
        program: program.rsplit('/').next().unwrap_or(program).to_string(),
        candidates,
    })
}

/// What a DOS session saved, sent as `session-ended` when DOSBox quits.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionReport {
    os: GuestOs,
    app_name: String,
    /// The document's name, when one was opened.
    document: Option<String>,
    changes: Vec<SessionChange>,
    /// For a document session: what to ask the user about (verify.rs).
    verify: Option<verify::Pending>,
    /// Which app it was, when that isn't settled yet.
    identify: Option<IdentifyAsk>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SessionChange {
    /// Relative to the guest's library folder.
    path: String,
    /// What to call it: a document's original name, else its file name.
    name: String,
    new: bool,
}

/// Launches `entry` (with `program` and its `args`). For DOS, snapshots
/// drive C: first and reports what changed when DOSBox quits, picking up
/// files saved into `C:\DOCS` as documents.
fn start(
    app: AppHandle,
    state: &AppState,
    entry: LibraryApp,
    program: Option<String>,
    args: Option<String>,
    document: Option<(LibraryDoc, verify::Pending)>,
) -> Result<(), String> {
    let id = entry.id.clone();
    if let Some(e) = launch_conflict(&id, entry.os, &running_map(state)) {
        return Err(e);
    }
    let emu = Emulator::for_os(entry.os);
    let chosen = emulator::chosen(&state.library, emu);
    let (bin, _) = emu.locate(app.path().resource_dir().ok().as_deref(), chosen.as_deref()).ok_or_else(|| emu.missing_message())?;
    let conf_path = write_launch_config(&state.library, &entry, program.as_deref(), args.as_deref())?;
    let ran = program.clone();
    let os_root = state.library.os_root(entry.os);
    let before = (entry.os == GuestOs::Dos).then(|| documents::snapshot(&os_root));

    let mut child = emu.spawn(&bin, &conf_path).map_err(|e| format!("Couldn't start {}: {e}", emu.name()))?;
    let running = state.running.clone();
    running.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), entry.os);
    let _ = app.emit("running-changed", ());
    std::thread::spawn(move || {
        let _ = child.wait();
        running.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
        if let Some(before) = before {
            let library = &app.state::<AppState>().library;
            let changes = documents::changes(&before, &documents::snapshot(&os_root));
            let _ = library.adopt_documents(entry.os);
            // Identified during the session, in its details, needn't be asked.
            let now = library.get(&entry.id).unwrap_or_else(|_| entry.clone());
            let identify = identify_ask(&now, ran.as_deref());
            // A document session always reports, to ask whether it worked.
            if !changes.is_empty() || document.is_some() || identify.is_some() {
                let docs = library.documents().unwrap_or_default();
                let changes = changes.into_iter().map(|c: Change| SessionChange { name: display_name(&docs, entry.os, &c.path), path: c.path, new: c.new }).collect();
                let (document, verify) = match document {
                    Some((d, p)) => (Some(d.name), Some(p)),
                    None => (None, None),
                };
                let report = SessionReport { os: entry.os, app_name: entry.name.clone(), document, changes, verify, identify };
                let _ = app.emit("session-ended", report);
            }
        }
        let _ = app.emit("running-changed", ());
    });
    Ok(())
}

/// A document's original name for a library-relative path, else the
/// path's file name.
fn display_name(docs: &[LibraryDoc], os: GuestOs, path: &str) -> String {
    docs.iter()
        .find(|d| d.os == os && d.file.eq_ignore_ascii_case(path))
        .map(|d| d.name.clone())
        .unwrap_or_else(|| path.rsplit('/').next().unwrap_or(path).to_string())
}

/// An app or a document, from `import_item`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedItem {
    app: Option<LibraryApp>,
    document: Option<LibraryDoc>,
}

/// Imports a dropped or picked path. For DOS, a folder, zip or program is
/// an app and any other file is a document. Other guests take apps only.
#[tauri::command]
pub async fn import_item(app: AppHandle, os: String, path: String) -> Result<ImportedItem, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    tauri::async_runtime::spawn_blocking(move || {
        let library = &app.state::<AppState>().library;
        let path = PathBuf::from(path);
        if os == GuestOs::Dos && !documents::is_dos_app_source(&path) {
            Ok(ImportedItem { app: None, document: Some(library.import_document(os, &path)?) })
        } else {
            Ok(ImportedItem { app: Some(library.import(os, &path)?), document: None })
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_documents(state: State<AppState>) -> Result<Vec<LibraryDoc>, String> {
    state.library.documents()
}

/// The library's apps that can open a document, best first.
#[tauri::command]
pub fn document_openers(state: State<AppState>, id: String) -> Result<Vec<Opener>, String> {
    let doc = state.library.document(&id)?;
    Ok(match doc.os {
        GuestOs::Dos => documents::dos_openers(&doc.file, doc.opens_with.as_deref(), &state.library.list()?, &verify::tallies(&state.library)),
        _ => Vec::new(),
    })
}

#[tauri::command]
pub fn remove_document(state: State<AppState>, id: String) -> Result<(), String> {
    let doc = state.library.document(&id)?;
    if running_map(&state).values().any(|os| *os == doc.os) {
        return Err(format!("Quit {} before removing a document it can see.", Emulator::for_os(doc.os).name()));
    }
    state.library.remove_document(&id)
}

/// Keeps the user's note of what goes wrong running an app.
#[tauri::command]
pub fn set_app_errors(state: State<AppState>, id: String, errors: String) -> Result<LibraryApp, String> {
    state.library.set_errors(&id, &errors)
}

/// Whether an app Floppy doesn't know shares its errors in Export Findings.
#[tauri::command]
pub fn set_app_share_errors(state: State<AppState>, id: String, share: bool) -> Result<LibraryApp, String> {
    state.library.set_share_errors(&id, share)
}

/// Says which document extensions an app opens.
#[tauri::command]
pub fn set_app_opens(state: State<AppState>, id: String, exts: Vec<String>) -> Result<LibraryApp, String> {
    state.library.set_opens(&id, &exts)
}

/// A library file's absolute path, for Show in Finder. `path` is relative
/// to the guest's library folder, as `session-ended` reports it.
#[tauri::command]
pub fn library_file(state: State<AppState>, os: String, path: String) -> Result<String, String> {
    Ok(guest_file(&state.library, &os, &path)?.to_string_lossy().into_owned())
}

fn guest_file(library: &Library, os: &str, path: &str) -> Result<PathBuf, String> {
    let os = GuestOs::parse(os).ok_or("Unknown guest OS.")?;
    let rel = Path::new(path);
    if rel.is_absolute() || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("Refusing an unexpected path: {path}"));
    }
    Ok(library.os_root(os).join(rel))
}

/// Copies a library file into `dest_dir`, under the document's original
/// name where it has one, never over an existing file. Returns the copy.
#[tauri::command]
pub fn export_file(state: State<AppState>, os: String, path: String, dest_dir: String) -> Result<String, String> {
    let src = guest_file(&state.library, &os, &path)?;
    let guest = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    let name = display_name(&state.library.documents()?, guest, &path);
    let dest_dir = PathBuf::from(dest_dir);
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    let dest = std::iter::once(dest_dir.join(&name))
        .chain((2..).map(|n| dest_dir.join(format!("{stem} {n}{ext}"))))
        .find(|p| !p.exists())
        .expect("unbounded");
    std::fs::copy(&src, &dest).map_err(|e| format!("Couldn't export {name}: {e}"))?;
    Ok(dest.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(identity: Option<crate::library::Identity>) -> LibraryApp {
        LibraryApp {
            id: "dos-word".into(),
            os: GuestOs::Dos,
            name: "WORD".into(),
            dir: "WORD".into(),
            program: Some("WORD.EXE".into()),
            programs: vec!["WORD.EXE".into(), "SETUP.EXE".into()],
            source_name: "WORD".into(),
            added: 0,
            opens: vec![],
            program_ids: Default::default(),
            identity,
            favorite: false,
            errors: String::new(),
            named_by_user: false,
            share_errors: false,
        }
    }

    #[test]
    fn asks_what_a_program_named_like_a_handler_was() {
        let ask = identify_ask(&word(None), Some("WORD.EXE")).unwrap();
        assert_eq!((ask.program.as_str(), ask.candidates.as_slice()), ("WORD.EXE", &["Microsoft Word (DOS)".to_string()][..]));
        // Not for a program no handler is named like, a DOS prompt, or an
        // app already identified.
        assert!(identify_ask(&word(None), Some("SETUP.EXE")).is_none());
        assert!(identify_ask(&word(None), None).is_none());
        let known = crate::library::Identity { handler: None, version: None, by: crate::library::IdentifiedBy::User };
        assert!(identify_ask(&word(Some(known)), Some("WORD.EXE")).is_none());
    }

    #[test]
    fn one_mac_or_amiga_at_a_time_but_many_dos_apps() {
        let running: HashMap<String, GuestOs> =
            [("dos-wp".to_string(), GuestOs::Dos), ("mac-macwrite".to_string(), GuestOs::MacClassic)].into();
        assert!(launch_conflict("dos-wp", GuestOs::Dos, &running).unwrap().contains("already running"));
        assert!(launch_conflict("dos-pkzip", GuestOs::Dos, &running).is_none());
        assert!(launch_conflict("mac-macpaint", GuestOs::MacClassic, &running).unwrap().contains("Basilisk II"));
        assert!(launch_conflict("amiga-lemmings", GuestOs::Amiga, &running).is_none());
    }
}

/// Old disks attached now that macOS couldn't mount (media.rs).
#[tauri::command]
pub fn old_media(media: State<Arc<MediaWatch>>) -> Vec<OldMedia> {
    media.list()
}

/// Stops offering a disk until it's detached.
#[tauri::command]
pub fn dismiss_media(app: AppHandle, media: State<Arc<MediaWatch>>, device: String) {
    media.dismiss(&device);
    let _ = app.emit("old-media-changed", media.list());
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct MediaProgress {
    device: String,
    done: u64,
    total: u64,
}

/// Copies an offered old disk into the library, asking macOS for read
/// access, and imports it into the guest its contents belong to. Emits
/// `media-progress` while copying. The disk is no longer offered after.
#[tauri::command]
pub async fn copy_old_media(app: AppHandle, device: String) -> Result<LibraryApp, String> {
    let media = app.state::<Arc<MediaWatch>>().find(&device).ok_or("That disk isn't attached any more.")?;
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let library = &handle.state::<AppState>().library;
        let dir = library.run_dir().join(format!("media-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let total = media.size;
        let emitter = handle.clone();
        let copied = media::copy_to_image(&media, &dir, |done| {
            let _ = emitter.emit("media-progress", MediaProgress { device: media.device.clone(), done, total });
        });
        let imported = copied.and_then(|(image, os)| library.import(os, &image));
        let _ = std::fs::remove_dir_all(&dir);
        imported
    })
    .await
    .map_err(|e| e.to_string())?;
    if result.is_ok() {
        let watch = app.state::<Arc<MediaWatch>>();
        watch.dismiss(&device);
        let _ = app.emit("old-media-changed", watch.list());
    }
    result
}
