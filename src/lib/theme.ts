/**
 * UI theme: "modern" (the default ansiapps look) or "ansiapps" (the
 * old-school DOS look every ansiapps app offers as its alternative, see
 * docs/ansiapps-theme.md). The choice is a `data-theme` attribute on
 * <html>, which src/ansiapps-theme.css keys off. index.html's inline
 * script applies the stored choice before first paint, so it must use the
 * same key and value.
 */

export type Theme = "modern" | "ansiapps";

/** Also read by the inline script in index.html. */
const THEME_KEY = "floppy.theme";

export function loadTheme(): Theme {
  try {
    return localStorage.getItem(THEME_KEY) === "ansiapps" ? "ansiapps" : "modern";
  } catch {
    return "modern";
  }
}

export function applyTheme(theme: Theme) {
  if (theme === "ansiapps") document.documentElement.dataset.theme = "ansiapps";
  else delete document.documentElement.dataset.theme;
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // localStorage unavailable — the theme just won't persist across launches.
  }
}
