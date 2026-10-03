/**
 * Text-mode drawing for the ANSIapps theme (docs/ansiapps-theme.md, "Text
 * only"; docs/ansiapps-textmode.md is the reference). In that theme every
 * frame, shadow and arrow is made of CP437 characters from the one font,
 * the way a DOS program drew its screen.
 *
 * The real UI stays plain HTML: buttons, inputs and headings keep their
 * semantics. On top, this adds `aria-hidden` layers of box-drawing text:
 *
 * - Frames: a double (`╔═╗║╚╝`) or single (`┌─┐│└┘`) border around a box,
 *   with its title set into the top border (` TITLE `, centered, as Turbo
 *   Vision did), and optionally Turbo Vision's shadow: dark cells two
 *   columns right and one row down.
 * - Dropdown arrows: `▼` over the right end of each select.
 *
 * Nothing is measured. Each border row is a flex row of long runs of `═`
 * or `─` that CSS clips to the box, and each side a clipped column of
 * `║` or `│`, so the frame follows any size and never needs redrawing on
 * resize. The run before a title is rounded down to whole cells, so the
 * title starts on one (the grid: docs/ansiapps-theme.md). Buttons are flat
 * bars with no shadow (ansiapps-theme.css).
 *
 * A MutationObserver decorates boxes as React renders them, and
 * `stopTextMode` removes every layer, so the modern theme is untouched.
 */

type Kind = "double" | "single" | "none";

interface FrameRule {
  selector: string;
  kind: Kind;
  /** Where the title comes from: a heading inside the box. */
  title?: string;
  /** Turbo Vision's shadow: two cells right, one row down. */
  shadow?: boolean;
}

/** Which boxes get which frame. Order doesn't matter; one frame per box. */
const FRAMES: FrameRule[] = [
  { selector: ".panel", kind: "double", title: ":scope > h2" },
  { selector: ".dialog-box", kind: "double", title: ":scope > h2", shadow: true },
  { selector: ".gear-menu", kind: "single", shadow: true },
  { selector: ".system-setup", kind: "single", title: ":scope > h3" },
  { selector: ".session-report", kind: "single" },
  { selector: ".setup-drop", kind: "single" },
  { selector: ".guest-tile", kind: "single" },
  { selector: ".drop-box", kind: "double", shadow: true },
  { selector: ".error, .status-message", kind: "none", shadow: true },
];

const GLYPHS: Record<Exclude<Kind, "none">, { tl: string; tr: string; bl: string; br: string; h: string; v: string }> = {
  double: { tl: "╔", tr: "╗", bl: "╚", br: "╝", h: "═", v: "║" },
  single: { tl: "┌", tr: "┐", bl: "└", br: "┘", h: "─", v: "│" },
};

/** Longer than any box: CSS clips the rest. */
const RUN = 320;
const COLUMN = 120;

const LAYER = "tm-frame";
const ARROW = "tm-arrow";

function span(className: string, text = ""): HTMLSpanElement {
  const s = document.createElement("span");
  s.className = className;
  s.textContent = text;
  return s;
}

/** The title as plain text, from the box's heading (its icon has none). */
function titleOf(el: Element, rule: FrameRule): string {
  if (!rule.title) return "";
  const h = el.querySelector(rule.title);
  if (!h) return "";
  // Its parts (an icon glyph, then the words) joined by a space.
  const parts = [...h.childNodes].map((n) => (n.textContent ?? "").replace(/\s+/g, " ").trim()).filter(Boolean);
  return parts.join(" ");
}

function buildFrame(rule: FrameRule, title: string): HTMLSpanElement {
  const layer = span(`${LAYER} ${LAYER}--${rule.kind}${rule.shadow ? ` ${LAYER}--shadow` : ""}`);
  layer.setAttribute("aria-hidden", "true");
  layer.dataset.title = title;
  if (rule.kind !== "none") {
    const g = GLYPHS[rule.kind];
    const top = span("tm-row tm-top");
    // The title is centered by whole cells (ansiapps-theme.css), so the
    // run before it needs the title's width in cells: one per character.
    if (title) top.style.setProperty("--tm-title-cells", String([...` ${title} `].length));
    top.append(span("tm-corner", g.tl), span("tm-fill", g.h.repeat(RUN)));
    if (title) top.append(span("tm-title", ` ${title} `), span("tm-fill", g.h.repeat(RUN)));
    top.append(span("tm-corner", g.tr));
    const bottom = span("tm-row tm-bottom");
    bottom.append(span("tm-corner", g.bl), span("tm-fill", g.h.repeat(RUN)), span("tm-corner", g.br));
    const column = Array(COLUMN).fill(g.v).join("\n");
    layer.append(top, span("tm-side tm-left", column), span("tm-side tm-right", column), bottom);
  }
  if (rule.shadow) {
    // Two cells wide beside the box, one row below it, starting a row down
    // and two columns in, as Turbo Vision offsets it.
    layer.append(
      span("tm-shadow tm-shadow-right", Array(COLUMN).fill("██").join("\n")),
      span("tm-shadow tm-shadow-bottom", "█".repeat(RUN)),
    );
  }
  return layer;
}

function decorate(root: ParentNode) {
  for (const rule of FRAMES) {
    for (const el of root.querySelectorAll<HTMLElement>(rule.selector)) {
      const title = titleOf(el, rule);
      const existing = el.querySelector<HTMLElement>(`:scope > .${LAYER}`);
      if (existing && existing.dataset.title === title) continue;
      existing?.remove();
      el.classList.add("tm-framed");
      el.append(buildFrame(rule, title));
    }
  }
  for (const select of root.querySelectorAll("select")) {
    const next = select.nextElementSibling;
    if (next?.classList.contains(ARROW)) continue;
    const arrow = span(ARROW, "▼");
    arrow.setAttribute("aria-hidden", "true");
    select.after(arrow);
  }
}

let observer: MutationObserver | null = null;
let pending = 0;

/** Draws the text-mode layers now and as the UI changes. */
export function startTextMode() {
  if (observer) return;
  decorate(document);
  observer = new MutationObserver((records) => {
    // Our own layers change the DOM too: skip records that only touch them.
    const ours = (n: Node) => n instanceof Element && (n.classList.contains(LAYER) || n.classList.contains(ARROW));
    const relevant = records.some(
      (r) =>
        !(r.target instanceof Element && r.target.closest(`.${LAYER}`)) &&
        (r.type === "characterData" || [...r.addedNodes, ...r.removedNodes].some((n) => !ours(n))),
    );
    if (!relevant || pending) return;
    pending = requestAnimationFrame(() => {
      pending = 0;
      decorate(document);
    });
  });
  observer.observe(document.body, { childList: true, subtree: true, characterData: true });
}

/** Removes every text-mode layer (back to the modern theme). */
export function stopTextMode() {
  observer?.disconnect();
  observer = null;
  if (pending) cancelAnimationFrame(pending);
  pending = 0;
  document.querySelectorAll(`.${LAYER}, .${ARROW}`).forEach((n) => n.remove());
  document.querySelectorAll(".tm-framed").forEach((n) => n.classList.remove("tm-framed"));
}
