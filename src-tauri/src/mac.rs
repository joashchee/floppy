//! Classic Mac OS guest specifics: keeping resource forks and Finder info
//! intact on import, finding the apps, and the Basilisk II prefs file that
//! boots them.
//!
//! A classic Mac app's code lives in its resource fork, and the Finder
//! only treats a file as an app when its Finder info says type `APPL`.
//! Basilisk II's shared folder (`extfs`) on a macOS host reads both
//! natively: the fork at `file/..namedfork/rsrc` and Finder info in the
//! `com.apple.FinderInfo` attribute. On Linux it keeps them in hidden
//! helper folders beside the file: `.rsrc/<name>` and `.finf/<name>`.
//! So an import has to land both there:
//!
//! - Folders: `fs::copy` on macOS copies forks and attributes. Elsewhere
//!   the folder copy brings any `.rsrc`/`.finf` folders along.
//! - Zips made on a Mac carry them as AppleDouble files
//!   (`__MACOSX/…/._name`, or `._name` next to the file). They're
//!   merged back into the file they describe (`absorb_appledouble`).
//! - MacBinary (`.bin`) files are decoded (`decode_macbinary`).
//!
//! StuffIt, BinHex and Compact Pro archives are copied as-is, to expand
//! inside the Mac with the user's own StuffIt Expander. Disk images are
//! mounted as extra disks when launched.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// HFS file names are at most 31 characters.
pub const MAX_NAME: usize = 31;

/// Disk image extensions Basilisk II mounts as a disk (it detects Disk
/// Copy 4.2 headers itself).
pub const DISK_IMAGE_EXTS: &[&str] = &["dsk", "img", "image", "dc42", "hfv", "hda", "toast", "iso"];

/// Archives to expand inside the Mac, with (type, creator) codes the
/// period expanders recognize.
const ARCHIVES: &[(&str, &[u8; 4], &[u8; 4])] = &[
    ("sit", b"SIT!", b"SIT!"),
    ("sitx", b"SITX", b"SITx"),
    ("hqx", b"TEXT", b"BnHq"),
    ("cpt", b"PACT", b"CPCT"),
    ("sea", b"APPL", b"aust"),
];

/// Extensions of MacBinary files.
pub const MACBINARY_EXTS: &[&str] = &["bin", "macbin"];

fn ext_of(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_disk_image(name: &str) -> bool {
    DISK_IMAGE_EXTS.contains(&ext_of(name).as_str())
}

fn archive_codes(name: &str) -> Option<([u8; 4], [u8; 4])> {
    let ext = ext_of(name);
    ARCHIVES.iter().find(|(e, _, _)| *e == ext).map(|(_, t, c)| (**t, **c))
}

// ---------------------------------------------------------------------
// Forks and Finder info (host side, see docs/platform-parity.md).

/// The 32 bytes of Finder info (FInfo + FXInfo).
pub type FinderInfo = [u8; 32];

#[cfg(target_os = "macos")]
const FINDER_INFO_ATTR: &[u8] = b"com.apple.FinderInfo\0";

#[cfg(target_os = "macos")]
fn c_path(path: &Path) -> io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(io::Error::other)
}

/// Reads a file's Finder info, or `None` if it has none.
#[cfg(target_os = "macos")]
pub fn read_finder_info(path: &Path) -> Option<FinderInfo> {
    let p = c_path(path).ok()?;
    let mut buf = [0u8; 32];
    // SAFETY: valid NUL-terminated strings and a 32-byte buffer.
    let n = unsafe {
        libc::getxattr(p.as_ptr(), FINDER_INFO_ATTR.as_ptr().cast(), buf.as_mut_ptr().cast(), buf.len(), 0, 0)
    };
    (n == 32).then_some(buf)
}

#[cfg(target_os = "macos")]
pub fn write_finder_info(path: &Path, info: &FinderInfo) -> io::Result<()> {
    let p = c_path(path)?;
    // SAFETY: valid NUL-terminated strings and a 32-byte buffer.
    let r = unsafe {
        libc::setxattr(p.as_ptr(), FINDER_INFO_ATTR.as_ptr().cast(), info.as_ptr().cast(), info.len(), 0, 0)
    };
    if r == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
pub fn write_resource_fork(path: &Path, data: &[u8]) -> io::Result<()> {
    fs::write(path.join("..namedfork/rsrc"), data)
}

#[cfg(all(test, target_os = "macos"))]
pub fn resource_fork_len(path: &Path) -> u64 {
    fs::metadata(path.join("..namedfork/rsrc")).map(|m| m.len()).unwrap_or(0)
}

// Elsewhere Basilisk II's extfs keeps them in helper folders beside the
// file, which it hides from the Mac: the fork in `.rsrc/<name>` and the
// Finder info (FInfo then FXInfo) in `.finf/<name>`.

#[cfg(not(target_os = "macos"))]
pub const HELPER_DIRS: &[&str] = &[".rsrc", ".finf"];

#[cfg(not(target_os = "macos"))]
fn helper_path(path: &Path, dir: &str) -> io::Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| io::Error::other("no file name"))?;
    Ok(path.parent().unwrap_or(Path::new("")).join(dir).join(name))
}

