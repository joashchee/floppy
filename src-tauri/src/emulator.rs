//! Finding and starting each guest OS's emulator. Every emulator runs as a
//! separate executable (never linked into Floppy), started with a config
//! file Floppy writes for that launch, so the user's own emulator settings
//! can't change how an app boots.
//!
//! - DOS: DOSBox Staging, bundled (`scripts/fetch-dosbox.sh`).
//! - Amiga: FS-UAE, bundled (`scripts/fetch-fs-uae.sh`).
//! - Classic Mac: Basilisk II, bundled on macOS and Linux
//!   (`scripts/fetch-basilisk.sh`, Floppy's own build of a pinned upstream
//!   commit, since upstream publishes no binaries). A separate install or a
//!   located copy works too.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde::Serialize;

use crate::library::{GuestOs, Library};

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum EmulatorSource {
    /// `FLOPPY_DOSBOX` / `FLOPPY_FSUAE` / `FLOPPY_BASILISK` (development override).
    Env,
    /// Picked by the user with Locate (`library/emulators.json`).
    Chosen,
    /// Shipped inside Floppy.
    Bundled,
    /// A separate install.
    Installed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Emulator {
    DosboxStaging,
    BasiliskII,
    FsUae,
}

impl Emulator {
    pub fn for_os(os: GuestOs) -> Self {
        match os {
            GuestOs::Dos => Emulator::DosboxStaging,
            GuestOs::MacClassic => Emulator::BasiliskII,
            GuestOs::Amiga => Emulator::FsUae,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Emulator::DosboxStaging => "DOSBox Staging",
            Emulator::BasiliskII => "Basilisk II",
            Emulator::FsUae => "FS-UAE",
        }
    }

    /// Its key in `library/emulators.json`.
    fn key(self) -> &'static str {
        match self {
            Emulator::DosboxStaging => "dosbox-staging",
            Emulator::BasiliskII => "basilisk-ii",
            Emulator::FsUae => "fs-uae",
        }
    }

    fn env_var(self) -> &'static str {
        match self {
            Emulator::DosboxStaging => "FLOPPY_DOSBOX",
            Emulator::BasiliskII => "FLOPPY_BASILISK",
            Emulator::FsUae => "FLOPPY_FSUAE",
        }
    }

    /// The bundled copy: its folder under Floppy's resources, and the
    /// executable relative to that folder.
    fn bundled(self) -> Option<(&'static str, &'static str)> {
        match self {
            Emulator::DosboxStaging => Some(("dosbox", DOSBOX_BIN)),
            Emulator::FsUae => Some(("fs-uae", FSUAE_BIN)),
            Emulator::BasiliskII => BASILISK_BIN.map(|bin| ("basilisk", bin)),
        }
    }

    /// What the user sees when the emulator is missing. A release build
    /// bundles all three, so there the fix is reinstalling Floppy; a
    /// development build needs the fetch script.
    pub fn missing_message(self) -> String {
        if self.bundled().is_none() && cfg!(target_os = "windows") {
            return format!(
                "{} isn't bundled for Windows. Install a compatible Windows build or use Locate {}….",
                self.name(),
                self.name()
            );
        }
        let (script, install) = match self {
            Emulator::DosboxStaging => (
                "fetch-dosbox.sh",
                if cfg!(target_os = "linux") {
                    "install dosbox-staging"
                } else if cfg!(target_os = "windows") {
                    "install it under Program Files"
                } else {
                    "install it in /Applications"
                },
            ),
            Emulator::FsUae => (
                "fetch-fs-uae.sh",
                if cfg!(target_os = "linux") {
                    "install fs-uae"
                } else if cfg!(target_os = "windows") {
                    "install it under Program Files"
                } else {
                    "install it in /Applications"
                },
            ),
            Emulator::BasiliskII => (
                "fetch-basilisk.sh",
                if cfg!(target_os = "linux") {
                    "install BasiliskII so it's on your PATH"
                } else if cfg!(target_os = "windows") {
                    "locate BasiliskII.exe"
                } else {
                    "install BasiliskII.app in /Applications"
                },
            ),
        };
        let bundled = if cfg!(debug_assertions) && cfg!(target_os = "windows") {
            "Run scripts/fetch-windows-emulators.ps1".to_string()
        } else if cfg!(debug_assertions) {
            format!("Run scripts/{script}")
        } else {
            "Reinstall Floppy (it includes one)".into()
        };
        format!("{} wasn't found. {bundled}, {install}, or use Locate {}….", self.name(), self.name())
    }

    /// Where the emulator is, checked in this order: the environment
    /// override, the copy the user located (`chosen`), the bundled copy,
    /// then a standard install. A located copy that has since moved is
    /// skipped.
    pub fn locate(self, resource_dir: Option<&Path>, chosen: Option<&Path>) -> Option<(PathBuf, EmulatorSource)> {
        if let Some(p) = std::env::var_os(self.env_var()).map(PathBuf::from) {
            if p.is_file() {
                return Some((p, EmulatorSource::Env));
            }
        }
        if let Some(p) = chosen.filter(|p| p.is_file()) {
            return Some((p.to_path_buf(), EmulatorSource::Chosen));
        }
        if let Some((dir, bin)) = self.bundled() {
            #[cfg_attr(not(debug_assertions), allow(unused_mut))]
            let mut bundled: Vec<PathBuf> = resource_dir.map(|r| r.join(dir).join(bin)).into_iter().collect();
            // `tauri dev` and tests run from target/debug, where resources may
            // not have been copied, so fall back to the fetched copy in the
            // source tree. #[cfg], not cfg!(), so this dev-machine path never
            // reaches a release binary.
            #[cfg(debug_assertions)]
            bundled.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join(dir).join(bin));
            if let Some(p) = bundled.into_iter().find(|p| p.is_file()) {
                return Some((p, EmulatorSource::Bundled));
            }
        }
        installed_paths(self).into_iter().find(|p| p.is_file()).map(|p| (p, EmulatorSource::Installed))
    }

    /// Starts the emulator with only Floppy's config for this launch.
    pub fn spawn(self, bin: &Path, conf: &Path) -> std::io::Result<Child> {
        let mut cmd = Command::new(bin);
        match self {
            Emulator::DosboxStaging => {
                cmd.arg("--noprimaryconf").arg("--nolocalconf").arg("--conf").arg(conf);
            }
            // A prefs file given with --config is read instead of
            // ~/.basilisk_ii_prefs, and its xpram (PRAM) file is kept next
            // to it.
            Emulator::BasiliskII => {
                cmd.arg("--config").arg(conf);
            }
            // FS-UAE reads only the config file named on the command line
            // (plus the base_dir it sets), not Default.fs-uae.
            Emulator::FsUae => {
                cmd.arg(conf);
            }
        }
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    }
}

