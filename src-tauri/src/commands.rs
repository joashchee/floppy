//! Tauri commands the frontend calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::emulator::{Emulator, EmulatorSource};
use crate::library::{GuestOs, GuestSystem, Library, LibraryApp, SystemFile};
use crate::cd::{self, CdImport};
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
    let found = emu.locate(app.path().resource_dir().ok().as_deref());
    let system = library.system(os)?;
    let rom_note = match (os, library.system_file(os, SystemFile::Rom)?) {
        (GuestOs::Amiga, Some(p)) => kickstart_note(&p),
        _ => None,
    };
    let blocker = if found.is_none() {
        Some(emu.missing_message().to_string())
    } else {
        match os {
            GuestOs::Dos => None,
            GuestOs::MacClassic if system.rom.is_none() => Some("Add a Mac ROM file to start the Mac.".into()),
            GuestOs::MacClassic if system.boot.is_none() => {
                Some("Add a startup disk image (System 7 to Mac OS 8.1) to start the Mac.".into())
            }
            GuestOs::Amiga if system.rom.is_none() => Some("Add a Kickstart ROM to start the Amiga.".into()),
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

#[tauri::command]
pub fn list_apps(state: State<AppState>) -> Result<Vec<LibraryApp>, String> {
    state.library.list()
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

#[tauri::command]
pub fn set_guest_model(state: State<AppState>, os: String, model: String) -> Result<GuestSystem, String> {
    let os = GuestOs::parse(&os).ok_or("Unknown guest OS.")?;
    state.library.set_model(os, &model)
}

/// Writes the missing-files list (see cd.rs) to `path`. Returns how many files are missing (0 writes nothing).
#[tauri::command]
pub fn write_missing_list(state: State<AppState>, path: String) -> Result<usize, String> {
    let slots = cd::missing_slots(&state.library)?;
    match cd::missing_list(&slots) {
        Some(list) => std::fs::write(&path, list).map_err(|e| format!("Couldn't save the list: {e}"))?,
        None => return Ok(0),
    }
    Ok(slots.len())
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

#[tauri::command]
pub fn running_apps(state: State<AppState>) -> Vec<String> {
    running_map(&state).into_keys().collect()
}

#[tauri::command]
pub fn take_startup_import(state: State<AppState>) -> Option<StartupImport> {
    state.startup.lock().unwrap_or_else(|p| p.into_inner()).take()
}

/// Writes the per-launch config for `entry` and returns its path.
/// `boot_only` boots the guest without the app: a DOS prompt in the
/// app's folder, or plain Mac OS / Workbench.
fn write_launch_config(library: &Library, entry: &LibraryApp, boot_only: bool) -> Result<PathBuf, String> {
    let program = if boot_only { None } else { entry.program.as_deref() };
    let app_dir = library.os_root(entry.os).join(&entry.dir);
    let run = library.run_dir();
    let (conf, sub, ext) = match entry.os {
        GuestOs::Dos => {
            let mount_root = library.os_root(entry.os);
            let conf = dos::dosbox_conf(&dos::Launch { mount_root: &mount_root, app_dir: &entry.dir, program, exit_after: true })?;
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
            let kickstart = library.system_file(entry.os, SystemFile::Rom)?.ok_or("Add a Kickstart ROM first.")?;
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
                kickstart: &kickstart,
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
    if let Some(e) = launch_conflict(&id, entry.os, &running_map(&state)) {
        return Err(e);
    }
    let emu = Emulator::for_os(entry.os);
    let (bin, _) = emu.locate(app.path().resource_dir().ok().as_deref()).ok_or(emu.missing_message())?;
    let conf_path = write_launch_config(&state.library, &entry, prompt_only)?;

    let mut child = emu.spawn(&bin, &conf_path).map_err(|e| format!("Couldn't start {}: {e}", emu.name()))?;
    let running = state.running.clone();
    running.lock().unwrap_or_else(|p| p.into_inner()).insert(id.clone(), entry.os);
    let _ = app.emit("running-changed", ());
    std::thread::spawn(move || {
        let _ = child.wait();
        running.lock().unwrap_or_else(|p| p.into_inner()).remove(&id);
        let _ = app.emit("running-changed", ());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