#[cfg(not(target_os = "macos"))]
fn write_helper(path: &Path, dir: &str, data: &[u8]) -> io::Result<()> {
    let out = helper_path(path, dir)?;
    if let Some(d) = out.parent() {
        fs::create_dir_all(d)?;
    }
    fs::write(out, data)
}

/// Reads a file's Finder info, or `None` if it has none. Basilisk II
/// accepts a bare FInfo (16 bytes) too, so a short file is padded.
#[cfg(not(target_os = "macos"))]
pub fn read_finder_info(path: &Path) -> Option<FinderInfo> {
    let bytes = fs::read(helper_path(path, ".finf").ok()?).ok()?;
    if bytes.len() < 16 {
        return None;
    }
    let mut info = [0u8; 32];
    let n = bytes.len().min(32);
    info[..n].copy_from_slice(&bytes[..n]);
    Some(info)
}

#[cfg(not(target_os = "macos"))]
pub fn write_finder_info(path: &Path, info: &FinderInfo) -> io::Result<()> {
    write_helper(path, ".finf", info)
}

#[cfg(not(target_os = "macos"))]
pub fn write_resource_fork(path: &Path, data: &[u8]) -> io::Result<()> {
    write_helper(path, ".rsrc", data)
}

#[cfg(all(test, not(target_os = "macos")))]
pub fn resource_fork_len(path: &Path) -> u64 {
    helper_path(path, ".rsrc").and_then(fs::metadata).map(|m| m.len()).unwrap_or(0)
}

/// Whether `name` is one of Basilisk II's hidden fork folders, which hold
/// other files' forks rather than files of their own.
fn is_fork_helper(name: &std::ffi::OsStr) -> bool {
    #[cfg(target_os = "macos")]
    let helpers: &[&str] = &[];
    #[cfg(not(target_os = "macos"))]
    let helpers = HELPER_DIRS;
    helpers.iter().any(|h| name == *h)
}

/// A file's type code, if it has Finder info.
pub fn file_type(path: &Path) -> Option<[u8; 4]> {
    read_finder_info(path).map(|i| [i[0], i[1], i[2], i[3]]).filter(|t| t != &[0; 4])
}

// ---------------------------------------------------------------------
// AppleDouble.

/// The resource fork and Finder info an AppleDouble file carries.
#[derive(Debug, Default, PartialEq)]
pub struct AppleDouble {
    pub resource_fork: Option<Vec<u8>>,
    pub finder_info: Option<FinderInfo>,
}

/// Parses an AppleDouble (or AppleSingle) header, or `None` if `bytes`
/// isn't one.
pub fn parse_appledouble(bytes: &[u8]) -> Option<AppleDouble> {
    let be32 = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let magic = be32(0)?;
    if magic != 0x0005_1607 && magic != 0x0005_1600 {
        return None;
    }
    let count = u16::from_be_bytes([*bytes.get(24)?, *bytes.get(25)?]) as usize;
    let mut out = AppleDouble::default();
    for i in 0..count {
        let e = 26 + i * 12;
        let (id, off, len) = (be32(e)?, be32(e + 4)? as usize, be32(e + 8)? as usize);
        let data = bytes.get(off..off.checked_add(len)?)?;
        match id {
            2 if !data.is_empty() => out.resource_fork = Some(data.to_vec()),
            // macOS writes a longer entry 9 (Finder info followed by other
            // extended attributes). Only the first 32 bytes are Finder info.
            9 if data.len() >= 32 => out.finder_info = Some(data[..32].try_into().ok()?),
            _ => {}
        }
    }
    Some(out)
}

fn apply_appledouble(target: &Path, ad: &AppleDouble) -> io::Result<()> {
    if let Some(info) = &ad.finder_info {
        if info != &[0; 32] {
            write_finder_info(target, info)?;
        }
    }
    if let Some(rsrc) = &ad.resource_fork {
        if target.is_file() {
            write_resource_fork(target, rsrc)?;
        }
    }
    Ok(())
}

