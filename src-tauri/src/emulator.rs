//! Finding and starting each guest OS's emulator. Every emulator runs as a
//! separate executable (never linked into Floppy), started with a config
//! file Floppy writes for that launch, so the user's own emulator settings
//! can't change how an app boots.
//!
//! - DOS: DOSBox Staging, bundled (`scripts/fetch-dosbox.sh`).
//! - Amiga: FS-UAE, bundled (`scripts/fetch-fs-uae.sh`).
//! - Classic Mac: Basilisk II, found as a separate install. Its macOS
//!   builds are only published on a forum behind a browser check, so a
//!   build script can't fetch and pin one (see README).

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde::Serialize;

use crate::library::GuestOs;

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum EmulatorSource {
    /// `FLOPPY_DOSBOX` / `FLOPPY_FSUAE` / `FLOPPY_BASILISK` (development override).
    Env,
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
            Emulator::BasiliskII => None,
        }
    }

    /// What the user sees when the emulator is missing.
    pub fn missing_message(self) -> &'static str {
        match self {
            Emulator::DosboxStaging => {
                "DOSBox Staging wasn't found. Run scripts/fetch-dosbox.sh, or install it in /Applications."
            }
            Emulator::FsUae => "FS-UAE wasn't found. Run scripts/fetch-fs-uae.sh, or install it in /Applications.",
            Emulator::BasiliskII => {
                "Basilisk II wasn't found. Install BasiliskII.app in /Applications (or ~/Applications) and try again."
            }
        }
    }

    /// Where the emulator is, checked in this order: the environment
    /// override, the bundled copy, then a standard install.
    pub fn locate(self, resource_dir: Option<&Path>) -> Option<(PathBuf, EmulatorSource)> {
        if let Some(p) = std::env::var_os(self.env_var()).map(PathBuf::from) {
            if p.is_file() {
                return Some((p, EmulatorSource::Env));
            }
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

#[cfg(not(target_os = "macos"))]
fn installed_paths(_emu: Emulator) -> Vec<PathBuf> {
    Vec::new()
}
