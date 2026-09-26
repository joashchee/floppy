import { useEffect, useMemo, useState, type ReactElement } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Dialog } from "./components/Dialog";
import { ProgressBar } from "./components/ProgressBar";
import {
  AmigaAppIcon,
  AppMarkIcon,
  ChipIcon,
  DosAppIcon,
  FileIcon,
  FolderIcon,
  MacAppIcon,
  PlayIcon,
  PromptIcon,
  TrashIcon,
} from "./components/icons";
import {
  AMIGA_MODELS,
  bootsAsDisk,
  GUEST_LABEL,
  GUEST_OSES,
  guestPath,
  type GuestOs,
  type GuestStatus,
  type LibraryApp,
  type CdImport,
  type StartupImport,
} from "./lib/types";
import "./App.css";

/** Per-guest wording and file types. Mac and Amiga apps often have no extension, so their pickers take any file. */
const GUEST_UI: Record<
  GuestOs,
  {
    icon: () => ReactElement;
    importFilter: string[] | null;
    importFileLabel: string;
    dropHint: string;
    libraryDesc: string;
    emptyHint: string;
    bootOnlyLabel: string;
    bootOnlyTitle: string;
  }
> = {
  dos: {
    icon: () => <DosAppIcon />,
    importFilter: ["zip", "exe", "com", "bat"],
    importFileLabel: "Import Zip or Program…",
    dropHint: "A DOS program's folder, a zip, or an .EXE, .COM or .BAT",
    libraryDesc: "Everything here is on drive C: in DOSBox, so apps can reach each other's files.",
    emptyHint: "Drop a DOS program's folder, a zip, or an .EXE onto this window, or use the Import buttons.",
    bootOnlyLabel: "DOS Prompt",
    bootOnlyTitle: "Boot DOSBox at a prompt in this app's folder",
  },
  "mac-classic": {
    icon: () => <MacAppIcon />,
    importFilter: null,
    importFileLabel: "Import File…",
    dropHint: "A Mac app's folder, a zip made on a Mac, MacBinary (.bin), StuffIt/BinHex, or a disk image",
    libraryDesc:
      "Everything here is on the Unix volume on the Mac's desktop. Resource forks are kept, so apps copied from a Mac disk still open.",
    emptyHint:
      "Drop a Mac app's folder, a zip made on a Mac, a MacBinary (.bin) file, a StuffIt or BinHex archive, or a disk image onto this window.",
    bootOnlyLabel: "Start Mac OS",
    bootOnlyTitle: "Start Mac OS without mounting this app's disk image",
  },
  amiga: {
    icon: () => <AmigaAppIcon />,
    importFilter: null,
    importFileLabel: "Import Disk or File…",
    dropHint: "An Amiga program's folder, a zip, or a disk image (.adf, .adz, .dms, .hdf)",
    libraryDesc: "Everything here is on the Floppy: drive in the Amiga, so apps can reach each other's files.",
    emptyHint: "Drop an .adf disk image, a zip, or an Amiga program's folder onto this window.",
    bootOnlyLabel: "Start Workbench",
    bootOnlyTitle: "Boot your Workbench without this app's disk",
  },
};

function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function emulatorLabel(s: GuestStatus | undefined): string {
  if (!s) return "Checking…";
  if (!s.found) return `${s.emulator}: not found`;
  if (s.source === "installed") return `${s.emulator} (installed)`;
  if (s.source === "env") return `${s.emulator} (env override)`;
  return s.emulator;
}

