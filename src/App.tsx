import { Fragment, useEffect, useMemo, useRef, useState, type ReactElement } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { Dialog } from "./components/Dialog";
import { applyTheme, loadTheme, type Theme } from "./lib/theme";
import { isLinux, SHOW_IN_FILES } from "./lib/platform";
import { ProgressBar } from "./components/ProgressBar";
import {
  AmigaAppIcon,
  AppMarkIcon,
  ChipIcon,
  DiscIcon,
  DosAppIcon,
  ExportIcon,
  FileIcon,
  FolderIcon,
  GearIcon,
  InfoIcon,
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
  handlerCandidates,
  type HandlerInfo,
  identityLabel,
  type LibraryApp,
  type CdImport,
  docPath,
  docTypeFolder,
  DOCS_DIR,
  guestFilePath,
  type ImportedItem,
  type LibraryDoc,
  type Opener,
  type FindingsSummary,
  type DropChoice,
  type DropClassification,
  type LearnSummary,
  type KnowledgeSummary,
  type AiInfo,
  type SessionReport,
  type MediaProgress,
  type OldMedia,
  type SetupTracking,
  type BackupMade,
  type BackupStatus,
  type SetupReportKind,
  type SetupSource,
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
    docsDesc: string;
    bootOnlyLabel: string;
    bootOnlyTitle: string;
  }
> = {
  dos: {
    icon: () => <DosAppIcon />,
    importFilter: ["zip", "exe", "com", "bat"],
    importFileLabel: "Import Zip or Program…",
    dropHint: "A DOS program's folder, a zip, an .EXE, .COM or .BAT, or a document to open in one",
    libraryDesc: "Everything here is on drive C: in DOSBox, so apps can reach each other's files.",
    emptyHint:
      "Drop a DOS program's folder, a zip, or an .EXE onto this window, or use the Import buttons. Drop an old document (a .WP5, .WK1, .DBF…) to open it in an app that made it.",
    docsDesc: "Files in C:\\DOCS, in a folder per file type (C:\\DOCS\\WP5). What an app saves there is sorted when DOSBox quits.",
    bootOnlyLabel: "DOS Prompt",
    bootOnlyTitle: "Boot DOSBox at a prompt in this app's folder",
  },
  "mac-classic": {
    icon: () => <MacAppIcon />,
    importFilter: null,
    importFileLabel: "Import File…",
    dropHint: "A Mac app's folder, a zip made on a Mac, MacBinary (.bin), StuffIt/BinHex, a disk image, or a document",
    libraryDesc:
      "Everything here is on the Unix volume on the Mac's desktop. Resource forks are kept, so apps copied from a Mac disk still open.",
    emptyHint:
      "Drop a Mac app's folder, a zip made on a Mac, a MacBinary (.bin) file, a StuffIt or BinHex archive, or a disk image onto this window.",
    docsDesc:
      "Files in the Documents folder on the Unix volume, in a folder per file type (Documents:TEXT). What an app saves there is sorted when the Mac quits.",
    bootOnlyLabel: "Start Mac OS",
    bootOnlyTitle: "Start Mac OS without mounting this app's disk image",
  },
  amiga: {
    icon: () => <AmigaAppIcon />,
    importFilter: null,
    importFileLabel: "Import Disk or File…",
    dropHint: "An Amiga program's folder, a zip, a disk image (.adf, .adz, .dms, .hdf), or a document",
    libraryDesc: "Everything here is on the Floppy: drive in the Amiga, so apps can reach each other's files.",
    emptyHint: "Drop an .adf disk image, a zip, or an Amiga program's folder onto this window.",
    docsDesc:
      "Files in Floppy:Documents, in a folder per file type (Documents/ILBM). What an app saves there is sorted when the Amiga quits.",
    bootOnlyLabel: "Start Workbench",
    bootOnlyTitle: "Boot your Workbench without this app's disk",
  },
};

/** Each guest's emulator, as the Quit button names it. */
const EMULATOR_LABEL: Record<GuestOs, string> = { dos: "DOSBox", "mac-classic": "Basilisk II", amiga: "FS-UAE" };

/** What quitting a running guest takes, before and after Floppy has asked it to quit. */
function quitHint(os: GuestOs, asked: boolean): string {
  if (os === "mac-classic") {
    return asked
      ? "The Mac was asked to shut down: answer it in Basilisk II's window. If it can't, Force Quit stops it at once, like switching a real Mac off, so anything unsaved in the Mac is lost."
      : "To quit, choose Shut Down from the Mac's Special menu, or press Ctrl-Esc in its window. Closing the window only asks the Mac to shut down.";
  }
  return asked ? "Still running? Force Quit stops it at once." : "Save your work in the app before quitting.";
}

/** commands.rs `guest_id`: the running-app ID of a guest started on its own (`start_guest`). */
function guestRunId(os: GuestOs): string {
  return `guest-${os}`;
}

const GUESTS: GuestOs[] = ["dos", "mac-classic", "amiga"];

function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function emulatorLabel(s: GuestStatus | undefined): string {
  if (!s) return "Checking…";
  if (!s.found) return `${s.emulator}: not found`;
  if (s.source === "installed") return `${s.emulator} (installed)`;
  if (s.source === "env") return `${s.emulator} (env override)`;
  if (s.source === "chosen") return `${s.emulator} (located)`;
  return s.emulator;
}

/** How each kind of setup source reads on the setup screen. */
const SOURCE_KIND_LABEL: Record<SetupSource["kind"], string> = { free: "Free", paid: "Paid", own: "From yours" };

const REPORT_KIND_LABEL: Record<SetupReportKind, string> = {
  "source-broken": "A source stopped working",
  "didnt-work": "My file didn't work",
  "better-source": "I found a better source",
};

/** The Locate Basilisk II… tooltip: which copy starts the Mac now. */
function basiliskTitle(s: GuestStatus | undefined): string {
  switch (s?.found ? s.source : null) {
    case "chosen":
      return "Using the Basilisk II you located. Pick another to change it.";
    case "bundled":
      return "Using the Basilisk II built into Floppy. Pick another build to use it instead.";
    case "installed":
      return isLinux
        ? "Using the BasiliskII on your PATH. Pick another build to use it instead."
        : "Using the Basilisk II in Applications. Pick another build to use it instead.";
    case "env":
      return "Using FLOPPY_BASILISK, which overrides any copy picked here.";
    default:
      return isLinux
        ? "Basilisk II runs the classic Mac. Pick the BasiliskII program wherever it is."
        : "Basilisk II runs the classic Mac. Pick BasiliskII.app wherever it is.";
  }
}

function findingsTotal(f: FindingsSummary): number {
  return f.handlerTests + f.identities + f.fileTypes + f.systemFiles + f.appErrors + f.setupReports + f.dropChoices;
}

/** "3 test results, 1 app version": what a findings export holds. */
function describeFindings(f: FindingsSummary): string {
  const part = (n: number, one: string, many: string) => (n ? `${n} ${n === 1 ? one : many}` : "");
  return (
    [
      part(f.handlerTests, "test result", "test results"),
      part(f.identities, "app you identified", "apps you identified"),
      part(f.fileTypes, "file type you added", "file types you added"),
      part(f.systemFiles, "unlisted setup file", "unlisted setup files"),
      part(f.appErrors, "app's errors", "apps' errors"),
      part(f.setupReports, "setup report", "setup reports"),
      part(f.dropChoices, "drop choice", "drop choices"),
    ]
      .filter(Boolean)
      .join(", ") || "nothing yet"
  );
}

/** "2 app versions, 1 file type": what learning from findings added. */
function describeLearned(l: LearnSummary | KnowledgeSummary): string {
  const part = (n: number, one: string, many: string) => (n ? `${n} ${n === 1 ? one : many}` : "");
  const drops = "dropChoices" in l ? l.dropChoices : l.dropRules;
  return (
    [
      part(l.versions, "app version", "app versions"),
      part(l.fileTypes, "file type", "file types"),
      part(l.tests, "test result", "test results"),
      part(l.systemFiles, "setup file", "setup files"),
      part(drops, "drop choice", "drop choices"),
    ]
      .filter(Boolean)
      .join(", ") || "nothing new"
  );
}

/** Where a drop was decided to go, for the message after it. */
function describeChoice(c: DropChoice): string {
  const guest = GUEST_LABEL[c.os];
  return c.to === "app" ? `a ${guest} app` : c.to === "document" ? `a ${guest} document` : `a ${guest} setup file`;
}

