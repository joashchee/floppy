#!/usr/bin/env python3
"""Builds the knowledge pack for the current Floppy AI version, so Floppy
users can update what their Floppy knows without a new release.

    scripts/make-ai-pack.py [--sign KEYFILE] [--out DIR] [--site SITE]
                            [--allow-unsigned] [--material PATH=LICENCE=SOURCE ...]

The version is the top row of docs/floppy-ai.md's versions table. The
pack is a findings zip (findings.rs): floppy-findings.json with a
"pack" entry, a README.txt, and under materials/ the living documents
Floppy reads when it's built (app-handlers.md, legal-setupfiles.md,
file-handling.md, floppy-ai.md). Floppy learns from them with the same
code that reads them at build time (learned.rs).

--sign KEY    sign the pack with the maintainers' key (a file made by
              `cargo run --example ai-pack-key -- new KEY`, kept outside
              every repo), whose public key is in docs/floppy-ai.md's "Pack
              keys". Only a signed pack raises Floppy's AI version or
              changes where it points for setup files; an unsigned one is
              learned from as ordinary findings.
--out DIR     where to write "Floppy AI <N>.zip" (default dist/floppy-ai).
--site SITE   also publish it into an ansiapps-site checkout:
              public/downloads/floppy-ai/floppy-ai-<N>.zip, and
              src/data/floppy-ai.json (version, date, what, link, size,
              SHA-256), which Floppy's card on the site shows. A version
              already published is never replaced with different bytes:
              bump the version in docs/floppy-ai.md instead. It must be
              signed, unless --allow-unsigned (before a key exists).
--material    an extra file for materials/: a plain .md or .txt (at most
              1 MiB), with the licence it's shared under and where it
              came from. Only licences that allow redistribution are
              taken. Never guest software, ROMs or disk images (Floppy's
              rule 3); Floppy only keeps .md and .txt anyway.

The zip is reproducible: the same documents make the same bytes. Its ID
carries a hash of the materials, so a changed pack is learned again.

Standard library only. It never contacts anything.
"""

import hashlib
import io
import json
import os
import re
import shutil
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = "https://github.com/joashchee/floppy/blob/main/"
DOCS = ["app-handlers.md", "legal-setupfiles.md", "file-handling.md", "floppy-ai.md"]
# Licences whose text may be put on ansiapps.com for anyone to download.
LICENCES = {
    "GPL-2.0-or-later", "GPL-2.0-only", "GPL-3.0-or-later", "GPL-3.0-only", "LGPL-2.1-or-later",
    "MIT", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Apache-2.0", "MPL-2.0", "Zlib",
    "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "Public-Domain",
}
MAX_MATERIAL = 1 << 20
# Zip entries get a fixed time, so the same documents make the same zip.
FIXED_TIME = (2026, 1, 1, 0, 0, 0)

README = """Floppy AI {version} ({date})

A knowledge pack for Floppy: what Floppy knows about old files, apps and
setup, as of this version. {what}.

To update your Floppy: drop this zip on Floppy's window, or choose Learn
from Findings... in its gear menu. Floppy reads the documents under
materials/ and learns what's new: app versions it recognizes by
fingerprint, file types apps open, setup files known to be good copies,
where to get setup files, and where dropped files go. Nothing it already
knows is overruled, and Forget What Was Learned... undoes it. The header
then says "AI {version}".

It holds no software, ROMs or disk images: only the plain-text documents
listed in floppy-findings.json, each with its licence. They're Floppy's
own living documents, from {repo}docs/.
"""


def die(msg):
    sys.exit(f"make-ai-pack: {msg}")


def top_version(doc):
    m = re.search(r"<!-- ai-versions:start -->\n(.*?)<!-- ai-versions:end -->", doc, re.S)
    rows = [l for l in (m.group(1) if m else "").splitlines() if l.startswith("|")][2:]
    if not rows:
        die("docs/floppy-ai.md has no AI version.")
    cells = [c.strip() for c in rows[0].strip().strip("|").split("|")]
    return int(cells[0]), cells[1], cells[2]


def floppy_version():
    text = open(os.path.join(ROOT, "src-tauri", "Cargo.toml"), encoding="utf-8").read()
    return re.search(r'^version = "([^"]+)"', text, re.M).group(1)


def material(path, licence, source):
    name = os.path.basename(path)
    if not re.fullmatch(r"[A-Za-z0-9._ -]{1,64}", name) or name.startswith(".") or not name.lower().endswith((".md", ".txt")):
        die(f"{name}: materials are plain .md or .txt files with short plain names.")
    if licence not in LICENCES:
        die(f"{name}: {licence!r} isn't a licence that allows sharing it on ansiapps.com ({', '.join(sorted(LICENCES))}).")
    data = open(path, "rb").read()
    if len(data) > MAX_MATERIAL:
        die(f"{name}: over 1 MiB.")
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        die(f"{name}: not UTF-8 text.")
    if b"\0" in data:
        die(f"{name}: not plain text.")
    return {"name": name, "license": licence, "source": source, "note": "",
            "sha256": hashlib.sha256(data).hexdigest()}, data


