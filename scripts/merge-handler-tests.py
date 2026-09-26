#!/usr/bin/env python3
"""Merges handler test reports (Floppy's "Export Test Report…", verify.rs)
into docs/app-handlers.md's "Tested in Floppy" and "Known versions" tables.

    scripts/merge-handler-tests.py [--doc PATH] REPORT.json [REPORT.json ...]

--doc merges into another copy of the document (for trying it out).

Each report is merged once: its ID is recorded in the document, and a
report already there is skipped. Counts add up per guest + app + version
+ program + file type, the latest test date and last few notes are kept,
and a check-log row is added. It also prints the *believed* entries that
tests now back up, for promoting to *verified* by hand.

A result whose app the user (or a fingerprint) confirmed, with a version
and the program's size and SHA-256 (reports from version 2 on), also
adds or updates that version's row in "Known versions", which Floppy
reads to recognize the same program in other libraries. A fingerprint
already listed as a different app or version is reported and left
alone: someone has to look.

Standard library only. It never contacts anything.
"""

import datetime as dt
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOC = os.path.join(ROOT, "docs", "app-handlers.md")
GUEST = {"dos": "DOS", "mac-classic": "Classic Mac", "amiga": "Amiga"}
TESTED_HEADER = ("| Guest | App | Version | Program | File type | Worked | Failed | Last tested | Notes |\n"
                 "|---|---|---|---|---|---|---|---|---|")
VERSIONS_HEADER = ("| Guest | App | Version | Program | Size | SHA-256 | Worked | Failed | Last tested |\n"
                   "|---|---|---|---|---|---|---|---|---|")
MAX_NOTES = 3


def cell(s):
    return str(s).replace("|", "/").replace("\n", " ").strip()


def cells_of(block):
    """Each table row's cells, header and separator skipped."""
    for line in block.strip().splitlines()[2:]:
        if line.strip().startswith("|"):
            yield [c.strip() for c in line.strip().strip("|").split("|")]


def read_tested(block):
    rows = {}
    for cells in cells_of(block):
        if len(cells) == 8:  # Before the Version column.
            cells = cells[:2] + [""] + cells[2:]
        if len(cells) != 9:
            continue
        guest, app, version, program, ftype, worked, failed, last, notes = cells
        program = program.strip("`")
        key = (guest, app.lower(), version, program.upper(), ftype.upper())
        rows[key] = {
            "guest": guest, "app": app, "version": version, "program": program, "type": ftype,
            "worked": int(worked or 0), "failed": int(failed or 0), "last": last,
            "notes": [n.strip() for n in notes.split(";") if n.strip()],
        }
    return rows


def read_versions(block):
    rows = {}
    for cells in cells_of(block):
        if len(cells) != 9:
            continue
        guest, app, version, program, size, sha, worked, failed, last = cells
        sha = sha.strip("`").lower()
        rows[sha] = {
            "guest": guest, "app": app, "version": version, "program": program.strip("`"),
            "size": int(size.replace(",", "") or 0), "sha": sha,
            "worked": int(worked or 0), "failed": int(failed or 0), "last": last,
        }
    return rows


