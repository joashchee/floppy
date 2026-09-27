//! Amiga guest specifics: finding programs and disk images, reading a
//! Kickstart ROM's version, and the FS-UAE config that boots an app.
//!
//! Most Amiga software ships as floppy images (ADF, or DMS-packed), which
//! boot on their own. Programs in a folder need Workbench to be started
//! from. Either way the whole Amiga library is also mounted as a hard
//! drive named `Floppy:`, so apps and files can reach each other.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Amiga file names are at most 30 characters (FFS).
pub const MAX_NAME: usize = 30;

/// Floppy images FS-UAE inserts in a drive.
pub const FLOPPY_EXTS: &[&str] = &["adf", "adz", "dms"];
/// Hard-disk files FS-UAE mounts as a hard drive.
pub const HARD_DISK_EXTS: &[&str] = &["hdf"];

/// First long word of an AmigaDOS executable (`HUNK_HEADER`).
const HUNK_HEADER: [u8; 4] = [0x00, 0x00, 0x03, 0xF3];

/// The volume name the Amiga library gets inside the Amiga.
pub const LIBRARY_VOLUME: &str = "Floppy";

fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_floppy_image(name: &str) -> bool {
    FLOPPY_EXTS.contains(&ext_of(name).as_str())
}

pub fn is_hard_disk_image(name: &str) -> bool {
    HARD_DISK_EXTS.contains(&ext_of(name).as_str())
}

pub fn is_disk_image(name: &str) -> bool {
    is_floppy_image(name) || is_hard_disk_image(name)
}

fn is_executable(path: &Path) -> bool {
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut magic)).is_ok() && magic == HUNK_HEADER
}

/// Stems of programs that set an app up rather than run it.
const NOT_THE_APP: &[&str] = &["install", "installer", "setup", "uninstall", "readme", "read me"];

/// How likely `rel` is what to launch: disk images first (they boot by
/// themselves), then shallow and name-matching paths; installers lose.
/// Among a set of disks, lower numbers win through the name sort.
fn score(rel: &str, name_hint: &str) -> i32 {
    let depth = rel.matches('/').count() as i32;
    let file = rel.rsplit('/').next().unwrap_or(rel);
    let stem = file.rsplit_once('.').map(|(s, _)| s).unwrap_or(file).to_lowercase();
    let hint = name_hint.to_lowercase();
    let mut s = -depth * 10;
    if is_disk_image(file) {
        s += 20;
    }
    if !hint.is_empty() && (stem == hint || (stem.len() >= 3 && hint.starts_with(&stem))) {
        s += 30;
    }
    if NOT_THE_APP.iter().any(|n| stem.starts_with(n)) {
        s -= 40;
    }
    s
}

