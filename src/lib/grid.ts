/**
 * The ANSIapps theme's grid (docs/ansiapps-theme.md, "The grid"): every
 * character sits in a cell of the 8 by 16 grid, counted from the
 * window's top left. The stylesheet does the placing; this file does the
 * two things CSS can't. Coupler's lib/grid.ts, for a window that resizes
 * and a page that scrolls (Coupler's screen is a fixed stage).
 */

export const CELL_WIDTH = 8;
export const CELL_HEIGHT = 16;

/**
 * Scrolling comes to rest on a whole row, so scrolled text is on the
 * grid too. One listener for every scrolling box, the page included; it
 * does nothing in the modern theme. Returns the function that removes it.
 */
export function installRowSnap(): () => void {
  const timers = new Map<Element, number>();
  const onScroll = (e: Event) => {
    if (document.documentElement.dataset.theme !== "ansiapps") return;
    const el = e.target === document ? document.scrollingElement : e.target;
    if (!(el instanceof HTMLElement)) return;
    window.clearTimeout(timers.get(el));
    // Once the scrolling has stopped, not during it: that would fight the trackpad.
    timers.set(
      el,
      window.setTimeout(() => {
        timers.delete(el);
        const snapped = Math.round(el.scrollTop / CELL_HEIGHT) * CELL_HEIGHT;
        if (snapped !== el.scrollTop) el.scrollTop = snapped;
      }, 120),
    );
  };
  document.addEventListener("scroll", onScroll, true);
  return () => {
    document.removeEventListener("scroll", onScroll, true);
    timers.forEach((t) => window.clearTimeout(t));
  };
}

export interface GridReport {
  checked: number;
  /** What's off the grid, in words: the text and the cell its first character is in. */
  off: string[];
}

const MARK = "grid-off";

/**
 * Dev-only (gear → Check the Grid, or Ctrl+Cmd+G with a dialog open):
 * measures every piece of text showing in the window, and every field,
 * button and progress bar, and lists the ones that aren't on a cell
 * boundary, outlining them. Run it on each screen and dialog after
 * changing the theme's CSS. Positions are counted from the window's top
 * left, as a text-mode screen's are; the page's own scroll rests on a
 * whole row (installRowSnap), so scroll it there before checking.
 */
export function checkGrid(root: HTMLElement = document.body): GridReport {
  document.querySelectorAll(`.${MARK}`).forEach((el) => el.classList.remove(MARK));
  const offBy = (v: number, cell: number) => {
    const r = ((v % cell) + cell) % cell;
    return Math.min(r, cell - r) > 0.05;
  };
  const showing = (el: Element | null) => {
    for (let e = el; e && e !== document.documentElement; e = e.parentElement) {
      const style = getComputedStyle(e);
      if (style.display === "none" || style.visibility === "hidden" || (e as HTMLElement).inert) return false;
      if (style.clipPath === "inset(50%)") return false; // a heading kept for screen readers only
      if (e.classList.contains("dialog-overlay") && !e.classList.contains("open")) return false;
    }
    return true;
  };
  const report: GridReport = { checked: 0, off: [] };
  const where = (x: number, y: number) => `${(x / CELL_WIDTH + 1).toFixed(2)}, ${(y / CELL_HEIGHT + 1).toFixed(2)}`;
  const flag = (el: Element, what: string, x: number, y: number) => {
    el.classList.add(MARK);
    report.off.push(`"${what}" at ${where(x, y)}`);
  };

  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node as Text;
    const first = text.data.search(/\S/);
    if (first < 0 || !text.parentElement || !showing(text.parentElement)) continue;
    const range = document.createRange();
    range.setStart(text, first);
    range.setEnd(text, text.data.length);
    for (const rect of range.getClientRects()) {
      if (rect.width === 0) continue;
      // Clipped away (a frame's long runs past the box's edge): not shown.
      if (rect.left >= window.innerWidth || rect.top >= window.innerHeight || rect.bottom <= 0) continue;
      report.checked++;
      // A width that isn't whole cells means a character came from another font.
      if (offBy(rect.left, CELL_WIDTH) || offBy(rect.top, CELL_HEIGHT) || offBy(rect.width, CELL_WIDTH)) {
        flag(text.parentElement, text.data.trim().slice(0, 24), rect.left, rect.top);
        break;
      }
    }
  }
  for (const el of root.querySelectorAll("input, textarea, select, button, .progress-bar")) {
    if (!showing(el)) continue;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0 || rect.bottom <= 0 || rect.top >= window.innerHeight) continue;
    report.checked++;
    if (offBy(rect.left, CELL_WIDTH) || offBy(rect.top, CELL_HEIGHT) || offBy(rect.width, CELL_WIDTH) || offBy(rect.height, CELL_HEIGHT)) {
      flag(el, el.getAttribute("aria-label") ?? el.getAttribute("placeholder") ?? el.textContent?.trim().slice(0, 24) ?? el.tagName.toLowerCase(), rect.left, rect.top);
    }
  }
  return report;
}
