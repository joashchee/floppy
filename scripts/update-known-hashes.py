#!/usr/bin/env python3
"""Regenerates src-tauri/src/known_files.rs: the size and SHA-1 of known-good
copies of every system file Floppy's setup asks for. The missing-files list
writes one nameless line per copy, so a tool that hashes the user's files can
find a copy stored under any name (see cd.rs).

    scripts/update-known-hashes.py [--tosec PATH]

PATH is a TOSEC DAT pack zip, or a folder of TOSEC .dat files. Without it the
pinned pack below is downloaded (about 100 MB).

Only published hash lists are read, never a guest file itself (CLAUDE.md
rule 3). Hashes are facts about files, not the files. This is a maintenance
tool run by hand, not app code, so it may use the network (rule 4).

Sources:
  Mac ROMs          MAME's Apple drivers (src/mame/apple/mac*.cpp), limited
                    to the 32-bit clean dumps Basilisk II runs (MAC_ROM_IDS).
  Mac startup disks Internet Archive file metadata for the items in
                    MAC_BOOT_DISKS: installed System disks and bootable
                    utility floppies. (Install CDs are left out: they're
                    read-only, and the startup slot needs a writable disk.)
  Kickstarts        FS-UAE's ROM table (rommgr.cpp) plus TOSEC's
                    "Commodore Amiga - Firmware", including Amiga Forever's
                    encrypted files (they need rom.key). Bad dumps [b],
                    overdumps [o] and betas are left out.
  Workbench disks   TOSEC's "Commodore Amiga - Operating Systems - Workbench"
                    boot disks, plus the FS-UAE launcher's list
                    (fsgs/amiga/workbenchdata.py). Bad dumps are left out.

Update docs/legal-setupfiles.md's check log after running it.
"""

import argparse
import io
import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "src-tauri", "src", "known_files.rs")

TOSEC_URL = (
    "https://www.tosecdev.org/downloads/category/59-2025-03-13"
    "?download=117:tosec-dat-pack-complete-4743-tosec-v2025-03-13"
)
MAME_DIR = "https://api.github.com/repos/mamedev/mame/contents/src/mame/apple"
MAME_RAW = "https://raw.githubusercontent.com/mamedev/mame/master/src/mame/apple/"
FSUAE_ROMMGR = "https://raw.githubusercontent.com/FrodeSolheim/fs-uae/main/rommgr.cpp"
FSUAE_WB = "https://raw.githubusercontent.com/FrodeSolheim/fs-uae-launcher/main/fsgs/amiga/workbenchdata.py"

# Mac ROMs Basilisk II can use (32-bit clean, version word 0x067C), by the
# checksum in their first long word, which MAME names each dump by.
MAC_ROM_IDS = {
    "F1ACAD13": "Quadra 610/650/800, Centris 610/650",
    "F1A6F343": "Quadra 800, Centris 610/650, earlier",
    "420DBFF3": "Quadra 700/900, PowerBook 140/170",
    "3DC27823": "Quadra 950",
    "FF7439EE": "Quadra 605, LC 475",
    "06684214": "Quadra 630, LC 580",
    "064DC91D": "LC 580, later",
    "ECBBC41C": "LC III",
    "EDE66CBD": "LC 520/550",
    "EAF1678D": "Macintosh TV",
    "ECD99DC0": "Color Classic",
    "4957EB49": "IIvi/IIvx",
    "36B7FB6C": "IIsi",
    "4147DD77": "IIfx",
    "350EACF0": "LC",
    "E33B2724": "PowerBook 160/165c/180/180c",
    "FBA22562": "PowerBook 150",
    "ECFA989B": "PowerBook Duo 210/230",
    "0024D346": "PowerBook Duo 270c",
    "015621D7": "PowerBook Duo 280/280c",
}

# (Internet Archive item, file, label). Files are checked to exist and their
# SHA-1 comes from the item's metadata.
MAC_BOOT_DISKS = [
    ("AppleMacintoshSystem753", "System7_5_3.img", "System 7.5.3, installed (25 MB)"),
    ("AppleMacintoshSystem701", "System7_0_1.img", "System 7.0.1, installed (10 MB)"),
    ("mac-os-7.6", "MacOS-7.6-Utilities1.img", "Mac OS 7.6 Disk Tools (Utilities 1)"),
    ("mac-os-7.6", "MacOS-7.6-Utilities2.img", "Mac OS 7.6 Utilities 2"),
]


def fetch(url):
    """Downloads with curl, which ships with macOS (the fetch scripts use it
    too), retrying transient failures."""
    out = subprocess.run(
        ["curl", "-sSfL", "--retry", "4", "--retry-all-errors", "-A", "floppy-update-known-hashes", url],
        capture_output=True,
    )
    if out.returncode != 0:
        sys.exit(f"Couldn't download {url}: {out.stderr.decode(errors='replace').strip()}")
    return out.stdout


