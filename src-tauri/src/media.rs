//! Old media macOS can't open: HFS floppies, CDs and drives (macOS dropped
//! HFS in 10.15), and Amiga drives. When one is attached and nothing on it
//! mounts, Floppy offers to copy it into its library, and the copy opens
//! in the matching emulator.
//!
//! - **Detecting:** a thread polls `/dev` for new whole disks (`diskN`).
//!   A few seconds after one appears, once macOS has had its chance to
//!   mount it, `diskutil` says whether anything on it mounted and what it
//!   holds. An external disk with nothing mounted, and no modern
//!   partitions, is offered (`assess`).
//! - **Copying:** raw disks belong to root, so Floppy reads one through
//!   `/usr/libexec/authopen`, which shows macOS's own password prompt and
//!   streams the disk. Only reading is ever asked for: the original is
//!   never written to. The first 64 KB say what the disk is (`identify`).
//!   Anything Floppy doesn't recognize is abandoned there.
//! - **Using:** the image is imported like any dropped disk image (`.dsk`
//!   for a Mac, `.hdf`/`.adf` for an Amiga), so Launch mounts it in Basilisk
//!   II or FS-UAE. Neither can add a disk while running, or read `/dev`
//!   without root, so a copy is the only way in.
//!
//! Only the macOS side is built (docs/platform-parity.md); elsewhere the
//! watcher is a stub and the disk-judging code goes unused.
#![cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::library::GuestOs;
use crate::{amiga, mac};

/// An attached disk macOS couldn't mount, offered for copying.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OldMedia {
    /// `disk4`: the whole disk.
    pub device: String,
    /// What macOS calls the media ("USB Floppy", "Apple CD-ROM").
    pub name: String,
    pub size: u64,
    /// The guest it probably belongs to, from its partition map. `None`
    /// until its contents are read.
    pub hint: Option<GuestOs>,
    /// Attached from a disk image file rather than a device.
    pub disk_image: bool,
}

/// Partition contents macOS mounts, or that belong to a modern system. A
/// disk with any of these isn't old media, even if nothing is mounted (the
/// user may have ejected its volumes).
const MODERN: &[&str] = &[
    "Apple_APFS",
    "Apple_APFS_ISC",
    "Apple_APFS_Recovery",
    "EFI",
    "Microsoft Basic Data",
    "Windows_FAT_32",
    "Windows_FAT_16",
    "Windows_NTFS",
    "DOS_FAT_12",
    "DOS_FAT_16",
    "DOS_FAT_32",
    "Linux",
    "Linux Filesystem",
    "Linux_Swap",
    "Apple_Boot",
    "Apple_CoreStorage",
];

/// Whole-disk contents old media can have: an Apple partition map (Mac
/// hard disks and CDs), a bare HFS volume (floppies), or nothing macOS
/// recognizes (an Amiga drive, an HFS floppy macOS no longer reads).
const OLD_WHOLE_DISK: &[&str] = &["Apple_partition_scheme", "Apple_HFS", ""];

fn string<'a>(d: &'a plist::Dictionary, key: &str) -> &'a str {
    d.get(key).and_then(plist::Value::as_string).unwrap_or("")
}

fn mounted(d: &plist::Dictionary) -> bool {
    !string(d, "MountPoint").is_empty()
}

