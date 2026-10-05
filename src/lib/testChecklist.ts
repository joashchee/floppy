/**
 * App Testing checklist data (CLAUDE.md's Testing section), the same
 * mechanism as every ansiapps app's. Lives only here, embedded in source,
 * never a separate spreadsheet or doc. Edit it in the same pass as the
 * feature it covers: add items when a feature lands, edit them when
 * behavior changes, and drop an item once it's been confirmed working.
 *
 * The overlay that renders this (components/AppTesting.tsx) is dev-only:
 * App.tsx only mounts it behind `import.meta.env.DEV`, so Vite drops it
 * from a production `tauri build`.
 */

export interface ChecklistItem {
  id: string;
  section: string;
  label: string;
  /** CSS selector for the "Go to app" jump target, omitted for a step with no single fixed control. */
  selector?: string;
}

export const CHECKLIST_DATA: ChecklistItem[] = [
  // The ANSIapps grid (2026-10-03)
  { id: "grid-main", section: "ANSIapps grid", label: "Gear → ANSIapps theme, then gear → Check the Grid on the DOS, Mac and Amiga tabs, with an app selected: it reports everything on the grid, nothing outlined in light magenta", selector: '[data-testid="gear-button"]' },
  { id: "grid-dialogs", section: "ANSIapps grid", label: "Open About Floppy, Backup Floppy System… and a Remove confirmation; in each, Ctrl+Cmd+G reports everything on the grid. Dialogs open at the same place, a few rows from the top, not centered on the window's height" },
  { id: "grid-menu", section: "ANSIapps grid", label: "With the gear menu open, Check the Grid from the keyboard (Ctrl+Cmd+G) passes; the menu's right edge lines up with the header's", selector: '[data-testid="gear-button"]' },
  { id: "grid-resize", section: "ANSIapps grid", label: "Resize the window to odd widths (drag slowly); the frames' right edges stay whole ║ characters and Check the Grid still passes" },
  { id: "grid-scroll", section: "ANSIapps grid", label: "Scroll the window with a trackpad and let go between rows: it settles on a whole row a moment later. In the modern theme it doesn't" },
  { id: "grid-startup", section: "ANSIapps grid", label: "Relaunch in the ANSIapps theme: the launch screen's glyph, name, bar and label sit on whole cells, one blank row apart, roughly centered" },
  { id: "grid-titles", section: "ANSIapps grid", label: "Panel and dialog titles set into the top border start on a whole cell (the ═ either side of them are whole characters), at any window width" },
  { id: "grid-drop", section: "ANSIapps grid", label: "Drag a file over the window on the Mac tab with setup files missing: the two drop targets sit side by side from the top left, their text at the left, frames whole" },

  // Caught up with the ansiapps family (2026-10-03)
  { id: "first-run", section: "ansiapps family", label: "First run (delete in-development-accepted in ~/Library/Application Support/com.ansiapps.floppy): before any window, a dialog says Floppy is still in development. I'll Be Back. quits with no window; OK opens the launch screen, and the next launch doesn't ask" },
  { id: "flat-buttons", section: "ansiapps family", label: "ANSIapps theme: buttons are flat green bars with no ▄/▀ shadow; pressing one turns it cyan without moving it, and the click lands" },
  { id: "licenses", section: "ansiapps family", label: "About Floppy → Show Licenses reveals the bundled license texts, third-party-licenses.txt and GPL-2.0.txt among them", selector: '[data-testid="gear-button"]' },
  { id: "app-testing-export", section: "ansiapps family", label: "This overlay: Export results… saves a .txt and says where; it holds only the checklist's status and notes" },

  // Windows build and platform behavior
  { id: "windows-build", section: "Windows", label: "On Windows, run .\\scripts\\build-windows.ps1 from a clean checkout: the pinned emulator archives verify, both MSI and NSIS installers are produced, and the release executable contains no user-profile path" },
  { id: "windows-emulators", section: "Windows", label: "Open Floppy on Windows: Locate filters for .exe files, DOSBox Staging and FS-UAE launch with bundled resources, Quit closes them normally, and Force Quit leaves no emulator process behind" },
  { id: "windows-iso", section: "Windows", label: "Import an ISO that was not mounted before: Floppy reads it and detaches it afterward; repeat with it already mounted and confirm Floppy leaves it mounted" },
];