/// Disk images and AmigaDOS executables under `root`, best launch
/// candidate first, as `/`-separated paths relative to `root`.
pub fn find_programs(root: &Path, name_hint: &str) -> Vec<String> {
    let mut found: Vec<String> = WalkDir::new(root)
        .min_depth(1)
        .max_depth(5)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let name = e.file_name().to_string_lossy();
            // .info files are Workbench icons, never the program.
            !name.to_ascii_lowercase().ends_with(".info") && (is_disk_image(&name) || is_executable(e.path()))
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

/// What a Kickstart ROM file is.
#[derive(Debug, PartialEq)]
pub enum Kickstart {
    /// A plain ROM image and its version (`40.68` is Kickstart 3.1).
    Plain { version: u16, revision: u16 },
    /// An Amiga Forever ROM, encrypted with a `rom.key` file.
    Encrypted,
}

/// Recognizes a Kickstart ROM from its first bytes, or `None` if it isn't one.
pub fn identify_kickstart(bytes: &[u8], len: u64) -> Option<Kickstart> {
    if bytes.starts_with(b"AMIROMTYPE1") {
        return Some(Kickstart::Encrypted);
    }
    // 256 KB ROMs start with 0x11114EF9, 512 KB ones with 0x11144EF9.
    let sized = matches!((bytes.get(..4)?, len), ([0x11, 0x11, 0x4E, 0xF9], 262_144) | ([0x11, 0x14, 0x4E, 0xF9], 524_288));
    if !sized {
        return None;
    }
    let version = u16::from_be_bytes([bytes[12], bytes[13]]);
    let revision = u16::from_be_bytes([bytes[14], bytes[15]]);
    Some(Kickstart::Plain { version, revision })
}

/// The 32-bit end-around-carry sum Kickstart ROMs and bootblocks use: a
/// valid one sums to 0xFFFFFFFF (`kickstart_checksum_do` in FS-UAE's
/// rommgr.cpp).
pub fn carry_sum_ok(bytes: &[u8]) -> bool {
    let mut sum: u32 = 0;
    for w in bytes.chunks_exact(4) {
        let (s, carry) = sum.overflowing_add(u32::from_be_bytes([w[0], w[1], w[2], w[3]]));
        sum = s + carry as u32;
    }
    sum == 0xFFFF_FFFF
}

/// A floppy image's volume name and whether its bootblock boots, or
/// `None` if it isn't an AmigaDOS floppy image (880 KB DD or 1.76 MB HD).
pub fn adf_volume(path: &Path) -> Option<(String, bool)> {
    use std::io::{Read, Seek, SeekFrom};
    let len = std::fs::metadata(path).ok()?.len();
    let root_block: u64 = match len {
        901_120 => 880,
        1_802_240 => 1760,
        _ => return None,
    };
    let mut f = std::fs::File::open(path).ok()?;
    let mut boot = [0u8; 1024];
    f.read_exact(&mut boot).ok()?;
    if &boot[0..3] != b"DOS" {
        return None;
    }
    let mut root = [0u8; 512];
    f.seek(SeekFrom::Start(root_block * 512)).ok()?;
    f.read_exact(&mut root).ok()?;
    let n = (root[432] as usize).min(30);
    let name = String::from_utf8_lossy(&root[433..433 + n]).into_owned();
    Some((name, carry_sum_ok(&boot)))
}

/// The Workbench release that goes with a Kickstart version, as it
/// appears in Workbench disk names ("Workbench3.1").
pub fn workbench_release(version: u16) -> Option<&'static str> {
    match version {
        33..=34 => Some("1.3"),
        36..=37 => Some("2.0"),
        39 => Some("3.0"),
        40 => Some("3.1"),
        _ => None,
    }
}

/// Decrypts an Amiga Forever ROM (`AMIROMTYPE1` header) with a `rom.key`,
/// or `None` if the key doesn't unlock it. As in FS-UAE's
/// `decode_cloanto_rom_do`: the data after the header is XORed with the
/// key, repeated, and the right key yields a Kickstart header.
pub fn decrypt_kickstart(rom: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let data = rom.strip_prefix(b"AMIROMTYPE1")?;
    if key.is_empty() || data.len() < 4 {
        return None;
    }
    let plain: Vec<u8> = data.iter().enumerate().map(|(i, b)| b ^ key[i % key.len()]).collect();
    let header = (plain[0] == 0x11 && matches!(plain[1], 0x11 | 0x14)) || (plain[2] == 0x4E && plain[3] == 0xF9);
    header.then_some(plain)
}

/// The Kickstart release a ROM version belongs to, for display.
pub fn kickstart_release(version: u16) -> &'static str {
    match version {
        30..=34 => "1.x",
        35..=36 => "2.0",
        37 => "2.04/2.05",
        39 => "3.0",
        40 => "3.1",
        45..=46 => "3.9 or later",
        47.. => "3.2 or later",
        _ => "unknown",
    }
}

/// FS-UAE models Floppy offers, in the order the setup shows them.
pub const MODELS: &[&str] = &["A500", "A500+", "A600", "A1200", "A4000/040"];

/// The Amiga model that fits a Kickstart version: 1.x on an A500, 2.x on
/// an A600, 3.x on an A1200.
pub fn model_for(version: u16) -> &'static str {
    match version {
        ..=34 => "A500",
        35..=38 => "A600",
        _ => "A1200",
    }
}

