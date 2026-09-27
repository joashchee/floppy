# The ANSIapps theme

Every ansiapps app ships two UI themes:

1. **Modern**: the default, dark token-based design (IBM Plex,
   `--bg`/`--panel`/`--line`/`--text`/`--muted`/`--accent`).
2. **ANSIapps**: an old-school DOS text-mode look, switched from the
   **gear menu** with the checkbox "ANSIapps theme (old-school DOS
   look)". It's remembered across launches (`localStorage`, key
   `floppy.theme`) and applied before first paint, so it never flashes
   the modern theme.

The look draws on int10h.org's
[Ultimate Oldschool PC Font Pack](https://int10h.org/oldschool-pc-fonts/fontlist/),
Norton Commander's blue file panels, and Turbo Vision's gray dialogs.

## How it's built

| Piece | Where | What |
|---|---|---|
| Theme state | `src/lib/theme.ts` | `Theme = "modern" \| "ansiapps"`, set as `data-theme="ansiapps"` on `<html>` |
| No-flash apply | `index.html` inline `<script>` | Reads the same key before the bundle loads, with a blue background |
| Styles | `src/ansiapps-theme.css` | Everything scoped to `:root[data-theme="ansiapps"]`, imported after the modern CSS, which stays untouched |
| Toggle | Gear menu checkbox (`data-testid="ansiapps-theme-toggle"`) | |
| Font and license | `public/fonts/ansiapps/` | The `.woff`, the pack's `LICENSE.TXT`, and a `README.txt` crediting it |
| Credit | About Floppy, "Credits" | The font's name, VileR, int10h.org, and CC BY-SA 4.0 |

The theme is mostly **token overrides**. The modern CSS routes every
color through the tokens, so the ANSIapps stylesheet re-points them,
then adds the structural changes below. Re-pointing tokens inside a
container (the gear menu, a dialog, a selected row) recolors everything
in it. Keep new UI on the tokens and the theme mostly follows for free,
and check new UI in both themes.

## Design rules

### Text only (standing convention, all ansiapps apps)

The ANSIapps theme draws **everything with text from the one monospaced
font**, exactly as a DOS text-mode program would: the screen is a grid
of 8×16-pixel cells, each holding one CP437 glyph in one of the 16
colors. `docs/ansiapps-textmode.md` is the expert reference (glyphs,
palette, Turbo Vision recipes, web rendering); these are its rules:

- **Grid:** the font at exactly 16px, rows exactly 16px (`line-height:
  16px`), every box sized and placed in whole cells (`ch`, `lh`), so
  every glyph starts on a whole pixel and stacked `│ ║ █` touch.
- **Frames:** CP437 box-drawing characters only (the 40 that exist:
  single, double, and single/double mixes; no rounded, heavy or dashed
  lines). Double for the active window or dialog, single for inactive
  ones and boxes inside them, `├─┤` for separators, the title set into
  the top border with a space each side.
- **Shadows:** cells, not blur: 2 columns right and 1 row down, dark
  gray on black with the characters under them still showing.
- **Buttons:** Turbo Vision style: the face on one row, its shadow `▄`
  beside it and `▀▀▀` below; pressed moves the face and drops the shadow.
- **Controls:** `[ ]`/`[X]`, `( )`/`(•)`, input lines as colored strips,
  dropdowns with `▐↓▌`, scroll bars `▲▒■▼`, progress `█▓▒░`.
- **Icons:** CP437 glyphs (`☺ ♪ ☼ ■ ≡ ► ▲ ↕ ⌂`…), not SVG.
- **Art and headings:** ANSI-art technique (shade ramps, clean edges,
  one light source), but every drawn word also exists as real text.
- **Accessibility:** the drawing is an `aria-hidden` layer over real
  HTML controls and headings; colors pass WCAG AA (the reference lists
  which classic pairs fail and their replacements).

### How Floppy draws it

- **Frames** (`src/lib/textmode.ts`): panels, dialogs, the gear menu,
  setup boxes, guest tiles, drop targets and messages get an
  `aria-hidden` layer of box characters: top and bottom rows of long
  `═`/`─` runs in a flex row that CSS clips to the box, `║`/`│` columns
  for the sides, the title (from the box's real heading, which stays for
  screen readers but is visually hidden) centered in the top border, and
  for windows and menus Turbo Vision's shadow, two cells of `█` right and
  one row below in black. Nothing is measured, so frames follow any size.
  A MutationObserver decorates boxes as React renders them;
  `stopTextMode()` removes every layer when the theme goes back to
  modern.
- **Buttons**: Turbo Vision's green bar, with `▄` beside it and a row of
  `▀` under it, drawn as CSS generated text with empty alt text
  (`content: "▄" / ""`) so screen readers skip it. Pressed, the face
  moves one cell right and the shadow goes.
- **Icons**: each icon in `components/icons.tsx` renders a CP437 twin
  (`☺ ☻ ○ ■ ¶ ► » ≡ ↑ ◘ ▬ ◙ i x`), shown instead of the SVG.
- **Rules and separators**: rows of `─`; the gear menu's separators join
  its frame as `├───┤`.
- **Controls**: `[ ]`/`[X]` checkboxes and `( )`/`(•)` radio buttons; black input strips; each select
  gets a `▼` cell (`.tm-arrow`) over its right end; status pills and
  source kinds as `[text]`.
- **Progress**: `█` over a `░` track.
- **Grid**: 16px font, 16px rows everywhere, spacing in whole cells.

What's still drawn by the platform: the page scrollbars (styled in
text-mode colors), native tooltips, and the dimmed overlay behind a
dialog (a color, like an attribute change).

