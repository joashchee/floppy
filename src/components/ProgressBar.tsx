/**
 * One reusable progress bar for every "this is taking a moment" moment in
 * the app (an import, or copying a system file). The shared design
 * system's component and CSS:
 *
 * - `value` (0–1): a determinate bar for a countable batch.
 * - `durationMs`: fills from 0% to 100% over a known duration. Pass a
 *   changing `key` at the call site to restart it.
 * - `indeterminate`: a sweeping bar for work of unknown length.
 */
import type { CSSProperties } from "react";

interface ProgressBarProps {
  value?: number;
  durationMs?: number;
  indeterminate?: boolean;
  label: string;
  className?: string;
}

export function ProgressBar({ value, durationMs, indeterminate, label, className }: ProgressBarProps) {
  const mode = indeterminate ? "indeterminate" : durationMs !== undefined ? "timed" : "determinate";
  const pct = value !== undefined ? Math.max(0, Math.min(100, Math.round(value * 100))) : undefined;
  const fillStyle: CSSProperties | undefined =
    mode === "timed" ? { animationDuration: `${durationMs}ms` } : mode === "determinate" ? { width: `${pct ?? 0}%` } : undefined;

  return (
    <div
      className={`progress-bar progress-bar-${mode}${className ? ` ${className}` : ""}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={mode === "determinate" ? pct : undefined}
    >
      <div className="progress-bar-fill" style={fillStyle} />
    </div>
  );
}
