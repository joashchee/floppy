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

/** Floppy's brand mark: the app icon's disk, in fixed colors. */
export function AppMarkIcon(props: IconProps) {
  return (
    <svg viewBox="96 96 320 320" {...props}>
      <path d="M112,96 H368 L416,144 V400 Q416,416 400,416 H112 Q96,416 96,400 V112 Q96,96 112,96 Z" fill="#4ade80" />
      <rect x="172" y="96" width="168" height="112" rx="6" fill="#d7dbe2" />
      <rect x="280" y="114" width="36" height="76" rx="4" fill="#14161a" />
      <rect x="144" y="256" width="224" height="160" rx="10" fill="#f4f1e8" />
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