/// What to boot.
pub struct Launch<'a> {
    /// Where FS-UAE keeps its own files (logs, floppy overlays, saves),
    /// so it never writes to ~/Documents/FS-UAE.
    pub base_dir: &'a Path,
    pub model: &'a str,
    /// The user's Kickstart, or `None` for FS-UAE's built-in AROS
    /// replacement (`kickstart_file = internal`).
    pub kickstart: Option<&'a Path>,
    /// The user's Workbench: a floppy image, a hard-disk file, or a
    /// folder holding an installed Workbench.
    pub workbench: Option<&'a Path>,
    /// The whole Amiga library, mounted as `Floppy:`.
    pub shared: &'a Path,
    /// The app's disk images, the one to boot first. Empty for a
    /// folder app, which is started from Workbench.
    pub app_disks: Vec<PathBuf>,
}

fn conf_path(p: &Path) -> Result<&str, String> {
    let s = p.to_str().ok_or("A path Floppy passes to FS-UAE isn't valid UTF-8.")?;
    if s.contains('\n') || s.contains('\r') {
        return Err("A path Floppy passes to FS-UAE contains a line break.".into());
    }
    Ok(s)
}

/// The config file Floppy passes to FS-UAE for one launch.
pub fn fsuae_conf(l: &Launch) -> Result<String, String> {
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let wb_is_floppy = l.workbench.is_some_and(|w| w.is_file() && is_floppy_image(&name_of(w)));
    let wb_floppy = l.workbench.filter(|_| wb_is_floppy);
    let wb_drive = l.workbench.filter(|_| !wb_is_floppy);

    let app_floppies: Vec<&Path> = l.app_disks.iter().map(PathBuf::as_path).filter(|p| is_floppy_image(&name_of(p))).collect();
    let app_hdfs: Vec<&Path> = l.app_disks.iter().map(PathBuf::as_path).filter(|p| is_hard_disk_image(&name_of(p))).collect();
    let boots_app = !l.app_disks.is_empty();
    if !boots_app && l.workbench.is_none() {
        return Err("This app needs Workbench to start from. Add a Workbench disk in the Amiga setup.".into());
    }

    let mut s = String::from("# Written by Floppy for each launch. Changes here are overwritten.\n[fs-uae]\n");
    s += &format!("base_dir = {}\n", conf_path(l.base_dir)?);
    s += &format!("amiga_model = {}\n", l.model);
    match l.kickstart {
        Some(k) => s += &format!("kickstart_file = {}\n", conf_path(k)?),
        // FS-UAE then loads no ROM file and doesn't scan for one: it
        // boots the AROS ROM in its own fs-uae.dat.
        None => s += "kickstart_file = internal\n",
    }

    // DF0: boots first (floppies have a higher boot priority than hard
    // drives). The app's first disk if it's a floppy, else Workbench's.
    // Every floppy is in the swap list (F12 menu) for multi-disk apps.
    let df0 = if l.app_disks.first().is_some_and(|d| is_floppy_image(&name_of(d))) {
        app_floppies.first().copied()
    } else if app_hdfs.is_empty() {
        wb_floppy
    } else {
        None
    };
    if let Some(d) = df0 {
        s += &format!("floppy_drive_0 = {}\n", conf_path(d)?);
    }
    let swap: Vec<&Path> = app_floppies.iter().copied().chain(wb_floppy).collect();
    for (i, d) in swap.iter().enumerate() {
        s += &format!("floppy_image_{i} = {}\n", conf_path(d)?);
    }

    // Hard drives: the app's hard-disk files, then Workbench's drive,
    // then the library, which never boots.
    let mut hd = 0;
    for (i, d) in app_hdfs.iter().enumerate() {
        s += &format!("hard_drive_{hd} = {}\n", conf_path(d)?);
        if i == 0 && df0.is_none() {
            s += &format!("hard_drive_{hd}_priority = 10\n");
        }
        hd += 1;
    }
    if let Some(w) = wb_drive {
        s += &format!("hard_drive_{hd} = {}\n", conf_path(w)?);
        if w.is_dir() {
            s += &format!("hard_drive_{hd}_label = Workbench\n");
        }
        hd += 1;
    }
    s += &format!("hard_drive_{hd} = {}\n", conf_path(l.shared)?);
    s += &format!("hard_drive_{hd}_label = {LIBRARY_VOLUME}\n");
    s += &format!("hard_drive_{hd}_priority = -128\n");
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    #[test]
    fn identifies_kickstarts() {
        let mut ks31 = vec![0u8; 16];
        ks31[..4].copy_from_slice(&[0x11, 0x14, 0x4E, 0xF9]);
        ks31[12..16].copy_from_slice(&[0, 40, 0, 68]);
        assert_eq!(identify_kickstart(&ks31, 524_288), Some(Kickstart::Plain { version: 40, revision: 68 }));
        // Right magic, wrong size.
        assert_eq!(identify_kickstart(&ks31, 1000), None);
        assert_eq!(identify_kickstart(b"AMIROMTYPE1\0\0\0\0\0", 524_299), Some(Kickstart::Encrypted));
        assert_eq!(identify_kickstart(b"not a rom at all", 262_144), None);
        assert_eq!((model_for(34), model_for(37), model_for(40)), ("A500", "A600", "A1200"));
        assert_eq!(kickstart_release(40), "3.1");
    }

    fn with_carry_checksum(mut bytes: Vec<u8>, at: usize) -> Vec<u8> {
        bytes[at..at + 4].copy_from_slice(&[0; 4]);
        let mut sum: u32 = 0;
        for w in bytes.chunks_exact(4) {
            let (s, c) = sum.overflowing_add(u32::from_be_bytes([w[0], w[1], w[2], w[3]]));
            sum = s + c as u32;
        }
        bytes[at..at + 4].copy_from_slice(&(!sum).to_be_bytes());
        bytes
    }

    #[test]
    fn checks_carry_sums_and_reads_adf_volumes() {
        let mut rom = vec![0u8; 1024];
        rom[..4].copy_from_slice(&[0x11, 0x14, 0x4E, 0xF9]);
        rom[500] = 0xAB;
        let rom = with_carry_checksum(rom, 1024 - 24);
        assert!(carry_sum_ok(&rom));
        let mut bad = rom.clone();
        bad[600] = 1;
        assert!(!carry_sum_ok(&bad));

        let t = TempDir::new();
        let mut adf = vec![0u8; 901_120];
        adf[0..4].copy_from_slice(b"DOS\0");
        adf[12] = 0x4E;
        let boot = with_carry_checksum(adf[..1024].to_vec(), 4);
        adf[..1024].copy_from_slice(&boot);
        let root = 880 * 512;
        adf[root + 432] = 12;
        adf[root + 433..root + 445].copy_from_slice(b"Workbench3.1");
        let p = t.path().join("wb.adf");
        fs::write(&p, &adf).unwrap();
        assert_eq!(adf_volume(&p), Some(("Workbench3.1".to_string(), true)));
        adf[12] = 0;
        fs::write(&p, &adf).unwrap();
        assert_eq!(adf_volume(&p), Some(("Workbench3.1".to_string(), false)));
        fs::write(&p, b"DOS").unwrap();
        assert_eq!(adf_volume(&p), None);
        assert_eq!(workbench_release(40), Some("3.1"));
    }

    #[test]
    fn finds_disks_and_executables() {
        let t = TempDir::new();
        let root = t.path();
        fs::create_dir_all(root.join("C")).unwrap();
        fs::write(root.join("ProTracker"), [0, 0, 3, 0xF3, 1, 2]).unwrap();
        fs::write(root.join("ProTracker.info"), [0xE3, 0x10]).unwrap();
        fs::write(root.join("Install-ProTracker"), [0, 0, 3, 0xF3]).unwrap();
        fs::write(root.join("C/LHA"), [0, 0, 3, 0xF3]).unwrap();
        fs::write(root.join("ReadMe.txt"), b"hi").unwrap();
        assert_eq!(find_programs(root, "ProTracker"), vec!["ProTracker", "C/LHA", "Install-ProTracker"]);

        let t = TempDir::new();
        fs::write(t.path().join("Game Disk 2.adf"), b"").unwrap();
        fs::write(t.path().join("Game Disk 1.adf"), b"").unwrap();
        assert_eq!(find_programs(t.path(), "Game"), vec!["Game Disk 1.adf", "Game Disk 2.adf"]);
    }

    #[test]
    fn conf_boots_app_floppy_with_workbench_in_swap_list() {
        let t = TempDir::new();
        let wb = t.path().join("Workbench.adf");
        fs::write(&wb, b"").unwrap();
        let conf = fsuae_conf(&Launch {
            base_dir: Path::new("/lib/run/fs-uae"),
            model: "A500",
            kickstart: Some(Path::new("/lib/system/amiga/kick13.rom")),
            workbench: Some(&wb),
            shared: Path::new("/lib/amiga"),
            app_disks: vec![PathBuf::from("/lib/amiga/Game/Disk1.adf"), PathBuf::from("/lib/amiga/Game/Disk2.adf")],
        })
        .unwrap();
        assert!(conf.contains("amiga_model = A500\n"));
        assert!(conf.contains("floppy_drive_0 = /lib/amiga/Game/Disk1.adf\n"));
        assert!(conf.contains("floppy_image_1 = /lib/amiga/Game/Disk2.adf\n"));
        assert!(conf.contains(&format!("floppy_image_2 = {}\n", wb.display())));
        assert!(conf.contains("hard_drive_0 = /lib/amiga\nhard_drive_0_label = Floppy\nhard_drive_0_priority = -128\n"));
    }

    #[test]
    fn conf_boots_workbench_folder_for_folder_apps() {
        let t = TempDir::new();
        let wb = t.path().join("Workbench");
        fs::create_dir_all(&wb).unwrap();
        let launch = |workbench| Launch {
            base_dir: Path::new("/b"),
            model: "A1200",
            kickstart: Some(Path::new("/k.rom")),
            workbench,
            shared: Path::new("/lib/amiga"),
            app_disks: vec![],
        };
        let conf = fsuae_conf(&launch(Some(&wb))).unwrap();
        assert!(!conf.contains("floppy_drive_0"));
        assert!(conf.contains(&format!("hard_drive_0 = {}\nhard_drive_0_label = Workbench\n", wb.display())));
        assert!(conf.contains("hard_drive_1 = /lib/amiga\n"));
        assert!(fsuae_conf(&launch(None)).unwrap_err().contains("Workbench"));
    }

    #[test]
    fn conf_boots_app_hard_disk_file() {
        let conf = fsuae_conf(&Launch {
            base_dir: Path::new("/b"),
            model: "A1200",
            kickstart: Some(Path::new("/k.rom")),
            workbench: None,
            shared: Path::new("/lib/amiga"),
            app_disks: vec![PathBuf::from("/lib/amiga/Demo/demo.hdf")],
        })
        .unwrap();
        assert!(conf.contains("hard_drive_0 = /lib/amiga/Demo/demo.hdf\nhard_drive_0_priority = 10\n"));
        assert!(conf.contains("hard_drive_1 = /lib/amiga\n"));
    }

    #[test]
    fn no_kickstart_means_fs_uaes_built_in_aros() {
        let conf = fsuae_conf(&Launch {
            base_dir: Path::new("/lib/run/fs-uae"),
            model: "A1200",
            kickstart: None,
            workbench: None,
            shared: Path::new("/lib/amiga"),
            app_disks: vec![PathBuf::from("/lib/amiga/Game/game.adf")],
        })
        .unwrap();
        assert!(conf.contains("kickstart_file = internal\n"), "{conf}");
    }
}