/// Asks an emulator to quit (SIGTERM), or with `force` makes it (SIGKILL).
/// DOSBox Staging and FS-UAE quit when asked. Basilisk II's SDL turns the
/// request into its window's close button, which presses the Mac's power
/// key: the Mac asks whether to shut down, and Basilisk II quits once it
/// has. Only `force` stops a Mac that can't answer.
pub fn stop(child: &mut Child, force: bool) -> std::io::Result<()> {
    #[cfg(not(target_os = "windows"))]
    if force {
        return child.kill();
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = Command::new("taskkill");
        cmd.args(["/PID", &child.id().to_string(), "/T"]);
        if force {
            cmd.arg("/F");
        }
        let status = cmd
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(std::io::Error::other(format!("taskkill exited with {status}")));
    }
    #[cfg(not(target_os = "windows"))]
    let status = Command::new("kill").arg("-TERM").arg(child.id().to_string()).stdin(Stdio::null()).status()?;
    #[cfg(not(target_os = "windows"))]
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("kill exited with {status}")))
    }
}

/// The copy of `emu` the user located, if any.
pub fn chosen(library: &Library, emu: Emulator) -> Option<PathBuf> {
    load_chosen(library).remove(emu.key()).map(PathBuf::from)
}

/// Remembers `picked` (an app bundle or the executable itself) as the copy
/// of `emu` to use, and returns the executable.
pub fn set_chosen(library: &Library, emu: Emulator, picked: &Path) -> Result<PathBuf, String> {
    let bin = executable_in(emu, picked)?;
    let mut all = load_chosen(library);
    all.insert(emu.key().to_string(), bin.to_string_lossy().into_owned());
    let path = library.emulators_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Couldn't save where {} is: {e}", emu.name()))?;
    Ok(bin)
}

