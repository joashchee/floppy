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

**Stylus** (decided 2026-09-29, not built yet) is the family's ANSI art
and animation tool: open source (MIT), always free, built on icy_tools.
It's where the theme's art will be drawn and shipped to every app as a
shared **theme pack** (see "ANSI art assets" below). Stylus is the one
ansiapps app that opens in the ANSIapps theme; Floppy keeps modern as
its default.

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
  HTML controls and headings; every text/background pair comes from the
  ranked list in `docs/ansiapps-color-contrast.md` (WCAG AA or better,
  only 32 of the palette's pairs pass), and icons and frames get 3:1.

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
- **Contrast**: look every pair up in `docs/ansiapps-color-contrast.md`,
  never eyeball it. On blue only white, yellow, light cyan, light green,
  light gray and light magenta read; on light gray only black and blue;
  on cyan and green only black. A colored mark that can land on several
  backgrounds (a warning on a row that can be selected, or in a dialog)
  gets its own black cell.
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
- **No easing, but frame-by-frame animation is allowed**: no fades,
  slides or easing, because a text-mode screen just redraws. Motion is
  whole cells redrawn, the way an ANSImation plays: today only the
  indeterminate progress bar, which steps in 12 whole moves (see Shared
  UI conventions); spinners and animated splash art once the theme pack
  has them. Nothing flashes more than 3 times a second, anything moving
  longer than 5 seconds can be paused, and `prefers-reduced-motion`
  gets one still frame.
- **Buttons** are Turbo Vision green (black text), primary light green,
  danger red with white text, disabled dark gray on light gray.
- **Floppy's own color keeps its meaning**: phosphor green becomes
  light green (`--accent`), used for primary buttons, section and row
  icons, and the progress fill.
- Icons are CP437 glyph twins today (`components/icons.tsx`); the
  theme pack's ANSI icons replace them once it exists.

## Shared UI conventions (every ansiapps app)

Decided 2026-09-27 across the ansiapps apps. Beyond the two themes,
every app shares the three behaviors below, and each must look right in
both themes. `CLAUDE.md` has how Floppy does them.

- **A loading screen from the first paint.** `index.html` holds a static
  splash (mark, name, empty progress slot and label) with inline styles,
  so it paints before any JS; the no-flash theme script runs first, so
  the ANSIapps splash is blue from the start. `StartupScreen.tsx` renders
  the same markup and adds a determinate bar over the launch-time steps.
  ANSIapps look: blue screen, white name, the mark as its CP437 glyph
  `◙` in light green (the text-only rule allows no image), the `░█` bar
  and a light-gray label, on whole 16px cells. It disappears rather
  than fading.
- **Feedback for every user activity.** Anything the user sets off shows
  a label and progress bar until it settles (`runActivity`,
  `ActivityStatus.tsx`, after 150 ms), then a status message or an
  error. The control acted on shows it's busy: disabled, or "Starting…".
  In the ANSIapps theme a busy row turns dark gray rather than
  transparent, since the palette allows no transparency.
- **Estimated completion on every counting progress bar**, under it:
  "About 4 min left, done around 14:32" (`useCompletionEstimate`,
  `src/lib/estimate.ts`), redone only at 20, 40, 60, 80 and 90% or every
  10 seconds so it holds steady. ANSIapps look: light-gray 16px text like
  other status lines. A determinate fill jumps to each new width, and the
  indeterminate sweep moves in 12 whole steps, because a stopped bar
  would look like a hang.

## ANSI art assets (the theme pack, authored in Stylus)

**Decided across the family 2026-09-29; the pack and its tooling aren't
built yet.** Text mode doesn't mean no graphics: the theme's graphics
(app marks, splash art, icons, empty states, spinners) and the drawn
parts of its controls (button faces, dialog and panel frames, title
bars, checkboxes, scroll bars, the progress fill) become **ANSI art and
animation**, drawn in Stylus and shared by every app.

- **The canvas:** the theme font (IBM VGA 8×16) at 8-px spacing, square
  pixels, the 16 `--dos-*` colors only, iCE colors on; `.XB` sources
  with SAUCE.
- **Control parts** are 9-slices in whole cells, one frame per state
  (normal, hover, pressed, focused, disabled). A `manifest.json` gives
  every part's slices, frame timings, alt text (or `decorative`) and the
  contrast role of its colored cells.
- **Rendered at build time** by Stylus's command-line render tool into
  PNGs (1× and 2×) and CSS (`border-image` 9-slices, `image-rendering:
  pixelated`, integer scales, `steps()` animation, a reduced-motion rest
  frame). Apps commit the generated files with the pack version and
  never run Stylus code.
- **Text stays text.** Every word is still HTML in the VGA font; the art
  is the frame around it, never a picture of words (the app's name in
  its logo art is the one exception, with alt text).
- **Accessibility:** text cells use a pair from the ranked list; icons
  and control borders 3:1 against their neighbors (both colors of a
  `░▒▓` shade); decorative art is `aria-hidden`; color is never the only
  cue; animations as in "Current rules".
- **License:** the pack is MIT (ansiapps' own art, in the Stylus repo),
  fine to ship beside Floppy's GPL code. Keep its MIT notice with the
  rendered files and credit it in About. Rendered PNGs of the VGA font's
  glyphs are fine to ship (VileR's readme exempts rendered images from
  ShareAlike); a converted font file still isn't.

**How it fits Floppy's text-only rule:** the pack's parts are drawn on
the same 8×16 grid with the same font and palette, so they're text-mode
art, and they're the one kind of image the ANSIapps theme may show.
Until a part exists, Floppy's text layers (`textmode.ts` frames, CSS
generated glyphs, CP437 icon twins) stay. When the pack lands:

1. Render it for Floppy and commit the PNGs and generated CSS.
2. Swap each text-drawn part for the pack's (frames, button faces,
   icons, the splash mark `◙`) one at a time, checking both themes.
3. Keep the About credit and the pack's MIT notice beside the files.

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
