/**
 * Inline SVG icons, one glyph per concept, drawn in currentColor so they
 * theme for free. Glyphs from the shared design system (folder, play,
 * trash) match the other ansiapps apps.
 */
import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const stroke = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** Floppy's brand mark: the app icon's 5¼" disk, in fixed colors. The
 * rim is thicker than the icon's so it still reads at header size. */
export function AppMarkIcon(props: IconProps) {
  return (
    <svg viewBox="66 66 380 380" {...props}>
      <path
        d="M80,72 H432 Q440,72 440,80 V124 H422 V158 H440 V432 Q440,440 432,440 H294 A8,8 0 0 0 278,440 H234 A8,8 0 0 0 218,440 H80 Q72,440 72,432 V80 Q72,72 80,72 Z"
        fill="#3d424b"
        stroke="#7a8392"
        strokeWidth="10"
        strokeLinejoin="round"
      />
      <rect x="100" y="96" width="280" height="66" rx="4" fill="#f4f1e8" />
      <rect x="100" y="96" width="280" height="16" fill="#4ade80" />
      <circle cx="256" cy="264" r="60" fill="#241c18" />
      <circle cx="256" cy="264" r="46" fill="none" stroke="#a8977e" strokeWidth="14" />
      <circle cx="256" cy="264" r="30" fill="#14161a" />
      <rect x="236" y="340" width="40" height="84" rx="20" fill="#241c18" />
    </svg>
  );
}

export function FolderIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <path {...stroke} d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
    </svg>
  );
}

/** A zip or a single program to import. */
export function FileIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <path {...stroke} d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z" />
      <path {...stroke} d="M14 3v5h5" />
    </svg>
  );
}

export function PlayIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <path fill="currentColor" d="M8 5v14l11-7z" />
    </svg>
  );
}

/** A DOS prompt: boot to C:\APP> without running anything. */
export function PromptIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <rect {...stroke} x="3" y="4" width="18" height="16" rx="2" />
      <path {...stroke} d="M7 9l3 3-3 3M12 15h5" />
    </svg>
  );
}

export function TrashIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <path {...stroke} d="M3 6h18" />
      <path {...stroke} d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <path {...stroke} d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
      <path {...stroke} d="M10 11v6M14 11v6" />
    </svg>
  );
}

/** A DOS app in the library list: a program window. */
export function DosAppIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <rect {...stroke} x="3" y="4" width="18" height="16" rx="2" />
      <path {...stroke} d="M3 8h18" />
    </svg>
  );
}

/** A classic Mac app: an all-in-one compact Mac. */
export function MacAppIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <rect {...stroke} x="5" y="2" width="14" height="20" rx="2" />
      <rect {...stroke} x="8" y="5" width="8" height="7" rx="1" />
      <path {...stroke} d="M12 17h4" />
    </svg>
  );
}

/** An Amiga app: a keyboard computer. */
export function AmigaAppIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <rect {...stroke} x="2" y="8" width="20" height="10" rx="2" />
      <path {...stroke} d="M6 12h.01M10 12h.01M14 12h.01M18 12h.01M8 15h8" />
    </svg>
  );
}

/** A guest's system files: a ROM chip. */
export function ChipIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <rect {...stroke} x="6" y="6" width="12" height="12" rx="1" />
      <path {...stroke} d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4" />
    </svg>
  );
}

/** The gear button: app-level settings and actions. */
export function GearIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <circle {...stroke} cx="12" cy="12" r="3" />
      <path
        {...stroke}
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
      />
    </svg>
  );
}

/** Save something out of Floppy: an arrow up out of a tray. */
export function ExportIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <path {...stroke} d="M12 15V3M8 7l4-4 4 4" />
      <path {...stroke} d="M4 15v4a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-4" />
    </svg>
  );
}

/** A disc to import from. */
export function DiscIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <circle {...stroke} cx="12" cy="12" r="9" />
      <circle {...stroke} cx="12" cy="12" r="2" />
    </svg>
  );
}

/** About the app. */
export function InfoIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" {...props}>
      <circle {...stroke} cx="12" cy="12" r="9" />
      <path {...stroke} d="M12 11v5M12 8h.01" />
    </svg>
  );
}
