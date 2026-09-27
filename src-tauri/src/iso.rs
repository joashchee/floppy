//! A small ISO 9660 writer and reader, for the discs Floppy makes itself
//! (backup.rs). Level 1: one root directory, 8.3 names, files under
//! 4 GB each. Every system mounts such a disc, so a user can always get
//! at its files without Floppy, and Floppy reads its own discs without
//! mounting anything.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const SECTOR: u64 = 2048;
/// Sectors 0–15 are the system area; the volume descriptors follow.
const PVD_SECTOR: u64 = 16;
const PATH_TABLE_L: u64 = 18;
const PATH_TABLE_M: u64 = 19;
const ROOT_SECTOR: u64 = 20;
const FIRST_FILE_SECTOR: u64 = 21;

/// What goes in the volume descriptor, beyond the files.
pub struct Volume<'a> {
    /// d-characters (A–Z, 0–9, `_`), up to 32.
    pub id: &'a str,
    /// a-characters (d-characters plus space and some punctuation), up to 128.
    pub application: &'a str,
    pub publisher: &'a str,
}

fn both16(v: u16) -> [u8; 4] {
    let (l, b) = (v.to_le_bytes(), v.to_be_bytes());
    [l[0], l[1], b[0], b[1]]
}

fn both32(v: u32) -> [u8; 8] {
    let (l, b) = (v.to_le_bytes(), v.to_be_bytes());
    [l[0], l[1], l[2], l[3], b[0], b[1], b[2], b[3]]
}

/// A padded, uppercased text field.
fn text(s: &str, len: usize) -> Vec<u8> {
    let mut v: Vec<u8> = s.to_ascii_uppercase().bytes().take(len).collect();
    v.resize(len, b' ');
    v
}

/// The date fields, from Unix seconds (UTC).
fn civil(unix: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, (secs / 3600) as u32, (secs % 3600 / 60) as u32, (secs % 60) as u32)
}

fn record_date(unix: u64) -> [u8; 7] {
    let (y, m, d, h, mi, s) = civil(unix);
    [(y - 1900).clamp(0, 255) as u8, m as u8, d as u8, h as u8, mi as u8, s as u8, 0]
}

fn volume_date(unix: u64) -> Vec<u8> {
    let (y, m, d, h, mi, s) = civil(unix);
    let mut v = format!("{y:04}{m:02}{d:02}{h:02}{mi:02}{s:02}00").into_bytes();
    v.push(0);
    v
}

/// One directory record. `name` is `\0` for "." and `\x01` for "..".
fn dir_record(name: &[u8], sector: u32, len: u32, dir: bool, unix: u64) -> Vec<u8> {
    let mut r = vec![0u8; 33];
    r[2..10].copy_from_slice(&both32(sector));
    r[10..18].copy_from_slice(&both32(len));
    r[18..25].copy_from_slice(&record_date(unix));
    r[25] = if dir { 2 } else { 0 };
    r[28..32].copy_from_slice(&both16(1));
    r[32] = name.len() as u8;
    r.extend_from_slice(name);
    if r.len() % 2 == 1 {
        r.push(0);
    }
    r[0] = r.len() as u8;
    r
}

fn sectors(len: u64) -> u64 {
    len.div_ceil(SECTOR)
}

/// Whether `name` is a level-1 file name: 8.3, uppercase d-characters.
fn valid_name(name: &str) -> bool {
    let (stem, ext) = name.split_once('.').unwrap_or((name, ""));
    let d = |s: &str| s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
    (1..=8).contains(&stem.len()) && ext.len() <= 3 && d(stem) && d(ext)
}