fn load_chosen(library: &Library) -> std::collections::BTreeMap<String, String> {
    std::fs::read(library.emulators_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// The program to run for a picked `.app` (its `Contents/MacOS/` executable,
/// by the usual name or the only file there) or a picked executable.
fn executable_in(emu: Emulator, picked: &Path) -> Result<PathBuf, String> {
    let not_it = || format!("{} isn't {}.", picked.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), emu.name());
    let bin = if picked.is_dir() {
        let macos = picked.join("Contents/MacOS");
        let usual = macos.join(match emu {
            Emulator::DosboxStaging => "dosbox",
            Emulator::BasiliskII => "BasiliskII",
            Emulator::FsUae => "fs-uae",
        });
        if usual.is_file() {
            usual
        } else {
            let files: Vec<PathBuf> = std::fs::read_dir(&macos)
                .map_err(|_| not_it())?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect();
            match <[PathBuf; 1]>::try_from(files) {
                Ok([only]) => only,
                Err(_) => return Err(not_it()),
            }
        }
    } else {
        picked.to_path_buf()
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&bin).map_err(|_| not_it())?.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(not_it());
        }
    }
    #[cfg(not(unix))]
    if !bin.is_file() {
        return Err(not_it());
    }
    Ok(bin)
}

#[cfg(target_os = "macos")]
const DOSBOX_BIN: &str = "DOSBox Staging.app/Contents/MacOS/dosbox";
#[cfg(target_os = "windows")]
const DOSBOX_BIN: &str = "dosbox.exe";
#[cfg(target_os = "linux")]
const DOSBOX_BIN: &str = "dosbox";

#[cfg(target_os = "macos")]
const FSUAE_BIN: &str = "FS-UAE.app/Contents/MacOS/fs-uae";
#[cfg(target_os = "windows")]
const FSUAE_BIN: &str = "Windows/x86-64/fs-uae.exe";
#[cfg(target_os = "linux")]
const FSUAE_BIN: &str = "Linux/x86-64/fs-uae";

// Floppy's own builds (.github/workflows/basilisk.yml): macOS and Linux.
#[cfg(target_os = "macos")]
const BASILISK_BIN: Option<&str> = Some("BasiliskII.app/Contents/MacOS/BasiliskII");
#[cfg(target_os = "linux")]
const BASILISK_BIN: Option<&str> = Some("BasiliskII");
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
const BASILISK_BIN: Option<&str> = None;

#[cfg(target_os = "macos")]
fn installed_paths(emu: Emulator) -> Vec<PathBuf> {
    let mut app_dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        app_dirs.push(PathBuf::from(home).join("Applications"));
    }
    match emu {
        Emulator::DosboxStaging => vec![PathBuf::from("/Applications/DOSBox Staging.app/Contents/MacOS/dosbox")],
        Emulator::FsUae => vec![PathBuf::from("/Applications/FS-UAE.app/Contents/MacOS/fs-uae")],
        // Community builds ship as "BasiliskII.app", sometimes inside a
        // "BasiliskII" folder, so look one level down too.
        Emulator::BasiliskII => {
            let mut found = Vec::new();
            for dir in app_dirs {
                find_basilisk_apps(&dir, 1, &mut found);
            }
            found
        }
    }
}

#[cfg(target_os = "macos")]
fn find_basilisk_apps(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        if !name.starts_with("basilisk") {
            continue;
        }
        if name.ends_with(".app") {
            out.push(p.join("Contents/MacOS/BasiliskII"));
        } else if depth > 0 && p.is_dir() {
            find_basilisk_apps(&p, depth - 1, out);
        }
    }
}

