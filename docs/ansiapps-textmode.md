# Draw whole interfaces from 256 glyphs

> **The ANSIapps text-mode reference.** Every ansiapps app's ANSIapps
> theme is drawn by these rules (`docs/ansiapps-theme.md`, "Text only").
> From deep research on 2026-09-27; each fact is tagged by where it came
> from, and **[R]**/**[N]** facts should be spot-checked (for example
> against real screens in Floppy's bundled DOSBox) before being relied
> on. Keep it current: when a check confirms or corrects a fact, update
> its tag here.

To draw a complete DOS-style interface from one monospaced font, treat the screen as a grid of 8×16-pixel cells, each holding one glyph with one foreground and one background colour. Draw frames, buttons, shadows, scroll bars and icons only from the roughly 60 CP437 characters built for that job, in the 16-colour VGA palette. Pixel-exact results come from integer geometry, not from font-smoothing switches. The font Floppy ships, int10h's **WebPlus IBM VGA 8×16 v2.2**, is built to make this possible. Measured from its file: every advance is exactly 8 px and every line exactly 16 px at `font-size: 16px`, every outline point sits on the pixel grid, and it holds all 256 CP437 characters. It also holds **only 40 of the 128 Unicode box-drawing characters**, so rounded corners, heavy lines and dashed lines don't exist and must be faked. The best component vocabulary to copy is Borland's Turbo Vision, whose frame characters, icons, shadow size and full colour palette can be read line by line from its source. That makes it far more trustworthy than recollections of Norton Commander or MS-DOS EDIT. The web side has one real trap: a character grid survives only if every text run starts on a whole pixel and rows are exactly one cell tall. **Floppy's current ANSIapps theme sets `line-height: 1.25` (20 px)**, which puts a 4 px gap between stacked `│`, `║` and `█` glyphs. That needs fixing wherever characters draw frames. None of the popular "DOS CSS" libraries draws its frames with characters. The recommended architecture is real HTML controls on a cell-snapped layout, with `aria-hidden` character layers redrawn only when the cell count changes. It is new to web practice, not a copy of existing work.

**How to read the evidence tags.** The research ran with most reference sites blocked (Wikipedia, int10h.org, FreeVGA, MDN, w3.org, roysac.com). Every fact below carries a tag saying where it came from:

| Tag | Meaning | Confidence |
|---|---|---|
| **[M]** | Measured this session from the font file Floppy ships ([`public/fonts/ansiapps/WebPlus_IBM_VGA_8x16.woff`](../public/fonts/ansiapps/WebPlus_IBM_VGA_8x16.woff)), using fontTools and a per-pixel rasterization | Highest |
| **[S]** | Read from source code or machine-readable tables (DOSBox Staging, Borland Turbo Vision via magiblot/tvision, Midnight Commander skin, ICU, CPython, xterm.js, WebTUI, TuiCss, 98.css) | High |
| **[C]** | Computed this session (arithmetic, WCAG contrast ratios) | High, given the inputs |
| **[N]** | From a search-result snippet of a page that could not be opened | Medium |
| **[R]** | Recollection or general knowledge that no fetched source confirmed | Low: spot-check before relying on it |
| **[I]** | The researchers' or this report's own inference or recommendation | Reasoned, not sourced |

## The font holds 256 CP437 glyphs, but only 40 of 128 box pieces

**Mapping.** Bytes 0x20–0x7E are ASCII. Bytes 0x80–0xFF map one-to-one to Unicode through Unicode's CP437.TXT, which CPython's `cp437` codec copies by machine **[S]** ([CPython cp437.py](https://github.com/python/cpython/blob/main/Lib/encodings/cp437.py)). The "smiley" glyphs at 0x01–0x1F and the house `⌂` at 0x7F have no official graphic mapping, since CP437.TXT maps those bytes to control codes. ICU's IBM-437 table lists the graphic readings as fallbacks **[S]** ([ICU ibm-437_P100-1995.ucm](https://raw.githubusercontent.com/unicode-org/icu/main/icu4c/source/data/mappings/ibm-437_P100-1995.ucm)). The WebPlus font draws each of these glyphs twice, at the control code point (U+0001…) and at the graphic one (U+263A…), with identical bitmaps. U+0000 is empty **[M]**. **In HTML, always emit the graphic code points** such as U+263A `☺` or U+25BA `►`. Fallback fonts and HTML parsing treat C0 control characters as invisible or special **[I]**. Two positions are commonly misread: 0xE1 is `ß` (U+00DF), not Greek beta, and 0xE6 is `µ` (U+00B5), the micro sign **[S]**.

The full byte-to-glyph grid:

| hi\lo | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | A | B | C | D | E | F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0x | · | ☺ | ☻ | ♥ | ♦ | ♣ | ♠ | • | ◘ | ○ | ◙ | ♂ | ♀ | ♪ | ♫ | ☼ |
| 1x | ► | ◄ | ↕ | ‼ | ¶ | § | ▬ | ↨ | ↑ | ↓ | → | ← | ∟ | ↔ | ▲ | ▼ |
| 2x–7x | ASCII | | | | | | | | | | | | | | | 7F = ⌂ |
| 8x | Ç | ü | é | â | ä | à | å | ç | ê | ë | è | ï | î | ì | Ä | Å |
| 9x | É | æ | Æ | ô | ö | ò | û | ù | ÿ | Ö | Ü | ¢ | £ | ¥ | ₧ | ƒ |
| Ax | á | í | ó | ú | ñ | Ñ | ª | º | ¿ | ⌐ | ¬ | ½ | ¼ | ¡ | « | » |
| Bx | ░ | ▒ | ▓ | │ | ┤ | ╡ | ╢ | ╖ | ╕ | ╣ | ║ | ╗ | ╝ | ╜ | ╛ | ┐ |
| Cx | └ | ┴ | ┬ | ├ | ─ | ┼ | ╞ | ╟ | ╚ | ╔ | ╩ | ╦ | ╠ | ═ | ╬ | ╧ |
| Dx | ╨ | ╤ | ╥ | ╙ | ╘ | ╒ | ╓ | ╫ | ╪ | ┘ | ┌ | █ | ▄ | ▌ | ▐ | ▀ |
| Ex | α | ß | Γ | π | Σ | σ | µ | τ | Φ | Θ | Ω | δ | ∞ | φ | ε | ∩ |
| Fx | ≡ | ± | ≥ | ≤ | ⌠ | ⌡ | ÷ | ≈ | ° | ∙ | · | √ | ⁿ | ² | ■ | NBSP |

### Box drawing is 11 single, 11 double and 18 mixed pieces, and nothing else

CP437 has exactly **40 box-drawing characters** **[S]**:

- **All single:** `─│┌┐└┘├┤┬┴┼`
- **All double:** `═║╔╗╚╝╠╣╦╩╬`
- **Double horizontal, single vertical:** `╒╕╘╛╞╡╤╧╪`
- **Single horizontal, double vertical:** `╓╖╙╜╟╢╥╨╫`

The font's cmap confirms that these 40 are the only box-drawing characters it holds **[M]**. It has none of the following: heavy lines (`━┃`), dashed lines (`┄┆`), rounded arcs (`╭╮╯╰`), diagonals (`╱╲╳`), or half-line stubs (`╴╵╶╷`).

That gives the one grammar rule of text-mode framing: **a junction is legal only if each axis is uniformly single or uniformly double** **[I]**. Two common cases:

- A single-line divider across a double-framed window starts with `╟` and ends with `╢`.
- A double title rule inside a single frame uses `╞═══╡`.

Anything else, such as a single line meeting a double line on the same axis, has no glyph. If a stylesheet or string asks for a missing glyph such as `╭`, a fallback font draws it with a different advance and stroke position. That breaks both the join and the rest of the row's grid **[I]**.

**Geometry of the line glyphs [M].** In the 8×16 cell:

| Line | Where it sits | Thickness |
|---|---|---|
| Single horizontal | Row 7 | 1 px |
| Single vertical | Columns 3–4 | **2 px wide** |
| Double horizontal | Rows 5 and 7 | two 1 px lines |
| Double vertical | Columns 2–3 and 5–6 | two 2 px lines |

Every connecting stroke runs to the cell edge, so neighbouring cells join seamlessly at an 8 px advance and a 16 px line pitch. Single frames therefore look slightly heavier on their vertical sides. That is authentic VGA, not a rendering bug. A related trap is the pipe character: **`|` (0x7C) is drawn as a broken bar** that doesn't span the cell, so it can never stand in for `│` in a frame **[M]**.

### Shades tile seamlessly, and the half blocks split 7:9

The shade glyphs are regular ordered dithers **[M]**. Their periods (4 px and 2 px) divide the cell, so large fields tile with no visible seams, which makes them good for desktops and scroll-bar tracks **[I]**:

- `░` is 25% ink.
- `▒` is a 50% one-pixel checkerboard.
- `▓` is 75% ink.

The half blocks are **not exact halves**: `▀` covers rows 0–6 (7 px) and `▄` covers rows 7–15 (9 px) **[M]**, and the shadeans author notes the same 9-of-16 split **[S]** ([shadeans README](https://raw.githubusercontent.com/hmderdoc/shadeans/main/README.md)). `▌` and `▐` are true 4-px halves. `■` is a free-floating 5×7 square (columns 1–5, rows 4–10) that touches no edge. `▬` is 7×4 **[M]**. The consequences:

- Half-block "pixels" are 8×7 on top and 8×9 below.
- A `▀` shadow row is thinner than a `▄` one.
- `▀` and `▄` are exact complements, so the fg/bg colour trick still covers the whole cell **[I]**.

### The icon vocabulary is small, and ✓ exists only in WebPlus

CP437 covers most of the icon vocabulary a text-mode app needs **[S/M]**:

| Use | Glyphs |
|---|---|
| Disclosure and scroll arrows | `▲▼►◄` (► and ◄ are 7×11, bigger than ▲ and ▼'s 7×7 **[M]**) |
| Direction and resize | `↑↓→←↕↔↨` |
| Menu | `≡` (three 7-px bars that don't join side by side **[M]**) |
| Check mark | `√` |
| Bullets and radio states | `•○◘◙∙·■` |
| Sound, settings, home, warning | `♪♫`, `☼`, `⌂`, `‼` |

The WebPlus variant adds `✓□▪▫●◦◊` and full Greek, Cyrillic and Hebrew **[M]**, but these are **not CP437**. They would vanish if the theme ever switched to a Px437 or Web437 font. Treat `✓` as a WebPlus luxury and prefer `√` or `X` **[I]**.

## VGA text mode is an 80×25 grid of 9×16 cells in a fixed 16-colour palette

**The grid.** DOSBox Staging's mode table defines VGA text modes 02h/03h as **720×400 pixels, 80×25 cells, each 9×16**, with video memory at 0xB8000 **[S]** ([DOSBox Staging int10_modes.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/ints/int10_modes.cpp)). Older adapters used smaller cells:

| Adapter | Screen | Cell |
|---|---|---|
| CGA | 640×200 | 8×8 |
| EGA | 640×350 | 8×14 |

VGA gets 50 rows by loading the ROM 8×8 font (INT 10h AX=1112h) **[S]** ([int10.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/ints/int10.cpp)). A 4:3 tube showing 720×400 has pixels about **1.35× taller than wide** **[N]** ([VOGONS](https://www.vogons.org/viewtopic.php?f=63&t=53687)), **[C]**.

The int10h font Floppy uses is the **8-dot** rendering. An 80-column row is 640 CSS px, with none of the 9-dot mode's gaps between characters. For an app theme, square pixels at 8×16 are the right trade: pixel-exact and simple to lay out. The 9×16 fonts and a 1.35 vertical stretch are only for a "screenshot-faithful" mode **[I]**.

**The ninth column.** In 9-dot modes the hardware adds a blank ninth pixel column. When Line Graphics Enable is on (the BIOS default), characters **0xC0–0xDF** copy their eighth column into it, so horizontal lines stay joined. DOSBox extends to the ninth pixel only when `chr >= 0xc0 && chr <= 0xdf` **[S]** ([vga_draw.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/hardware/video/vga_draw.cpp)). On real 9-dot hardware, `░▒▓` (0xB0–0xB2) therefore show a 1-px stripe between cells, while `█▄▀▐` stay solid **[I]**. The 8×16 font makes the whole question moot, since nothing is inserted between cells **[M]**.

### Use 0x00/0x55/0xAA/0xFF, and make colour 6 brown

VGA builds the 16 colours from 6-bit DAC levels 0x00, 0x15, 0x2A and 0x3F. Those convert exactly to 8-bit **00, 55, AA, FF**. Colour 6 is `{0x2a, 0x15, 0x00}` in DOSBox's "canonical CGA palette as emulated by VGA cards" **[S]** ([int10_modes.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/ints/int10_modes.cpp)):

| # | Name | Hex | # | Name | Hex |
|---|---|---|---|---|---|
| 0 | Black | `#000000` | 8 | Dark gray | `#555555` |
| 1 | Blue | `#0000AA` | 9 | Light blue | `#5555FF` |
| 2 | Green | `#00AA00` | 10 | Light green | `#55FF55` |
| 3 | Cyan | `#00AAAA` | 11 | Light cyan | `#55FFFF` |
| 4 | Red | `#AA0000` | 12 | Light red | `#FF5555` |
| 5 | Magenta | `#AA00AA` | 13 | Light magenta | `#FF55FF` |
| 6 | **Brown** | **`#AA5500`** | 14 | Yellow | `#FFFF55` |
| 7 | Light gray | `#AAAAAA` | 15 | White | `#FFFFFF` |

Two errors are common:

- **Colour 6 as dark yellow `#AAAA00`.** That is what clone CGA monitors showed. IBM's 5153 detected colour 6 and lowered its green **[N]** ([Wikipedia: CGA](https://en.wikipedia.org/wiki/Color_Graphics_Adapter)). EGA and VGA made brown a real palette entry, EGA's index 0x14 **[S]**.
- **Converting with a bare `v<<2`.** That gives `#545454`, `#A8A8A8` and `#FCFCFC`, an off-by-a-bit error **[C]**.

For a "real 5153" flavour, DOSBox also ships the measured IBM 5153 palette: brown ≈ `#A66900`, dark gray ≈ `#4D4D4D` **[S]**. Remember too that ANSI's colour order differs from the PC's. ANSI goes black, red, green, yellow, blue, magenta, cyan, white, so ANSI "yellow" (SGR 33) is PC colour 6, brown **[I]**.

### An attribute byte gives each cell one foreground and one background colour

Each cell is a character byte followed by an attribute byte **[S]** ([vga_draw.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/hardware/video/vga_draw.cpp)):

| Bits | Meaning |
|---|---|
| 0–3 | Foreground colour (all 16) |
| 4–6 | Background colour (8) |
| 7 | Blink by default; with blink disabled, a fourth background bit |

By BIOS default, bit 7 makes the **foreground** blink and only 8 backgrounds exist. During the off phase DOSBox draws the text in the background colour; the background itself never blinks. Disabling blink turns bit 7 into a fourth background bit, giving 16 backgrounds, the "iCE colours" of ANSI art **[S]** ([16colo.rs forum](https://forum.16colo.rs/t/ice-colors-or-blinking-text/27)). A period-strict theme uses only the 8 dark colours as backgrounds **[I]**. Written as `0xBF` (background nibble, foreground nibble), the common attributes are **[S/R]**:

- `0x07`: DOS default, light gray on black.
- `0x1F`: white on blue.
- `0x70`: black on light gray, for menus and status lines.
- `0x08`: dark gray on black, for shadows.

### The cursor is an underline on rows 13–14 and blinks as a hard on/off

The hardware cursor is a band of scan lines drawn in the cell's foreground colour **[S]** ([int10_char.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/ints/int10_char.cpp), [vga_draw.cpp](https://raw.githubusercontent.com/dosbox-staging/dosbox-staging/main/src/hardware/video/vga_draw.cpp)):

- Mode set leaves the default CGA underline (lines 6–7 of 8).
- On a 16-line VGA cell, the BIOS's CGA emulation moves it to **rows 13–14**, leaving row 15 blank below it.
- A start line of 0 gives a full block.

The cursor toggles every **16 frames** **[N]** ([FreeVGA](http://www.osdever.net/FreeVGA/vga/textcur.htm)). Blinking text toggles every 32 frames according to a QEMU patch **[N]**. DOSBox, however, blinks both at the 16-frame rate **[S]**, so the sources disagree.

At VGA text mode's ~70 Hz (449 total lines **[S]**), that works out to a cursor toggle every **≈229 ms**, and blinking text every ≈457 ms **[C]**. Both are hard on/off toggles, never fades. Selections and menus use reverse video, which swaps the nibbles: `0x07` becomes `0x70` **[R]**.

## Borland's source code is the most reliable pattern book

Turbo Vision (TV) is the only classic DOS UI framework whose rendering could be read at the level of its source. magiblot/tvision keeps Borland's 1994 files with their original constants **[S]** ([magiblot/tvision](https://github.com/magiblot/tvision)). Its component recipes are exact. Norton Commander colours are approximated through Midnight Commander's default skin, which clones NC **[S]** ([mc default.ini](https://github.com/MidnightCommander/mc/blob/master/misc/skins/default.ini)). EDIT/QBasic, Setup and BIOS screens rest on recollection **[R]**.

### Windows switch to a double frame when active, and shadows only recolour

`TFrame::frameChars` holds a single set (`└│┌├┘─┴┐┤┬┼`) and a double set (`╚║╔╟╝═╧╗╢╤`) **[S]** ([tvtext1.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tvtext1.cpp)). The double set's only junctions are **double-to-single** ones (`╟╢╤╧`), used where a framed child view meets the window border. The **active window gets the double frame**. Inactive windows, and a window being dragged, get the single frame **[S]** ([tframe.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tframe.cpp)). The frame parts:

| Part | Glyphs | Position |
|---|---|---|
| Title | text with one space each side | truncated to width−10, centred at (W−len)/2 |
| Close box | `[■]` | x = 2 |
| Zoom box | `[↑]`, or `[↕]` when maximized | x = W−5 |
| Window number (1–9) | digit | x = W−7 with a zoom box, else W−3 |
| Resize corner | `─┘` | x = W−2 on the bottom row |

The icons appear only on the active window. The brackets take the frame colour, while `■`, `↑` and `─┘` take the icon colour.

Every window, dialog and menu box casts a **2-column × 1-row shadow** (`shadowSize = {2,1}`, `shadowAttr = 0x08`). The shadow **keeps the underlying characters and only changes their attribute** **[S]** ([tview.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tview.cpp), [tvwrite.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tvwrite.cpp)). It narrows to 1 column in 43/50-line modes, where cells are half as tall **[S]** ([tprogram.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tprogram.cpp)).

An active 34-column blue window, with the shadow shown as `▒` purely to mark its footprint (in real rendering the cells underneath stay visible, recoloured):

```
╔═[■]════════ Setup ═══════1═[↑]═╗      frame 0x1F white/blue; ■ ↑ 0x1A light green
║                                ║▒▒    text 0x1E yellow/blue; selection 0x71
║                                ║▒▒
╚═══════════════════════════════─┘▒▒    ─┘ resize corner in icon colour
  ▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒    shadow: +2 cols, +1 row, attribute 0x08
```

`TDialog` differs from `TWindow` in three ways: it is gray by default, it has only a close box (no zoom, no resize), and its active frame is `0x7F` white on light gray **[S]** ([tdialog.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tdialog.cpp)). Stock TV message boxes (Warning, Error, Information, Confirm) are plain gray dialogs with 10×2 buttons, not red ones **[S]** ([msgbox.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/msgbox.cpp)).

### Controls: a half-block button shadow, cyan clusters and a `▐↓▌` history pill

**Buttons.** A TV button is a green face with a shadow of half blocks: `▄` in the right column of the first row, `█` below it on taller buttons, and a row of `▀` underneath offset by two columns. The shadow is drawn in the dialog's own colour, `0x70` on a gray dialog. **Pressing moves the face one column right, and the shadow disappears** **[S]** ([tbutton.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tbutton.cpp)). A 10×2 default button (`·` marks the dialog-gray background):

```
·   OK   ▄     cols 1–8 face: 0x2B light cyan/green (default), 0x20 black/green (normal),
··▀▀▀▀▀▀▀▀                    0x2F white/green (focused); hotkey 0x2E yellow; ▄▀ black on gray
```

The other TV controls, all colours from `cpAppColor` in [app.h](https://github.com/magiblot/tvision/blob/master/include/tvision/app.h) **[S]**:

| Control | Glyph recipe | Colours |
|---|---|---|
| Check boxes ([tcheckbo.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tcheckbo.cpp)) | `" [ ] "`, marker `X` at offset 2, label from offset 5 | `0x30` black on cyan; focused item `0x3F`; hotkey `0x3E`; cursor parked on the marker |
| Radio buttons ([tradiobu.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tradiobu.cpp)) | `" ( ) "`, marker `•` (0x07) | as check boxes |
| Input line ([tinputli.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tinputli.cpp)) | text from col 1; `◄` in col 0 / `►` in the last col when scrolled | `0x1F` white on blue; selection `0x2F`; arrows `0x1A` |
| History drop-down ([thistory.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/thistory.cpp)) | `▐↓▌`, a three-cell green "pill" after the input | sides `0x72`, arrow `0x20` |
| List viewer ([tlstview.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tlstview.cpp)) | `│` divider at the last cell of each column | `0x30`; focused `0x2F`; selected `0x3E`; divider `0x31` |
| Scroll bar ([tscrlbar.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tscrlbar.cpp)) | vertical `▲ ▒…■…▒ ▼`, horizontal `◄▒▒■▒▒►`; empty range = all `▓`, no thumb; sits on the window border | `0x31` in blue windows, `0x13` in gray |
| Labels | follow their control: turn white when it has focus | `0x70`, `0x7F`, hotkey `0x7E` |

TV has **no combo `▼` and no progress bar** **[S]**. The conventional DOS gauge is `47% ████████░░░░░░░░`, with `█` for done and `░` or `▒` for the rest. That form is common practice rather than something read from a sourced framework **[I]**.

### Menus, status line and desktop

The menu bar is row 0 in `0x70` with **red hotkeys** (`0x74`). The selected item is **black on green** (`0x20`), with a red hotkey on green. Disabled items are `0x78`. Each item is drawn as `' '+name+' '`, so the highlight includes one space of padding on each side **[S]** ([tmenubar.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tmenubar.cpp)). A drop-down box is a single frame with **a blank column outside it on both sides**, and the selection bar fills the whole inner width. The label starts at x = 3, a submenu gets `►` at W−4, and shortcuts are right-aligned to end at W−4. A nameless item draws a `├───┤` separator, and the box casts a shadow **[S]** ([tmenubox.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tmenubox.cpp)):

```
 ≡  File  Edit  Search  Run                 row 0, 0x70; hotkeys 0x74; "File" 0x20
   ┌──────────────────┐
   │ Open...      F3  │                      selected: 0x20 across the inner width
   │ Save         F2  │
   ├──────────────────┤
   │ Print           ►│                      shadow +2 cols, +1 row
   └──────────────────┘
```

The status line is the bottom row in the menu palette: ` F1 Help  Alt-X Exit │ hint`, with the keys in red and a `│ ` separator before the context hint **[S]** ([tstatusl.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tstatusl.cpp)). The desktop is `░` in **`0x71` blue on light gray**, which reads as a medium stipple **[S]** ([tvtext2.cpp](https://github.com/magiblot/tvision/blob/master/source/tvision/tvtext2.cpp)).

The full TV palette, useful as a ready "Borland" token set **[S]** ([app.h](https://github.com/magiblot/tvision/blob/master/include/tvision/app.h)):

| Role | Attributes |
|---|---|
| Desktop | 71 |
| Menu / status: normal, disabled, hotkey, selected, selected-disabled, selected-hotkey | 70 78 74 20 28 24 |
| Blue window: frame passive / active / icon, scroll bar ×2, text, selected text | 17 1F 1A 31 31 1E 71 |
| Gray dialog: frame | 70 7F 7A |
| Gray dialog: buttons normal / default / focused / disabled / hotkey / shadow | 20 2B 2F 78 2E 70 |
| Gray dialog: clusters | 30 3F 3E (disabled 38) |
| Gray dialog: input | 1F 2F 1A |
| Gray dialog: lists | 30 2F 3E 31 |

### Norton Commander and EDIT are secondhand and partly unverified

The **Midnight Commander** default skin **[S]** gives these NC-style colours:

| Element | Colours |
|---|---|
| Panels | light gray on blue |
| Cursor bar | black on cyan |
| Marked files | yellow |
| Menus | white on cyan; selection white on black; hotkeys yellow |
| Error dialogs | white on red, yellow title |
| Key bar | white-on-black digits, then black-on-cyan labels |
| Shadow | dark gray on black |

Its frames are single-line. NC itself is remembered as having cyan panel text and **double-line panel frames** **[R]**, and a snippet describes cyan-on-blue panels **[N]** ([ilyabirman.net UI Museum](https://ilyabirman.net/meanwhile/all/ui-museum-norton-commander-5-0/)).

**EDIT.COM and QBasic** are remembered with more gaps **[R]**:

- The editor area is white or light gray on blue.
- The menu bar is black on gray, with white hotkeys and a reverse-video selection.
- Dialogs are single-framed with a `├──┤` rule above `< OK >` / `< Cancel >` buttons (the `< OK >` form has one snippet behind it **[N]**: [PCjs QuickBASIC](https://www.pcjs.org/documents/books/mspl13/basic/qblearn/)).
- The status bar is on cyan.

MS-DOS and Windows 3.x Setup are remembered as a full blue screen with a gray `ENTER=Continue  F1=Help  F3=Exit` row at the bottom **[R]**. **Bottom line:** confirm these three looks against real video memory before calling a theme "EDIT-accurate". Floppy's bundled DOSBox can dump the attribute bytes at B800 **[I]**.

CUA's interaction rules fit all of these apps **[R]**; the guideline document itself wasn't fetched:

- F1 opens help, and F10 or a bare Alt opens the menu bar.
- Esc backs out, and Enter runs the default button.
- Tab moves between fields; arrow keys move within a group.
- `...` marks a choice that opens a dialog, and `►` marks a cascade.
- Disabled choices are dimmed but still shown.

## ANSI art rewards ramps, clean edges and a small palette

ANSI art is CP437 plus ECMA-48 SGR colour codes, as interpreted by `ANSI.SYS`, on an 80-column screen. Files are `.ANS`, and an optional 128-byte SAUCE trailer records width, font, iCE colours, letter spacing and aspect **[S]** ([ansilove README](https://raw.githubusercontent.com/ansilove/ansilove/master/README.md), [Go sauce package](https://pkg.go.dev/github.com/textmodes/sauce)). Renderers leave iCE colours off by default **[S]**.

The toolkit has three parts. The **shades `░▒▓` blend foreground and background at 25/50/75%**, which "turns the 16-colour palette into a few hundred usable tones." **Half blocks are for real edges.** And hand-drawn shading goes **between close colours rather than loud pairs**, with each region settling on one ramp **[S]** ([shadeans README](https://raw.githubusercontent.com/hmderdoc/shadeans/main/README.md)). The same source warns that the VGA palette "has no muted colours": anything soft is closer to dark gray than to a real colour **[S]**.

In practice that means building each hue as a ramp from black through its dark and bright forms to white, with shade glyphs as the in-betweens **[I]**:

| Ramp | Steps, dark to light |
|---|---|
| Blue | black → blue → light blue → white |
| Neutral | black → dark gray → light gray → white |
| Warm | black → red → brown → yellow → white |

**Half-block pixel art** treats a cell as two pixels: `▀` puts the foreground on top and the background below, `▄` the reverse **[I]**. Because a cell holds only two colours, plan the drawing so no cell ever needs three. Where an outline crosses a half cell, make that half the fill colour or black, never the average colour, which leaves stray mid-tone blocks along edges **[S]** ([shadeans](https://raw.githubusercontent.com/hmderdoc/shadeans/main/README.md)). Pixel-art corner rules carry over directly: don't fill a 2×2 at a corner, and use one pixel per edge **[N]** ([Pixel-Editor](https://www.pixel-editor.com/articles/pixel-art-outlines)).

There are three lettering styles: outline, filled block (`█` bodies rounded with `▀▄`) and rendered (a vertical ramp). The classic source of big CP437 lettering is TheDraw's TDF fonts; tdfiglet renders all 1,198 known ones **[S]** ([tdfiglet](https://raw.githubusercontent.com/tat3r/tdfiglet/master/README.md)). FIGlet is the 7-bit equivalent. Its fitting and smushing modes are specified in the FIGfont 2 document **[S]** ([figfont.txt](https://raw.githubusercontent.com/cmatsuoka/figlet/master/figfont.txt)). A filled block wordmark, three rows tall:

```
█▀▀ █   █▀█ █▀█ █▀█ █ █
█▀  █   █ █ █▀▀ █▀▀ ▀▄▀
▀   ▀▀▀ ▀▀▀ ▀   ▀    ▀
```

The quality bar that separates good work from amateur work comes from the sources read plus inference **[S/I]**:

- One light source, conventionally top-left, with every shadow on the same side.
- At most 2–3 ramps per piece.
- No isolated cells whose colour belongs to no neighbouring ramp.
- No cell-by-cell alternation of glyphs on top of the built-in dithers.
- Letters readable as flat silhouettes before any rendering is added.
- Art that survives `letter-spacing: 0` and a line height of exactly one cell.

The scene's own test is readability **[N]** ([pouët.net](https://www.pouet.net/topic.php?which=434&page=2)). ASCII-only (7-bit) art belongs wherever the font isn't controlled: clipboard text, logs, CLI help, READMEs **[I]**.

## Rendering on the web: integer geometry, glyph layers and real controls

**Crispness comes from geometry.** The font's metrics make crispness achievable **[M]**:

- 1600 units per em, ascent 1200, descent 400, line gap 0.
- Advance 800 for every glyph.
- Every contour on multiples of 100 units, so at `font-size: 16px` every edge lands on a whole CSS pixel.
- No hinting tables, and a `gasp` entry of grayscale.

At 16 px (or 32, 48…) with a 16 px line height, **one cell is exactly 8×16 CSS px**, `1ch` = 8 px and `1lh` = 16 px. The smoothing switches can't be relied on:

- **`-webkit-font-smoothing` works only on macOS** **[N]** ([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/font-smooth)).
- Chromium has a bug titled "-webkit-font-smoothing: none; has no effect", status unconfirmed **[N]** ([Chromium 40635769](https://issues.chromium.org/issues/40635769)).
- WebKitGTK follows the system's fontconfig settings **[N]**.
- Tauri on Linux has an open issue where WebKitGTK 2.50 draws fonts about one weight step bolder, with no workaround **[S]** ([tauri#14286](https://github.com/tauri-apps/tauri/issues/14286)).

What does work is integer geometry. Every glyph run must start on a whole device pixel. Avoid anything that produces half pixels:

- `translate(-50%)` and `margin: auto` in odd-width containers
- `%` and `fr` widths
- fractional `scrollTop`
- non-integer zoom or devicePixelRatio

On a 2× Retina screen each font pixel is exactly 2×2 device pixels, so anti-aliasing barely shows. At 1.25× or 1.5× scaling, blur can't be avoided with an outline font **[I]**.

A base rule for any character-drawn element **[I]**, which leaves ordinary UI text free to reflow:

```css
.tm { font: 16px/16px "WebPlus IBM VGA 8x16", monospace; white-space: pre;
      letter-spacing: 0; word-spacing: 0; font-kerning: none;
      font-variant-ligatures: none; font-synthesis: none; }
.tm-box { width: round(down, 100%, 8px); height: round(down, 100%, 16px); }
```

`round()` snapping is WebTUI's technique **[S]** ([WebTUI view.css](https://github.com/webtui/webtui/tree/main/packages/css/src)). Keep `font-display: block`, so the fallback font's different widths never flash across the grid **[I]**. The single most important repo-specific fix is to **override Floppy's `body { line-height: 1.25 }` with 16 px on every frame, shade and block layer**. The 20 px rows leave a 4 px gap between stacked `║`, `│` and `█` **[S]** (`src/ansiapps-theme.css` lines 72 and 272), **[I]**.

### Draw frames as `aria-hidden` text layers, sized in whole cells

The prior art falls short on this point. **WebTUI draws its boxes with CSS borders** centred by a translate. **TuiCss** uses `border: 6px double`, PNG textures, an 18 px font (resampling a 16 px-native design) and pixel `box-shadow`s. **BOOTSTRA.386** is stock Bootstrap in a DOS font **[S]** ([WebTUI](https://github.com/webtui/webtui), [TuiCss](https://github.com/vinibiavatti1/TuiCss/blob/master/dist/tuicss.css), [BOOTSTRA.386](https://github.com/kristopolous/BOOTSTRA.386)). Real glyph grids exist only in terminal emulators. **xterm.js** even draws box and block characters as vector paths per cell to keep lines continuous. It also corrects DPR rounding with a measured `letter-spacing` **[S]** ([xterm.js](https://github.com/xtermjs/xterm.js)). Textual's web mode streams into xterm.js **[S]** ([textual-serve](https://github.com/Textualize/textual-serve)).

For an app with real HTML controls, the recommended method is **[I]**:

1. Give each framed element an absolutely positioned `<pre aria-hidden="true">` layer.
2. Watch it with a shared ResizeObserver and compute `cols = floor(w/8)`, `rows = floor(h/16)`.
3. When either count changes, and only then, rewrite one text node: `╔` + `═`×(cols−2) + `╗`, the side rows, the bottom row, and the title spliced into the top row.
4. Snap the box's size in CSS rather than in the callback. Resizing an observed element inside its own callback triggers the "ResizeObserver loop" error **[N]** ([TrackJS](https://trackjs.com/javascript-errors/resizeobserver-loop-completed-with-undelivered-notifications/)).
5. Put `contain: strict` on the layers, so rewriting their text never reflows the content.

Two cheaper variants exist, both needing sizes that are already whole cells: pure CSS pseudo-elements holding long clipped runs (`content: "════…"`), and a column of `║` built with `"║\A║\A…"`.

Map the other components onto real controls **[I]**:

| Component | Faithful text rendering | Underlying control |
|---|---|---|
| Window shadow | two `backdrop-filter: brightness(.35)` strips, 16 px wide on the right and 16 px tall below, offset 16 px (recolours rather than hides what's beneath); cheap version `box-shadow: 16px 16px 0 rgb(0 0 0/.6)` in exact cell multiples | none (decorative) |
| Button | label plus `aria-hidden` `▄` and ` ▀▀▀▀`; `:active` drops the shadow and shifts the label one column right. **Not used by the ansiapps apps:** since 2026-09-30 their buttons are flat and never move, showing state by color alone (`ansiapps-theme.md`, "Buttons are just clickable") | `<button>` |
| Checkbox / radio | `[X]` / `(•)` in `label::before` (the 98.css pattern **[S]**: [98.css](https://github.com/jdan/98.css/blob/main/style.css)) | real input with `opacity: 0`, still focusable |
| Scroll bar | `aria-hidden` column `▲▒▒■▒▒▼` synced on `scroll`; scroll in 16 px steps | native scrolling container, native bar hidden (WebKit scrollbar pseudo-elements can't hold text) |
| Progress | `"█".repeat(n)+"░".repeat(cols-n)`, or WebTUI's `round(nearest, …, 1ch)` fill **[S]** | `<progress>` or `role="progressbar"` |
| Select | text `↓` or `▼` beside it | native `<select>` (popup stays native) |
| Focus | reverse video, plus a `►` marker or forced-colours outline so colour isn't the only cue | `:focus-visible` |
| Cursor / blink | 2 px bar at rows 13–14 of the cell; `animation: 457ms steps(1) infinite` (a full cycle) | disabled under `prefers-reduced-motion` |

### Accessibility: hide the drawing, keep the semantics, and check the palette's contrast

Screen readers read box characters and CSS generated content aloud, so every decorative glyph layer must be hidden. The safest form is an `aria-hidden="true"` element. The `content: "═" / ""` alt syntax is a secondary option, since Safari 17.4 and Firefox 128 support it only per recent snippets **[N]** ([Roselli](http://adrianroselli.com/2020/10/alternative-text-for-css-generated-content.html)). Three rules follow:

- **Art that means something** is a single `role="img"` with an `aria-label`, following W3C technique H86 and MDN **[S]** ([WCAG H86](https://raw.githubusercontent.com/w3c/wcag/main/techniques/html/H86.html), [MDN img role](https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/accessibility/aria/reference/roles/img_role/index.md)).
- **Glyph-only controls** get names: `[■]` becomes `aria-label="Close"`.
- **Titles drawn into a frame string** also exist as real headings.

xterm.js follows the same split. It hides its visual rows and builds a separate accessibility tree **[S]**. A strict grid can't honour WCAG 1.4.12's text-spacing overrides. Floppy's modern theme is the accessible alternative, and that should be stated in the docs **[I]**.

The period palette mostly passes WCAG AA (4.5:1), but several of **Turbo Vision's own highlight colours fail** **[C]**:

| Pair | Ratio | Verdict |
|---|---|---|
| White on blue / yellow on blue | 13.29 / 12.46 | AAA |
| Light gray on blue | 5.72 | AA |
| Black on light gray / black on cyan | 9.04 / 7.33 | AAA |
| Black on green (TV button) | 6.75 | AA |
| Cyan on blue (TV scroll bar) | 4.64 | AA, barely |
| White on red / yellow on red | 7.75 / 7.27 | AAA |
| Red hotkey on light gray (TV menu) | 3.34 | **fails** |
| White on green (TV focused button) | 3.11 | **fails** |
| Yellow on green (TV button hotkey) | 2.92 | **fails** |
| White on cyan | 2.86 | **fails** |
| Light cyan on green (TV default button) | 2.54 | **fails** |
| White on light gray | 2.32 | **fails** |
| Dark gray on black / dark gray on light gray | 2.82 / 3.21 | fail; use only for disabled items (WCAG exempts them) |

Where a failing TV pair carries meaning, keep the layout and swap the colour. For example, show focus with black on cyan, or mark a hotkey with a brighter colour on a darker background **[I]**.

### Do and don't

| Do | Don't |
|---|---|
| Use only the 40 CP437 box pieces, with each axis uniformly single or double | Use `╭ ━ ┄ ╱`, or mix single and double on one axis; the fallback glyph breaks the row |
| Set `font-size: 16px` and `line-height: 16px` on every drawn layer; scale by 2× or 3× | Use 18 px, `em`-scaled or 1.25 line heights for frames |
| Put every layer at whole-cell offsets from a whole-pixel origin | Centre with `translate(-50%)` or `justify-content: center` in fractional containers |
| Use `#AA5500` brown and the 00/55/AA/FF levels | Use `#AAAA00`, `#A8A8A8` or other bit-shifted values |
| Make shadows 2 columns × 1 row that recolour what's beneath | Use blurred or pixel-offset `box-shadow`s |
| Show state with glyph plus colour (`►`, `[X]`, `(•)`) and a real control underneath | Signal focus by colour alone, or draw controls with no native element |
| Use `√`/`X` for checks and `▲▼►◄↑↓` for arrows | Rely on WebPlus-only `✓ ● □` if the font might change |
| Blink with hard steps, and never under reduced motion | Fade blinks, or blink body text |
| Mark art as `role="img"` + label, or `aria-hidden` | Let screen readers read `═══╗` aloud |

## Conclusion

The research changes the shape of the problem. "Looking like DOS" is not mainly a font choice; it is a discipline of **cell arithmetic, a closed glyph set and attribute-only effects**. Shadows recolour, pressed buttons shift by a whole cell, and frames change weight to show focus. Turbo Vision's source turns what is usually folklore into a checkable specification, and the measured font shows the 8×16 web font can reproduce it pixel for pixel, as long as the stylesheet never introduces half pixels or 20 px rows. The same measurements add constraints that matter in practice: a 7:9 half-block split, a broken `|` and a 2-px vertical stroke.

The gap in prior art is the opportunity. Terminal emulators get the grid right but give up real controls. CSS kits keep the controls but fake the lines. A theme that pairs native, accessible elements with `aria-hidden` glyph layers redrawn per cell count would be more faithful than TuiCss or WebTUI and more accessible than xterm.js. Three things remain open before calling the ANSIapps theme expert-grade:

- **Verify EDIT and Norton Commander attributes** from DOSBox video memory. They are still recollection.
- **Screenshot-test crispness** in WKWebView, WebKitGTK (with the bold-rendering bug) and Chromium at 1×, 1.5× and 2×.
- **Choose deliberately which failing Turbo Vision colour pairs to adjust** for contrast, rather than copying them faithfully.