def sign(key, data):
    """The Ed25519 signature of data, by the cargo example that holds the
    signing code (the key's secret never leaves the maintainer's machine)."""
    import subprocess
    import tempfile
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        f.write(data)
        tmp = f.name
    try:
        r = subprocess.run(["cargo", "run", "--quiet", "--manifest-path", os.path.join(ROOT, "src-tauri", "Cargo.toml"),
                            "--example", "ai-pack-key", "--", "sign", key, tmp], capture_output=True, text=True)
    finally:
        os.unlink(tmp)
    sig = r.stdout.strip()
    if r.returncode != 0 or not re.fullmatch(r"[0-9a-f]{128}", sig):
        die(f"signing failed: {r.stderr.strip() or sig}")
    return sig


def build():
    args = sys.argv[1:]
    out_dir, site, extra, key, allow_unsigned = os.path.join(ROOT, "dist", "floppy-ai"), None, [], None, False
    while args:
        a = args.pop(0)
        if a == "--out" and args:
            out_dir = args.pop(0)
        elif a == "--site" and args:
            site = args.pop(0)
        elif a == "--sign" and args:
            key = args.pop(0)
        elif a == "--allow-unsigned":
            allow_unsigned = True
        elif a == "--material" and args:
            parts = args.pop(0).split("=", 2)
            if len(parts) != 3:
                die("--material takes PATH=LICENCE=SOURCE.")
            extra.append(parts)
        else:
            sys.exit(__doc__.split("\n\n")[1])

    ai_doc = open(os.path.join(ROOT, "docs", "floppy-ai.md"), encoding="utf-8").read()
    version, date, what = top_version(ai_doc)
    materials = [material(os.path.join(ROOT, "docs", d), "GPL-2.0-or-later", REPO + "docs/" + d) for d in DOCS]
    materials += [material(p, lic, src) for p, lic, src in extra]
    names = [m["name"] for m, _ in materials]
    if len(set(names)) != len(names):
        die("two materials share a name.")

    digest = hashlib.sha256()
    for m, data in materials:
        digest.update(m["name"].encode() + b"\0" + data)
    exported = int(__import__("datetime").datetime.strptime(date, "%Y-%m-%d").replace(
        tzinfo=__import__("datetime").timezone.utc).timestamp())
    findings = {
        "format": "floppy-findings",
        "version": 1,
        "id": f"floppy-ai-{version}-{digest.hexdigest()[:12]}",
        "floppyVersion": floppy_version(),
        "exported": exported,
        "aiVersion": version,
        "pack": {"aiVersion": version, "date": date, "what": what},
        "materials": [m for m, _ in materials],
    }

    if site and not key and not allow_unsigned:
        die("a pack for ansiapps.com must be signed: --sign KEYFILE (or --allow-unsigned before a key exists).")
    if key and not os.path.isfile(key):
        die(f"no key at {key}.")
    json_bytes = (json.dumps(findings, indent=2) + "\n").encode()
    signature = sign(key, json_bytes) if key else None

    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        def add(name, data):
            info = zipfile.ZipInfo(name, FIXED_TIME)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, data)
        add("floppy-findings.json", json_bytes)
        if signature:
            add("floppy-findings.sig", signature + "\n")
        add("README.txt", README.format(version=version, date=date, what=what.rstrip("."), repo=REPO))
        for m, data in materials:
            add("materials/" + m["name"], data)
    data = buf.getvalue()
    sha = hashlib.sha256(data).hexdigest()

    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, f"Floppy AI {version}.zip")
    open(out, "wb").write(data)
    print(f"Floppy AI {version} ({date}): {out}, {len(data)} bytes, SHA-256 {sha}")
    if not signature:
        print("Unsigned: Floppy learns from it as ordinary findings. It won't raise the AI version or change setup links.")

    if site:
        public = os.path.join(site, "public", "downloads", "floppy-ai")
        target = os.path.join(public, f"floppy-ai-{version}.zip")
        if os.path.exists(target) and open(target, "rb").read() != data:
            die(f"{target} is already published with other contents: bump the version in docs/floppy-ai.md.")
        os.makedirs(public, exist_ok=True)
        shutil.copyfile(out, target)
        info = {
            "version": version,
            "date": date,
            "what": what,
            "href": f"/downloads/floppy-ai/floppy-ai-{version}.zip",
            "bytes": len(data),
            "sha256": sha,
        }
        json_path = os.path.join(site, "src", "data", "floppy-ai.json")
        open(json_path, "w", encoding="utf-8").write(json.dumps(info, indent=2) + "\n")
        print(f"Published to {target} and {json_path}. Commit the site to put it live.")


if __name__ == "__main__":
    build()