/// Merges AppleDouble files under `root` back into the files they
/// describe, then removes them, `__MACOSX` folders, and `.DS_Store`.
/// Handles both a top-level `__MACOSX/<path>/._<name>` (Finder's
/// "Compress") and `._<name>` next to `<name>` (copies from non-Mac
/// volumes, `ditto` without `--sequesterRsrc`).
pub fn absorb_appledouble(root: &Path) -> io::Result<()> {
    let sequestered = root.join("__MACOSX");
    let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    for entry in WalkDir::new(root).min_depth(1).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        let Some(real) = name.strip_prefix("._") else { continue };
        let parent = entry.path().parent().unwrap_or(root);
        let target_parent = match parent.strip_prefix(&sequestered) {
            Ok(rel) => root.join(rel),
            Err(_) => parent.to_path_buf(),
        };
        pairs.push((entry.path().to_path_buf(), target_parent.join(real)));
    }
    for (ad_path, target) in &pairs {
        if target.exists() {
            if let Some(ad) = parse_appledouble(&fs::read(ad_path)?) {
                apply_appledouble(target, &ad)?;
            }
        }
        fs::remove_file(ad_path)?;
    }
    if sequestered.exists() {
        fs::remove_dir_all(&sequestered)?;
    }
    for entry in WalkDir::new(root).min_depth(1).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == ".DS_Store" {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// MacBinary.

/// CRC-16/XMODEM, as MacBinary II uses for its header.
fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

fn pad128(n: usize) -> usize {
    n.div_ceil(128) * 128
}

/// A decoded MacBinary file.
#[derive(Debug, PartialEq)]
pub struct MacBinary {
    /// The file's Mac name when it's plain ASCII, else `None` (Mac Roman
    /// names aren't translated).
    pub name: Option<String>,
    pub finder_info: FinderInfo,
    pub data: Vec<u8>,
    pub resource_fork: Vec<u8>,
}

/// Decodes MacBinary I, II or III, or `None` if `bytes` isn't MacBinary.
pub fn parse_macbinary(bytes: &[u8]) -> Option<MacBinary> {
    let h = bytes.get(..128)?;
    let name_len = h[1] as usize;
    if h[0] != 0 || h[74] != 0 || h[82] != 0 || !(1..=63).contains(&name_len) {
        return None;
    }
    let crc_ok = u16::from_be_bytes([h[124], h[125]]) == crc16_xmodem(&h[..124]);
    // MacBinary I has no CRC; its unused bytes 99..=125 are zero instead.
    if !crc_ok && h[99..=125].iter().any(|&b| b != 0) {
        return None;
    }
    let be32 = |o: usize| u32::from_be_bytes([h[o], h[o + 1], h[o + 2], h[o + 3]]) as usize;
    let (data_len, rsrc_len) = (be32(83), be32(87));
    let secondary = if crc_ok { pad128(u16::from_be_bytes([h[120], h[121]]) as usize) } else { 0 };
    let data_start = 128 + secondary;
    let rsrc_start = data_start.checked_add(pad128(data_len))?;
    let data = bytes.get(data_start..data_start.checked_add(data_len)?)?.to_vec();
    let resource_fork = bytes.get(rsrc_start..rsrc_start.checked_add(rsrc_len)?)?.to_vec();

    let mut info = [0u8; 32];
    info[0..8].copy_from_slice(&h[65..73]); // type, creator
    // Finder flags: high byte in the header, low byte from MacBinary II.
    // "Has been inited" (0x0100) is cleared so the Finder places the icon
    // afresh, as is its old window location.
    let flags = u16::from_be_bytes([h[73], if crc_ok { h[101] } else { 0 }]) & !0x0100;
    info[8..10].copy_from_slice(&flags.to_be_bytes());
    let raw_name = &h[2..2 + name_len];
    let name = raw_name
        .iter()
        .all(|b| b.is_ascii_graphic() || *b == b' ')
        .then(|| String::from_utf8_lossy(raw_name).into_owned());
    Some(MacBinary { name, finder_info: info, data, resource_fork })
}

/// MacBinary files bigger than this aren't judged: judging one means
/// reading it whole, and an old Mac file this big is really a disk image.
pub const MACBINARY_JUDGE_MAX: u64 = 64 << 20;

/// A file's MacBinary contents, if it's MacBinary and at most
/// `MACBINARY_JUDGE_MAX`: the header is checked before the rest is read.
pub fn read_macbinary(path: &Path) -> Option<MacBinary> {
    let len = fs::metadata(path).ok()?.len();
    if !(128..=MACBINARY_JUDGE_MAX).contains(&len) {
        return None;
    }
    let head = crate::library::read_head(path, 128).ok()?;
    if head.len() < 128 || head[0] != 0 || head[74] != 0 || head[82] != 0 || !(1..=63).contains(&head[1]) {
        return None;
    }
    parse_macbinary(&fs::read(path).ok()?)
}

/// Writes a MacBinary file's contents as a real file (data fork,
/// resource fork, Finder info) in `dest_dir`. Returns its name.
pub fn decode_macbinary(src: &Path, dest_dir: &Path, fallback_name: &str) -> Result<String, String> {
    let bytes = fs::read(src).map_err(|e| e.to_string())?;
    let mb = parse_macbinary(&bytes).ok_or("it isn't a MacBinary file")?;
    let name = sanitize_name(mb.name.as_deref().unwrap_or(fallback_name), MAX_NAME, "Untitled");
    let out = dest_dir.join(&name);
    fs::write(&out, &mb.data).map_err(|e| e.to_string())?;
    if !mb.resource_fork.is_empty() {
        write_resource_fork(&out, &mb.resource_fork).map_err(|e| e.to_string())?;
    }
    write_finder_info(&out, &mb.finder_info).map_err(|e| e.to_string())?;
    Ok(name)
}

/// Gives a copied archive the type and creator its expander looks for,
/// unless it already has Finder info.
pub fn tag_archive(path: &Path) -> io::Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    match archive_codes(&name) {
        Some((t, c)) if file_type(path).is_none() => {
            let mut info = [0u8; 32];
            info[0..4].copy_from_slice(&t);
            info[4..8].copy_from_slice(&c);
            write_finder_info(path, &info)
        }
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------
// Names and programs.

/// A folder or file name every guest filesystem accepts: no path
/// separators (`:` on the Mac, `/` on the Amiga and host), no control
/// characters, no leading dot (hidden on the host), Latin-1 only, and at
/// most `max` characters. Shared with the Amiga guest.
pub fn sanitize_name(name: &str, max: usize, fallback: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, ':' | '/' | '\\') && !c.is_control() && (*c as u32) <= 0xFF)
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.').trim();
    let out: String = trimmed.chars().take(max).collect::<String>().trim_end().to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

/// How likely `rel` is what the user wants to open. Apps beat disk
/// images, which beat archives; shallow and name-matching paths win.
fn score(rel: &str, kind: u8, name_hint: &str) -> i32 {
    let depth = rel.matches('/').count() as i32;
    let file = rel.rsplit('/').next().unwrap_or(rel).to_lowercase();
    let hint = name_hint.to_lowercase();
    let mut s = -depth * 10 - kind as i32 * 100;
    if !hint.is_empty() && (file == hint || file.starts_with(&hint) || hint.starts_with(&file)) {
        s += 30;
    }
    if file.contains("install") || file.contains("uninstall") || file.contains("read me") || file.contains("readme") {
        s -= 40;
    }
    s
}

/// What can be opened under `root`: apps (Finder type `APPL`), disk
/// images, and archives to expand, best candidate first, as `/`-separated
/// paths relative to `root`.
pub fn find_programs(root: &Path, name_hint: &str) -> Vec<String> {
    let mut found: Vec<(String, u8)> = WalkDir::new(root)
        .min_depth(1)
        .max_depth(6)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && is_fork_helper(e.file_name())))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let kind = if file_type(e.path()) == Some(*b"APPL") {
                0
            } else if is_disk_image(&name) {
                1
            } else if archive_codes(&name).is_some() {
                2
            } else {
                return None;
            };
            let rel = e.path().strip_prefix(root).ok()?;
            let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
            Some((parts?.join("/"), kind))
        })
        .collect();
    found.sort_by(|(a, ka), (b, kb)| score(b, *kb, name_hint).cmp(&score(a, *ka, name_hint)).then_with(|| a.cmp(b)));
    found.truncate(200);
    found.into_iter().map(|(p, _)| p).collect()
}

