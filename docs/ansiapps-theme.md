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

- **Palette**: only the 16 CGA/VGA text-mode colors (`--dos-*`). No
  other colors, no gradients except a solid-block progress fill, no
  transparency except modal dimming.
- **Surfaces**: the desktop and panels are blue with light-cyan frames.
  Menus and dialogs are light-gray windows with black text.
- **Type**: one bitmap font, IBM VGA 8x16, at 16px only, with no bold
  or italic and font smoothing off. Hierarchy comes from color and
  position.
- **Shapes**: square corners. Panels get a double-line frame with the
  title set into the top border. Dialog titles sit on a double rule
  (`═══ About Floppy ═══`). Boxes inside panels get single-line frames.
- **Depth**: hard black shadows with no blur (16px windows, 8px
  buttons). A pressed button shifts into its shadow.
- **Selection**: list rows are borderless lines, and the selected one
  (an app, a document, the chosen guest) is a cyan bar with black
  text. Menus highlight in green.
- **Controls**: checkboxes render as `[ ]` / `[X]`, text fields and
  pickers are black strips, and focus is a dashed white outline.
- **No motion** beyond the indeterminate progress bar.
- **Floppy's own color keeps its meaning**: phosphor green becomes
  light green (`--accent`), used for primary buttons, section and row
  icons, and the progress fill.
- Icons stay inline SVG in `currentColor` and pick up the palette.

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
