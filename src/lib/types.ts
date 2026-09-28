/** Mirrors of the Rust types the Tauri commands return (src-tauri/src/). */

export type GuestOs = "dos" | "mac-classic" | "amiga";

export const GUEST_OSES: GuestOs[] = ["dos", "mac-classic", "amiga"];

/** library.rs `LibraryApp`. */
export interface LibraryApp {
  id: string;
  os: GuestOs;
  name: string;
  /** The app's folder in its guest's library (an 8.3 name on DOS). */
  dir: string;
  /** What to launch, relative to `dir`, `/`-separated. */
  program: string | null;
  programs: string[];
  sourceName: string;
  /** Unix seconds. */
  added: number;
  /** Document extensions the user says this app opens (uppercase, no dot). */
  opens: string[];
  /** Each program's fingerprint, keyed like `programs` (DOS). */
  programIds: Record<string, { size: number; sha256: string }>;
  /** Which handler app this is and its version, once known; null until then. */
  identity: AppIdentity | null;
  /** The version its handler's documents open with. */
  favorite: boolean;
  /** What the user noted going wrong running it. */
  errors: string;
  /** The user renamed it, so Is and Version leave its name alone. */
  namedByUser: boolean;
  /** Share its errors in Export Findings, though it isn't a known app. */
  shareErrors: boolean;
}

/** library.rs `Identity`. `handler: null` means it isn't one of the known apps. */
export interface AppIdentity {
  handler: string | null;
  version: string | null;
  /** "hash": a program matched a known version; "user": the user said;
   *  "learned": it matched a version learned from someone else's findings. */
  by: "hash" | "user" | "learned";
}

/** handlers.rs `HandlerInfo`: a known app that opens old files. */
export interface HandlerInfo {
  name: string;
  programs: string[];
  exts: string[];
  /** Versions known by fingerprint. */
  versions: string[];
}

/** Handlers an app might be, going by its program names alone. */
export function handlerCandidates(app: LibraryApp, handlers: HandlerInfo[]): HandlerInfo[] {
  const names = app.programs.map((p) => p.split("/").pop()!.toUpperCase());
  return handlers.filter((h) => h.programs.some((p) => names.includes(p.toUpperCase())));
}

/** An app's identity for a list row: "WordPerfect 5.1", "WordPerfect?", or "". */
export function identityLabel(app: LibraryApp, handlers: HandlerInfo[]): string {
  const id = app.identity;
  if (id) return id.handler ? [id.handler, id.version].filter(Boolean).join(" ") : "";
  const guess = handlerCandidates(app, handlers);
  return guess.length ? `${guess.map((h) => h.name).join(" or ")}?` : "";
}

/** library.rs `LibraryDoc`: an old file to open in an app that made it. */
export interface LibraryDoc {
  id: string;
  os: GuestOs;
  /** Its original name. */
  name: string;
  /** Relative to the guest's library folder (`DOCS/LETTER.WP5`). */
  file: string;
  opensWith: string | null;
  added: number;
}

/** documents.rs `Opener`: an app in the library that can open a document. */
export interface Opener {
  appId: string;
  appName: string;
  program: string;
  version: string | null;
  /** The favorite version of its app. */
  favorite: boolean;
  why: string;
  /** How often it opened this file type correctly, and how often not (verify.rs). */
  worked: number;
  failed: number;
}

/** verify.rs `Pending`: what to ask after a document session. */
export interface PendingVerification {
  os: GuestOs;
  appId: string;
  appName: string;
  program: string;
  handler: string | null;
  version: string | null;
  size: number | null;
  sha256: string | null;
  fileType: string;
  document: string;
}

/** verify.rs `Tally`: every answer for one app and file type. */
export interface HandlerTest {
  os: GuestOs;
  /** The known app when confirmed, else the app's own name. */
  app: string;
  confirmed: boolean;
  version: string | null;
  program: string;
  size: number | null;
  sha256: string | null;
  fileType: string;
  worked: number;
  failed: number;
  lastTested: number;
  lastOutcome: "worked" | "failed";
  notes: string[];
}

/** findings.rs `Summary`: what Export Findings would share, new since the last export. */
export interface FindingsSummary {
  handlerTests: number;
  identities: number;
  fileTypes: number;
  systemFiles: number;
  appErrors: number;
  setupReports: number;
  /** Where you said dropped files go (drops.rs). */
  dropChoices: number;
  /** Unix seconds of the last export, if any. */
  lastExported: number | null;
}