def mac_roms():
    names = [f["name"] for f in json.loads(fetch(MAME_DIR)) if re.match(r"mac.*\.cpp$", f["name"])]
    found = {}
    load = re.compile(
        r'ROMX?_LOAD(?:_BIOS)?\s*\(\s*(?:\d+\s*,\s*)?"([^"]+)"\s*,\s*0x0+\s*,\s*0x0*(80000|100000)\s*,'
        r".*?SHA1\(\s*([0-9a-f]{40})\s*\)",
        re.I,
    )
    for name in names:
        for line in fetch(MAME_RAW + name).decode("utf-8", "replace").splitlines():
            m = load.search(line)
            if not m:
                continue
            rom_id = m.group(1).split(".")[0].upper()
            if rom_id in MAC_ROM_IDS:
                found[m.group(3).lower()] = (int(m.group(2), 16), f"{MAC_ROM_IDS[rom_id]} ({rom_id})")
    missing = set(MAC_ROM_IDS) - {label[-9:-1] for _, label in found.values()}
    if missing:
        print(f"warning: MAME no longer lists {sorted(missing)}", file=sys.stderr)
    return found


def mac_boot_disks():
    found = {}
    for item, file, label in MAC_BOOT_DISKS:
        files = json.loads(fetch(f"https://archive.org/metadata/{item}")).get("files", [])
        f = next((f for f in files if f.get("name") == file and f.get("sha1")), None)
        if not f:
            sys.exit(f"{item}/{file} is gone from the Internet Archive; update MAC_BOOT_DISKS.")
        found[f["sha1"].lower()] = (int(f["size"]), label)
    return found


def tosec_dats(path):
    """{dat name: xml bytes} for the Amiga Firmware and Workbench DATs."""
    wanted = ("Commodore Amiga - Firmware", "Commodore Amiga - Operating Systems - Workbench")
    out = {}
    if path and os.path.isdir(path):
        for name in os.listdir(path):
            if name.startswith(wanted) and name.endswith(".dat"):
                out[name] = open(os.path.join(path, name), "rb").read()
        return out
    data = open(path, "rb").read() if path else fetch(TOSEC_URL)
    with zipfile.ZipFile(io.BytesIO(data)) as z:
        for name in z.namelist():
            base = name.split("/")[-1]
            if name.startswith("TOSEC/") and base.startswith(wanted):
                out[base] = z.read(name)
    return out


def tosec_roms(dats, prefix):
    for name, xml in dats.items():
        if name.startswith(prefix):
            root = ET.fromstring(xml)
            for rom in root.iter("rom"):
                yield rom.get("name"), int(rom.get("size")), rom.get("sha1").lower()


def kickstarts(dats):
    found = {}
    s = fetch(FSUAE_ROMMGR).decode("utf-8", "replace")
    body = s[s.index("static struct romdata roms[]") :][:200000]
    entry = re.compile(r'\{\s*_T\("([^"]*)"\)\s*,(.*?)\}\s*,\s*(?=\{|ALTROM|\n\s*\{|\s*$)', re.S)
    for name, rest in entry.findall(body):
        if not name.startswith("KS ROM"):
            continue
        toks = [t.strip() for t in rest.replace("\n", " ").split(",")]
        size = next((int(t) for t in toks if t in ("262144", "524288")), None)
        hexes = [t for t in toks if re.fullmatch(r"0x[0-9a-fA-F]{1,8}", t)]
        if size is None or len(hexes) < 6:
            continue
        sha1 = "".join(h[2:].lower().zfill(8) for h in hexes[-5:])
        found[sha1] = (size, "Kickstart " + name[len("KS ROM ") :])
    for name, size, sha1 in tosec_roms(dats, "Commodore Amiga - Firmware"):
        if not name.startswith("Kickstart") or sha1 in found:
            continue
        if re.search(r"\[b|\[o\]|beta|alpha", name):
            continue
        # Plain dumps, and Amiga Forever's encrypted ones (11-byte header).
        if size in (262144, 524288, 262155, 524299):
            found[sha1] = (size, name[: -len(".rom")] if name.endswith(".rom") else name)
    return found