/// Writes an ISO 9660 image to `out` holding `files` (`NAME.EXT`, source
/// file) in its root, in that order.
pub fn write(out: &Path, volume: &Volume, files: &[(&str, &Path)], unix: u64) -> io::Result<()> {
    let mut entries = Vec::new();
    let mut next = FIRST_FILE_SECTOR;
    for (name, path) in files {
        if !valid_name(name) {
            return Err(io::Error::other(format!("{name} isn't an 8.3 disc name")));
        }
        let len = std::fs::metadata(path)?.len();
        let len32 = u32::try_from(len).map_err(|_| io::Error::other(format!("{name} is over 4 GB, too big for a disc file")))?;
        entries.push((name.to_string(), *path, next, len32));
        next += sectors(len).max(1);
    }
    let total = u32::try_from(next).map_err(|_| io::Error::other("the disc would be too big"))?;

    // The root directory: ".", "..", then each file.
    let mut root = dir_record(b"\0", ROOT_SECTOR as u32, SECTOR as u32, true, unix);
    root.extend(dir_record(b"\x01", ROOT_SECTOR as u32, SECTOR as u32, true, unix));
    for (name, _, sector, len) in &entries {
        root.extend(dir_record(format!("{name};1").as_bytes(), *sector as u32, *len, false, unix));
    }
    if root.len() as u64 > SECTOR {
        return Err(io::Error::other("too many files for one directory sector"));
    }

    let mut pvd = vec![0u8; SECTOR as usize];
    pvd[0] = 1;
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 1;
    pvd[8..40].copy_from_slice(&text("", 32));
    pvd[40..72].copy_from_slice(&text(volume.id, 32));
    pvd[80..88].copy_from_slice(&both32(total));
    pvd[120..124].copy_from_slice(&both16(1));
    pvd[124..128].copy_from_slice(&both16(1));
    pvd[128..132].copy_from_slice(&both16(SECTOR as u16));
    pvd[132..140].copy_from_slice(&both32(10));
    pvd[140..144].copy_from_slice(&(PATH_TABLE_L as u32).to_le_bytes());
    pvd[148..152].copy_from_slice(&(PATH_TABLE_M as u32).to_be_bytes());
    pvd[156..190].copy_from_slice(&dir_record(b"\0", ROOT_SECTOR as u32, SECTOR as u32, true, unix));
    pvd[190..318].copy_from_slice(&text("", 128));
    pvd[318..446].copy_from_slice(&text(volume.publisher, 128));
    pvd[446..574].copy_from_slice(&text("", 128));
    pvd[574..702].copy_from_slice(&text(volume.application, 128));
    for field in [702..739, 739..776, 776..813] {
        pvd[field].copy_from_slice(&text("", 37));
    }
    let date = volume_date(unix);
    pvd[813..830].copy_from_slice(&date);
    pvd[830..847].copy_from_slice(&date);
    pvd[847..864].copy_from_slice(b"0000000000000000\0");
    pvd[864..881].copy_from_slice(&date);
    pvd[881] = 1;

    let mut term = vec![0u8; SECTOR as usize];
    term[0] = 255;
    term[1..6].copy_from_slice(b"CD001");
    term[6] = 1;

    // The path tables: just the root, pointing at its directory.
    let table = |le: bool| {
        let mut t = vec![0u8; SECTOR as usize];
        t[0] = 1;
        let loc = ROOT_SECTOR as u32;
        t[2..6].copy_from_slice(&if le { loc.to_le_bytes() } else { loc.to_be_bytes() });
        t[6..8].copy_from_slice(&if le { 1u16.to_le_bytes() } else { 1u16.to_be_bytes() });
        t
    };

    let mut f = io::BufWriter::new(File::create(out)?);
    f.write_all(&vec![0u8; (PVD_SECTOR * SECTOR) as usize])?;
    f.write_all(&pvd)?;
    f.write_all(&term)?;
    f.write_all(&table(true))?;
    f.write_all(&table(false))?;
    root.resize(SECTOR as usize, 0);
    f.write_all(&root)?;
    for (_, path, _, len) in &entries {
        let copied = io::copy(&mut File::open(path)?, &mut f)?;
        if copied != u64::from(*len) {
            return Err(io::Error::other("a file changed size while it was written to the disc"));
        }
        let pad = sectors(copied).max(1) * SECTOR - copied;
        f.write_all(&vec![0u8; pad as usize])?;
    }
    f.flush()
}

