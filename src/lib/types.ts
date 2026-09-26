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
}

/** library.rs `GuestSystem`: user-supplied files under library/system/<os>/. */
export interface GuestSystem {
  rom: string | null;
  boot: string | null;
  model: string | null;
}

/** commands.rs `GuestStatus`. */
export interface GuestStatus {
  os: GuestOs;
  emulator: string;
  found: boolean;
  source: "env" | "bundled" | "installed" | null;
  system: GuestSystem;
  romNote: string | null;
  /** Why apps can't launch yet; null when they can. */
  blocker: string | null;
}

/** commands.rs `StartupImport`: the result of a `floppy import …` launch. */
export interface StartupImport {
  app: LibraryApp | null;
  error: string | null;
}

/** cd.rs `CdImport`: what an Import Files Disc (or dropped setup files) set up. */
export interface CdImport {
  added: string[];
  stillMissing: string[];
  /** Dropped zips and disc images that couldn't be opened, with why. */
  skipped: string[];
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