/// Whether `id` (from `diskutil list -plist`) is old media: nothing on
/// it mounted, no APFS container backed by it, no modern partitions, and
/// a whole-disk content old media has. `info` is `diskutil info -plist id`.
pub fn assess(id: &str, list: &plist::Value, info: &plist::Dictionary) -> Option<OldMedia> {
    let all = list.as_dictionary()?.get("AllDisksAndPartitions")?.as_array()?;
    let entries: Vec<&plist::Dictionary> = all.iter().filter_map(plist::Value::as_dictionary).collect();
    let disk = entries.iter().find(|d| string(d, "DeviceIdentifier") == id)?;
    let parts: Vec<&plist::Dictionary> =
        disk.get("Partitions").and_then(plist::Value::as_array).into_iter().flatten().filter_map(plist::Value::as_dictionary).collect();

    if info.get("Internal").and_then(plist::Value::as_boolean).unwrap_or(true) {
        return None;
    }
    let size = info.get("Size").or_else(|| info.get("TotalSize")).and_then(plist::Value::as_unsigned_integer).unwrap_or(0);
    if size == 0 {
        return None;
    }
    if mounted(disk) || parts.iter().any(|p| mounted(p) || MODERN.contains(&string(p, "Content"))) {
        return None;
    }
    // APFS volumes mount from a synthesized disk whose physical store is a
    // partition of this one.
    let part_ids: HashSet<&str> = parts.iter().map(|p| string(p, "DeviceIdentifier")).collect();
    let backs_apfs = entries.iter().any(|e| {
        e.get("APFSPhysicalStores")
            .and_then(plist::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(plist::Value::as_dictionary)
            .any(|s| part_ids.contains(string(s, "DeviceIdentifier")) || string(s, "DeviceIdentifier") == id)
    });
    let content = string(disk, "Content");
    if backs_apfs || !OLD_WHOLE_DISK.contains(&content) {
        return None;
    }
    let hint = (content != "" || parts.iter().any(|p| string(p, "Content").starts_with("Apple_"))).then_some(GuestOs::MacClassic);
    let name = [string(info, "MediaName"), string(info, "IORegistryEntryName")]
        .into_iter()
        .find(|n| !n.is_empty())
        .unwrap_or("Untitled disk")
        .to_string();
    Some(OldMedia { device: id.to_string(), name, size, hint, disk_image: string(info, "BusProtocol") == "Disk Image" })
}

/// Bytes read before deciding what a disk is.
pub const HEAD_BYTES: usize = 64 * 1024;

/// What a disk holds, from its first bytes, and the extension its image
/// gets: an Apple partition map, or an HFS, HFS+ or MFS volume, is a Mac
/// disk; an Amiga Rigid Disk Block or AmigaDOS volume is an Amiga one.
pub fn identify(head: &[u8], size: u64) -> Option<(GuestOs, &'static str)> {
    let at = |o: usize, sig: &[u8]| head.get(o..o + sig.len()) == Some(sig);
    if at(0, b"ER") || at(1024, b"BD") || at(1024, b"H+") || at(1024, b"HX") || at(1024, &[0xD2, 0xD7]) {
        return Some((GuestOs::MacClassic, "dsk"));
    }
    // The RDB can sit in any of the first 16 blocks.
    if (0..16).any(|b| at(b * 512, b"RDSK")) {
        return Some((GuestOs::Amiga, "hdf"));
    }
    if at(0, b"DOS") && head.get(3).is_some_and(|&f| f <= 7) {
        return Some((GuestOs::Amiga, if size == 901_120 { "adf" } else { "hdf" }));
    }
    None
}

/// `disk4`, and nothing else: the only device names Floppy reads.
pub fn is_whole_disk(id: &str) -> bool {
    id.strip_prefix("disk").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The offered media, shared between the watcher thread and commands.
#[derive(Default)]
pub struct MediaWatch {
    pub offered: Mutex<Vec<OldMedia>>,
    /// Disks the user ignored or already copied, until they're detached.
    pub dismissed: Mutex<HashSet<String>>,
}

impl MediaWatch {
    pub fn list(&self) -> Vec<OldMedia> {
        self.offered.lock().map(|l| l.clone()).unwrap_or_default()
    }

    pub fn find(&self, device: &str) -> Option<OldMedia> {
        self.list().into_iter().find(|m| m.device == device)
    }

    /// Stops offering `device` until it's detached.
    pub fn dismiss(&self, device: &str) {
        if let Ok(mut d) = self.dismissed.lock() {
            d.insert(device.to_string());
        }
        if let Ok(mut l) = self.offered.lock() {
            l.retain(|m| m.device != device);
        }
    }
}

/// Watches for old media until the app quits, calling `changed` with the
/// offered list whenever it changes.
#[cfg(target_os = "macos")]
pub fn watch(state: std::sync::Arc<MediaWatch>, changed: impl Fn(Vec<OldMedia>) + Send + 'static) {
    use std::time::{Duration, Instant};
    std::thread::spawn(move || {
        // Give macOS this long to mount a new disk before judging it.
        const SETTLE: Duration = Duration::from_secs(5);
        let mut seen: HashMap<String, Instant> = HashMap::new();
        let mut checked: HashSet<String> = HashSet::new();
        let start = Instant::now();
        loop {
            let now: HashSet<String> = std::fs::read_dir("/dev")
                .map(|d| d.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| is_whole_disk(n)).collect())
                .unwrap_or_default();
            let mut dirty = false;
            // Disks attached before Floppy started are judged right away.
            for id in &now {
                seen.entry(id.clone()).or_insert(if start.elapsed() < Duration::from_secs(1) { start - SETTLE } else { Instant::now() });
            }
            seen.retain(|id, _| now.contains(id));
            checked.retain(|id| now.contains(id));
            if let Ok(mut d) = state.dismissed.lock() {
                d.retain(|id| now.contains(id));
            }
            if let Ok(mut l) = state.offered.lock() {
                let before = l.len();
                l.retain(|m| now.contains(&m.device));
                dirty |= l.len() != before;
            }
            let due: Vec<String> = seen.iter().filter(|(id, t)| t.elapsed() >= SETTLE && !checked.contains(*id)).map(|(id, _)| id.clone()).collect();
            if !due.is_empty() {
                let list = diskutil_plist(&["list", "-plist"]);
                for id in due {
                    checked.insert(id.clone());
                    let (Some(list), Some(info)) = (list.as_ref(), diskutil_plist(&["info", "-plist", &id])) else { continue };
                    let Some(info) = info.as_dictionary() else { continue };
                    let dismissed = state.dismissed.lock().is_ok_and(|d| d.contains(&id));
                    if let Some(media) = assess(&id, list, info).filter(|_| !dismissed) {
                        if let Ok(mut l) = state.offered.lock() {
                            l.push(media);
                            dirty = true;
                        }
                    }
                }
            }
            if dirty {
                changed(state.list());
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

#[cfg(target_os = "macos")]
fn diskutil_plist(args: &[&str]) -> Option<plist::Value> {
    let out = std::process::Command::new("/usr/sbin/diskutil").args(args).output().ok()?;
    out.status.success().then(|| plist::Value::from_reader_xml(&out.stdout[..]).ok()).flatten()
}

/// Copies `media` into a disk image in `dir`, reading the raw disk through
/// `authopen` (macOS asks for the user's password). `progress` gets the
/// bytes copied so far. Returns the image and the guest it's for. The
/// image is named after its volume where it has one.
#[cfg(target_os = "macos")]
pub fn copy_to_image(media: &OldMedia, dir: &Path, progress: impl Fn(u64)) -> Result<(PathBuf, GuestOs), String> {
    use std::process::{Command, Stdio};
    if !is_whole_disk(&media.device) {
        return Err("That isn't a disk Floppy can read.".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut child = Command::new("/usr/libexec/authopen")
        .arg(format!("/dev/r{}", media.device))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't ask macOS for access to the disk: {e}"))?;
    let mut out = child.stdout.take().ok_or("No output from authopen.")?;

    let mut head = vec![0u8; HEAD_BYTES];
    let mut got = 0;
    while got < head.len() {
        match out.read(&mut head[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) => return Err(format!("Couldn't read {}: {e}", media.name)),
        }
    }
    head.truncate(got);
    if got == 0 {
        let _ = child.kill();
        let mut err = String::new();
        let _ = child.stderr.take().map(|mut e| e.read_to_string(&mut err));
        let _ = child.wait();
        return Err(format!(
            "macOS didn't allow reading {}. {}",
            media.name,
            if err.trim().is_empty() { "The password prompt may have been cancelled." } else { err.trim() }
        ));
    }
    let Some((os, ext)) = identify(&head, media.size) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("Floppy doesn't recognize what's on {}. It isn't a classic Mac (HFS) or Amiga disk.", media.name));
    };

    let part = dir.join(format!("copy.{ext}"));
    let copied = (|| -> std::io::Result<u64> {
        let mut f = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(&part)?);
        f.write_all(&head)?;
        let mut total = head.len() as u64;
        let mut buf = vec![0u8; 1 << 20];
        let mut last = 0;
        loop {
            let n = out.read(&mut buf)?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
            total += n as u64;
            if total - last >= 4 << 20 {
                progress(total);
                last = total;
            }
        }
        f.flush()?;
        Ok(total)
    })();
    let status = child.wait();
    let total = copied.map_err(|e| format!("Couldn't copy {}: {e}", media.name))?;
    if !status.is_ok_and(|s| s.success()) || total < media.size {
        return Err(format!("{} stopped reading part way through ({} of {} bytes).", media.name, total, media.size));
    }
    progress(total);

    let volume = match os {
        GuestOs::MacClassic => mac::volume_name(&part),
        _ => amiga::adf_volume(&part).map(|(v, _)| v),
    };
    let stem = mac::sanitize_name(volume.as_deref().unwrap_or(&media.name), 27, "Old disk");
    let image = dir.join(format!("{stem}.{ext}"));
    std::fs::rename(&part, &image).map_err(|e| e.to_string())?;
    Ok((image, os))
}

#[cfg(not(target_os = "macos"))]
pub fn copy_to_image(_media: &OldMedia, _dir: &Path, _progress: impl Fn(u64)) -> Result<(PathBuf, GuestOs), String> {
    Err("Copying old disks needs macOS for now.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plist(xml: &str) -> plist::Value {
        plist::Value::from_reader_xml(xml.as_bytes()).unwrap()
    }

    fn info(internal: bool, size: u64, media: &str, bus: &str) -> plist::Dictionary {
        let mut d = plist::Dictionary::new();
        d.insert("Internal".into(), internal.into());
        d.insert("Size".into(), size.into());
        d.insert("MediaName".into(), media.into());
        d.insert("BusProtocol".into(), bus.into());
        d
    }

    /// `diskutil list -plist`: the boot SSD (APFS, backing disk3), an HFS
    /// floppy with nothing mounted, an Apple-partitioned drive, a FAT stick
    /// that mounted, an external APFS drive, and a blank card reader.
    const LIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict><key>AllDisksAndPartitions</key><array>
 <dict><key>Content</key><string>GUID_partition_scheme</string><key>DeviceIdentifier</key><string>disk0</string>
  <key>Partitions</key><array>
   <dict><key>Content</key><string>Apple_APFS_ISC</string><key>DeviceIdentifier</key><string>disk0s1</string></dict>
   <dict><key>Content</key><string>Apple_APFS</string><key>DeviceIdentifier</key><string>disk0s2</string></dict>
  </array></dict>
 <dict><key>APFSPhysicalStores</key><array><dict><key>DeviceIdentifier</key><string>disk0s2</string></dict></array>
  <key>DeviceIdentifier</key><string>disk3</string><key>Content</key><string>Apple_APFS_Container</string></dict>
 <dict><key>Content</key><string></string><key>DeviceIdentifier</key><string>disk4</string></dict>
 <dict><key>Content</key><string>Apple_partition_scheme</string><key>DeviceIdentifier</key><string>disk5</string>
  <key>Partitions</key><array>
   <dict><key>Content</key><string>Apple_Driver43</string><key>DeviceIdentifier</key><string>disk5s1</string></dict>
   <dict><key>Content</key><string>Apple_HFS</string><key>DeviceIdentifier</key><string>disk5s2</string></dict>
  </array></dict>
 <dict><key>Content</key><string>FDisk_partition_scheme</string><key>DeviceIdentifier</key><string>disk6</string>
  <key>Partitions</key><array>
   <dict><key>Content</key><string>DOS_FAT_32</string><key>DeviceIdentifier</key><string>disk6s1</string>
    <key>MountPoint</key><string>/Volumes/STICK</string></dict>
  </array></dict>
 <dict><key>Content</key><string>GUID_partition_scheme</string><key>DeviceIdentifier</key><string>disk7</string>
  <key>Partitions</key><array>
   <dict><key>Content</key><string>Apple_APFS</string><key>DeviceIdentifier</key><string>disk7s2</string></dict>
  </array></dict>
 <dict><key>Content</key><string></string><key>DeviceIdentifier</key><string>disk8</string></dict>
</array></dict></plist>"#;

    #[test]
    fn offers_only_unmounted_old_media() {
        let list = plist(LIST);
        let ext = |size| info(false, size, "USB Floppy", "USB");
        // A bare floppy macOS couldn't mount: its contents decide the guest later.
        assert_eq!(
            assess("disk4", &list, &ext(1_474_560)),
            Some(OldMedia { device: "disk4".into(), name: "USB Floppy".into(), size: 1_474_560, hint: None, disk_image: false })
        );
        // An Apple partition map is a Mac disk.
        assert_eq!(assess("disk5", &list, &info(false, 40 << 20, "SCSI Drive", "Disk Image")).unwrap().hint, Some(GuestOs::MacClassic));
        assert!(assess("disk5", &list, &info(false, 40 << 20, "SCSI Drive", "Disk Image")).unwrap().disk_image);
        // The internal SSD, a stick that mounted, an APFS drive and an empty reader aren't.
        assert_eq!(assess("disk0", &list, &info(true, 500 << 30, "SSD", "Apple Fabric")), None);
        assert_eq!(assess("disk6", &list, &ext(8 << 30)), None);
        assert_eq!(assess("disk7", &list, &ext(1 << 40)), None);
        assert_eq!(assess("disk8", &list, &ext(0)), None);
        assert_eq!(assess("disk9", &list, &ext(1)), None);
    }

    #[test]
    fn identifies_mac_and_amiga_disks() {
        let mut head = vec![0u8; HEAD_BYTES];
        assert_eq!(identify(&head, 1 << 20), None);
        head[1024..1026].copy_from_slice(b"BD");
        assert_eq!(identify(&head, 1_474_560), Some((GuestOs::MacClassic, "dsk")));

        let mut apm = vec![0u8; HEAD_BYTES];
        apm[0..2].copy_from_slice(b"ER");
        assert_eq!(identify(&apm, 40 << 20), Some((GuestOs::MacClassic, "dsk")));

        let mut rdb = vec![0u8; HEAD_BYTES];
        rdb[3 * 512..3 * 512 + 4].copy_from_slice(b"RDSK");
        assert_eq!(identify(&rdb, 1 << 30), Some((GuestOs::Amiga, "hdf")));

        let mut dos = vec![0u8; 4096];
        dos[0..4].copy_from_slice(b"DOS\x01");
        assert_eq!(identify(&dos, 901_120), Some((GuestOs::Amiga, "adf")));
        assert_eq!(identify(&dos, 100 << 20), Some((GuestOs::Amiga, "hdf")));
        dos[3] = 0x20;
        assert_eq!(identify(&dos, 901_120), None);
    }

    #[test]
    fn reads_only_whole_disk_names() {
        assert!(is_whole_disk("disk4") && is_whole_disk("disk12"));
        for bad in ["disk", "disk4s1", "rdisk4", "disk4/../../etc/passwd", "../disk4", "disk 4"] {
            assert!(!is_whole_disk(bad), "{bad}");
        }
    }

    #[test]
    fn dismissed_media_stays_hidden() {
        let w = MediaWatch::default();
        w.offered.lock().unwrap().push(OldMedia { device: "disk4".into(), name: "x".into(), size: 1, hint: None, disk_image: false });
        assert!(w.find("disk4").is_some());
        w.dismiss("disk4");
        assert!(w.list().is_empty() && w.dismissed.lock().unwrap().contains("disk4"));
    }
}