/// On Linux an install is a program on `PATH`. Distributions name DOSBox
/// Staging either `dosbox-staging` or plain `dosbox`, and a plain `dosbox`
/// may be the original DOSBox, which doesn't take `--noprimaryconf`, so that
/// one counts only if `--version` says it's Staging.
#[cfg(target_os = "linux")]
fn installed_paths(emu: Emulator) -> Vec<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    match emu {
        Emulator::DosboxStaging => {
            let mut found = on_path(&path, "dosbox-staging");
            found.extend(on_path(&path, "dosbox").into_iter().filter(|p| is_dosbox_staging(p)));
            found
        }
        Emulator::FsUae => on_path(&path, "fs-uae"),
        Emulator::BasiliskII => on_path(&path, "BasiliskII"),
    }
}

/// Every executable called `name` in the `PATH`-style list `path`, in order.
#[cfg(target_os = "linux")]
fn on_path(path: &std::ffi::OsStr, name: &str) -> Vec<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
        .collect()
}

#[cfg(target_os = "linux")]
fn is_dosbox_staging(bin: &Path) -> bool {
    Command::new(bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).to_ascii_lowercase().contains("staging"))
}

#[cfg(target_os = "windows")]
fn installed_paths(emu: Emulator) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    windows_installed_paths(emu, &roots, &std::env::var_os("PATH").unwrap_or_default())
}