### Current rules

- **Palette**: only the 16 CGA/VGA text-mode colors (`--dos-*`). No
  other colors, no gradients except a solid-block progress fill, no
  transparency except modal dimming.
- **Surfaces**: the desktop and panels are blue with light-cyan frames.
  Menus and dialogs are light-gray windows with black text.
- **Type**: one bitmap font, IBM VGA 8x16, at 16px only, with no bold
  or italic and font smoothing off. Hierarchy comes from color and
  position.
- **Frames**: panels and dialogs double (`╔═╗`), menus and boxes inside
  panels single (`┌─┐`), titles centered in the top border.
- **Depth**: Turbo Vision shadows in text: `█` cells two columns right
  and one row down for windows and menus, `▄`/`▀` for buttons. A pressed
  button shifts into its shadow.
- **Selection**: list rows are borderless lines, and the selected one
  (an app, a document, the chosen guest) is a cyan bar with black
  text. Menus highlight in green.
- **Controls**: checkboxes render as `[ ]` / `[X]`, radio buttons as
  `( )` / `(•)`, text fields and
  pickers are black strips with a `▼` cell, and focus is reverse video
  (black on white).
- **No motion** beyond the indeterminate progress bar.
- **Buttons** are Turbo Vision green (black text), primary light green,
  danger red with white text, disabled dark gray on light gray.
- **Floppy's own color keeps its meaning**: phosphor green becomes
  light green (`--accent`), used for primary buttons, section and row
  icons, and the progress fill.
- Icons are still inline SVG in `currentColor` (to be replaced by
  CP437 glyphs, see "Text only").

## Font licensing

The only third-party asset is the font: **IBM VGA 8x16**
(`WebPlus_IBM_VGA_8x16.woff`, the "Plus" variant for wide Unicode
coverage) from **The Ultimate Oldschool PC Font Pack v2.2 by VileR**,
https://int10h.org/oldschool-pc-fonts/, **CC BY-SA 4.0**, from the
official `oldschool_pc_font_pack_v2.2_web.zip` (downloaded 2026-09-26).
SHA-256 `4823e093c7f2f69641eaafb6f3465185afa63877cda58d341b6b317fd53dfd1f`.

- It ships **unmodified** as its own file, beside GPL code, which is
  aggregation of a separate work. Using a font to display text doesn't
  make Floppy an adaptation of it, so ShareAlike covers only the font.
- **Never** subset it, convert it (to woff2, say), rename glyphs, merge
  it into another font, or inline it as a data URI. Any of those makes
  an adapted font that must itself be CC BY-SA and marked as modified.
- **Always** keep `LICENSE.TXT` and `README.txt` beside it, and the
  About credit.
- Check any new theme asset the same way before use, and record it
  here.

The theme's CSS is ansiapps' own work, under Floppy's license here.