/// One file inside an ISO, readable and seekable on its own.
pub struct IsoFile {
    file: File,
    start: u64,
    len: u64,
    pos: u64,
}

impl Read for IsoFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.len - self.pos;
        if left == 0 {
            return Ok(0);
        }
        let n = (buf.len() as u64).min(left) as usize;
        self.file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = self.file.read(&mut buf[..n])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for IsoFile {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => p as i128,
            SeekFrom::End(d) => self.len as i128 + d as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
        };
        if pos < 0 {
            return Err(io::Error::other("seek before the start of the file"));
        }
        self.pos = (pos as u64).min(self.len);
        Ok(self.pos)
    }
}

/// Opens the file called `name` (`NAME.EXT`, any `;1` version) in the root
/// directory of the ISO at `path`.
pub fn open(path: &Path, name: &str) -> io::Result<IsoFile> {
    let mut file = File::open(path)?;
    let mut pvd = vec![0u8; SECTOR as usize];
    file.seek(SeekFrom::Start(PVD_SECTOR * SECTOR))?;
    file.read_exact(&mut pvd)?;
    if &pvd[0..6] != b"\x01CD001" {
        return Err(io::Error::other("not an ISO 9660 disc image"));
    }
    let root_sector = u32::from_le_bytes(pvd[158..162].try_into().unwrap()) as u64;
    let root_len = u32::from_le_bytes(pvd[166..170].try_into().unwrap()) as u64;
    let mut dir = vec![0u8; root_len.min(1 << 20) as usize];
    file.seek(SeekFrom::Start(root_sector * SECTOR))?;
    file.read_exact(&mut dir)?;
    let mut i = 0;
    while i < dir.len() {
        let len = dir[i] as usize;
        if len == 0 {
            // Records don't cross sectors: skip to the next one.
            i = (i / SECTOR as usize + 1) * SECTOR as usize;
            continue;
        }
        let rec = dir.get(i..i + len).ok_or_else(|| io::Error::other("damaged directory"))?;
        let name_len = rec[32] as usize;
        let raw = rec.get(33..33 + name_len).ok_or_else(|| io::Error::other("damaged directory"))?;
        let found = String::from_utf8_lossy(raw);
        if found.split(';').next().unwrap_or("").eq_ignore_ascii_case(name) {
            let sector = u32::from_le_bytes(rec[2..6].try_into().unwrap()) as u64;
            let size = u32::from_le_bytes(rec[10..14].try_into().unwrap()) as u64;
            return Ok(IsoFile { file, start: sector * SECTOR, len: size, pos: 0 });
        }
        i += len;
    }
    Err(io::Error::new(io::ErrorKind::NotFound, format!("{name} isn't on the disc")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn writes_a_disc_it_can_read_back() {
        let t = TempDir::new();
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::write(&a, vec![7u8; 5000]).unwrap();
        std::fs::write(&b, b"hello").unwrap();
        let iso = t.path().join("x.iso");
        let vol = Volume { id: "FLOPPY_TEST", application: "FLOPPY TEST DISC", publisher: "FLOPPY" };
        write(&iso, &vol, &[("DATA.BIN", &a), ("README.TXT", &b)], 1_790_500_000).unwrap();

        assert_eq!(std::fs::metadata(&iso).unwrap().len() % SECTOR, 0);
        assert_eq!(crate::discs::iso_application_id(&iso).as_deref(), Some("FLOPPY TEST DISC"));
        let mut s = String::new();
        open(&iso, "readme.txt").unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "hello");
        let mut data = open(&iso, "DATA.BIN").unwrap();
        data.seek(SeekFrom::Start(4990)).unwrap();
        let mut tail = Vec::new();
        data.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, vec![7u8; 10]);
        assert!(open(&iso, "NOPE.TXT").is_err());
        assert!(write(&iso, &vol, &[("toolongname.txt", &b)], 0).is_err());
    }

    #[test]
    fn dates_are_civil() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(civil(1_790_500_000).0, 2026);
        assert_eq!(&volume_date(951_782_400)[..8], b"20000229");
    }
}