#[cfg(target_os = "windows")]
fn windows_installed_paths(
    emu: Emulator,
    roots: &[PathBuf],
    path: &std::ffi::OsStr,
) -> Vec<PathBuf> {
    match emu {
        Emulator::DosboxStaging => roots
            .iter()
            .flat_map(|root| {
                [
                    root.join("DOSBox Staging/dosbox.exe"),
                    root.join("DOSBox Staging/dosbox-staging.exe"),
                ]
            })
            .filter(|p| p.is_file())
            .collect(),
        Emulator::FsUae => roots
            .iter()
            .flat_map(|root| {
                [
                    root.join("FS-UAE/fs-uae.exe"),
                    root.join("FS-UAE/Windows/x86-64/fs-uae.exe"),
                ]
            })
            .filter(|p| p.is_file())
            .collect(),
        Emulator::BasiliskII => {
            let mut candidates = on_path(path, "BasiliskII.exe");
            candidates.extend(roots.iter().flat_map(|root| {
                [
                    root.join("BasiliskII/BasiliskII.exe"),
                    root.join("Basilisk II/BasiliskII.exe"),
                ]
            }));
            candidates.retain(|p| p.is_file());
            candidates
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn installed_paths(_emu: Emulator) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn on_path(path: &std::ffi::OsStr, name: &str) -> Vec<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .filter(|p| p.is_file())
        .collect()
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;
    use crate::testutil::TempDir;

    fn program(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"windows executable").unwrap();
    }

    #[test]
    fn finds_emulators_in_windows_install_roots_and_path() {
        let root = TempDir::new();
        let path_root = TempDir::new();
        program(&root.path().join("DOSBox Staging/dosbox.exe"));
        program(&root.path().join("FS-UAE/Windows/x86-64/fs-uae.exe"));
        program(&root.path().join("Basilisk II/BasiliskII.exe"));
        program(&path_root.path().join("BasiliskII.exe"));
        let path = std::env::join_paths([path_root.path()]).unwrap();
        let roots = [root.path().to_path_buf()];

        assert_eq!(
            windows_installed_paths(Emulator::DosboxStaging, &roots, &path),
            vec![root.path().join("DOSBox Staging/dosbox.exe")]
        );
        assert_eq!(
            windows_installed_paths(Emulator::FsUae, &roots, &path),
            vec![root.path().join("FS-UAE/Windows/x86-64/fs-uae.exe")]
        );
        assert_eq!(
            windows_installed_paths(Emulator::BasiliskII, &roots, &path),
            vec![path_root.path().join("BasiliskII.exe"), root.path().join("Basilisk II/BasiliskII.exe")]
        );
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::os::unix::fs::PermissionsExt;

    fn exe(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn quit_asks_and_force_quit_makes() {
        let wait_gone = |c: &mut Child| {
            for _ in 0..40 {
                if c.try_wait().unwrap().is_some() {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            false
        };
        // An emulator that quits when asked.
        let mut c = Command::new("sleep").arg("30").spawn().unwrap();
        stop(&mut c, false).unwrap();
        assert!(wait_gone(&mut c));
        // One that only asks its guest (Basilisk II and the Mac's power
        // key): still running until forced.
        let mut c = Command::new("sh").arg("-c").arg("trap '' TERM; sleep 30").spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        stop(&mut c, false).unwrap();
        assert!(!wait_gone(&mut c), "ignored the request, as a Mac that can't answer does");
        stop(&mut c, true).unwrap();
        assert!(wait_gone(&mut c));
    }

    #[test]
    fn locating_an_app_bundle_remembers_its_executable() {
        let tmp = TempDir::new();
        let lib = Library::new(tmp.path().join("library"));
        let app = tmp.path().join("Downloads/basilisk-test/BasiliskII.app");
        exe(&app.join("Contents/MacOS/BasiliskII"));

        assert_eq!(chosen(&lib, Emulator::BasiliskII), None);
        let bin = set_chosen(&lib, Emulator::BasiliskII, &app).unwrap();
        assert_eq!(bin, app.join("Contents/MacOS/BasiliskII"));
        assert_eq!(chosen(&lib, Emulator::BasiliskII), Some(bin.clone()));
        assert_eq!(chosen(&lib, Emulator::FsUae), None);

        let found = Emulator::BasiliskII.locate(None, chosen(&lib, Emulator::BasiliskII).as_deref());
        assert_eq!(found, Some((bin, EmulatorSource::Chosen)));
    }

    #[test]
    fn a_renamed_bundle_uses_its_only_executable() {
        let tmp = TempDir::new();
        let app = tmp.path().join("Basilisk II 2026.app");
        exe(&app.join("Contents/MacOS/basilisk"));
        assert_eq!(executable_in(Emulator::BasiliskII, &app).unwrap(), app.join("Contents/MacOS/basilisk"));
    }

    #[test]
    fn a_bare_executable_can_be_located() {
        let tmp = TempDir::new();
        let bin = tmp.path().join("BasiliskII");
        exe(&bin);
        assert_eq!(executable_in(Emulator::BasiliskII, &bin).unwrap(), bin);
    }

    #[test]
    fn things_that_arent_programs_are_refused() {
        let tmp = TempDir::new();
        let text = tmp.path().join("readme.txt");
        std::fs::write(&text, b"hi").unwrap();
        assert!(executable_in(Emulator::BasiliskII, &text).is_err());
        assert!(executable_in(Emulator::BasiliskII, tmp.path()).is_err());
        let lib = Library::new(tmp.path().join("library"));
        assert!(set_chosen(&lib, Emulator::BasiliskII, &text).is_err());
        assert_eq!(chosen(&lib, Emulator::BasiliskII), None);
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn the_bundled_copy_is_found_in_resources() {
        let tmp = TempDir::new();
        let bin = tmp.path().join("basilisk").join(BASILISK_BIN.unwrap());
        exe(&bin);
        let found = Emulator::BasiliskII.locate(Some(tmp.path()), None);
        assert!(matches!(found, Some((_, EmulatorSource::Bundled))), "{found:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn installs_are_found_on_path_and_plain_dosbox_must_be_staging() {
        let tmp = TempDir::new();
        let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let script = |p: &Path, body: &str| {
            std::fs::write(p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
        };
        script(&a.join("dosbox"), "echo 'DOSBox version 0.74-3'");
        script(&b.join("dosbox"), "echo 'dosbox-staging, version 0.83.0'");
        script(&b.join("fs-uae"), "");
        std::fs::write(a.join("fs-uae"), b"not executable").unwrap();
        let path = std::env::join_paths([&a, &b]).unwrap();

        let staging: Vec<_> = on_path(&path, "dosbox").into_iter().filter(|p| is_dosbox_staging(p)).collect();
        assert_eq!(staging, vec![b.join("dosbox")]);
        assert_eq!(on_path(&path, "fs-uae"), vec![b.join("fs-uae")]);
        assert!(on_path(&path, "BasiliskII").is_empty());
    }

    #[test]
    fn a_located_copy_that_moved_is_skipped() {
        let tmp = TempDir::new();
        let gone = tmp.path().join("gone/BasiliskII");
        let found = Emulator::BasiliskII.locate(None, Some(&gone));
        assert!(found.is_none_or(|(_, source)| source != EmulatorSource::Chosen));
    }
}
