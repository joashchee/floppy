#!/usr/bin/env python3
"""Merges Floppy findings into Floppy's living documents, so the next
release recognizes what users found.

    scripts/merge-findings.py [--root DIR] FINDINGS [FINDINGS ...]

FINDINGS is a zip from Floppy's gear menu, Export Findings… (it holds
floppy-findings.json, findings.rs), that JSON on its own, or an older
"Export Test Report…" JSON (handler tests only). --root merges into
another copy of the repo (for trying it out): DIR/docs/*.md and
DIR/src-tauri/src/known_files.rs.

What goes where:
  docs/app-handlers.md
    Tested in Floppy     handler test results, per guest + app + version
                         + program + file type. Counts add up.
    Known versions       confirmed app + version + program fingerprint
                         (from test results and from identities). Floppy
                         recognizes these programs without asking.
    Reported file types  extensions users said a known app opens, not in
                         its row above. Floppy offers the app for them.
    Reported problems    what users wrote in an app's Errors field, per
                         app + version + program (an app nobody identified
                         is "<its name> (unidentified)"). For people to
                         read; Floppy doesn't use it.
  docs/legal-setupfiles.md
    Reported by users    ROMs and Workbench floppies in use that no
                         published list has. The missing-files list asks
                         for them. Rows that known_files.rs now lists are
                         dropped.

Floppy reads the other four tables straight from the documents when it's
built, so merging is all it takes. Each findings file is merged once
(its ID is recorded in app-handlers.md), and both documents get a
check-log row. A fingerprint already listed as a different app or
version is reported and left alone: someone has to look. The script
also suggests *believed* entries that tests now back up.

Review the diff before committing: findings come from users.

Standard library only. It never contacts anything.
"""

import datetime as dt
import json
import os
import re
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GUEST = {"dos": "DOS", "mac-classic": "Classic Mac", "amiga": "Amiga"}
MAX_NOTES = 3
HEADERS = {
    "tested": "| Guest | App | Version | Program | File type | Worked | Failed | Last tested | Notes |",
    "versions": "| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |",
    "filetypes": "| Guest | App | Extension | Reports | Last reported |",
    "reported": "| Slot | What | Size | SHA-1 | Reports | Last reported |",
    "problems": "| Guest | App | Version | Program | Errors | Reported |",
}


def cell(s):
    return str(s).replace("|", "/").replace("\n", " ").strip()


def block(doc, name):
    """The (start, end, rows) of a marked table; rows are cell lists."""
    m = re.search(rf"<!-- {name}:start -->\n(.*?)<!-- {name}:end -->", doc, re.S)
    if not m:
        sys.exit(f"No {name}:start/{name}:end markers.")
    rows = []
    for line in m.group(1).strip().splitlines()[2:]:
        if line.strip().startswith("|"):
            rows.append([c.strip().strip("`") for c in line.strip().strip("|").split("|")])
    return m, rows


def replace_block(doc, name, lines):
    header = HEADERS[name]
    sep = "|" + "---|" * (header.count("|") - 1)
    body = "\n".join([header, sep] + lines)
    return re.sub(rf"<!-- {name}:start -->\n.*?<!-- {name}:end -->",
                  lambda _: f"<!-- {name}:start -->\n{body}\n<!-- {name}:end -->", doc, flags=re.S)


def load(path):
    """A findings file or old test report, as one findings dict."""
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as z:
            name = next((n for n in z.namelist() if n.endswith("floppy-findings.json")), None)
            if not name:
                sys.exit(f"{path} has no floppy-findings.json.")
            data = json.loads(z.read(name))
    else:
        data = json.load(open(path, encoding="utf-8"))
    fmt = data.get("format")
    if fmt == "floppy-findings" and data.get("version", 0) >= 1:
        return data
    if fmt == "floppy-handler-tests" and data.get("version", 0) >= 1:
        return {"id": data["id"], "handlerTests": data.get("tallies", [])}
    sys.exit(f"{path} isn't Floppy findings or a handler test report.")


def day(unix):
    return dt.datetime.fromtimestamp(unix, dt.timezone.utc).strftime("%Y-%m-%d")