// ---------------------------------------------------------------------
// Recognizing system files by content (for the files disc import).

/// ROM version word Basilisk II requires: a 32-bit clean ROM
/// (`ROM_VERSION_32` in its rom_patches.h). Its `CheckROM` refuses the
/// others (Mac Plus, SE/Classic, original Mac II).
const ROM_VERSION_32: u16 = 0x067C;

/// Whether a file is a Mac ROM Basilisk II boots: 512 KB or 1 MB with a
/// 32-bit clean version word at offset 8.
pub fn is_basilisk_rom(head: &[u8], len: u64) -> bool {
    matches!(len, 524_288 | 1_048_576) && head.get(8..10) == Some(&ROM_VERSION_32.to_be_bytes()[..])
}

/// Whether a whole ROM matches the checksum in its first long word (the
/// sum of every 16-bit word after it). Only used to prefer one copy over
/// another, never to reject a ROM: this hasn't been checked against a
/// real ROM here, since Floppy can't ship one to test with.
pub fn rom_checksum_ok(rom: &[u8]) -> bool {
    let Some(stored) = rom.get(..4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])) else { return false };
    let sum = rom[4..].chunks_exact(2).fold(0u32, |acc, w| acc.wrapping_add(u16::from_be_bytes([w[0], w[1]]) as u32));
    sum == stored
}