/** commands.rs `ImportedItem`. */
export interface ImportedItem {
  app: LibraryApp | null;
  document: LibraryDoc | null;
}

/** commands.rs `SessionReport`: what a DOS session saved. */
export interface SessionReport {
  os: GuestOs;
  appName: string;
  document: string | null;
  changes: { path: string; name: string; new: boolean }[];
  /** For a document session: what to ask the user about. */
  verify: PendingVerification | null;
  /** Which app it was, when that isn't settled yet. */
  identify: IdentifyAsk | null;
}

/** commands.rs `IdentifyAsk`: a program named like a known app ran; which app was it? */
export interface IdentifyAsk {
  appId: string;
  appName: string;
  /** The program's file name (`WORD.EXE`). */
  program: string;
  /** Known apps with a program of that name. */
  candidates: string[];
}

/** A document's path as DOS sees it. */
/** A library-relative path as its guest writes it: `C:\\DOCS\\WP5\\LETTER.WP5`, `Unix:Documents:TEXT:Letter`, `Floppy:Documents/ILBM/Sunset.iff`. */
export function guestFilePath(os: GuestOs, file: string): string {
  switch (os) {
    case "dos":
      return `C:\\${file.replace(/\//g, "\\")}`;
    case "mac-classic":
      return `Unix:${file.replace(/\//g, ":")}`;
    case "amiga":
      return `Floppy:${file}`;
  }
}

/** Where a document is inside its guest. */
export function docPath(doc: LibraryDoc): string {
  return guestFilePath(doc.os, doc.file);
}

/** documents.rs `docs_dir`: each guest's documents folder. */
export const DOCS_DIR: Record<GuestOs, string> = { dos: "DOCS", "mac-classic": "Documents", amiga: "Documents" };

/** The type folder a document is sorted into (documents.rs `type_folder`): `WP5`, `TEXT`, `ILBM`. */
export function docTypeFolder(doc: LibraryDoc): string {
  const parts = doc.file.split("/");
  return parts.length >= 3 ? parts[1] : "";
}

/** library.rs `GuestSystem`: user-supplied files under library/system/<os>/. */
export interface GuestSystem {
  rom: string | null;
  boot: string | null;
  model: string | null;
  /** Amiga only: FS-UAE's built-in AROS Kickstart stands in while `rom` is empty. */
  aros: boolean;
}

/** commands.rs `GuestStatus`. */
export interface GuestStatus {
  os: GuestOs;
  emulator: string;
  found: boolean;
  source: "env" | "chosen" | "bundled" | "installed" | null;
  system: GuestSystem;
  romNote: string | null;
  /** Why apps can't launch yet; null when they can. */
  blocker: string | null;
}

/** commands.rs `StartupImport`: the result of a `floppy import …` launch. */
export interface StartupImport {
  document?: LibraryDoc | null;
  app: LibraryApp | null;
  error: string | null;
}

/** cd.rs `CdImport`: what an Import Files Disc (or dropped setup files) set up. */
export interface CdImport {
  added: string[];
  stillMissing: string[];
  /** Dropped zips and disc images that couldn't be opened, with why. */
  skipped: string[];
  /** Files on a disc with a manifest that couldn't be used ("name: reason"), now on the ignore list. */
  unusable: string[];
  /** What the disc's manifest told Floppy (discs.rs), when it had one. */
  disc: DiscReport | null;
  /** Slots a system backup had a file for, kept because they were set up already (backup.rs). */
  kept: string[];
}

/** discs.rs `DiscReport`. */
export interface DiscReport {
  producer: string;
  alreadyImported: boolean;
  /** Slots the user's drives had nothing usable for, now left off the list. */
  notOnDrives: string[];
}

/** commands.rs `SetupTracking`: what earlier files discs taught Floppy. */
export interface SetupTracking {
  ignored: number;
  notOnDrives: string[];
}

/** media.rs `OldMedia`: an attached disk macOS couldn't mount. */
export interface OldMedia {
  device: string;
  name: string;
  size: number;
  /** The guest its partition map suggests, before its contents are read. */
  hint: GuestOs | null;
  diskImage: boolean;
}

/** commands.rs `MediaProgress`. */
export interface MediaProgress {
  device: string;
  done: number;
  total: number;
}

/** amiga.rs `MODELS`. */
export const AMIGA_MODELS = ["A500", "A500+", "A600", "A1200", "A4000/040"];