def handler_exts(doc):
    """App name -> extensions, from the DOS table's Opens column."""
    out = {}
    section = re.search(r"## DOS\n(.*?)\n## ", doc, re.S)
    for line in (section.group(1) if section else "").splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) >= 3 and cells[2].startswith("."):
            out[cells[0].lower()] = {e.lstrip(".").upper() for e in cells[2].split()}
    return out


def add_log_row(doc, text):
    today = dt.date.today().isoformat()
    log = re.search(r"## Check log\n\n\| Date \| What was checked \| Result \|\n\|---\|---\|---\|\n", doc)
    return doc[: log.end()] + f"| {today} | {text} |\n" + doc[log.end():] if log else doc


def main():
    args = sys.argv[1:]
    root = ROOT
    if args[:1] == ["--root"] and len(args) > 1:
        root, args = args[1], args[2:]
    if not args:
        sys.exit(__doc__.split("\n\n")[1])
    handlers_path = os.path.join(root, "docs", "app-handlers.md")
    setup_path = os.path.join(root, "docs", "legal-setupfiles.md")
    known_rs = os.path.join(root, "src-tauri", "src", "known_files.rs")
    hdoc = open(handlers_path, encoding="utf-8").read()
    sdoc = open(setup_path, encoding="utf-8").read()
    published = set(re.findall(r'sha1: "([0-9a-f]{40})"', open(known_rs, encoding="utf-8").read()))

    reg = re.search(r"<!-- merged-reports:(.*?)-->", hdoc)
    merged_ids = [i for i in reg.group(1).split() if i] if reg else []

    tested = {}
    for c in block(hdoc, "tested")[1]:
        if len(c) == 8:  # Before the Version column.
            c = c[:2] + [""] + c[2:]
        if len(c) != 9:
            continue
        g, app, ver, prog, ftype, w, f, last, notes = c
        tested[(g, app.lower(), ver, prog.upper(), ftype.upper())] = {
            "guest": g, "app": app, "version": ver, "program": prog, "type": ftype, "worked": int(w or 0),
            "failed": int(f or 0), "last": last, "notes": [n.strip() for n in notes.split(";") if n.strip()]}
    versions = {}
    for c in block(hdoc, "versions")[1]:
        if len(c) == 9:
            g, app, ver, prog, size, sha, w, f, last = c
            versions[sha.lower()] = {"guest": g, "app": app, "version": ver, "program": prog,
                                     "size": int(size.replace(",", "") or 0), "sha": sha.lower(),
                                     "worked": int(w or 0), "failed": int(f or 0), "last": last}
    filetypes = {}
    for c in block(hdoc, "filetypes")[1]:
        if len(c) == 5:
            g, app, ext, reports, last = c
            ext = ext.lstrip(".").upper()
            filetypes[(g, app.lower(), ext)] = {"guest": g, "app": app, "ext": ext, "reports": int(reports or 0), "last": last}
    problems = {}
    for c in block(hdoc, "problems")[1]:
        if len(c) == 6:
            g, app, ver, prog, errors, last = c
            problems[(g, app.lower(), ver, prog.upper(), errors)] = {
                "guest": g, "app": app, "version": ver, "program": prog, "errors": errors, "last": last}
    reported = {}
    for c in block(sdoc, "reported")[1]:
        if len(c) == 6:
            slot, what, size, sha1, reports, last = c
            reported[sha1.lower()] = {"slot": slot, "what": what, "size": int(size.replace(",", "") or 0),
                                      "sha1": sha1.lower(), "reports": int(reports or 0), "last": last}
    table_exts = handler_exts(hdoc)

    merged, skipped, conflicts, new_problems = 0, 0, [], []

    def add_version(guest, app, version, program, size, sha, worked, failed, last):
        known = versions.get(sha)
        if known and (known["app"].lower(), known["version"]) != (app.lower(), version):
            c = f"{program} {sha[:12]}…: listed as {known['app']} {known['version']}, findings say {app} {version}"
            if c not in conflicts:
                conflicts.append(c)
            return
        known = versions.setdefault(sha, {"guest": guest, "app": app, "version": version, "program": program,
                                          "size": size, "sha": sha, "worked": 0, "failed": 0, "last": ""})
        known["worked"] += worked
        known["failed"] += failed
        known["last"] = max(known["last"], last)

    for path in args:
        data = load(path)
        if data["id"] in merged_ids:
            print(f"Skipped {path}: already merged.")
            skipped += 1
            continue
        exported = day(data.get("exported", dt.datetime.now().timestamp()))

        for t in data.get("handlerTests", []):
            guest = GUEST.get(t["os"], t["os"])
            version = cell(t.get("version") or "")
            key = (guest, t["app"].lower(), version, t["program"].upper(), t["fileType"].upper())
            last = day(t["lastTested"])
            row = tested.setdefault(key, {"guest": guest, "app": t["app"], "version": version, "program": t["program"],
                                          "type": t["fileType"], "worked": 0, "failed": 0, "last": "", "notes": []})
            row["worked"] += t["worked"]
            row["failed"] += t["failed"]
            row["last"] = max(row["last"], last)
            for n in t.get("notes", []):
                n = cell(n).replace(";", ",")
                if n and n not in row["notes"]:
                    row["notes"].append(n)
            row["notes"] = row["notes"][-MAX_NOTES:]
            sha = (t.get("sha256") or "").lower()
            if t.get("confirmed") and version and t.get("size") and re.fullmatch(r"[0-9a-f]{64}", sha):
                add_version(guest, t["app"], version, t["program"], t["size"], sha, t["worked"], t["failed"], last)

        for i in data.get("identities", []):
            sha = (i.get("sha256") or "").lower()
            version = cell(i.get("version") or "")
            if version and re.fullmatch(r"[0-9a-f]{64}", sha):
                add_version(GUEST.get(i["os"], i["os"]), i["app"], version, i["program"], i["size"], sha, 0, 0, exported)

        for f in {(x["os"], x["app"], x["ext"].upper()) for x in data.get("fileTypes", [])}:
            os_, app, ext = f
            if ext in table_exts.get(app.lower(), set()):
                continue
            guest = GUEST.get(os_, os_)
            row = filetypes.setdefault((guest, app.lower(), ext), {"guest": guest, "app": app, "ext": ext, "reports": 0, "last": ""})
            row["reports"] += 1
            row["last"] = max(row["last"], exported)

        for s in {x["sha1"].lower(): x for x in data.get("systemFiles", [])}.values():
            sha1 = s["sha1"].lower()
            if sha1 in published or not re.fullmatch(r"[0-9a-f]{40}", sha1):
                continue
            row = reported.setdefault(sha1, {"slot": s["slot"], "what": cell(s["what"]), "size": s["size"],
                                             "sha1": sha1, "reports": 0, "last": ""})
            row["reports"] += 1
            row["last"] = max(row["last"], exported)

        for e in data.get("appErrors", []):
            guest = GUEST.get(e["os"], e["os"])
            version = cell(e.get("version") or "")
            program = cell(e.get("program") or "")
            sha = (e.get("sha256") or "").lower()
            if re.fullmatch(r"[0-9a-f]{64}", sha):
                program = f"{program} {sha[:12]}" if program else sha[:12]
            errors = cell(e.get("errors") or "")
            if not errors:
                continue
            # An app nobody identified goes under the user's name for it.
            app = e["app"] if e.get("known", True) else f"{cell(e['app'])} (unidentified)"
            row = problems.setdefault((guest, app.lower(), version, program.upper(), errors),
                                      {"guest": guest, "app": app, "version": version, "program": program,
                                       "errors": errors, "last": ""})
            row["last"] = max(row["last"], exported)
            new_problems.append(row)

        merged_ids.append(data["id"])
        merged += 1

    if not merged:
        print("Nothing new to merge.")
        return

    # Rows a published list or the app tables now have.
    dropped = [r for r in reported.values() if r["sha1"] in published]
    reported = {k: r for k, r in reported.items() if k not in published}
    filetypes = {k: r for k, r in filetypes.items() if r["ext"] not in table_exts.get(r["app"].lower(), set())}

    hdoc = replace_block(hdoc, "tested", [
        f"| {r['guest']} | {cell(r['app'])} | {cell(r['version'])} | `{cell(r['program'])}` | {cell(r['type'])} | "
        f"{r['worked']} | {r['failed']} | {r['last']} | {'; '.join(r['notes'])} |"
        for r in sorted(tested.values(), key=lambda r: (r["guest"], r["app"].lower(), r["version"], r["type"]))])
    hdoc = replace_block(hdoc, "versions", [
        f"| {r['guest']} | {cell(r['app'])} | {cell(r['version'])} | `{cell(r['program'])}` | {r['size']} | "
        f"`{r['sha']}` | {r['worked']} | {r['failed']} | {r['last']} |"
        for r in sorted(versions.values(), key=lambda r: (r["guest"], r["app"].lower(), r["version"], r["sha"]))])
    hdoc = replace_block(hdoc, "filetypes", [
        f"| {r['guest']} | {cell(r['app'])} | .{r['ext']} | {r['reports']} | {r['last']} |"
        for r in sorted(filetypes.values(), key=lambda r: (r["guest"], r["app"].lower(), r["ext"]))])
    hdoc = replace_block(hdoc, "problems", [
        f"| {r['guest']} | {cell(r['app'])} | {cell(r['version'])} | {'`' + r['program'] + '`' if r['program'] else ''} | "
        f"{r['errors']} | {r['last']} |"
        for r in sorted(problems.values(), key=lambda r: (r["guest"], r["app"].lower(), r["version"], r["last"]))])
    registry = "<!-- merged-reports: " + " ".join(merged_ids) + " -->"
    hdoc = re.sub(r"<!-- merged-reports:.*?-->", lambda _: registry, hdoc) if reg else hdoc + "\n" + registry + "\n"
    sdoc = replace_block(sdoc, "reported", [
        f"| {r['slot']} | {cell(r['what'])} | {r['size']} | `{r['sha1']}` | {r['reports']} | {r['last']} |"
        for r in sorted(reported.values(), key=lambda r: (r["slot"], r["what"], r["sha1"]))])

    files = f"{merged} findings {'file' if merged == 1 else 'files'}"
    hdoc = add_log_row(hdoc, f"Merged {files} | {len(tested)} tested combinations, {len(versions)} known versions, "
                             f"{len(filetypes)} reported file types, {len(problems)} reported problems")
    sdoc = add_log_row(sdoc, f"Merged {files} | {len(reported)} setup files reported by users")
    open(handlers_path, "w", encoding="utf-8").write(hdoc)
    open(setup_path, "w", encoding="utf-8").write(sdoc)

    print(f"Merged {files}{f', skipped {skipped}' if skipped else ''}: {len(tested)} tested combinations, "
          f"{len(versions)} known versions, {len(filetypes)} reported file types, {len(problems)} reported problems, "
          f"{len(reported)} reported setup files.")
    for r in new_problems:
        print(f"Problem reported with {r['app']} {r['version']}".rstrip() + f": {r['errors']}")
    for r in dropped:
        print(f"Dropped reported {r['what']} ({r['sha1'][:12]}…): known_files.rs lists it now.")
    for c in conflicts:
        print(f"Not added, needs a look: {c}")
    # Believed entries that tests now back up: worked, never failed, and
    # filed under that app's name (confirmed, not just a same-named program).
    for line in hdoc.splitlines():
        if "believed" not in line or not line.startswith("|"):
            continue
        app = line.split("|")[1].strip().lower()
        for r in tested.values():
            if (r["worked"] and not r["failed"] and r["app"].lower() == app
                    and f"`{r['program']}`".lower() in line.lower()):
                print(f"Consider marking verified (tested in Floppy {r['worked']}x): "
                      f"{line.split('|')[1].strip()} ({r['program']}, {r['type']})")
    for r in filetypes.values():
        if r["reports"] >= 2:
            print(f"Consider adding .{r['ext']} to {r['app']}'s row ({r['reports']} reports), then delete it "
                  "from Reported file types and add it to HANDLERS.")


if __name__ == "__main__":
    main()