fn read_at(f: &mut fs::File, off: u64, len: usize) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    f.seek(SeekFrom::Start(off)).ok()?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn be16(b: &[u8], o: usize) -> u16 {
    u16::from_be_bytes([b[o], b[o + 1]])
}

fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

/// The name of the bootable (blessed) HFS or HFS+ volume in a disk image,
/// or `None` if it has none. A volume is bootable when its Finder info
/// names a blessed System Folder. Handles raw volumes, Disk Copy 4.2
/// images (84-byte header), Apple-partitioned hard-disk images, and
/// HFS+ volumes inside an HFS wrapper.
pub fn bootable_volume(path: &Path) -> Option<String> {
    first_volume(path, true)
}

/// The name of the first HFS or HFS+ volume in a disk image, bootable or
/// not (see `bootable_volume` for the layouts it reads). Only macOS's
/// old-media watcher (`media.rs`) uses it so far.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn volume_name(path: &Path) -> Option<String> {
    first_volume(path, false)
}

fn first_volume(path: &Path, bootable_only: bool) -> Option<String> {
    let mut f = fs::File::open(path).ok()?;
    let mut bases = vec![0u64, 84];
    // Apple partition map: "ER" in block 0, "PM" entries from block 1.
    if read_at(&mut f, 0, 2).as_deref() == Some(b"ER") {
        let mut count = 1u32;
        let mut i = 1u32;
        while i <= count.min(64) {
            let Some(e) = read_at(&mut f, u64::from(i) * 512, 512) else { break };
            if &e[0..2] != b"PM" {
                break;
            }
            count = be32(&e, 4);
            if e[48..80].starts_with(b"Apple_HFS") {
                bases.push(u64::from(be32(&e, 8)) * 512);
            }
            i += 1;
        }
    }
    bases.into_iter().find_map(|base| volume_at(&mut f, base, bootable_only))
}