def main():
    args = sys.argv[1:]
    doc_path = DOC
    if args[:1] == ["--doc"] and len(args) > 1:
        doc_path, args = args[1], args[2:]
    if not args:
        sys.exit(__doc__.split("\n\n")[1])
    doc = open(doc_path, encoding="utf-8").read()
    m = re.search(r"<!-- tested:start -->\n(.*?)<!-- tested:end -->\n<!-- merged-reports:(.*?)-->", doc, re.S)
    if not m:
        sys.exit("docs/app-handlers.md has no tested:start/tested:end markers.")
    v = re.search(r"<!-- versions:start -->\n(.*?)<!-- versions:end -->", doc, re.S)
    if not v:
        sys.exit("docs/app-handlers.md has no versions:start/versions:end markers.")
    rows = read_tested(m.group(1))
    versions = read_versions(v.group(1))
    merged_ids = [i for i in m.group(2).split() if i]

    merged, skipped, conflicts = 0, 0, []
    for path in args:
        report = json.load(open(path, encoding="utf-8"))
        if report.get("format") != "floppy-handler-tests" or report.get("version", 0) < 1:
            sys.exit(f"{path} isn't a Floppy handler test report.")
        if report["id"] in merged_ids:
            print(f"Skipped {path}: already merged.")
            skipped += 1
            continue
        for t in report.get("tallies", []):
            guest = GUEST.get(t["os"], t["os"])
            version = cell(t.get("version") or "")
            key = (guest, t["app"].lower(), version, t["program"].upper(), t["fileType"].upper())
            last = dt.datetime.fromtimestamp(t["lastTested"], dt.timezone.utc).strftime("%Y-%m-%d")
            row = rows.setdefault(key, {"guest": guest, "app": t["app"], "version": version, "program": t["program"],
                                        "type": t["fileType"], "worked": 0, "failed": 0, "last": "", "notes": []})
            row["worked"] += t["worked"]
            row["failed"] += t["failed"]
            row["last"] = max(row["last"], last)
            for n in t.get("notes", []):
                n = cell(n).replace(";", ",")
                if n and n not in row["notes"]:
                    row["notes"].append(n)
            row["notes"] = row["notes"][-MAX_NOTES:]

            # A confirmed app, version and fingerprint: a known version.
            sha = (t.get("sha256") or "").lower()
            if not (t.get("confirmed") and version and t.get("size") and re.fullmatch(r"[0-9a-f]{64}", sha)):
                continue
            known = versions.get(sha)
            if known and (known["app"].lower(), known["version"]) != (t["app"].lower(), version):
                conflict = (f"{t['program']} {sha[:12]}…: listed as {known['app']} {known['version']}, "
                            f"a report says {t['app']} {version}")
                if conflict not in conflicts:
                    conflicts.append(conflict)
                continue
            known = versions.setdefault(sha, {"guest": guest, "app": t["app"], "version": version,
                                              "program": t["program"], "size": t["size"], "sha": sha,
                                              "worked": 0, "failed": 0, "last": ""})
            known["worked"] += t["worked"]
            known["failed"] += t["failed"]
            known["last"] = max(known["last"], last)
        merged_ids.append(report["id"])
        merged += 1

    if not merged:
        print("Nothing new to merge.")
        return
    lines = [TESTED_HEADER]
    for r in sorted(rows.values(), key=lambda r: (r["guest"], r["app"].lower(), r["version"], r["type"])):
        lines.append(f"| {r['guest']} | {cell(r['app'])} | {cell(r['version'])} | `{cell(r['program'])}` | "
                     f"{cell(r['type'])} | {r['worked']} | {r['failed']} | {r['last']} | {'; '.join(r['notes'])} |")
    new_block = ("<!-- tested:start -->\n" + "\n".join(lines) + "\n<!-- tested:end -->\n<!-- merged-reports: "
                 + " ".join(merged_ids) + " -->")
    doc = doc[: m.start()] + new_block + doc[m.end():]

    vlines = [VERSIONS_HEADER]
    for r in sorted(versions.values(), key=lambda r: (r["guest"], r["app"].lower(), r["version"], r["sha"])):
        vlines.append(f"| {r['guest']} | {cell(r['app'])} | {cell(r['version'])} | `{cell(r['program'])}` | "
                      f"{r['size']} | `{r['sha']}` | {r['worked']} | {r['failed']} | {r['last']} |")
    doc = re.sub(r"<!-- versions:start -->\n.*?<!-- versions:end -->",
                 lambda _: "<!-- versions:start -->\n" + "\n".join(vlines) + "\n<!-- versions:end -->", doc, flags=re.S)

    today = dt.date.today().isoformat()
    log_row = (f"| {today} | Merged {merged} handler test {'report' if merged == 1 else 'reports'} | "
               f"{len(rows)} tested combinations, {len(versions)} known versions |")
    log = re.search(r"## Check log\n\n\| Date \| What was checked \| Result \|\n\|---\|---\|---\|\n", doc)
    if log:
        doc = doc[: log.end()] + log_row + "\n" + doc[log.end():]
    open(doc_path, "w", encoding="utf-8").write(doc)
    print(f"Merged {merged} report(s){f', skipped {skipped}' if skipped else ''}. "
          f"{len(rows)} combinations, {len(versions)} known versions.")
    for c in conflicts:
        print(f"Not added, needs a look: {c}")

    # Believed entries that tests now back up: worked, never failed, and
    # filed under that app's name (so the user confirmed which app it was,
    # not just a program called the same).
    for line in doc.splitlines():
        if "believed" not in line or not line.startswith("|"):
            continue
        app = line.split("|")[1].strip().lower()
        for r in rows.values():
            if (r["worked"] and not r["failed"] and r["app"].lower() == app
                    and f"`{r['program']}`".lower() in line.lower()):
                print(f"Consider marking verified (tested in Floppy {r['worked']}x): {line.split('|')[1].strip()} ({r['program']}, {r['type']})")


if __name__ == "__main__":
    main()
