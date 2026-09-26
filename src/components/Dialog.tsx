import type { ReactNode } from "react";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  actions?: ReactNode;
}

/**
 * Themed dialog (the shared design system's dialog) — replaces
 * window.alert/confirm so every confirmation/prompt in the app shares one
 * styling and animation. Stays mounted while closed (opacity/pointer-events
 * toggled via `.open`) so it can transition in rather than popping.
 */
export function Dialog({ open, onClose, title, children, actions }: DialogProps) {
  return (
    <div
      className={`dialog-overlay${open ? " open" : ""}`}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="dialog-box" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
        <h2 id="dialog-title">{title}</h2>
        {children}
        <div className="dialog-actions">
          {actions ?? (
            <button type="button" className="primary" onClick={onClose}>
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