def workbench_disks(dats):
    found = {}
    for name, size, sha1 in tosec_roms(dats, "Commodore Amiga - Operating Systems - Workbench"):
        if size != 901120 or not name.lower().endswith(".adf") or re.search(r"\[b", name):
            continue
        # In a multi-disk set, the boot disk is the one labeled (Workbench):
        # disk 1 is the Install disk from 2.1 on. A set without disk labels
        # boots from disk 1.
        disk = re.search(r"\(Disk (\d+) of \d+\)(\([^)]*\))?", name)
        if disk and disk.group(2) != "(Workbench)" and (disk.group(2) or disk.group(1) != "1"):
            continue
        if "not bootable" in name:
            continue
        found[sha1] = (size, name[: -len(".adf")])
    # The FS-UAE launcher's lists: TOSEC names in comments above each hash.
    s = fetch(FSUAE_WB).decode("utf-8", "replace")
    for block in re.findall(r"wb_\d+_floppies = \[(.*?)\]", s, re.S):
        comment = []
        for line in block.splitlines():
            line = line.strip()
            if line.startswith("#"):
                comment.append(line.lstrip("# ").strip())
            elif m := re.match(r'"([0-9a-f]{40})"', line):
                label = " ".join(comment).removesuffix(".adf") or "Workbench"
                found.setdefault(m.group(1), (901120, label))
                comment = []
    return found


def rust_table(name, doc, rows):
    lines = [doc, f"pub const {name}: &[KnownFile] = &["]
    for sha1, (size, label) in sorted(rows.items(), key=lambda r: (r[1][1].lower(), r[0])):
        label = label.replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'    KnownFile {{ sha1: "{sha1}", size: {size}, label: "{label}" }},')
    lines.append("];")
    return "\n".join(lines)


HEADER = '''//! SHA-1s of known-good copies of every system file Floppy's setup asks
//! for, so the missing-files list can say what Floppy is looking for by
//! content as well as by name. A user's copy is often stored under a name
//! no list could guess; a tool that has hashed the user's files can still
//! find it by one of these (cd.rs).
//!
//! **Generated by `scripts/update-known-hashes.py`. Don't edit by hand:**
//! change the script (its docstring lists the sources) and run it again.
//!
//! These are facts about files, not the files themselves (rule 3). Floppy
//! never relies on them: it still recognizes every file by its contents,
//! so a copy that isn't listed here still works when it's imported
//! directly.

/// A file's SHA-1, its size in bytes, and what it is.
pub struct KnownFile {
    pub sha1: &'static str,
    pub size: u64,
    pub label: &'static str,
}
'''

TESTS = '''
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_well_formed() {
        let mut seen = std::collections::HashSet::new();
        let tables: [(&[KnownFile], fn(u64) -> bool); 4] = [
            (MAC_ROMS, |s| matches!(s, 524_288 | 1_048_576)),
            (MAC_BOOT_DISKS, |s| s >= 400 * 1024),
            // Plain dumps, and Amiga Forever's encrypted ones (11-byte header).
            (KICKSTARTS, |s| matches!(s, 262_144 | 524_288 | 262_155 | 524_299)),
            (WORKBENCH_DISKS, |s| s == 901_120),
        ];
        for (table, size_ok) in tables {
            assert!(!table.is_empty());
            for f in table {
                assert!(f.sha1.len() == 40 && f.sha1.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{}", f.sha1);
                assert!(size_ok(f.size), "{} {}", f.label, f.size);
                assert!(!f.label.is_empty() && !f.label.contains(['\\n', '\\t']));
                assert!(seen.insert(f.sha1), "duplicate {}", f.sha1);
            }
        }
    }
}
'''


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--tosec", help="TOSEC DAT pack zip, or a folder of TOSEC .dat files")
    args = ap.parse_args()

    dats = tosec_dats(args.tosec)
    if len(dats) < 2:
        sys.exit("The TOSEC pack is missing the Amiga Firmware or Workbench DAT.")
    tables = [
        ("MAC_ROMS", "/// Mac ROMs Basilisk II can use, from MAME. The ROM's own checksum is in brackets.", mac_roms()),
        ("MAC_BOOT_DISKS", "/// Mac startup disks: installed System disks and bootable utility floppies,\n/// from Internet Archive file metadata.", mac_boot_disks()),
        ("KICKSTARTS", "/// Kickstart ROMs, from FS-UAE's ROM table and TOSEC. Encrypted Amiga\n/// Forever copies also need their rom.key.", kickstarts(dats)),
        ("WORKBENCH_DISKS", "/// Workbench boot disks, from TOSEC and the FS-UAE launcher.", workbench_disks(dats)),
    ]
    rs = HEADER + "\n" + "\n\n".join(rust_table(n, d, rows) for n, d, rows in tables) + "\n" + TESTS
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(rs)
    for name, _, rows in tables:
        print(f"{name}: {len(rows)}")
    print(f"Wrote {os.path.relpath(OUT, ROOT)}")


if __name__ == "__main__":
    main()