export const GUEST_LABEL: Record<GuestOs, string> = {
  dos: "DOS",
  "mac-classic": "Classic Mac",
  amiga: "Amiga",
};

const MAC_DISK_IMAGE = /\.(dsk|img|image|dc42|hfv|hda|toast|iso)$/i;
const AMIGA_DISK_IMAGE = /\.(adf|adz|dms|hdf)$/i;

/** Whether launching `program` boots it as a disk, rather than starting the guest for the user to open it. */
export function bootsAsDisk(os: GuestOs, program: string | null): boolean {
  if (!program) return false;
  if (os === "mac-classic") return MAC_DISK_IMAGE.test(program);
  if (os === "amiga") return AMIGA_DISK_IMAGE.test(program);
  return true;
}

/**
 * A path as the guest shows it: `C:\WP51\BIN\WP.EXE` on DOS,
 * `Unix:MacWrite:MacWrite II` on the Mac (Basilisk II names its shared
 * volume "Unix"), `Floppy:ProTracker/ProTracker` on the Amiga.
 */
export function guestPath(app: LibraryApp, program?: string | null): string {
  switch (app.os) {
    case "dos":
      return `C:\\${app.dir}${program ? `\\${program.replace(/\//g, "\\")}` : ""}`;
    case "mac-classic":
      return `Unix:${app.dir}${program ? `:${program.replace(/\//g, ":")}` : ""}`;
    case "amiga":
      return `Floppy:${app.dir}${program ? `/${program}` : ""}`;
  }
}

/** cd.rs `SetupSource`: where to get a setup file ("Where Floppy points you" in docs/legal-setupfiles.md). */
export interface SetupSource {
  /** A slot label: "Mac ROM", "Mac startup disk", "Kickstart ROM", "Workbench disk". */
  slot: string;
  kind: "free" | "paid" | "own";
  name: string;
  url: string;
  note: string;
}

/** findings.rs `SETUP_REPORT_KINDS`. */
export type SetupReportKind = "source-broken" | "didnt-work" | "better-source";

/** backup.rs `Status`: whether to offer a system backup, and what it would hold. */
export interface BackupStatus {
  slots: string[];
  complete: boolean;
  offer: boolean;
  /** Unix seconds. */
  lastBackup: number | null;
}

/** backup.rs `Made`: what a backup disc held. */
export interface BackupMade {
  slots: string[];
  originalBytes: number;
  discBytes: number;
}

/** drops.rs `Choice`: where a dropped item goes. */
export interface DropChoice {
  to: "app" | "document" | "setup";
  os: GuestOs;
}

/** drops.rs `Signature`: what kind of item was dropped, never its name. */
export interface DropSignature {
  kind: "file" | "folder" | "zip";
  ext: string;
  content: string;
}

/** drops.rs `DropOption`: one place a dropped item could go. */
export interface DropOption {
  choice: DropChoice;
  /** "Document, DOS". */
  label: string;
  /** 1 to 3: it could, its name says so, its contents say so. */
  score: number;
  why: string;
}

/** drops.rs `Classification`: what Floppy made of a dropped item. */
export interface DropClassification {
  path: string;
  name: string;
  signature: DropSignature;
  /** Best first. */
  options: DropOption[];
  /** Set when there's no need to ask. */
  decided: DropChoice | null;
  decidedBy: string | null;
  /** ".wp5 files like this". */
  sameFor: string;
}

/** learned.rs `LearnSummary`: what learning from one findings file did. */
export interface LearnSummary {
  already: boolean;
  own: boolean;
  versions: number;
  fileTypes: number;
  tests: number;
  systemFiles: number;
  dropChoices: number;
  setupSources: number;
  materials: number;
  /** A signed knowledge pack's Floppy AI version. */
  aiVersion: number | null;
  /** It called itself a knowledge pack without the maintainers' signature. */
  unsignedPack: boolean;
  forMaintainers: number;
  skipped: string[];
}

/** learned.rs `KnowledgeSummary`: what Floppy has learned from findings. */
export interface KnowledgeSummary {
  sources: number;
  versions: number;
  fileTypes: number;
  tests: number;
  systemFiles: number;
  dropRules: number;
  materials: number;
}

/** ai.rs `AiInfo`: Floppy AI's version (docs/floppy-ai.md). */
export interface AiInfo {
  /** This Floppy's: the build's, or a newer knowledge pack's. */
  version: number;
  date: string;
  builtin: number;
  fromPack: boolean;
  learnedFrom: number;
}