fn volume_at(f: &mut fs::File, base: u64, bootable_only: bool) -> Option<String> {
    let v = read_at(f, base + 1024, 512)?;
    match &v[0..2] {
        b"BD" => {
            let name_len = (v[36] as usize).min(27);
            let raw = &v[37..37 + name_len];
            let name = if raw.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
                String::from_utf8_lossy(raw).into_owned()
            } else {
                "Untitled".into()
            };
            // An HFS wrapper around an HFS+ volume: the real Finder info is
            // in the embedded volume.
            if &v[0x7C..0x7E] == b"H+" {
                let embedded = base + u64::from(be16(&v, 28)) * 512 + u64::from(be16(&v, 0x7E)) * u64::from(be32(&v, 20));
                return volume_at(f, embedded, bootable_only).map(|_| name);
            }
            (!bootable_only || be32(&v, 92) != 0).then_some(name)
        }
        b"H+" | b"HX" => (!bootable_only || be32(&v, 0x50) != 0).then(|| "HFS+ volume".to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Basilisk II.

/// What to boot.
pub struct Launch<'a> {
    /// The user's Mac ROM (Mac II, IIci, Quadra and similar).
    pub rom: &'a Path,
    /// The user's startup disk image, with System 7 to Mac OS 8.1.
    pub boot_disk: &'a Path,
    /// Floppy's Mac library, shared into the Mac as a volume.
    pub shared: &'a Path,
    /// Extra disk images to mount (the app's own, when it is one).
    pub disks: Vec<PathBuf>,
    /// RAM in megabytes.
    pub ram_mb: u32,
}

fn prefs_path(p: &Path) -> Result<&str, String> {
    let s = p.to_str().ok_or("A path Floppy passes to Basilisk II isn't valid UTF-8.")?;
    if s.contains('\n') || s.contains('\r') {
        return Err("A path Floppy passes to Basilisk II contains a line break.".into());
    }
    Ok(s)
}

/// The prefs file Floppy passes to Basilisk II for one launch. It
/// emulates a Quadra 900 (68040 with FPU), the model with the widest
/// System 7 to Mac OS 8.1 support.
pub fn basilisk_prefs(l: &Launch) -> Result<String, String> {
    let mut s = String::from("# Written by Floppy for each launch. Changes here are overwritten.\n");
    s += &format!("rom {}\n", prefs_path(l.rom)?);
    s += &format!("disk {}\n", prefs_path(l.boot_disk)?);
    for d in &l.disks {
        s += &format!("disk {}\n", prefs_path(d)?);
    }
    s += &format!("extfs {}\n", prefs_path(l.shared)?);
    s += "bootdrive 0\nbootdriver 0\n";
    s += &format!("ramsize {}\n", u64::from(l.ram_mb) * 1024 * 1024);
    s += "modelid 14\ncpu 4\nfpu true\n";
    s += "screen win/800/600\n";
    s += "nocdrom true\nnogui true\njit false\n";
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    fn appledouble(rsrc: &[u8], info: &FinderInfo) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&0x0005_1607u32.to_be_bytes());
        v.extend_from_slice(&0x0002_0000u32.to_be_bytes());
        v.extend_from_slice(&[0; 16]);
        v.extend_from_slice(&2u16.to_be_bytes());
        let info_off = 26 + 24;
        // A macOS-style long entry 9: Finder info plus trailing attribute data.
        let info_len = 32 + 8;
        let rsrc_off = info_off + info_len;
        for (id, off, len) in [(9u32, info_off, info_len), (2, rsrc_off, rsrc.len())] {
            v.extend_from_slice(&id.to_be_bytes());
            v.extend_from_slice(&(off as u32).to_be_bytes());
            v.extend_from_slice(&(len as u32).to_be_bytes());
        }
        v.extend_from_slice(info);
        v.extend_from_slice(b"ATTRDATA");
        v.extend_from_slice(rsrc);
        v
    }

    fn app_info() -> FinderInfo {
        let mut i = [0u8; 32];
        i[0..8].copy_from_slice(b"APPLMWII");
        i
    }

    fn macbinary(name: &str, info: &[u8; 8], data: &[u8], rsrc: &[u8], mb2: bool) -> Vec<u8> {
        let mut h = [0u8; 128];
        h[1] = name.len() as u8;
        h[2..2 + name.len()].copy_from_slice(name.as_bytes());
        h[65..73].copy_from_slice(info);
        h[73] = 0x01; // "inited" high byte bit, which decoding clears
        h[83..87].copy_from_slice(&(data.len() as u32).to_be_bytes());
        h[87..91].copy_from_slice(&(rsrc.len() as u32).to_be_bytes());
        if mb2 {
            h[122] = 129;
            h[123] = 129;
            let crc = crc16_xmodem(&h[..124]);
            h[124..126].copy_from_slice(&crc.to_be_bytes());
        }
        let mut v = h.to_vec();
        v.extend_from_slice(data);
        v.resize(128 + pad128(data.len()), 0);
        v.extend_from_slice(rsrc);
        v.resize(v.len() + (pad128(rsrc.len()) - rsrc.len()), 0);
        v
    }

    #[test]
    fn parses_appledouble() {
        let ad = parse_appledouble(&appledouble(b"RSRC", &app_info())).unwrap();
        assert_eq!(ad.resource_fork.as_deref(), Some(&b"RSRC"[..]));
        assert_eq!(ad.finder_info, Some(app_info()));
        assert!(parse_appledouble(b"not appledouble at all, nope").is_none());
        // Entries pointing past the end are rejected, not a panic.
        let mut bad = appledouble(b"RSRC", &app_info());
        bad.truncate(60);
        assert!(parse_appledouble(&bad).is_none());
    }

    #[test]
    fn parses_macbinary_i_and_ii() {
        for mb2 in [false, true] {
            let bytes = macbinary("MacWrite II", b"APPLMWII", b"data!", &[7; 300], mb2);
            let mb = parse_macbinary(&bytes).unwrap();
            assert_eq!(mb.name.as_deref(), Some("MacWrite II"));
            assert_eq!(&mb.finder_info[0..8], b"APPLMWII");
            assert_eq!(mb.finder_info[8] & 0x01, 0, "inited flag cleared");
            assert_eq!(mb.data, b"data!");
            assert_eq!(mb.resource_fork, vec![7; 300]);
        }
        let mut corrupt = macbinary("X", b"APPLXXXX", b"", b"", true);
        corrupt[124] ^= 0xFF;
        corrupt[100] = 1;
        assert!(parse_macbinary(&corrupt).is_none());
        assert!(parse_macbinary(&[0u8; 50]).is_none());
        // Lengths past the end of the file.
        let mut short = macbinary("X", b"APPLXXXX", b"abc", b"", true);
        short.truncate(129);
        assert!(parse_macbinary(&short).is_none());
    }

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize_name("MacWrite II", 31, "App"), "MacWrite II");
        assert_eq!(sanitize_name("a:b/c", 31, "App"), "abc");
        assert_eq!(sanitize_name(".hidden", 31, "App"), "hidden");
        assert_eq!(sanitize_name("日本", 31, "App"), "App");
        assert_eq!(sanitize_name("Café", 31, "App"), "Café");
        assert_eq!(sanitize_name(&"x".repeat(40), 31, "App").len(), 31);
        assert_eq!(sanitize_name("ClarisWorks 4.0 Folder With Long Name", 31, "App"), "ClarisWorks 4.0 Folder With Lon");
    }

    #[test]
    fn a_huge_file_is_never_read_whole_to_judge_it() {
        let t = crate::testutil::TempDir::new();
        let big = t.path().join("huge.bin");
        let f = fs::File::create(&big).unwrap();
        // Sparse: costs nothing on disk, but reading it whole would.
        f.set_len(MACBINARY_JUDGE_MAX + 1).unwrap();
        assert!(read_macbinary(&big).is_none());
        let small = t.path().join("small.bin");
        fs::write(&small, [0u8; 200]).unwrap();
        assert!(read_macbinary(&small).is_none(), "not MacBinary: no name length");
    }

    #[test]
        fn prefs_boot_the_disk_and_share_the_library() {
        let p = basilisk_prefs(&Launch {
            rom: Path::new("/lib/system/mac/Quadra.ROM"),
            boot_disk: Path::new("/lib/system/mac/System 7.6.dsk"),
            shared: Path::new("/lib/mac"),
            disks: vec![PathBuf::from("/lib/mac/Game/Game.dsk")],
            ram_mb: 64,
        })
        .unwrap();
        assert!(p.contains("rom /lib/system/mac/Quadra.ROM\n"));
        assert!(p.contains("disk /lib/system/mac/System 7.6.dsk\ndisk /lib/mac/Game/Game.dsk\n"));
        assert!(p.contains("extfs /lib/mac\n"));
        assert!(p.contains("ramsize 67108864\n"));
        assert!(p.contains("nogui true\n"));
        let bad = PathBuf::from("/odd\npath");
        assert!(basilisk_prefs(&Launch { rom: &bad, boot_disk: &bad, shared: &bad, disks: vec![], ram_mb: 8 }).is_err());
    }

    fn hfs_image(blessed: bool, header: usize) -> Vec<u8> {
        let mut img = vec![0u8; header + 1024 + 512];
        let mdb = header + 1024;
        img[mdb..mdb + 2].copy_from_slice(b"BD");
        img[mdb + 36] = 12;
        img[mdb + 37..mdb + 49].copy_from_slice(b"Macintosh HD");
        if blessed {
            img[mdb + 92..mdb + 96].copy_from_slice(&42u32.to_be_bytes());
        }
        img
    }

    #[test]
    fn recognizes_bootable_hfs_images() {
        let t = TempDir::new();
        for (name, img) in [("raw.dsk", hfs_image(true, 0)), ("dc42.image", hfs_image(true, 84))] {
            let p = t.path().join(name);
            fs::write(&p, img).unwrap();
            assert_eq!(bootable_volume(&p).as_deref(), Some("Macintosh HD"), "{name}");
        }
        let p = t.path().join("data.dsk");
        fs::write(&p, hfs_image(false, 0)).unwrap();
        assert_eq!(bootable_volume(&p), None);

        // An Apple-partitioned hard-disk image with HFS in block 64.
        let mut img = vec![0u8; 64 * 512];
        img[0..2].copy_from_slice(b"ER");
        img[512..514].copy_from_slice(b"PM");
        img[516..520].copy_from_slice(&1u32.to_be_bytes());
        img[520..524].copy_from_slice(&64u32.to_be_bytes());
        img[512 + 48..512 + 57].copy_from_slice(b"Apple_HFS");
        img.extend(hfs_image(true, 0));
        let p = t.path().join("hd.hda");
        fs::write(&p, img).unwrap();
        assert_eq!(bootable_volume(&p).as_deref(), Some("Macintosh HD"));
        fs::write(&p, b"short").unwrap();
        assert_eq!(bootable_volume(&p), None);
    }

    #[test]
    fn recognizes_basilisk_roms() {
        let mut rom = vec![0u8; 524_288];
        rom[8..10].copy_from_slice(&[0x06, 0x7C]);
        rom[100] = 5;
        let sum: u32 = rom[4..].chunks_exact(2).map(|w| u16::from_be_bytes([w[0], w[1]]) as u32).sum();
        rom[..4].copy_from_slice(&sum.to_be_bytes());
        assert!(is_basilisk_rom(&rom[..16], rom.len() as u64));
        assert!(rom_checksum_ok(&rom));
        rom[200] = 1;
        assert!(!rom_checksum_ok(&rom));
        // A Mac Plus ROM (version 0x0075) or the wrong size isn't usable.
        assert!(!is_basilisk_rom(&[0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x75], 131_072));
        assert!(!is_basilisk_rom(&rom[..16], 262_144));
    }

    /// The resource fork as the host stores it for Basilisk II.
    fn fork(path: &Path) -> Vec<u8> {
        #[cfg(target_os = "macos")]
        let at = path.join("..namedfork/rsrc");
        #[cfg(not(target_os = "macos"))]
        let at = path.parent().unwrap().join(".rsrc").join(path.file_name().unwrap());
        fs::read(at).unwrap_or_default()
    }

    #[test]
    fn absorbs_sequestered_and_sibling_appledouble() {
        let t = TempDir::new();
        let root = t.path();
        fs::create_dir_all(root.join("MacWrite/Tools")).unwrap();
        fs::write(root.join("MacWrite/MacWrite II"), b"").unwrap();
        fs::write(root.join("MacWrite/Tools/Helper"), b"").unwrap();
        fs::create_dir_all(root.join("__MACOSX/MacWrite")).unwrap();
        fs::write(root.join("__MACOSX/MacWrite/._MacWrite II"), appledouble(b"CODE", &app_info())).unwrap();
        fs::write(root.join("MacWrite/Tools/._Helper"), appledouble(b"HELP", &app_info())).unwrap();
        fs::write(root.join("MacWrite/._Gone"), appledouble(b"X", &app_info())).unwrap();
        fs::write(root.join("MacWrite/.DS_Store"), b"").unwrap();
        absorb_appledouble(root).unwrap();

        let app = root.join("MacWrite/MacWrite II");
        assert_eq!(fork(&app), b"CODE");
        assert_eq!(read_finder_info(&app), Some(app_info()));
        assert_eq!(resource_fork_len(&root.join("MacWrite/Tools/Helper")), 4);
        assert!(!root.join("__MACOSX").exists());
        assert!(!root.join("MacWrite/._Gone").exists());
        assert!(!root.join("MacWrite/.DS_Store").exists());
        assert_eq!(find_programs(root, "MacWrite"), vec!["MacWrite/MacWrite II", "MacWrite/Tools/Helper"]);
    }

    #[test]
    fn decodes_macbinary_to_a_real_file() {
        let t = TempDir::new();
        let src = t.path().join("macwrite.bin");
        fs::write(&src, macbinary("MacWrite II", b"APPLMWII", b"data", b"CODE", true)).unwrap();
        let name = decode_macbinary(&src, t.path(), "macwrite").unwrap();
        assert_eq!(name, "MacWrite II");
        let out = t.path().join(&name);
        assert_eq!(fs::read(&out).unwrap(), b"data");
        assert_eq!(fork(&out), b"CODE");
        assert_eq!(file_type(&out), Some(*b"APPL"));
    }

    #[test]
    fn forks_mark_apps_and_archives_get_tagged() {
        let t = TempDir::new();
        let a = t.path().join("App");
        fs::write(&a, b"").unwrap();
        write_resource_fork(&a, b"CODE").unwrap();
        write_finder_info(&a, &app_info()).unwrap();
        assert_eq!(resource_fork_len(&a), 4);
        assert_eq!(file_type(&a), Some(*b"APPL"));
        // A disk image with a fork of its own is still one disk image.
        fs::write(t.path().join("Extra.dsk"), b"").unwrap();
        write_resource_fork(&t.path().join("Extra.dsk"), b"ckid").unwrap();

        let sit = t.path().join("Game.sit");
        fs::write(&sit, b"SIT!").unwrap();
        tag_archive(&sit).unwrap();
        assert_eq!(read_finder_info(&sit).map(|i| i[4..8].to_vec()), Some(b"SIT!".to_vec()));
        assert_eq!(find_programs(t.path(), "App"), vec!["App", "Extra.dsk", "Game.sit"]);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fs_copy_keeps_forks() {
        // library.rs copies folders with fs::copy; this pins down that it
        // carries the resource fork and Finder info on macOS.
        let t = TempDir::new();
        let a = t.path().join("App");
        fs::write(&a, b"").unwrap();
        write_resource_fork(&a, b"CODE").unwrap();
        write_finder_info(&a, &app_info()).unwrap();
        let b = t.path().join("Copy");
        fs::copy(&a, &b).unwrap();
        assert_eq!(resource_fork_len(&b), 4);
        assert_eq!(file_type(&b), Some(*b"APPL"));
    }
}