/** " Last exported 27 Sep 2026.", or nothing before the first export. */
function lastExported(f: FindingsSummary): string {
  if (!f.lastExported) return "";
  const when = new Date(f.lastExported * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
  return ` Last exported ${when}.`;
}

/** Where a drag over the window will drop: an app import, or setup files. */
type DropZone = "app" | "setup";

/** Labels of the Mac and Amiga system files still missing (cd.rs `missing_slots`). */
/** The system files `guest` still needs: none for DOS, whose DOS comes with DOSBox. */
function missingSetupFiles(statuses: GuestStatus[], guest: GuestOs): string[] {
  const s = statuses.find((x) => x.os === guest);
  if (!s) return [];
  const out: string[] = [];
  if (guest === "mac-classic") {
    if (!s.system.rom) out.push("Mac ROM");
    if (!s.system.boot) out.push("Mac startup disk");
  } else if (guest === "amiga") {
    if (!s.system.rom) out.push("Kickstart ROM");
    if (!s.system.boot) out.push("Workbench disk");
  }
  return out;
}

function formatBytes(n: number): string {
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

/** What an import of system files did, for the status line. */
function describeImport(r: CdImport): string {
  const added = r.added.length ? `Added ${r.added.join(", ")}.` : "Found nothing new to add.";
  const missing = r.stillMissing.length ? ` Still missing: ${r.stillMissing.join(", ")}.` : "";
  const from = r.disc ? `From a ${r.disc.producer || "files"} disc${r.disc.alreadyImported ? " you imported before" : ""}: ` : "";
  const n = r.unusable.length;
  const unusable = n
    ? ` ${n} ${n === 1 ? "copy" : "copies"} on it couldn't be used. The next Missing-Files List asks for ${n === 1 ? "it" : "them"} to be left out.`
    : "";
  const gone = r.disc?.notOnDrives.length
    ? ` Your drives have no usable ${r.disc.notOnDrives.join(" or ")}, so the next list stops asking for ${r.disc.notOnDrives.length === 1 ? "it" : "them"}.`
    : "";
  const kept = r.kept?.length ? ` Kept what was set up already: ${r.kept.join(", ")}.` : "";
  return from + added + kept + missing + unusable + gone;
}

/** request.rs `RequestSummary`: what asking Diskette would ask for. */
interface RequestSummary {
  setup: number;
  apps: number;
}

/** commands.rs `DiscImport`: what a disc brought. */
interface DiscImport {
  setup: CdImport;
  apps: { imported: string[]; already: string[]; failed: string[] };
}

function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** What Floppy would ask Diskette for, in words. */
function describeRequest(r: RequestSummary): string {
  const parts = [
    r.setup ? `${r.setup === 1 ? "the setup file" : `the ${r.setup} setup files`}` : null,
    r.apps ? `${plural(r.apps, "old app")} that open old files` : null,
  ].filter(Boolean);
  return parts.join(" and ");
}

/** How often to check whether Diskette is running. */
const DISKETTE_POLL_MS = 5000;

function App() {
  const [apps, setApps] = useState<LibraryApp[]>([]);
  const [guest, setGuest] = useState<GuestOs>("dos");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [running, setRunning] = useState<Set<string>>(new Set());
  // Apps Floppy has asked to quit: their Quit button forces it next.
  const [quitAsked, setQuitAsked] = useState<Set<string>>(new Set());
  const [statuses, setStatuses] = useState<GuestStatus[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  // For listeners set up once: whether something is already in progress.
  const busyRef = useRef(busy);
  busyRef.current = busy;
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [dropZone, setDropZone] = useState<DropZone | null>(null);
  const [oldMedia, setOldMedia] = useState<OldMedia[]>([]);
  const [mediaProgress, setMediaProgress] = useState<MediaProgress | null>(null);
  const [tracking, setTracking] = useState<SetupTracking>({ ignored: 0, notOnDrives: [] });
  const [documents, setDocuments] = useState<LibraryDoc[]>([]);
  const [selectedDocId, setSelectedDocId] = useState<string | null>(null);
  const [session, setSession] = useState<SessionReport | null>(null);
  const [verifyNote, setVerifyNote] = useState("");
  // Known DOS apps that open old files (handlers.rs), for "Is" menus.
  const [dosHandlers, setDosHandlers] = useState<HandlerInfo[]>([]);
  // The after-session "Which app was this?" answer being chosen.
  const [identifyChoice, setIdentifyChoice] = useState("");
  const [identifyVersion, setIdentifyVersion] = useState("");
  // What Export Findings would share, counted when the gear menu opens.
  const [findings, setFindings] = useState<FindingsSummary | null>(null);
  // What Floppy learned from findings dropped on it (learned.rs).
  const [knowledge, setKnowledge] = useState<KnowledgeSummary | null>(null);
  // Floppy AI's version (ai.rs), shown next to the app's.
  const [ai, setAi] = useState<AiInfo | null>(null);
  // Drops with no clear winner, asked about one at a time (drops.rs).
  const [askDrops, setAskDrops] = useState<DropClassification[]>([]);
  const [dropPick, setDropPick] = useState(0);
  const [dropRemember, setDropRemember] = useState(true);
  const [confirmForget, setConfirmForget] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<LibraryApp | null>(null);
  const [gearOpen, setGearOpen] = useState(false);
  const [disketteRunning, setDisketteRunning] = useState(false);
  const [request, setRequest] = useState<RequestSummary>({ setup: 0, apps: 0 });
  // "Not Now" on the Ask Diskette strip, until Floppy is reopened.
  const [askDismissed, setAskDismissed] = useState(false);
  const [aboutOpen, setAboutOpen] = useState(false);
  // Where to get each setup file (docs/legal-setupfiles.md, read at build time).
  const [sources, setSources] = useState<SetupSource[]>([]);
  // Set once the user opens a source: Floppy then checks Downloads whenever
  // the window comes back to the front, until setup is done.
  const [watchDownloads, setWatchDownloads] = useState(false);
  // The Report a Setup Problem dialog: which slot, or null when closed.
  const [reportSlot, setReportSlot] = useState<string | null>(null);
  const [reportKind, setReportKind] = useState<SetupReportKind>("source-broken");
  const [reportSource, setReportSource] = useState("");
  const [reportNote, setReportNote] = useState("");
  // The system backup: whether to offer Burn A CD, and the gear menu's dialog.
  const [backup, setBackup] = useState<BackupStatus | null>(null);
  const [backupOpen, setBackupOpen] = useState(false);
  const [theme, setTheme] = useState<Theme>(loadTheme);
  const gearRef = useRef<HTMLDivElement>(null);
  const [nameDraft, setNameDraft] = useState("");

  const ui = GUEST_UI[guest];
  const status = statuses.find((s) => s.os === guest);
  const guestApps = useMemo(
    () => apps.filter((a) => a.os === guest).sort((a, b) => a.name.localeCompare(b.name)),
    [apps, guest],
  );
  const selected = guestApps.find((a) => a.id === selectedId) ?? null;
  const guestDocs = useMemo(
    () => documents.filter((d) => d.os === guest).sort((a, b) => a.name.localeCompare(b.name)),
    [documents, guest],
  );
  // The documents folder's type folders, in order, each with its files.
  const docGroups = useMemo(
    () => {
      const groups = new Map<string, LibraryDoc[]>();
      for (const d of guestDocs) {
        const folder = docTypeFolder(d);
        groups.set(folder, [...(groups.get(folder) ?? []), d]);
      }
      return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
    },
    [guestDocs],
  );
  const selectedDoc = guestDocs.find((d) => d.id === selectedDocId) ?? null;
  const guestRunning = running.has(guestRunId(guest)) || apps.some((a) => a.os === guest && running.has(a.id));
  // Only the selected guest's: the setup strip and drop target show on its tab alone.
  const missingSetup = useMemo(() => missingSetupFiles(statuses, guest), [statuses, guest]);
  const setupNeeded = missingSetup.length > 0;
  const requestTotal = request.setup + request.apps;

  // Whether Diskette is running, so Floppy can offer to ask it (request.rs).
  useEffect(() => {
    let alive = true;
    const check = () =>
      invoke<boolean>("diskette_running").then(
        (r) => alive && setDisketteRunning(r),
        () => {},
      );
    void check();
    const timer = setInterval(check, DISKETTE_POLL_MS);
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, []);

  useEffect(() => {
    invoke<SetupSource[]>("setup_sources").then(setSources, () => {});
  }, []);

  // Any guest still missing setup files: while one is, a finished download
  // is worth looking for.
  const anySetupMissing = statuses.some((s) => s.os !== "dos" && (!s.system.rom || !s.system.boot));

  // Whether the system is complete and not yet backed up, kept current as setup changes.
  useEffect(() => {
    invoke<BackupStatus>("backup_status").then(setBackup, () => {});
  }, [statuses]);

  // However a real Kickstart arrived (dropped, Downloads, a files disc,
  // Choose…), say that it now replaces AROS.
  const amigaStatus = statuses.find((s) => s.os === "amiga");
  const wasOnAros = useRef(false);
  useEffect(() => {
    if (!amigaStatus) return;
    const onAros = amigaStatus.system.aros && !amigaStatus.system.rom;
    if (wasOnAros.current && amigaStatus.system.rom) {
      const which = amigaStatus.romNote ?? amigaStatus.system.rom;
      setMessage(`Floppy found a real Kickstart (${which}) and now starts the Amiga with it instead of AROS.`);
    }
    wasOnAros.current = onAros;
  }, [amigaStatus]);

  // After the user opened a source in the browser, look in Downloads each
  // time Floppy comes back to the front, quietly unless something's found.
  useEffect(() => {
    if (!watchDownloads || !anySetupMissing) return;
    const onFront = () => {
      if (document.visibilityState === "visible" && !busyRef.current) void lookInDownloads(true);
    };
    window.addEventListener("focus", onFront);
    document.addEventListener("visibilitychange", onFront);
    return () => {
      window.removeEventListener("focus", onFront);
      document.removeEventListener("visibilitychange", onFront);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watchDownloads, anySetupMissing]);

  // What a request would ask for, kept current as setup and the library change.
  useEffect(() => {
    if (!disketteRunning) return;
    invoke<RequestSummary>("request_summary").then(setRequest, () => {});
  }, [disketteRunning, apps, statuses]);
  // Basilisk II and FS-UAE use the system files, so they can't change mid-run.
  const systemInUse =
    running.has(guestRunId("mac-classic")) || running.has(guestRunId("amiga")) || apps.some((a) => a.os !== "dos" && running.has(a.id));
  // What's running, apps and guests started on their own, for the Quit strip.
  const runningItems: { id: string; os: GuestOs; name: string }[] = [
    ...GUESTS.filter((os) => running.has(guestRunId(os))).map((os) => ({ id: guestRunId(os), os, name: GUEST_LABEL[os] })),
    ...apps.filter((a) => running.has(a.id)),
  ];

  useEffect(() => applyTheme(theme), [theme]);

  // The gear menu closes on a click outside it or Escape.
  useEffect(() => {
    if (!gearOpen) return;
    function onPointerDown(e: MouseEvent) {
      if (gearRef.current && !gearRef.current.contains(e.target as Node)) setGearOpen(false);
    }
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setGearOpen(false);
    }
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [gearOpen]);

  useEffect(() => {
    if (gearOpen) invoke<FindingsSummary>("findings_summary").then(setFindings, () => setFindings(null));
    if (gearOpen) invoke<KnowledgeSummary>("knowledge_summary").then(setKnowledge, () => setKnowledge(null));
  }, [gearOpen]);

  /** Runs a gear-menu item, closing the menu first. */
  function fromGear(action: () => void) {
    setGearOpen(false);
    action();
  }

  useEffect(() => {
    setNameDraft(selected?.name ?? "");
  }, [selected?.id, selected?.name]);

  async function refresh(): Promise<LibraryApp[]> {
    invoke<AiInfo>("ai_info").then(setAi, () => {});
    const list = await invoke<LibraryApp[]>("list_apps");
    setApps(list);
    setDocuments(await invoke<LibraryDoc[]>("list_documents"));
    return list;
  }

  async function refreshStatuses() {
    setStatuses(await invoke<GuestStatus[]>("guest_statuses"));
    setTracking(await invoke<SetupTracking>("setup_tracking"));
  }

  /** Empties the ignore list (discs.rs), so the next files disc may bring those copies again. */
  async function forgetIgnored() {
    try {
      await invoke("forget_ignored_files");
      await refreshStatuses();
      setMessage("Forgot the ignored copies. The next Missing-Files List no longer asks for them to be left out.");
    } catch (e) {
      fail(e);
    }
  }

  /** Puts a system file the drives couldn't supply back on the missing-files list. */
  async function askAgain(slot: string) {
    try {
      await invoke("ask_again", { slot });
      await refreshStatuses();
      setMessage(`The next Missing-Files List asks for a ${slot} again.`);
    } catch (e) {
      fail(e);
    }
  }

  async function refreshRunning() {
    const now = new Set(await invoke<string[]>("running_apps"));
    setRunning(now);
    setQuitAsked((asked) => new Set([...asked].filter((id) => now.has(id))));
  }

  /** Asks an app's emulator to quit, and forces it the second time (commands.rs `quit_app`). */
  /** Starts a guest with no app, once none of its setup files is missing (commands.rs `start_guest`). */
  async function startGuest(os: GuestOs) {
    setError(null);
    try {
      await invoke("start_guest", { os });
      setMessage(
        os === "dos"
          ? "Starting DOS at a C:\\ prompt. Every app in the library is on drive C:."
          : os === "mac-classic"
            ? "Starting the Mac. Your apps and documents are on its Unix volume."
            : "Starting the Amiga. Your apps and documents are on its Floppy: drive.",
      );
    } catch (e) {
      fail(e);
    }
  }

  async function quitApp(app: { id: string; os: GuestOs }) {
    const force = quitAsked.has(app.id);
    setError(null);
    try {
      await invoke("quit_app", { id: app.id, force });
      if (!force) setQuitAsked((asked) => new Set(asked).add(app.id));
    } catch (e) {
      fail(e);
    }
  }

  function fail(e: unknown) {
    setMessage(null);
    setError(String(e));
  }

  function select(app: LibraryApp) {
    setGuest(app.os);
    setSelectedDocId(null);
    setSelectedId(app.id);
  }

  function selectDoc(doc: LibraryDoc) {
    setGuest(doc.os);
    setSelectedId(null);
    setSelectedDocId(doc.id);
  }

  useEffect(() => {
    invoke<HandlerInfo[]>("handler_catalog", { os: "dos" }).then(setDosHandlers, () => {});
  }, []);

  // A new "Which app was this?" question starts on the likeliest answer.
  useEffect(() => {
    setIdentifyChoice(session?.identify?.candidates[0] ?? "");
    setIdentifyVersion("");
  }, [session?.identify]);

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
        } else if (startup?.document) {
          await refresh();
          selectDoc(startup.document);
          setMessage(`Added ${startup.document.name} as ${docPath(startup.document)}.`);
        } else if (startup?.error) {
          setError(startup.error);
        }
      } catch (e) {
        fail(e);
      }
    })();
    const unlisten = listen("running-changed", () => void refreshRunning());
    // What a guest saved into its documents folder, sorted once it quit.
    const unlistenDocs = listen("documents-changed", () => void refresh());
    // Files opened with Floppy (a disc Diskette sends back, request.rs).
    // Taken once here too, for files that arrived before the window.
    const unlistenOpened = listen("files-opened", () => void openedRef.current());
    void openedRef.current();
    // Old disks macOS couldn't mount (media.rs), and copy progress.
    invoke<OldMedia[]>("old_media").then(setOldMedia, () => {});
    const unlistenMedia = listen<OldMedia[]>("old-media-changed", (e) => setOldMedia(e.payload));
    const unlistenProgress = listen<MediaProgress>("media-progress", (e) => setMediaProgress(e.payload));
    // What a DOS session saved (documents.rs), listed once DOSBox quits.
    const unlistenSession = listen<SessionReport>("session-ended", (e) => {
      setSession(e.payload);
      void refresh();
    });
    return () => {
      unlisten.then((f) => f());
      unlistenDocs.then((f) => f());
      unlistenSession.then((f) => f());
      unlistenOpened.then((f) => f());
      unlistenMedia.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, []);

  useEffect(() => {
    // While setup files are missing, the overlay has two targets; the drop
    // goes to the one under the cursor. Otherwise every drop is an app.
    const zoneAt = ({ x, y }: { x: number; y: number }): DropZone | null => {
      if (!setupNeeded) return "app";
      const el = document.elementFromPoint(x / window.devicePixelRatio, y / window.devicePixelRatio);
      return (el?.closest<HTMLElement>("[data-drop-zone]")?.dataset.dropZone as DropZone | undefined) ?? null;
    };
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "enter" || event.payload.type === "over") {
        setDragging(true);
        setDropZone(zoneAt(event.payload.position));
      } else if (event.payload.type === "leave") {
        setDragging(false);
        setDropZone(null);
      } else if (event.payload.type === "drop") {
        const zone = zoneAt(event.payload.position);
        setDragging(false);
        setDropZone(null);
        if (zone) void routeDrop(zone, event.payload.paths);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
    // Re-subscribe when the guest changes (drops import into the guest on
    // screen) and when setup is finished (the setup target goes away).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [guest, setupNeeded]);

  /** A disc Diskette made (Burn A CD) is imported whole, wherever it's dropped; anything else goes to the zone. */
  async function routeDrop(zone: DropZone, paths: string[]) {
    const discs: string[] = [];
    const rest: string[] = [];
    const backups: string[] = [];
    for (const p of paths) {
      if (await invoke<boolean>("is_backup_disc", { path: p })) backups.push(p);
      else (await invoke<boolean>("is_burn_disc", { path: p }) ? discs : rest).push(p);
    }
    // A system backup restores wherever it's dropped (backup.rs, via cd.rs).
    if (backups.length) await addSetupFiles(backups);
    for (const d of discs) await importDisc(d);
    if (!rest.length) return;
    if (zone === "setup") return addSetupFiles(rest);
    // Findings teach Floppy (learned.rs); everything else finds its place.
    const others: string[] = [];
    for (const p of rest) {
      if (await invoke<boolean>("is_findings", { path: p })) await learnFindings(p);
      else others.push(p);
    }
    if (others.length) await placePaths(others);
  }

  /**
   * Puts each dropped item where it belongs (drops.rs): straight away when
   * there's a clear winner or an answer for its kind, else it's queued for
   * the "Where does this go?" dialog.
   */
  async function placePaths(paths: string[]) {
    setError(null);
    setMessage(null);
    const ask: DropClassification[] = [];
    const failures: string[] = [];
    const done: string[] = [];
    let last: ImportedItem | null = null;
    for (const path of paths) {
      setBusy(`Looking at ${baseName(path)}`);
      try {
        const c = await invoke<DropClassification>("classify_drop", { os: guest, path });
        if (!c.decided) {
          ask.push(c);
          continue;
        }
        if (c.decided.to === "setup") {
          setBusy(null);
          await addSetupFiles([path]);
          continue;
        }
        setBusy(`Adding ${c.name}`);
        last = await invoke<ImportedItem>("import_as", { path, choice: c.decided });
        const by = c.decidedBy && c.decidedBy !== "its contents" && c.decidedBy !== "nothing else fits" ? ` (going by ${c.decidedBy})` : "";
        done.push(`${c.name} as ${describeChoice(c.decided)}${by}`);
      } catch (e) {
        failures.push(String(e));
      }
    }
    setBusy(null);
    await refresh();
    if (last?.app) select(last.app);
    else if (last?.document) selectDoc(last.document);
    if (done.length) setMessage(`Added ${done.join("; ")}.`);
    if (failures.length) setError(failures.join("\n"));
    if (ask.length) {
      setDropPick(0);
      setDropRemember(true);
      setAskDrops((queue) => [...queue, ...ask]);
    }
  }

  /** The user's answer for the first queued drop: kept (and remembered, if ticked), then carried out. */
  async function answerDrop(skip: boolean) {
    const c = askDrops[0];
    if (!c) return;
    setAskDrops((queue) => queue.slice(1));
    setDropPick(0);
    setDropRemember(true);
    if (skip) return;
    const option = c.options[dropPick];
    if (!option) return;
    try {
      await invoke("record_drop_choice", {
        signature: c.signature,
        choice: option.choice,
        offered: c.options.map((o) => o.choice),
        remember: dropRemember,
      });
      if (option.choice.to === "setup") {
        await addSetupFiles([c.path]);
        return;
      }
      setBusy(`Adding ${c.name}`);
      const item = await invoke<ImportedItem>("import_as", { path: c.path, choice: option.choice });
      setBusy(null);
      await refresh();
      if (item.app) select(item.app);
      else if (item.document) selectDoc(item.document);
      setMessage(`Added ${c.name} as ${describeChoice(option.choice)}.`);
    } catch (e) {
      setBusy(null);
      fail(e);
    }
  }

  /** Learns from another Floppy's findings (learned.rs). */
  async function learnFindings(path: string) {
    setError(null);
    setBusy(`Learning from ${baseName(path)}`);
    try {
      const l = await invoke<LearnSummary>("learn_findings", { path });
      await refresh();
      if (l.own) setMessage(`${baseName(path)} is this Floppy's own findings: nothing to learn from it.`);
      else if (l.already) setMessage(`Floppy already learned from ${baseName(path)}.`);
      else {
        const people = l.forMaintainers
          ? ` ${l.forMaintainers} note${l.forMaintainers === 1 ? "" : "s"} for Floppy's maintainers (errors, setup reports) stay in the file.`
          : "";
        const pack = l.aiVersion
          ? ` Floppy AI is now version ${l.aiVersion}.`
          : l.unsignedPack
            ? " It calls itself a Floppy AI pack but isn't signed by Floppy's maintainers, so it was learned from like anyone's findings: the AI version and where Floppy points you for setup files stay as they were."
            : "";
        const sources = l.setupSources ? ` ${l.setupSources} setup source${l.setupSources === 1 ? "" : "s"} updated.` : "";
        const kept = l.materials ? ` ${l.materials} material${l.materials === 1 ? "" : "s"} kept to read (About Floppy).` : "";
        setMessage(`Learned from ${baseName(path)}: ${describeLearned(l)}.${pack}${sources}${kept}${people}`);
      }
      if (l.skipped.length) setError(`Left out:\n${l.skipped.join("\n")}`);
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  async function revealLearned() {
    try {
      await revealItemInDir(await invoke<string>("learned_folder"));
    } catch (e) {
      fail(e);
    }
  }

  async function pickFindings() {
    const picked = await open({ multiple: true, title: "Learn from Floppy findings", filters: [{ name: "Findings", extensions: ["zip", "json"] }] });
    if (Array.isArray(picked)) for (const p of picked) await learnFindings(p);
  }

  async function forgetLearned() {
    setConfirmForget(false);
    try {
      await invoke("forget_learned");
      await refresh();
      setMessage("Forgot everything learned from findings. Your own answers and test results are kept.");
    } catch (e) {
      fail(e);
    }
  }

  /** Files opened with Floppy: a disc from Diskette, or anything Open With sent. */
  async function takeOpened() {
    try {
      const paths = await invoke<string[]>("take_opened_files");
      if (paths.length) await routeDrop("app", paths);
    } catch (e) {
      fail(e);
    }
  }
  // The listener is registered once, so it calls the latest render's function.
  const openedRef = useRef(takeOpened);
  openedRef.current = takeOpened;

  /** Imports everything on a disc Diskette made: setup files, then apps (commands.rs `import_disc`). */
  async function importDisc(path: string) {
    setError(null);
    setMessage(null);
    setBusy(`Reading ${baseName(path)}`);
    try {
      const r = await invoke<DiscImport>("import_disc", { path });
      await refresh();
      await refreshStatuses();
      const apps = r.apps.imported.length ? ` Imported ${r.apps.imported.join(", ")}.` : "";
      const already = r.apps.already.length ? ` Already in the library: ${r.apps.already.join(", ")}.` : "";
      setMessage(describeImport(r.setup) + apps + already);
      const problems = [...r.setup.skipped, ...r.apps.failed];
      if (problems.length) setError(problems.join("\n"));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  /** Hands Diskette a list of everything still missing (request.rs); its disc comes back as an opened file. */
  async function askDiskette() {
    setError(null);
    try {
      const r = await invoke<RequestSummary>("ask_diskette");
      setAskDismissed(true);
      setMessage(
        `Asked Diskette for ${describeRequest(r)}. If it finds any on your drives, it offers to Burn A CD, and the disc comes back here.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Imports one after another: each copy is disk-bound, so running them in parallel wouldn't be faster. */
  async function importPaths(paths: string[]) {
    setError(null);
    setMessage(null);
    let last: ImportedItem | null = null;
    const failures: string[] = [];
    for (const path of paths) {
      setBusy(`Importing ${baseName(path)}`);
      try {
        // An app, or else a document for the guest's documents folder.
        last = await invoke<ImportedItem>("import_item", { os: guest, path });
      } catch (e) {
        failures.push(String(e));
      }
    }
    setBusy(null);
    await refresh();
    if (last?.app) {
      select(last.app);
      setMessage(
        paths.length === 1 ? `Imported ${last.app.name} as ${guestPath(last.app)}.` : `Imported ${paths.length - failures.length} of ${paths.length}.`,
      );
    } else if (last?.document) {
      selectDoc(last.document);
      setMessage(
        paths.length === 1
          ? `Added ${last.document.name} as ${docPath(last.document)}.`
          : `Imported ${paths.length - failures.length} of ${paths.length}.`,
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

  /** Adds files as documents, whatever they are (a zip too), sorted by type. */
  async function pickDocuments() {
    const picked = await open({ multiple: true, title: `Add documents to ${guestFilePath(guest, DOCS_DIR[guest])}` });
    if (!Array.isArray(picked) || !picked.length) return;
    setError(null);
    setMessage(null);
    let last: LibraryDoc | null = null;
    const failures: string[] = [];
    for (const path of picked) {
      setBusy(`Adding ${baseName(path)}`);
      try {
        last = await invoke<LibraryDoc>("add_document", { os: guest, path });
      } catch (e) {
        failures.push(String(e));
      }
    }
    setBusy(null);
    await refresh();
    if (last) {
      selectDoc(last);
      setMessage(picked.length === 1 ? `Added ${last.name} as ${docPath(last)}.` : `Added ${picked.length - failures.length} of ${picked.length}.`);
    }
    if (failures.length) setError(failures.join("\n"));
  }

  /** Opens a document in the chosen app; what it saves is listed when DOSBox quits. */
  async function openDocument(doc: LibraryDoc, opener: Opener) {
    setError(null);
    setSession(null);
    try {
      await invoke("open_document", { id: doc.id, appId: opener.appId, program: opener.program });
      setMessage(`Opening ${doc.name} in ${opener.appName}. What it saves is listed when DOSBox quits.`);
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  async function removeDocument(doc: LibraryDoc) {
    try {
      await invoke("remove_document", { id: doc.id });
      if (selectedDocId === doc.id) setSelectedDocId(null);
      setMessage(`Removed ${doc.name}.`);
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  async function revealLibraryFile(os: GuestOs, path: string) {
    try {
      await revealItemInDir(await invoke<string>("library_file", { os, path }));
    } catch (e) {
      fail(e);
    }
  }

  async function exportLibraryFile(os: GuestOs, path: string) {
    const dest = await open({ directory: true, title: "Export to this folder" });
    if (typeof dest !== "string") return;
    try {
      const copy = await invoke<string>("export_file", { os, path, destDir: dest });
      setMessage(`Exported ${baseName(copy)}.`);
    } catch (e) {
      fail(e);
    }
  }

  async function setAppOpens(app: LibraryApp, text: string) {
    const exts = text.split(/[\s,;]+/).filter(Boolean);
    try {
      await invoke("set_app_opens", { id: app.id, exts });
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  /** Says which known app (and version) an app is; null: not confirmed yet. */
  async function setIdentity(app: LibraryApp, identity: { handler: string | null; version: string | null } | null) {
    try {
      await invoke("set_app_identity", { id: app.id, identity });
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  /** Makes an app the version its app's documents open with. */
  async function makeFavorite(app: LibraryApp) {
    try {
      await invoke("set_favorite_app", { id: app.id });
      await refresh();
      setMessage(`${app.identity?.handler ?? app.name} documents now open with ${app.name}.`);
    } catch (e) {
      fail(e);
    }
  }

  /** Answers (or puts off) the after-session "Which app was this?". */
  async function answerIdentify(confirm: boolean) {
    const ask = session?.identify;
    if (!ask || !session) return;
    if (confirm) {
      const handler = identifyChoice === OTHER_APP ? null : identifyChoice;
      try {
        await invoke("set_app_identity", {
          id: ask.appId,
          identity: { handler, version: handler ? identifyVersion.trim() || null : null },
        });
        await refresh();
        setMessage(
          handler
            ? `Noted: ${ask.appName} is ${[handler, identifyVersion.trim()].filter(Boolean).join(" ")}.`
            : `Noted: ${ask.appName} isn't one of the apps Floppy knows.`,
        );
      } catch (e) {
        fail(e);
        return;
      }
    }
    const rest = { ...session, identify: null };
    setSession(rest.changes.length || rest.verify ? rest : null);
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

  /** Points Floppy at an emulator that isn't where it looks (bundled, /Applications, ~/Applications or PATH). */
  async function locateEmulator(os: GuestOs) {
    const name = statuses.find((s) => s.os === os)?.emulator ?? "the emulator";
    // Linux programs have no extension to filter on.
    const filters = isLinux ? [] : [{ name: "Application", extensions: ["app"] }];
    const path = await open({ title: `Locate ${name}`, filters });
    if (typeof path !== "string") return;
    setError(null);
    setMessage(null);
    try {
      await invoke<GuestStatus>("locate_emulator", { os, path });
      await refreshStatuses();
      setMessage(`Floppy will start ${GUEST_LABEL[os]} apps with ${baseName(path)}.`);
    } catch (e) {
      fail(e);
    }
  }

  /** Opens where to get a setup file in the browser, then watches Downloads for it. */
  async function openSource(src: SetupSource) {
    setError(null);
    try {
      await openUrl(src.url);
      setWatchDownloads(true);
      setMessage(
        `Opened ${src.name} in your browser. When the download has finished, come back here: Floppy looks in your Downloads folder and adds it by itself.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Fills missing setup files from the Downloads folder (cd.rs `import_from_downloads`). */
  async function lookInDownloads(quiet: boolean) {
    if (!quiet) {
      setError(null);
      setMessage(null);
      setBusy("Looking in Downloads");
    }
    try {
      const r = await invoke<CdImport>("import_from_downloads");
      if (r.added.length) {
        await refreshStatuses();
        setMessage(describeImport(r));
      } else if (!quiet) {
        setMessage(
          "Nothing in your Downloads folder fills a missing setup file yet. If a download is still going, Floppy checks again when you come back to this window.",
        );
        setWatchDownloads(true);
      }
    } catch (e) {
      if (!quiet) fail(e);
    } finally {
      if (!quiet) setBusy(null);
    }
  }

  /** Burn A CD: a compressed disc image of the system's setup files and settings (backup.rs). */
  async function burnBackup() {
    setBackupOpen(false);
    const day = new Date().toISOString().slice(0, 10);
    const path = await save({
      title: "Burn A CD: back up Floppy's system",
      defaultPath: `Floppy System Backup ${day}.iso`,
      filters: [{ name: "Disc image", extensions: ["iso"] }],
    });
    if (!path) return;
    setError(null);
    setMessage(null);
    setBusy("Burning a CD");
    try {
      const r = await invoke<BackupMade>("make_backup", { path });
      setBackup(await invoke<BackupStatus>("backup_status"));
      setMessage(
        `Burned ${baseName(path)}: ${r.slots.join(", ")}, ${formatBytes(r.originalBytes)} compressed to ${formatBytes(r.discBytes)}. Keep it somewhere safe. To restore, open it with Floppy or drop it on this window.`,
      );
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  async function declineBackup() {
    try {
      await invoke("decline_backup");
      setBackup(await invoke<BackupStatus>("backup_status"));
    } catch (e) {
      fail(e);
    }
  }

  /** Opens Report a Setup Problem for `slot`, guessing the likely kind. */
  function startReport(slot: string, filled: boolean) {
    setReportSlot(slot);
    setReportKind(filled ? "didnt-work" : "source-broken");
    setReportSource("");
    setReportNote("");
  }

  /** Keeps the report for the next Export Findings (findings.rs `add_setup_report`). */
  async function sendReport() {
    if (!reportSlot) return;
    try {
      await invoke("add_setup_report", {
        slot: reportSlot,
        kind: reportKind,
        source: reportSource.trim() || null,
        note: reportNote.trim(),
      });
      setReportSlot(null);
      setMessage(
        "Thanks. The report goes out with your next Export Findings… (gear menu), so Floppy's list of sources can be fixed for everyone. Nothing is sent until you export and share it.",
      );
    } catch (e) {
      fail(e);
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

  /** Adds whatever system files are among `paths`: the files themselves, folders, zips or disc images (cd.rs `import_dropped`). */
  async function addSetupFiles(paths: string[]) {
    if (!paths.length) return;
    setError(null);
    setMessage(null);
    setBusy(`Checking ${paths.length === 1 ? baseName(paths[0]) : `${paths.length} items`}`);
    try {
      const r = await invoke<CdImport>("import_setup_files", { paths });
      await refreshStatuses();
      setMessage(describeImport(r));
      if (r.skipped.length) setError(r.skipped.join("\n"));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  async function pickSetupFiles() {
    const picked = await open({ multiple: true, title: "Choose ROMs, startup disks, or zips or disc images holding them" });
    if (Array.isArray(picked)) await addSetupFiles(picked);
  }

  /** Records whether the app opened the document's file type correctly (verify.rs). */
  async function answerVerification(worked: boolean) {
    if (!session?.verify) return;
    try {
      await invoke("record_verification", { pending: session.verify, worked, note: verifyNote.trim() || null });
      setSession({ ...session, verify: null });
      setVerifyNote("");
      await refresh();
      setMessage(
        `Recorded that ${session.verify.appName} ${worked ? "opened" : "didn't open"} ${session.verify.fileType} files correctly.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Saves what Floppy has learned as a zip, for scripts/merge-findings.py. Nothing leaves otherwise. */
  async function exportFindings() {
    const date = new Date().toISOString().slice(0, 10);
    const path = await save({
      title: "Export Findings",
      defaultPath: `Floppy findings ${date}.zip`,
      filters: [{ name: "Zip", extensions: ["zip"] }],
    });
    if (!path) return;
    try {
      const n = await invoke<FindingsSummary>("export_findings", { path });
      setMessage(
        `Saved ${baseName(path)}: ${describeFindings(n)}. It holds no documents, files or file names. Drop it on another Floppy to teach it, or send it to Floppy's maintainers so every Floppy learns it from the next release.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Saves the wanted-apps list (handlers.rs): old apps that open old files, for a disc maker to gather. */
  async function saveWantedApps() {
    const path = await save({
      title: "Save the wanted-apps list",
      defaultPath: "Floppy wants apps.txt",
      filters: [{ name: "Text", extensions: ["txt"] }],
    });
    if (!path) return;
    setError(null);
    try {
      const count = await invoke<number>("write_wanted_apps", { path });
      setMessage(
        count === 0
          ? "Every app Floppy knows opens old files is already in the library."
          : `Saved ${baseName(path)}, asking for ${count} app files. Gather them into a disc image or folder (Diskette's Burn A CD does this), then use Import Apps Disc.`,
      );
    } catch (e) {
      fail(e);
    }
  }

  /** Imports the old apps on a disc made from the wanted-apps list. */
  async function importAppsDisc(directory: boolean) {
    const path = await open(
      directory
        ? { directory: true, title: "Import a folder of gathered apps" }
        : { title: "Import an apps disc", filters: [{ name: "Disc image", extensions: ["iso", "cdr", "dmg", "toast"] }] },
    );
    if (typeof path !== "string") return;
    setError(null);
    setMessage(null);
    setBusy(`Reading ${baseName(path)}`);
    try {
      const r = await invoke<{ imported: string[]; already: string[]; failed: string[] }>("import_apps_disc", { path });
      await refresh();
      const imported = r.imported.length ? `Imported ${r.imported.join(", ")}.` : "Found no new apps.";
      const already = r.already.length ? ` Already in the library: ${r.already.join(", ")}.` : "";
      setMessage(imported + already);
      if (r.failed.length) setError(r.failed.join("\n"));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
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
      setMessage(describeImport(r));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
    }
  }

  /**
   * Copies an old disk into the library (macOS asks for the password to
   * read it) and imports it into the guest its contents belong to, then
   * opens it there when that guest is ready and free.
   */
  async function copyMedia(m: OldMedia) {
    setError(null);
    setMessage(null);
    setBusy(`Copying ${m.name}`);
    setMediaProgress({ device: m.device, done: 0, total: m.size });
    try {
      const app = await invoke<LibraryApp>("copy_old_media", { device: m.device });
      const list = await refresh();
      await refreshStatuses();
      select(app);
      const s = (await invoke<GuestStatus[]>("guest_statuses")).find((x) => x.os === app.os);
      const guestBusy = list.some((a) => a.os === app.os && running.has(a.id));
      const copied = `Copied ${m.name} as ${guestPath(app)}.`;
      if (s && !s.blocker && !guestBusy && app.program) {
        await invoke("launch_app", { id: app.id, promptOnly: false });
        setMessage(`${copied} Opening it in ${GUEST_LABEL[app.os]}.`);
      } else if (s?.blocker) {
        setMessage(`${copied} It opens once ${GUEST_LABEL[app.os]} is set up: ${s.blocker}`);
      } else {
        setMessage(`${copied} Launch it when the ${GUEST_LABEL[app.os]} running now has quit.`);
      }
    } catch (e) {
      fail(e);
    } finally {
      setBusy(null);
      setMediaProgress(null);
    }
  }

  async function ignoreMedia(m: OldMedia) {
    await invoke("dismiss_media", { device: m.device }).catch(fail);
  }

  async function setModel(model: string) {
    try {
      await invoke("set_guest_model", { os: guest, model });
      await refreshStatuses();
    } catch (e) {
      fail(e);
    }
  }

  /** Turns the Amiga's built-in AROS replacement Kickstart on or off (commands.rs `set_aros`). */
  async function useAros(on: boolean) {
    setError(null);
    try {
      await invoke<GuestStatus>("set_aros", { on });
      await refreshStatuses();
      setMessage(
        on
          ? "The Amiga starts with the free AROS Kickstart for now. When Floppy finds a real Kickstart ROM (dropped here, in Downloads, or on a files disc) it switches to it by itself."
          : "Stopped using AROS. Add a Kickstart ROM to start the Amiga.",
      );
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

  /** Keeps what went wrong running an app, for the user and Export Findings. */
  async function setAppErrors(app: LibraryApp, errors: string) {
    if (errors.trim() === app.errors) return;
    try {
      await invoke("set_app_errors", { id: app.id, errors });
      await refresh();
    } catch (e) {
      fail(e);
    }
  }

  /** Whether an app Floppy doesn't know shares its errors in Export Findings. */
  async function setShareErrors(app: LibraryApp, share: boolean) {
    try {
      await invoke("set_app_share_errors", { id: app.id, share });
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
  const basiliskStatus = statuses.find((s) => s.os === "mac-classic");
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
            {ai && (
              <span
                className="version-tag ai-version"
                title={
                  ai.fromPack
                    ? `Floppy AI ${ai.version} (${ai.date}), learned from a knowledge pack. This build knows AI ${ai.builtin}.`
                    : `Floppy AI ${ai.version}${ai.date ? ` (${ai.date})` : ""}: what Floppy knows about old files, apps and setup.${ai.learnedFrom ? ` Plus what it learned from ${ai.learnedFrom} findings file${ai.learnedFrom === 1 ? "" : "s"}.` : ""}`
                }
              >
                AI {ai.version}
                {ai.fromPack || ai.learnedFrom ? "+" : ""}
              </span>
            )}
          </h1>
          <p>Run the old apps your files need, in the OS they were made for.</p>
        </div>
        <div className="header-actions">
          <span className={`status-pill${status && !status.found ? " warn" : ""}`}>{emulatorLabel(status)}</span>
          <div className="gear-menu-wrap" ref={gearRef}>
            <button
              type="button"
              className="small icontext-btn gear-btn"
              data-testid="gear-button"
              title="Settings & about"
              aria-label="Settings & about"
              aria-haspopup="true"
              aria-expanded={gearOpen}
              onClick={() => setGearOpen((v) => !v)}
            >
              <GearIcon width={16} height={16} />
            </button>
            <div className={`gear-menu${gearOpen ? " open" : ""}`}>
              <label className="menu-item menu-item-checkbox">
                <input
                  type="checkbox"
                  data-testid="ansiapps-theme-toggle"
                  checked={theme === "ansiapps"}
                  onChange={(e) => setTheme(e.currentTarget.checked ? "ansiapps" : "modern")}
                />
                <span>ANSIapps theme (old-school DOS look)</span>
              </label>
              <button
                type="button"
                className="menu-item"
                disabled={!!busy}
                title={basiliskTitle(basiliskStatus)}
                onClick={() => fromGear(() => void locateEmulator("mac-classic"))}
              >
                <FolderIcon />
                <span>Locate Basilisk II…</span>
              </button>
              <div className="menu-sep" />
              <div className="menu-note">Old apps you own that open old files</div>
              <button type="button" className="menu-item" disabled={!!busy} onClick={() => fromGear(() => void saveWantedApps())}>
                <ExportIcon />
                <span>Save Wanted-Apps List…</span>
              </button>
              <button type="button" className="menu-item" disabled={!!busy} onClick={() => fromGear(() => void importAppsDisc(false))}>
                <DiscIcon />
                <span>Import Apps Disc…</span>
              </button>
              <button type="button" className="menu-item" disabled={!!busy} onClick={() => fromGear(() => void importAppsDisc(true))}>
                <FolderIcon />
                <span>Import Apps Folder…</span>
              </button>
              <button
                type="button"
                className="menu-item"
                disabled={!!busy || !disketteRunning || requestTotal === 0}
                title={
                  !disketteRunning
                    ? "Open Diskette first: it looks through your cataloged drives"
                    : requestTotal === 0
                      ? "Floppy has everything it can ask for"
                      : undefined
                }
                onClick={() => fromGear(() => void askDiskette())}
              >
                <DiscIcon />
                <span>Ask Diskette for Missing Files</span>
              </button>
              <div className="menu-sep" />
              <button
                type="button"
                className="menu-item"
                disabled={!findings || findingsTotal(findings) === 0}
                title={
                  findings && findingsTotal(findings) > 0
                    ? `New since the last export: ${describeFindings(findings)}, as a zip to teach another Floppy or Floppy's maintainers. No documents, files or file names.${lastExported(findings)}`
                    : findings?.lastExported
                      ? `Nothing new since the last export.${lastExported(findings)}`
                      : "Nothing to share yet: test results, apps you identified, file types you added, unlisted ROMs and apps' errors show up here."
                }
                onClick={() => fromGear(() => void exportFindings())}
              >
                <ExportIcon />
                <span>Export Findings{findings && findingsTotal(findings) > 0 ? ` (${findingsTotal(findings)})` : ""}…</span>
              </button>
              <button
                type="button"
                className="menu-item"
                disabled={!!busy}
                title={
                  knowledge?.sources
                    ? `Learned from ${knowledge.sources} findings file${knowledge.sources === 1 ? "" : "s"} so far: ${describeLearned(knowledge)}. Add another's, or drop one on the window.`
                    : "Teach this Floppy what another Floppy learned: pick its Export Findings zip, or drop it on the window."
                }
                onClick={() => fromGear(() => void pickFindings())}
              >
                <FolderIcon />
                <span>Learn from Findings…</span>
              </button>
              {!!knowledge?.sources && (
                <button
                  type="button"
                  className="menu-item"
                  title={`Forget ${describeLearned(knowledge)}, learned from ${knowledge.sources} findings file${knowledge.sources === 1 ? "" : "s"}.`}
                  onClick={() => fromGear(() => setConfirmForget(true))}
                >
                  <TrashIcon />
                  <span>Forget What Was Learned…</span>
                </button>
              )}
              <div className="menu-sep" />
              <div className="menu-note">Your setup files</div>
              <button
                type="button"
                className="menu-item"
                disabled={!!busy || !backup?.slots.length}
                title={
                  backup?.slots.length
                    ? "Burn A CD: one compressed disc image of your setup files and settings, to restore Floppy in one step."
                    : "Nothing to back up yet: add a Mac or Amiga setup file first."
                }
                onClick={() => fromGear(() => setBackupOpen(true))}
              >
                <DiscIcon />
                <span>Backup Floppy System…</span>
              </button>
              <div className="menu-sep" />
              <button type="button" className="menu-item" onClick={() => fromGear(() => setAboutOpen(true))}>
                <InfoIcon />
                <span>About Floppy</span>
              </button>
            </div>
          </div>
        </div>
      </header>

      {error && <div className="error">{error}</div>}
      {message && <div className="status-message">{message}</div>}
      {session && (
        <div className="session-report">
          <div className="session-report-head">
            <strong>
              {session.appName}
              {session.document ? ` (${session.document})` : ""}
              {session.changes.length
                ? ` saved ${session.changes.length} ${session.changes.length === 1 ? "file" : "files"}`
                : " didn't save anything"}
            </strong>
            <button type="button" className="small" onClick={() => setSession(null)}>
              Done
            </button>
          </div>
          {session.verify && (
            <div className="verify-row">
              <span>
                Did {session.verify.appName} open {session.verify.document} correctly?
              </span>
              <input
                type="text"
                placeholder="Note (optional)"
                value={verifyNote}
                onChange={(e) => setVerifyNote(e.target.value)}
              />
              <button type="button" className="small primary" onClick={() => void answerVerification(true)}>
                Worked
              </button>
              <button type="button" className="small" onClick={() => void answerVerification(false)}>
                Didn't Work
              </button>
            </div>
          )}
          <ul>
            {session.changes.map((c) => (
              <li key={c.path}>
                <span className="session-file">
                  {c.name}
                  <span className="row-meta">
                    {c.new ? "New" : "Changed"} · {`C:\\${c.path.replace(/\//g, "\\")}`}
                  </span>
                </span>
                <button type="button" className="small" onClick={() => void revealLibraryFile(session.os, c.path)}>
                  {SHOW_IN_FILES}
                </button>
                <button type="button" className="small" onClick={() => void exportLibraryFile(session.os, c.path)}>
                  Export…
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}

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
                setSelectedDocId(null);
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
        {setupNeeded && (
          <button
            type="button"
            className="icontext-btn"
            onClick={() => void importFilesDisc(false)}
            disabled={!!busy || systemInUse}
            title="Add ROMs and startup disks from a disc image of gathered system files"
          >
            <span className="btn-icon">
              <ChipIcon />
            </span>
            Import Files Disc…
          </button>
        )}
      </div>

      {backup?.offer && (
        <div className="setup-drop media-offer">
          <span className="setup-drop-icon">
            <DiscIcon />
          </span>
          <p className="setup-drop-text">
            <strong>Floppy's system is complete.</strong> Burn A CD to back it up: one compressed disc image of your{" "}
            {backup.slots.join(", ")} and their settings, so a new computer or a reinstall is set up again in one step.
          </p>
          <button type="button" className="small primary" onClick={() => void burnBackup()} disabled={!!busy}>
            Burn A CD…
          </button>
          <button type="button" className="small" onClick={() => void declineBackup()} disabled={!!busy}>
            Not Now
          </button>
        </div>
      )}

      {status && !status.found && (
        <div className="setup-drop">
          <span className="setup-drop-icon">
            <ChipIcon />
          </span>
          <p className="setup-drop-text">
            <strong>{GUEST_LABEL[guest]} apps can't start yet.</strong> {status.blocker} If you have a copy somewhere
            else, point Floppy at it.
          </p>
          <button type="button" className="small primary" onClick={() => void locateEmulator(guest)} disabled={!!busy}>
            Locate {status.emulator}…
          </button>
        </div>
      )}

      {setupNeeded && (
        <div className="setup-drop">
          <span className="setup-drop-icon">
            <ChipIcon />
          </span>
          <p className="setup-drop-text">
            {guest === "amiga" && status?.system.aros && !status.system.rom && (
              <>
                <strong>Running on the free AROS Kickstart for now.</strong> A real Kickstart ROM runs far more Amiga
                software, and Floppy switches to one by itself as soon as it finds it.{" "}
              </>
            )}
            <strong>Setup files needed:</strong> {missingSetup.join(", ")}. Drop them onto this window as files, folders,
            zips or disc images, or download them (see where below) and Floppy picks them up from your Downloads
            folder. It recognizes each one by its contents, whatever it's called.
          </p>
          <button type="button" className="small" onClick={() => void lookInDownloads(false)} disabled={!!busy || systemInUse}>
            Look in Downloads
          </button>
          <button type="button" className="small" onClick={() => void pickSetupFiles()} disabled={!!busy || systemInUse}>
            Choose Files…
          </button>
        </div>
      )}

      {disketteRunning && requestTotal > 0 && !askDismissed && (
        <div className="setup-drop media-offer">
          <span className="setup-drop-icon">
            <DiscIcon />
          </span>
          <p className="setup-drop-text">
            <strong>Diskette is running.</strong> Ask it for {describeRequest(request)} Floppy still needs? It looks
            through your cataloged drives, and if it finds any, offers to Burn A CD and sends the disc back here.
          </p>
          <button type="button" className="small primary" onClick={() => void askDiskette()} disabled={!!busy}>
            Ask Diskette
          </button>
          <button type="button" className="small" onClick={() => setAskDismissed(true)}>
            Not Now
          </button>
        </div>
      )}

      {oldMedia.map((m) => (
        <div className="setup-drop media-offer" key={m.device}>
          <span className="setup-drop-icon">
            <ChipIcon />
          </span>
          <p className="setup-drop-text">
            <strong>
              {m.name}
              {m.diskImage ? " (disk image)" : ""}, {formatBytes(m.size)}
            </strong>{" "}
            is attached, but macOS can't open it.{" "}
            {m.hint ? `It looks like a ${GUEST_LABEL[m.hint]} disk. ` : ""}
            Floppy can copy it and open the copy in {m.hint ? GUEST_LABEL[m.hint] : "the emulator it belongs to"}. macOS
            asks for your password to read it, and the original isn't changed.
          </p>
          <button type="button" className="small primary" onClick={() => void copyMedia(m)} disabled={!!busy}>
            Copy and Open
          </button>
          <button type="button" className="small" onClick={() => void ignoreMedia(m)} disabled={!!busy}>
            Ignore
          </button>
        </div>
      ))}

      {runningItems.map((a) => (
          <div className="setup-drop running-offer" key={`running-${a.id}`}>
            <span className="setup-drop-icon">{GUEST_UI[a.os].icon()}</span>
            <p className="setup-drop-text">
              <strong>{a.name}</strong> is running in {EMULATOR_LABEL[a.os]}. {quitHint(a.os, quitAsked.has(a.id))}
            </p>
            <button type="button" className={`small${quitAsked.has(a.id) ? " danger" : ""}`} onClick={() => void quitApp(a)}>
              {quitAsked.has(a.id) ? "Force Quit" : `Quit ${EMULATOR_LABEL[a.os]}`}
            </button>
          </div>
        ))}

      {busy && (
        <div className="scan-status-row">
          <span className="scan-status-label">
            {busy}
            {mediaProgress && mediaProgress.done > 0
              ? ` (${formatBytes(mediaProgress.done)} of ${formatBytes(mediaProgress.total)})`
              : "…"}
          </span>
          {mediaProgress && mediaProgress.done > 0 ? (
            <ProgressBar value={mediaProgress.done / Math.max(1, mediaProgress.total)} label={busy} />
          ) : (
            <ProgressBar indeterminate label={busy} />
          )}
        </div>
      )}

      <div className="layout">
        <div className="layout-column">
        <section className="panel">
          <h2>
            <span className="section-icon">{ui.icon()}</span>
            {GUEST_LABEL[guest]} Library
          </h2>
          {guest === "dos" && status?.complete && status.found && (
            <div className="detail-actions guest-start">
              <button
                type="button"
                className="primary icontext-btn"
                disabled={!!busy || running.has(guestRunId("dos"))}
                onClick={() => void startGuest("dos")}
                title="DOSBox at a C:\ prompt, with every app in the library on drive C:"
              >
                <span className="btn-icon">
                  <PromptIcon />
                </span>
                {running.has(guestRunId("dos")) ? "DOS Is Running" : "Start DOS"}
              </button>
            </div>
          )}
          <p className="desc">{ui.libraryDesc}</p>
          {guest !== "dos" && status && (
            <SystemSetup
              status={status}
              disabled={!!busy || guestRunning}
              onChoose={(kind, directory) => void chooseSystemFile(kind, directory)}
              onModel={(m) => void setModel(m)}
              onSaveList={() => void saveMissingList()}
              onImportFolder={() => void importFilesDisc(true)}
              tracking={tracking}
              onForgetIgnored={() => void forgetIgnored()}
              onAskAgain={(slot) => void askAgain(slot)}
              sources={sources}
              onOpenSource={(src) => void openSource(src)}
              onReport={startReport}
              onAros={(on) => void useAros(on)}
              onStart={() => void startGuest(guest)}
              started={running.has(guestRunId(guest))}
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
                    onClick={() => select(app)}
                    onDoubleClick={() => app.program && !launchBlocked && void launch(app, false)}
                  >
                    <span className="row-icon">{ui.icon()}</span>
                    <span className="row-main">
                      <span className="row-name">{app.name}</span>
                      <span className="row-meta">
                        {guestPath(app, app.program)}
                        {app.os === "dos" && identityLabel(app, dosHandlers) ? ` · ${identityLabel(app, dosHandlers)}` : ""}
                      </span>
                    </span>
                    {app.favorite && (
                      <span className="status-pill" title={`${app.identity?.handler} documents open with this version`}>
                        Favorite
                      </span>
                    )}
                    {running.has(app.id) && <span className="status-pill running">Running</span>}
                  </button>
                </li>
              ))}
            </ul>
          )}

        </section>

        <section className="panel docs-panel">
          <h2>
            <span className="section-icon">
              <FileIcon />
            </span>
            {GUEST_LABEL[guest]} Docs
          </h2>
          <p className="desc">{ui.docsDesc}</p>
          {docGroups.length === 0 ? (
            <p className="empty">
              No documents yet. Add old files here, or drop them on this window, and Floppy sorts them by type.
            </p>
          ) : (
            docGroups.map(([folder, docs]) => (
              <Fragment key={folder}>
                <h3 className="list-heading">
                  <span className="section-icon">
                    <FolderIcon />
                  </span>
                  {folder}{" "}
                  <span className="list-heading-meta">
                    {guestFilePath(guest, `${DOCS_DIR[guest]}/${folder}`)} · {docs.length}
                  </span>
                </h3>
                <ul className="app-list">
                  {docs.map((doc) => (
                    <li key={doc.id}>
                      <button
                        type="button"
                        className={`app-row${doc.id === selectedDocId ? " selected" : ""}`}
                        onClick={() => selectDoc(doc)}
                      >
                        <span className="row-icon">
                          <FileIcon />
                        </span>
                        <span className="row-main">
                          <span className="row-name">{doc.name}</span>
                          {/* Its name in the guest, when that isn't the name it came with. */}
                          {baseName(doc.file) !== doc.name && <span className="row-meta">{baseName(doc.file)}</span>}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </Fragment>
            ))
          )}
          <div className="detail-actions">
            <button type="button" className="icontext-btn" onClick={() => void pickDocuments()} disabled={!!busy}>
              <span className="btn-icon">
                <FileIcon />
              </span>
              Add Documents…
            </button>
          </div>
        </section>
        </div>

        <section className="panel">
          {selectedDoc ? (
            <DocumentDetails
              doc={selectedDoc}
              apps={apps}
              busy={!!busy}
              onOpen={(o) => void openDocument(selectedDoc, o)}
              onReveal={() => void revealLibraryFile(selectedDoc.os, selectedDoc.file)}
              onExport={() => void exportLibraryFile(selectedDoc.os, selectedDoc.file)}
              onRemove={() => void removeDocument(selectedDoc)}
              canStart={!!status?.complete && !!status.found && !status.blocker}
              startBlocked={!!busy || guestRunning}
              onStart={() => void startGuest(selectedDoc.os)}
            />
          ) : !selected ? (
            <p className="empty">
              {guest === "dos"
                ? "Select an app to launch it or change what it opens, or a document to open it in its app."
                : "Select an app to launch it, or a document to see where it is in the guest."}
            </p>
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
              {selected.os === "dos" && (
                <IdentityFields
                  app={selected}
                  apps={apps}
                  handlers={dosHandlers}
                  onIdentity={(identity) => void setIdentity(selected, identity)}
                  onFavorite={() => void makeFavorite(selected)}
                />
              )}
              {selected.os === "dos" && (
                <label className="field">
                  <span className="field-label">Also opens</span>
                  <input
                    key={`${selected.id}-${selected.opens.join(",")}`}
                    type="text"
                    placeholder="Extensions, e.g. TXT, DOC"
                    defaultValue={selected.opens.join(", ")}
                    onBlur={(e) => void setAppOpens(selected, e.currentTarget.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") e.currentTarget.blur();
                    }}
                  />
                </label>
              )}
              <label className="field">
                <span className="field-label">Errors</span>
                <textarea
                  key={`${selected.id}-${selected.errors}`}
                  rows={2}
                  maxLength={4000}
                  placeholder="What goes wrong running it, if anything"
                  defaultValue={selected.errors}
                  onBlur={(e) => void setAppErrors(selected, e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") {
                      e.currentTarget.value = selected.errors;
                      e.currentTarget.blur();
                    }
                  }}
                />
                {selected.errors && selected.identity?.handler && (
                  <span className="system-note">Goes in Export Findings, so Floppy's maintainers can look into it.</span>
                )}
              </label>
              {selected.errors && !selected.identity?.handler && (
                <label className="field-check">
                  <input
                    type="checkbox"
                    checked={selected.shareErrors}
                    onChange={(e) => void setShareErrors(selected, e.currentTarget.checked)}
                  />
                  <span>
                    Share in Export Findings, to help Floppy improve. It goes with this app's name ("{selected.name}") and its
                    program's name and fingerprint.
                    {selected.os === "dos" && " Or set Is, if it's one of the known apps."}
                  </span>
                </label>
              )}
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
                  {SHOW_IN_FILES}
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
              <LaunchHint
                app={selected}
                blocker={blocker}
                guestRunning={guest !== "dos" && guestRunning}
                onAros={guest === "amiga" && !!status?.system.aros && !status.system.rom}
              />
            </div>
          )}
        </section>
      </div>

      <Dialog
        open={!!askDrops[0]}
        onClose={() => void answerDrop(true)}
        title={askDrops[0] ? `Where does ${askDrops[0].name} go?` : "Where does this go?"}
        actions={
          <>
            <button type="button" onClick={() => void answerDrop(true)}>
              Skip
            </button>
            <button type="button" className="primary" onClick={() => void answerDrop(false)}>
              Add
            </button>
          </>
        }
      >
        {askDrops[0] && (
          <>
            <p>
              Floppy can't tell for sure, so you decide.
              {askDrops.length > 1 ? ` ${askDrops.length - 1} more after this one.` : ""}
            </p>
            <div className="drop-options" role="radiogroup" aria-label="Where it goes">
              {askDrops[0].options.map((o, i) => (
                <label key={`${o.choice.to}-${o.choice.os}`} className="drop-option">
                  <input type="radio" name="drop-choice" checked={dropPick === i} onChange={() => setDropPick(i)} />
                  <span>
                    <span className="drop-option-label">{o.label}</span>
                    <span className="drop-option-why">{o.why}</span>
                  </span>
                </label>
              ))}
            </div>
            <label className="field-check drop-remember">
              <input type="checkbox" checked={dropRemember} onChange={(e) => setDropRemember(e.target.checked)} />
              <span>Do the same for other {askDrops[0].sameFor} without asking</span>
            </label>
            <p className="system-note">
              Your answer also goes in your next Export Findings, so Floppy learns where these go.
            </p>
          </>
        )}
      </Dialog>

      <Dialog
        open={confirmForget}
        onClose={() => setConfirmForget(false)}
        title="Forget what was learned?"
        actions={
          <>
            <button type="button" onClick={() => setConfirmForget(false)}>
              Cancel
            </button>
            <button type="button" className="danger" onClick={() => void forgetLearned()}>
              Forget
            </button>
          </>
        }
      >
        <p>
          Floppy forgets {knowledge ? describeLearned(knowledge) : "what it learned"} from findings dropped on it. What it was
          built knowing, your own answers and your test results stay. Drop the findings again to relearn them.
        </p>
      </Dialog>

      <Dialog
        open={!!session?.identify}
        onClose={() => void answerIdentify(false)}
        title="Which app was this?"
        actions={
          <>
            <button type="button" onClick={() => void answerIdentify(false)}>
              Ask Later
            </button>
            <button type="button" className="primary" onClick={() => void answerIdentify(true)}>
              Confirm
            </button>
          </>
        }
      >
        {session?.identify && (
          <>
            <p>
              {session.identify.appName} ran {session.identify.program}.{" "}
              {session.identify.candidates.length > 1
                ? "Several apps have a program by that name"
                : "Other apps can have a program by that name too, and versions share it"}
              , so Floppy asks once. Documents then open in the right app, and test results count for it.
            </p>
            <label className="field">
              <span className="field-label">It was</span>
              <select value={identifyChoice} onChange={(e) => setIdentifyChoice(e.target.value)}>
                {session.identify.candidates.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
                <option value={OTHER_APP}>Something else</option>
              </select>
            </label>
            {identifyChoice !== OTHER_APP && (
              <label className="field">
                <span className="field-label">Version (if you know it)</span>
                <input
                  type="text"
                  list="identify-versions"
                  placeholder="e.g. 5.1"
                  value={identifyVersion}
                  onChange={(e) => setIdentifyVersion(e.target.value)}
                />
                <datalist id="identify-versions">
                  {dosHandlers
                    .find((h) => h.name === identifyChoice)
                    ?.versions.map((v) => <option key={v} value={v} />)}
                </datalist>
              </label>
            )}
          </>
        )}
      </Dialog>

      <Dialog
        open={!!reportSlot}
        onClose={() => setReportSlot(null)}
        title="Report a setup problem"
        actions={
          <>
            <button type="button" onClick={() => setReportSlot(null)}>
              Cancel
            </button>
            <button
              type="button"
              className="primary"
              disabled={!reportNote.trim() && !reportSource.trim()}
              onClick={() => void sendReport()}
            >
              Keep for Export
            </button>
          </>
        }
      >
        <p>
          Floppy keeps this until you choose Export Findings… in the gear menu, and never sends it by itself. It holds what
          you write here and, for a file that didn't work, what Floppy recognized the file as (its type, size and
          fingerprint), never its name or where it is.
        </p>
        <label className="field">
          <span className="field-label">About</span>
          <select value={reportSlot ?? ""} onChange={(e) => setReportSlot(e.target.value)}>
            {(guest === "amiga" ? ["Kickstart ROM", "Workbench disk"] : ["Mac ROM", "Mac startup disk"]).map((slot) => (
              <option key={slot} value={slot}>
                {slot}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field-label">What happened</span>
          <select value={reportKind} onChange={(e) => setReportKind(e.target.value as SetupReportKind)}>
            {(Object.keys(REPORT_KIND_LABEL) as SetupReportKind[]).map((k) => (
              <option key={k} value={k}>
                {REPORT_KIND_LABEL[k]}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field-label">{reportKind === "better-source" ? "Where it is (a link)" : "Which source (optional)"}</span>
          <input
            type="text"
            list="report-sources"
            placeholder="https://…"
            value={reportSource}
            onChange={(e) => setReportSource(e.target.value)}
          />
          <datalist id="report-sources">
            {sources
              .filter((src) => src.slot === reportSlot)
              .map((src) => (
                <option key={src.url} value={src.url} />
              ))}
          </datalist>
        </label>
        <label className="field">
          <span className="field-label">Details</span>
          <textarea
            rows={3}
            placeholder={
              reportKind === "didnt-work"
                ? "e.g. the Mac shows a flashing question mark"
                : reportKind === "better-source"
                  ? "e.g. free, and includes every version"
                  : "e.g. the page says the item was removed"
            }
            value={reportNote}
            onChange={(e) => setReportNote(e.target.value)}
          />
        </label>
      </Dialog>

      <Dialog
        open={backupOpen}
        onClose={() => setBackupOpen(false)}
        title="Backup Floppy System"
        actions={
          <>
            <button type="button" onClick={() => setBackupOpen(false)}>
              Cancel
            </button>
            <button type="button" className="primary" disabled={!!busy} onClick={() => void burnBackup()}>
              Burn A CD…
            </button>
          </>
        }
      >
        <p>
          Burns a CD: one compressed disc image (.iso) of everything that makes Floppy work, the same backup Floppy offers
          when its system is complete. It holds your {backup?.slots.length ? backup.slots.join(", ") : "setup files"}
          {backup?.slots.includes("Kickstart ROM") ? ", and settings such as the Amiga model" : ""}. Your apps and documents
          aren't included.
        </p>
        <p>
          To restore, on this computer or a new one, open the disc image with Floppy or drop it on its window. Floppy
          checks every file and fills in whatever isn't set up yet, and never replaces what is.
          {backup?.lastBackup
            ? ` Last backed up ${new Date(backup.lastBackup * 1000).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" })}.`
            : ""}
        </p>
        <p>These are your own copies of copyrighted system software: keep the disc for yourself.</p>
      </Dialog>

      <Dialog open={aboutOpen} onClose={() => setAboutOpen(false)} title={`About Floppy v${__APP_VERSION__}`}>
        <p>Run the old apps your files need, in the OS they were made for.</p>
        <ul className="about-emulators">
          {statuses.map((s) => (
            <li key={s.os}>
              <span>{GUEST_LABEL[s.os]}</span>
              <span className={s.found ? "" : "warn"}>{emulatorLabel(s)}</span>
            </li>
          ))}
        </ul>
        <p>
          Floppy is free, always, and published only at ansiapps.com. It's free software under the GNU GPL, version 2 or
          later. The emulators run as separate programs under their own
          licenses. Floppy works offline: no network, no telemetry, no accounts. It never includes ROMs, operating
          systems or apps; you bring your own.
        </p>
        <h3 className="about-section-title">Floppy AI {ai?.version ?? ""}</h3>
        <p>
          What Floppy knows about old files, apps and setup{ai?.date ? `, as of ${ai.date}` : ""}
          {ai?.fromPack ? `, from a knowledge pack (this build knows AI ${ai.builtin})` : ""}. It grows from findings: drop
          a knowledge pack from ansiapps.com, or anyone's Export Findings zip, on this window to update it without a new
          release.
          {ai?.learnedFrom ? ` Learned from ${ai.learnedFrom} findings file${ai.learnedFrom === 1 ? "" : "s"} so far.` : ""}
        </p>
        {!!knowledge?.materials && (
          <div className="detail-actions">
            <button type="button" className="icontext-btn" onClick={() => void revealLearned()}>
              <span className="btn-icon">
                <FolderIcon />
              </span>
              Show Materials ({knowledge.materials})
            </button>
          </div>
        )}
        <h3 className="about-section-title">Credits</h3>
        <p data-testid="about-credits">
          The ANSIapps theme's font is IBM VGA 8x16 from The Ultimate Oldschool PC Font Pack by VileR
          (int10h.org/oldschool-pc-fonts), licensed under CC BY-SA 4.0 and included unmodified.
        </p>
      </Dialog>

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

      <div className={`drop-overlay${dragging ? " show" : ""}${setupNeeded ? " split" : ""}`}>
        <div className={`drop-box${dropZone === "app" ? " active" : ""}`} data-drop-zone="app">
          Drop to import into {GUEST_LABEL[guest]}
          <small>{ui.dropHint}</small>
        </div>
        {setupNeeded && (
          <div className={`drop-box${dropZone === "setup" ? " active" : ""}`} data-drop-zone="setup">
            Drop to add setup files
            <small>
              Still needed: {missingSetup.join(", ")}. Files, folders, zips or disc images, under any name.
            </small>
          </div>
        )}
      </div>
    </main>
  );
}

/** The "Is" menu's value for an app that isn't one of the known ones. */
const OTHER_APP = "\u0000other";

/** What a DOS app is (which known app, which version), and whether its app's documents open with it. */
function IdentityFields({
  app,
  apps,
  handlers,
  onIdentity,
  onFavorite,
}: {
  app: LibraryApp;
  apps: LibraryApp[];
  handlers: HandlerInfo[];
  onIdentity: (identity: { handler: string | null; version: string | null } | null) => void;
  onFavorite: () => void;
}) {
  const id = app.identity;
  const handler = id?.handler ?? null;
  const candidates = handlerCandidates(app, handlers);
  const others = handlers.filter((h) => !candidates.includes(h));
  const versions = handlers.find((h) => h.name === handler)?.versions ?? [];
  const siblings = handler ? apps.filter((a) => a.os === app.os && a.identity?.handler === handler) : [];
  const note = !id
    ? candidates.length
      ? "Floppy is going by its program names only. Say which app this is, so documents open in the right one and test results count for it."
      : null
    : id.by === "hash"
      ? "Recognized: one of its programs matches a known version exactly."
      : id.by === "learned"
        ? "Going by findings someone shared: one of its programs matches a version they identified. Floppy hasn't checked it itself, so correct it here if it's wrong."
      : !handler
        ? "Not one of the known apps, so it opens only the file types in Also opens."
        : null;
  return (
    <>
      <label className="field">
        <span className="field-label">Is</span>
        <select
          value={!id ? "" : (handler ?? OTHER_APP)}
          onChange={(e) => {
            const v = e.target.value;
            onIdentity(v === "" ? null : { handler: v === OTHER_APP ? null : v, version: v === handler ? (id?.version ?? null) : null });
          }}
        >
          {!id && <option value="">Not confirmed yet</option>}
          {candidates.length > 0 && (
            <optgroup label="Named like its programs">
              {candidates.map((h) => (
                <option key={h.name} value={h.name}>
                  {h.name}
                </option>
              ))}
            </optgroup>
          )}
          <optgroup label="Other apps that open old files">
            {others.map((h) => (
              <option key={h.name} value={h.name}>
                {h.name}
              </option>
            ))}
          </optgroup>
          <option value={OTHER_APP}>Something else</option>
        </select>
        {note && <span className="system-note">{note}</span>}
      </label>
      {handler && (
        <label className="field">
          <span className="field-label">Version</span>
          <input
            key={`${app.id}-${id?.version ?? ""}`}
            type="text"
            list={`versions-${app.id}`}
            placeholder="e.g. 5.1"
            defaultValue={id?.version ?? ""}
            onBlur={(e) => {
              const v = e.currentTarget.value.trim();
              if (v !== (id?.version ?? "")) onIdentity({ handler, version: v || null });
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") e.currentTarget.blur();
            }}
          />
          <datalist id={`versions-${app.id}`}>
            {versions.map((v) => (
              <option key={v} value={v} />
            ))}
          </datalist>
        </label>
      )}
      {handler && siblings.length > 1 && (
        <label className="field-check">
          <input type="checkbox" checked={app.favorite} disabled={app.favorite} onChange={onFavorite} />
          <span>
            {app.favorite
              ? `Favorite: ${handler} documents open with this version.`
              : `Make this the favorite: open ${handler} documents with this version.`}
          </span>
        </label>
      )}
    </>
  );
}

/** What pressing Launch will do, when it isn't simply "run this program". */
function LaunchHint({
  app,
  blocker,
  guestRunning,
  onAros,
}: {
  app: LibraryApp;
  blocker: string | null;
  guestRunning: boolean;
  onAros: boolean;
}) {
  if (blocker) return <p className="system-note warn">{blocker}</p>;
  if (onAros && !guestRunning) {
    return (
      <p className="launch-hint">
        Runs on the free AROS Kickstart. If {app.name} crashes or won't start, a real Kickstart ROM most likely fixes it:
        see the Amiga system box for where to get one.
      </p>
    );
  }
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

/** An opener's track record with this file type, for the "Open with" menu. */
function testLabel(o: Opener): string {
  if (!o.worked && !o.failed) return "";
  const parts = [o.worked ? `worked ${o.worked}×` : "", o.failed ? `failed ${o.failed}×` : ""].filter(Boolean);
  return ` · ${parts.join(", ")}`;
}

/** A document: the apps in the library that can open it, and opening it in one. */
function DocumentDetails({
  doc,
  apps,
  busy,
  onOpen,
  onReveal,
  onExport,
  onRemove,
  canStart,
  startBlocked,
  onStart,
}: {
  doc: LibraryDoc;
  apps: LibraryApp[];
  busy: boolean;
  onOpen: (o: Opener) => void;
  onReveal: () => void;
  onExport: () => void;
  onRemove: () => void;
  /** Mac and Amiga: an app of the guest's to start it with, so the user can open the document there. */
  /** Mac and Amiga: the guest can be started on its own (its setup is complete). */
  canStart: boolean;
  startBlocked: boolean;
  onStart: () => void;
}) {
  const [openers, setOpeners] = useState<Opener[]>([]);
  const [choice, setChoice] = useState(0);
  // Re-match when the document or the library's apps change.
  useEffect(() => {
    invoke<Opener[]>("document_openers", { id: doc.id }).then(
      (o) => {
        setOpeners(o);
        setChoice(0);
      },
      () => setOpeners([]),
    );
  }, [doc.id, apps]);
  const opener = openers[choice];
  const ext = doc.file.includes(".") ? doc.file.split(".").pop()!.toUpperCase() : "";
  return (
    <div className="app-details">
      <h3 className="doc-title">{doc.name}</h3>
      {doc.os !== "dos" ? (
        <p className="system-note">
          Floppy can't open {GUEST_LABEL[doc.os]} documents in their app by itself yet.{" "}
          {canStart
            ? `Start ${GUEST_LABEL[doc.os]}, then open ${docPath(doc)} there.`
            : `Finish the ${GUEST_LABEL[doc.os]} setup files first, then start it and open ${docPath(doc)} there.`}
        </p>
      ) : openers.length > 0 ? (
        <label className="field">
          <span className="field-label">Open with</span>
          <select value={choice} onChange={(e) => setChoice(Number(e.target.value))}>
            {openers.map((o, i) => (
              <option key={`${o.appId}-${o.program}`} value={i}>
                {`${o.appName}${o.version && !o.appName.includes(o.version) ? ` ${o.version}` : ""} (${o.program})${o.favorite ? " · favorite" : ""}${testLabel(o)}`}
              </option>
            ))}
          </select>
          {opener && <span className="system-note">{opener.why}</span>}
        </label>
      ) : (
        <p className="system-note">
          {ext
            ? `No app in the library opens .${ext} files yet. Import one, or add ${ext} to an app's "Also opens".`
            : "This file has no extension, so Floppy can't tell which app opens it. Add its name's type to an app's \"Also opens\", or rename it."}
        </p>
      )}
      <dl className="details-grid">
        <dt>In {GUEST_LABEL[doc.os]}</dt>
        <dd className="details-path">{docPath(doc)}</dd>
        <dt>Type</dt>
        <dd>{docTypeFolder(doc) || "Unsorted"}</dd>
        <dt>Added</dt>
        <dd>{new Date(doc.added * 1000).toLocaleString()}</dd>
      </dl>
      <div className="detail-actions">
        {doc.os === "dos" ? (
          <button type="button" className="primary icontext-btn" disabled={busy || !opener} onClick={() => opener && onOpen(opener)}>
            <span className="btn-icon">
              <PlayIcon />
            </span>
            Open
          </button>
        ) : (
          <button
            type="button"
            className="primary icontext-btn"
            disabled={busy || !canStart || startBlocked}
            onClick={onStart}
          >
            <span className="btn-icon">
              <PlayIcon />
            </span>
            Start {GUEST_LABEL[doc.os]}
          </button>
        )}
        <button type="button" className="icontext-btn" onClick={onReveal}>
          <span className="btn-icon">
            <FolderIcon />
          </span>
          {SHOW_IN_FILES}
        </button>
        <button type="button" className="icontext-btn" onClick={onExport}>
          <span className="btn-icon">
            <FileIcon />
          </span>
          Export…
        </button>
        <button type="button" className="danger icontext-btn" onClick={onRemove}>
          <span className="btn-icon">
            <TrashIcon />
          </span>
          Remove
        </button>
      </div>
    </div>
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
  tracking,
  onForgetIgnored,
  onAskAgain,
  sources,
  onOpenSource,
  onReport,
  onAros,
  onStart,
  started,
}: {
  status: GuestStatus;
  disabled: boolean;
  onChoose: (kind: "rom" | "boot", directory: boolean) => void;
  onModel: (model: string) => void;
  onSaveList: () => void;
  onImportFolder: () => void;
  tracking: SetupTracking;
  onForgetIgnored: () => void;
  onAskAgain: (slot: string) => void;
  sources: SetupSource[];
  onOpenSource: (src: SetupSource) => void;
  onReport: (slot: string, filled: boolean) => void;
  onAros: (on: boolean) => void;
  /** Starts the guest on its own, shown once none of its setup files is missing. */
  onStart: () => void;
  started: boolean;
}) {
  const amiga = status.os === "amiga";
  const { rom, boot, model, aros } = status.system;
  const runningAros = amiga && aros && !rom;
  // cd.rs slot labels, which the list and discs.rs use.
  const romSlot = amiga ? "Kickstart ROM" : "Mac ROM";
  const bootSlot = amiga ? "Workbench disk" : "Mac startup disk";
  /** "Not on your drives" and Ask Again, when a files disc found no usable copy. */
  const notOnDrives = (slot: string) =>
    tracking.notOnDrives.includes(slot) && (
      <>
        <span className="system-note" title="A files disc made from your last list found no usable copy on your drives, so the list stopped asking for it.">
          Not on your drives
        </span>
        <button type="button" className="small" disabled={disabled} onClick={() => onAskAgain(slot)}>
          Ask Again
        </button>
      </>
    );
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
        <span className={`system-value${rom || runningAros ? "" : " missing"}`}>
          {rom ? `${rom}${status.romNote ? ` · ${status.romNote}` : ""}` : runningAros ? "AROS (free replacement, built in)" : "Not added"}
        </span>
        <button type="button" className="small" disabled={disabled} onClick={() => onChoose("rom", false)}>
          Choose…
        </button>
        {runningAros && (
          <button type="button" className="small" disabled={disabled} onClick={() => onAros(false)}>
            Stop Using AROS
          </button>
        )}
        {!rom && notOnDrives(romSlot)}
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
        {!boot && notOnDrives(bootSlot)}
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
      {status.complete && status.found && !status.blocker && (
        <div className="detail-actions guest-start">
          <button type="button" className="primary icontext-btn" disabled={disabled} onClick={onStart}>
            <span className="btn-icon">
              <PlayIcon />
            </span>
            {started ? `${GUEST_LABEL[status.os]} Is Running` : `Start ${GUEST_LABEL[status.os]}`}
          </button>
          <span className="system-note">
            {amiga ? "Boots Workbench, with your apps on the Floppy: drive." : "Boots the startup disk, with your apps on the Unix volume."}
          </span>
        </div>
      )}
      {!rom && (
        <SetupSources
          slot={romSlot}
          note={runningAros ? "runs far more software than AROS" : undefined}
          sources={sources}
          disabled={disabled}
          onOpen={onOpenSource}
        />
      )}
      {amiga && !rom && !aros && (
        <div className="setup-sources">
          <h4>Or start now with the free AROS Kickstart</h4>
          <div className="setup-source">
            <span className="setup-source-name">
              <span className="source-kind free">Free</span> <strong>AROS replacement Kickstart (built in)</strong>
            </span>
            <button type="button" className="small primary" disabled={disabled} onClick={() => onAros(true)}>
              Use AROS for Now
            </button>
            <span className="system-note setup-source-note">
              An open-source stand-in for Commodore's ROM that comes with FS-UAE, so there's nothing to download. It runs
              some games and demos on bootable disks, but many programs crash or refuse to start on it, and it doesn't boot
              Commodore's Workbench, so apps that need Workbench won't run. When Floppy finds a real Kickstart ROM, it
              switches to it by itself.
            </span>
          </div>
        </div>
      )}
      {!boot && (
        <SetupSources
          slot={bootSlot}
          optional={amiga}
          sources={sources}
          disabled={disabled}
          onOpen={onOpenSource}
        />
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
          {tracking.ignored > 0 && (
            <button
              type="button"
              className="small"
              disabled={disabled}
              onClick={onForgetIgnored}
              title="Earlier files discs brought these copies, and Floppy couldn't use them. The list asks for them to be left out."
            >
              Forget {tracking.ignored} Ignored {tracking.ignored === 1 ? "Copy" : "Copies"}
            </button>
          )}
          <span className="system-note">
            Lists the files still needed, to find on your drives (for example with Diskette's Burn A CD). Floppy checks
            each file's contents when you import them.
          </span>
        </div>
      )}
      <div className="system-row">
        <span className="system-note">Something not working, or found a better source?</span>
        <button type="button" className="small" onClick={() => onReport(!rom ? romSlot : !boot ? bootSlot : romSlot, !!rom && !!boot)}>
          Report a Setup Problem…
        </button>
      </div>
    </div>
  );
}

/** Where to get one missing setup file: the rows of "Where Floppy points you" (docs/legal-setupfiles.md). */
function SetupSources({
  slot,
  optional,
  note,
  sources,
  disabled,
  onOpen,
}: {
  slot: string;
  optional?: boolean;
  /** Why it's worth getting, shown after the heading. */
  note?: string;
  sources: SetupSource[];
  disabled: boolean;
  onOpen: (src: SetupSource) => void;
}) {
  const rows = sources.filter((s) => s.slot === slot);
  if (!rows.length) return null;
  return (
    <div className="setup-sources">
      <h4>
        Where to get a {slot}
        {optional ? " (optional)" : ""}
        {note ? ` (${note})` : ""}
      </h4>
      <ul>
        {rows.map((src) => (
          <li key={src.url + src.name} className="setup-source">
            <span className="setup-source-name">
              <span className={`source-kind ${src.kind}`}>{SOURCE_KIND_LABEL[src.kind]}</span> <strong>{src.name}</strong>
            </span>
            <button type="button" className="small" disabled={disabled} onClick={() => onOpen(src)} title={src.url}>
              Open Page
            </button>
            <span className="system-note setup-source-note">{src.note}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

export default App;
