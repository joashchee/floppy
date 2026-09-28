/**
 * Launch screen shown over the app until the startup calls (library,
 * guest statuses, known apps, setup sources, a startup import) have all
 * answered. It continues index.html's static splash, which has the same
 * markup and styles, so the window is never blank: the splash paints
 * before any JS loads, and this adds the progress bar once React mounts.
 * Fades out when `done` turns true, then unmounts.
 */
import { useEffect, useState } from "react";
import { describeEstimate, useCompletionEstimate } from "../lib/estimate";
import { ProgressBar } from "./ProgressBar";

/** Must match `.startup-screen.leaving`'s transition in index.html. */
const FADE_MS = 200;

interface StartupScreenProps {
  /** 0–1 share of startup steps finished. */
  progress: number;
  /** What's still loading, e.g. "Loading the library…". */
  label: string;
  done: boolean;
}

export function StartupScreen({ progress, label, done }: StartupScreenProps) {
  const [gone, setGone] = useState(false);
  // Shown in the label line: the bar's own estimate line has no room here.
  const estimate = useCompletionEstimate(done ? undefined : progress);

  useEffect(() => {
    if (!done) return;
    const t = setTimeout(() => setGone(true), FADE_MS);
    return () => clearTimeout(t);
  }, [done]);

  if (gone) return null;
  return (
    <div className={`startup-screen${done ? " leaving" : ""}`} aria-busy={!done}>
      <img className="startup-mark" src="/favicon.svg" alt="" />
      <div className="startup-glyph" aria-hidden="true">
        ◙
      </div>
      <div className="startup-name">Floppy</div>
      <div className="startup-progress">
        <ProgressBar value={progress} label="Starting Floppy" hideEstimate />
      </div>
      <div className="startup-label">
        {label}
        {estimate && ` ${describeEstimate(estimate)}`}
      </div>
    </div>
  );
}
