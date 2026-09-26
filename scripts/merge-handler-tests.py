#!/usr/bin/env python3
"""Merges handler test reports (Floppy's "Export Test Report…", verify.rs)
into docs/app-handlers.md's "Tested in Floppy" table.

    scripts/merge-handler-tests.py [--doc PATH] REPORT.json [REPORT.json ...]

--doc merges into another copy of the document (for trying it out).

Each report is merged once: its ID is recorded in the document, and a
report already there is skipped. Counts add up per guest + app + program +
file type, the latest test date and last few notes are kept, and a
check-log row is added. It also prints the *believed* entries that tests
now back up, for promoting to *verified* by hand.

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
HEADER = "| Guest | App | Program | File type | Worked | Failed | Last tested | Notes |\n|---|---|---|---|---|---|---|---|"
MAX_NOTES = 3


def cell(s):
    return str(s).replace("|", "/").replace("\n", " ").strip()


def read_table(block):
    rows = {}
    for line in block.splitlines()[2:]:
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 8:
            continue
        guest, app, program, ftype, worked, failed, last, notes = cells
        key = (guest, app.lower(), program.strip("`").upper(), ftype.upper())
        rows[key] = {
            "guest": guest, "app": app, "program": program.strip("`"), "type": ftype,
            "worked": int(worked or 0), "failed": int(failed or 0), "last": last,
            "notes": [n.strip() for n in notes.split(";") if n.strip()],
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
    rows = read_table(m.group(1).strip())
    merged_ids = [i for i in m.group(2).split() if i]

    merged, skipped = 0, 0
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
            key = (guest, t["app"].lower(), t["program"].upper(), t["fileType"].upper())
            last = dt.datetime.fromtimestamp(t["lastTested"], dt.timezone.utc).strftime("%Y-%m-%d")
            row = rows.setdefault(key, {"guest": guest, "app": t["app"], "program": t["program"],
                                        "type": t["fileType"], "worked": 0, "failed": 0, "last": "", "notes": []})
            row["worked"] += t["worked"]
            row["failed"] += t["failed"]
            row["last"] = max(row["last"], last)
            for n in t.get("notes", []):
                n = cell(n).replace(";", ",")
                if n and n not in row["notes"]:
                    row["notes"].append(n)
            row["notes"] = row["notes"][-MAX_NOTES:]
        merged_ids.append(report["id"])
        merged += 1

    if not merged:
        print("Nothing new to merge.")
        return
    lines = [HEADER]
    for r in sorted(rows.values(), key=lambda r: (r["guest"], r["app"].lower(), r["type"])):
        lines.append(f"| {r['guest']} | {cell(r['app'])} | `{cell(r['program'])}` | {cell(r['type'])} | "
                     f"{r['worked']} | {r['failed']} | {r['last']} | {'; '.join(r['notes'])} |")
    new_block = "<!-- tested:start -->\n" + "\n".join(lines) + "\n<!-- tested:end -->\n<!-- merged-reports: " + " ".join(merged_ids) + " -->"
    doc = doc[: m.start()] + new_block + doc[m.end():]

    today = dt.date.today().isoformat()
    log_row = f"| {today} | Merged {merged} handler test {'report' if merged == 1 else 'reports'} | {len(rows)} tested combinations in the table |"
    log = re.search(r"## Check log\n\n\| Date \| What was checked \| Result \|\n\|---\|---\|---\|\n", doc)
    if log:
        doc = doc[: log.end()] + log_row + "\n" + doc[log.end():]
    open(doc_path, "w", encoding="utf-8").write(doc)
    print(f"Merged {merged} report(s){f', skipped {skipped}' if skipped else ''}. {len(rows)} combinations in the table.")

    # Believed entries that tests now back up: worked, never failed.
    for line in doc.splitlines():
        if "believed" not in line or not line.startswith("|"):
            continue
        for r in rows.values():
            if r["worked"] and not r["failed"] and f"`{r['program']}`".lower() in line.lower():
                print(f"Consider marking verified (tested in Floppy {r['worked']}x): {line.split('|')[1].strip()} ({r['program']}, {r['type']})")


if __name__ == "__main__":
    main()