function App() {
  const [apps, setApps] = useState<LibraryApp[]>([]);
  const [guest, setGuest] = useState<GuestOs>("dos");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [running, setRunning] = useState<Set<string>>(new Set());
  const [statuses, setStatuses] = useState<GuestStatus[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<LibraryApp | null>(null);
  const [nameDraft, setNameDraft] = useState("");

  const ui = GUEST_UI[guest];
  const status = statuses.find((s) => s.os === guest);
  const guestApps = useMemo(
    () => apps.filter((a) => a.os === guest).sort((a, b) => a.name.localeCompare(b.name)),
    [apps, guest],
  );
  const selected = guestApps.find((a) => a.id === selectedId) ?? null;
  const guestRunning = apps.some((a) => a.os === guest && running.has(a.id));

  useEffect(() => {
    setNameDraft(selected?.name ?? "");
  }, [selected?.id, selected?.name]);

  async function refresh(): Promise<LibraryApp[]> {
    const list = await invoke<LibraryApp[]>("list_apps");
    setApps(list);
    return list;
  }

  async function refreshStatuses() {
    setStatuses(await invoke<GuestStatus[]>("guest_statuses"));
  }

  async function refreshRunning() {
    setRunning(new Set(await invoke<string[]>("running_apps")));
  }

  function fail(e: unknown) {
    setMessage(null);
    setError(String(e));
  }

  function select(app: LibraryApp) {
    setGuest(app.os);
    setSelectedId(app.id);
  }

  useEffect(() => {
    void (async () => {
      try {
        await refresh();
        await refreshStatuses();
        const startup = await invoke<StartupImport | null>("take_startup_import");
        if (startup?.app) {
          await refresh();
          select(startup.app);
          setMessage(`Imported ${startup.app.name} as ${guestPath(startup.app)}.`);
        } else if (startup?.error) {
          setError(startup.error);
        }
      } catch (e) {
        fail(e);
      }
    })();
    const unlisten = listen("running-changed", () => void refreshRunning());
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setDragging(true);
      } else if (event.payload.type === "leave") {
        setDragging(false);
      } else if (event.payload.type === "drop") {
        setDragging(false);
        void importPaths(event.payload.paths);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
    // Re-subscribe when the guest changes, so drops import into the guest on screen.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [guest]);

  /** Imports one after another: each copy is disk-bound, so running them in parallel wouldn't be faster. */
  async function importPaths(paths: string[]) {
    setError(null);
    setMessage(null);
    let last: LibraryApp | null = null;
    const failures: string[] = [];
    for (const path of paths) {
      setBusy(`Importing ${baseName(path)}`);
      try {
        last = await invoke<LibraryApp>("import_app", { os: guest, path });
      } catch (e) {
        failures.push(String(e));
      }
    }
    setBusy(null);
    await refresh();
    if (last) {
      select(last);
      setMessage(
        paths.length === 1 ? `Imported ${last.name} as ${guestPath(last)}.` : `Imported ${paths.length - failures.length} of ${paths.length}.`,
      );
    }
    if (failures.length) setError(failures.join("\n"));
  }

  async function pickFolder() {
    const path = await open({ directory: true, title: `Import a ${GUEST_LABEL[guest]} program's folder` });
    if (typeof path === "string") await importPaths([path]);
  }

  async function pickFiles() {
    const picked = await open({
      multiple: true,
      title: `Import into the ${GUEST_LABEL[guest]} library`,
      filters: ui.importFilter ? [{ name: "Importable files", extensions: ui.importFilter }] : undefined,
    });
    if (Array.isArray(picked) && picked.length) await importPaths(picked);
  }

  async function chooseSystemFile(kind: "rom" | "boot", directory: boolean) {
    const title =
      kind === "rom"
        ? guest === "amiga"
          ? "Choose your Kickstart ROM"
          : "Choose your Mac ROM"
        : guest === "amiga"
          ? "Choose your Workbench disk or folder"
          : "Choose your Mac startup disk image";
    const path = await open({ directory, title });
    if (typeof path !== "string") return;
    setError(null);
    setMessage(null);
    setBusy(`Copying ${baseName(path)}`);
    try {
      await invoke("set_system_file", { os: guest, kind, path });
      await refreshStatuses();
      setMessage(`Added ${baseName(path)}.`);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  /** Saves the missing-files list: the system files still needed, one file name per line (cd.rs). */
  async function saveMissingList() {
    const path = await save({
      title: "Save the missing-files list",
      defaultPath: "Floppy needs.txt",
      filters: [{ name: "Text", extensions: ["txt"] }],
    });
    if (!path) return;
    setError(null);
    try {
      const count = await invoke<number>("write_missing_list", { path });
      setMessage(
        count === 0
          ? "Nothing is missing: every guest has its system files."
          : `Saved ${baseName(path)}. Gather the files it names into a disc image or folder (Diskette's Burn A CD does this), then import that.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Sets up every guest it can from a files disc: a disc image or folder of gathered system files. */
  async function importFilesDisc(directory: boolean) {
    const path = await open(
      directory
        ? { directory: true, title: "Import a folder of system files" }
        : { title: "Import a files disc", filters: [{ name: "Disc image", extensions: ["iso", "cdr", "dmg", "toast"] }] },
    );
    if (typeof path !== "string") return;
    setError(null);
    setMessage(null);
    setBusy(`Reading ${baseName(path)}`);
    try {
      const r = await invoke<CdImport>("import_files_disc", { path });
      await refreshStatuses();
      const added = r.added.length ? `Added ${r.added.join(", ")}.` : "Found nothing new to add.";
      const missing = r.stillMissing.length ? ` Still missing: ${r.stillMissing.join(", ")}.` : "";
      setMessage(added + missing);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  async function setModel(model: string) {
    try {
      await invoke("set_guest_model", { os: guest, model });
      await refreshStatuses();
    } catch (e) {
      fail(e);
    }
  }

  async function launch(app: LibraryApp, promptOnly: boolean) {
    setError(null);
    try {
      await invoke("launch_app", { id: app.id, promptOnly });
    } catch (e) {
      fail(e);
    }
  }

  async function setProgram(app: LibraryApp, program: string) {
    try {
      await invoke("set_program", { id: app.id, program });
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  async function commitName(app: LibraryApp) {
    const name = nameDraft.trim();
    if (!name || name === app.name) {
      setNameDraft(app.name);
      return;
    }
    try {
      await invoke("rename_app", { id: app.id, name });
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  async function reveal(app: LibraryApp) {
    try {
      await revealItemInDir(await invoke<string>("app_folder", { id: app.id }));
    } catch (e) {
      fail(e);
    }
  }

  async function remove(app: LibraryApp) {
    setConfirmRemove(null);
    try {
      await invoke("remove_app", { id: app.id });
      if (selectedId === app.id) setSelectedId(null);
      setMessage(`Removed ${app.name}.`);
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  const blocker = status?.blocker ?? null;
  // A Mac or Amiga shares one startup disk between its apps, so only one runs at a time.
  const launchBlocked = !status || !!blocker || (guest !== "dos" && guestRunning);
  const workbenchMissing = guest === "amiga" && !status?.system.boot;

  return (
    <main className="app">
      <header className="top">
        <div className="title-group">
          <h1>
            <span className="app-mark" aria-hidden="true">
              <AppMarkIcon />
            </span>{" "}
            Floppy <span className="version-tag">v{__APP_VERSION__}</span>
          </h1>
          <p>Run the old apps your files need, in the OS they were made for.</p>
        </div>
        <div className="header-actions">
          <span className={`status-pill${status && !status.found ? " warn" : ""}`}>{emulatorLabel(status)}</span>
        </div>
      </header>

      {error && <div className="error">{error}</div>}
      {message && <div className="status-message">{message}</div>}

      <div className="guest-tiles" role="tablist" aria-label="Guest OS">
        {GUEST_OSES.map((os) => {
          const s = statuses.find((x) => x.os === os);
          const count = apps.filter((a) => a.os === os).length;
          return (
            <button
              key={os}
              type="button"
              role="tab"
              aria-selected={os === guest}
              className={`guest-tile${os === guest ? " selected" : ""}`}
              onClick={() => {
                setGuest(os);
                setSelectedId(null);
              }}
            >
              <span className="guest-tile-icon">{GUEST_UI[os].icon()}</span>
              <span className="guest-tile-main">
                <span className="guest-tile-name">{GUEST_LABEL[os]}</span>
                <span className={`guest-tile-meta${s?.blocker ? " warn" : ""}`}>
                  {s?.blocker ? "Needs setup" : `${count} ${count === 1 ? "app" : "apps"} · ${s?.emulator ?? ""}`}
                </span>
              </span>
            </button>
          );
        })}
      </div>

      <div className="toolbar">
        <button type="button" className="primary icontext-btn" onClick={() => void pickFolder()} disabled={!!busy}>
          <span className="btn-icon">
            <FolderIcon />
          </span>
          Import Folder…
        </button>
        <button type="button" className="icontext-btn" onClick={() => void pickFiles()} disabled={!!busy}>
          <span className="btn-icon">
            <FileIcon />
          </span>
          {ui.importFileLabel}
        </button>
        <button
          type="button"
          className="icontext-btn"
          onClick={() => void importFilesDisc(false)}
          disabled={!!busy || apps.some((a) => a.os !== "dos" && running.has(a.id))}
          title="Add ROMs and startup disks from a disc image of gathered system files"
        >
          <span className="btn-icon">
            <ChipIcon />
          </span>
          Import Files Disc…
        </button>
      </div>

      {busy && (
        <div className="scan-status-row">
          <span className="scan-status-label">{busy}…</span>
          <ProgressBar indeterminate label={busy} />
        </div>
      )}

      <div className="layout">
        <section className="panel">
          <h2>
            <span className="section-icon">{ui.icon()}</span>
            {GUEST_LABEL[guest]} Library
          </h2>
          <p className="desc">{ui.libraryDesc}</p>

          {guest !== "dos" && status && (
            <SystemSetup
              status={status}
              disabled={!!busy || guestRunning}
              onChoose={(kind, directory) => void chooseSystemFile(kind, directory)}
              onModel={(m) => void setModel(m)}
              onSaveList={() => void saveMissingList()}
              onImportFolder={() => void importFilesDisc(true)}
            />
          )}

          {guestApps.length === 0 ? (
            <p className="empty">No apps yet. {ui.emptyHint}</p>
          ) : (
            <ul className="app-list">
              {guestApps.map((app) => (
                <li key={app.id}>
                  <button
                    type="button"
                    className={`app-row${app.id === selectedId ? " selected" : ""}`}
                    onClick={() => setSelectedId(app.id)}
                    onDoubleClick={() => app.program && !launchBlocked && void launch(app, false)}
                  >
                    <span className="row-icon">{ui.icon()}</span>
                    <span className="row-main">
                      <span className="row-name">{app.name}</span>
                      <span className="row-meta">{guestPath(app, app.program)}</span>
                    </span>
                    {running.has(app.id) && <span className="status-pill running">Running</span>}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="panel">
          {!selected ? (
            <p className="empty">Select an app to launch it or change what it opens.</p>
          ) : (
            <div className="app-details">
              <label className="field">
                <span className="field-label">Name</span>
                <input
                  type="text"
                  value={nameDraft}
                  onChange={(e) => setNameDraft(e.target.value)}
                  onBlur={() => void commitName(selected)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                    if (e.key === "Escape") {
                      setNameDraft(selected.name);
                      e.currentTarget.blur();
                    }
                  }}
                />
              </label>
              <label className="field">
                <span className="field-label">{selected.os === "dos" ? "Runs" : "Opens"}</span>
                <select value={selected.program ?? ""} onChange={(e) => void setProgram(selected, e.target.value)}>
                  {selected.programs.map((p) => (
                    <option key={p} value={p}>
                      {guestPath(selected, p)}
                    </option>
                  ))}
                </select>
              </label>
              <dl className="details-grid">
                <dt>Folder</dt>
                <dd className="details-path">{guestPath(selected)}</dd>
                <dt>Imported from</dt>
                <dd>{selected.sourceName}</dd>
                <dt>Added</dt>
                <dd>{new Date(selected.added * 1000).toLocaleString()}</dd>
              </dl>
              <div className="detail-actions">
                <button
                  type="button"
                  className="primary icontext-btn"
                  onClick={() => void launch(selected, false)}
                  disabled={launchBlocked || !selected.program || running.has(selected.id)}
                >
                  <span className="btn-icon">
                    <PlayIcon />
                  </span>
                  {running.has(selected.id) ? "Running" : "Launch"}
                </button>
                <button
                  type="button"
                  className="icontext-btn"
                  onClick={() => void launch(selected, true)}
                  disabled={launchBlocked || running.has(selected.id) || workbenchMissing}
                  title={ui.bootOnlyTitle}
                >
                  <span className="btn-icon">
                    <PromptIcon />
                  </span>
                  {ui.bootOnlyLabel}
                </button>
                <button type="button" className="icontext-btn" onClick={() => void reveal(selected)}>
                  <span className="btn-icon">
                    <FolderIcon />
                  </span>
                  Show in Finder
                </button>
                <button
                  type="button"
                  className="danger icontext-btn"
                  onClick={() => setConfirmRemove(selected)}
                  disabled={running.has(selected.id) || (guest !== "dos" && guestRunning)}
                >
                  <span className="btn-icon">
                    <TrashIcon />
                  </span>
                  Remove
                </button>
              </div>
              <LaunchHint app={selected} blocker={blocker} guestRunning={guest !== "dos" && guestRunning} />
            </div>
          )}
        </section>
      </div>

      <Dialog
        open={!!confirmRemove}
        onClose={() => setConfirmRemove(null)}
        title={`Remove ${confirmRemove?.name ?? ""}?`}
        actions={
          <>
            <button type="button" onClick={() => setConfirmRemove(null)}>
              Cancel
            </button>
            <button type="button" className="danger" onClick={() => confirmRemove && void remove(confirmRemove)}>
              Remove
            </button>
          </>
        }
      >
        <p>
          This deletes {confirmRemove ? guestPath(confirmRemove) : "the folder"} from Floppy's {GUEST_LABEL[guest]} library,
          including any files the app saved there. The original you imported from isn't touched.
        </p>
      </Dialog>

      <div className={`drop-overlay${dragging ? " show" : ""}`}>
        <div className="drop-box">
          Drop to import into {GUEST_LABEL[guest]}
          <small>{ui.dropHint}</small>
        </div>
      </div>
    </main>
  );
}

/** What pressing Launch will do, when it isn't simply "run this program". */
function LaunchHint({ app, blocker, guestRunning }: { app: LibraryApp; blocker: string | null; guestRunning: boolean }) {
  if (blocker) return <p className="system-note warn">{blocker}</p>;
  if (guestRunning) {
    return (
      <p className="launch-hint">
        One {GUEST_LABEL[app.os]} runs at a time, because its apps share one startup disk. Quit it to launch another.
      </p>
    );
  }
  if (app.os === "dos" || bootsAsDisk(app.os, app.program)) return null;
  const where = app.os === "mac-classic" ? "Mac OS" : "Workbench";
  return (
    <p className="launch-hint">
      Launch starts {where}. Open <strong>{guestPath(app, app.program)}</strong> from there.
    </p>
  );
}

/** The user-supplied ROM and startup disk a Mac or Amiga needs. Floppy never includes these. */
function SystemSetup({
  status,
  disabled,
  onChoose,
  onModel,
  onSaveList,
  onImportFolder,
}: {
  status: GuestStatus;
  disabled: boolean;
  onChoose: (kind: "rom" | "boot", directory: boolean) => void;
  onModel: (model: string) => void;
  onSaveList: () => void;
  onImportFolder: () => void;
}) {
  const amiga = status.os === "amiga";
  const { rom, boot, model } = status.system;
  return (
    <div className="system-setup">
      <h3>
        <span className="section-icon">
          <ChipIcon />
        </span>
        {amiga ? "Amiga system" : "Mac system"}
      </h3>
      <div className="system-row">
        <span className="system-label">{amiga ? "Kickstart ROM" : "Mac ROM"}</span>
        <span className={`system-value${rom ? "" : " missing"}`}>
          {rom ? `${rom}${status.romNote ? ` · ${status.romNote}` : ""}` : "Not added"}
        </span>
        <button type="button" className="small" disabled={disabled} onClick={() => onChoose("rom", false)}>
          Choose…
        </button>
      </div>
      <div className="system-row">
        <span className="system-label">{amiga ? "Workbench" : "Startup disk"}</span>
        <span className={`system-value${boot ? "" : " missing"}`}>{boot ?? (amiga ? "Not added (optional)" : "Not added")}</span>
        <button type="button" className="small" disabled={disabled} onClick={() => onChoose("boot", false)}>
          {amiga ? "Disk…" : "Choose…"}
        </button>
        {amiga && (
          <button type="button" className="small" disabled={disabled} onClick={() => onChoose("boot", true)}>
            Folder…
          </button>
        )}
      </div>
      {amiga && (
        <div className="system-row">
          <span className="system-label">Model</span>
          <select value={model ?? "A1200"} disabled={disabled} onChange={(e) => onModel(e.target.value)}>
            {AMIGA_MODELS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </div>
      )}
      <p className="system-note">
        {amiga
          ? "Floppy doesn't include Amiga system software. Use a Kickstart ROM and Workbench you own (from your own Amiga, or Amiga Forever). Workbench is needed for apps that aren't bootable disks."
          : "Floppy doesn't include Mac system software. Use a ROM from a Mac you own (IIci, IIsi, Quadra, Centris or similar) and a disk image with System 7 to Mac OS 8.1 installed. Floppy emulates a Quadra 900 with Basilisk II."}
      </p>
      {(!rom || !boot) && (
        <div className="system-row">
          <button type="button" className="small" disabled={disabled} onClick={onSaveList}>
            Save Missing-Files List…
          </button>
          <button type="button" className="small" disabled={disabled} onClick={onImportFolder}>
            Import Folder of Files…
          </button>
          <span className="system-note">
            Lists the files still needed, to find on your drives (for example with Diskette's Burn A CD). Floppy checks
            each file's contents when you import them.
          </span>
        </div>
      )}
    </div>
  );
}

export default App;
