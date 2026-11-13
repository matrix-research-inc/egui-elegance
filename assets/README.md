# Bundled assets

## `elegance-symbols.ttf`

A ~15 KB font combining a subset of DejaVu Sans (arrows, math ellipsis,
modifier keys, delete keys, return arrows, disclosure triangles) with a
small set of [Lucide](https://lucide.dev) UI icons (upload, download,
search, pin, copy, alert, network, zoom-in, zoom-out, power, trash, pencil,
check, x).

The source font, DejaVu Sans, is derived from Bitstream Vera. Its license
permits redistribution of modified/subset fonts provided the font is renamed
to not contain "Bitstream" or "Vera" — this subset is renamed
`Elegance Symbols`. The full license text (covering both Bitstream Vera and
Arev contributions) is preserved in `elegance-symbols-LICENSE.txt`.

### Glyph set

| Codepoint | Glyph | Description               |
| --------- | ----- | ------------------------- |
| U+2190    | ←     | leftwards arrow           |
| U+2191    | ↑     | upwards arrow             |
| U+2192    | →     | rightwards arrow          |
| U+2193    | ↓     | downwards arrow           |
| U+21A9    | ↩     | leftwards arrow with hook |
| U+21B2    | ↲     | downwards arrow with corner left |
| U+21B5    | ↵     | downwards arrow with corner leftwards |
| U+21E5    | ⇥     | rightwards arrow to bar (tab key) |
| U+21E7    | ⇧     | upwards white arrow (shift key) |
| U+21EA    | ⇪     | upwards white arrow from bar (caps lock) |
| U+22EE    | ⋮     | vertical ellipsis         |
| U+22EF    | ⋯     | midline horizontal ellipsis |
| U+2303    | ⌃     | up arrowhead (control key) |
| U+2318    | ⌘     | place of interest (command key) |
| U+2325    | ⌥     | option key                |
| U+2326    | ⌦     | erase to the right (forward delete) |
| U+2327    | ⌧     | x in rectangle (clear key) |
| U+232B    | ⌫     | erase to the left (backspace) |
| U+23CE    | ⏎     | return symbol             |
| U+25B4    | ▴     | small up-pointing triangle |
| U+25B8    | ▸     | small right-pointing triangle |
| U+25BE    | ▾     | small down-pointing triangle |
| U+25C2    | ◂     | small left-pointing triangle |
| U+2713    | ✓     | check mark (Lucide `check` — overrides DejaVu) |
| U+2717    | ✗     | ballot x (Lucide `x` — overrides DejaVu) |
| U+E000    | (PUA) | upload tray (Lucide `upload`) |
| U+E001    | (PUA) | download tray (Lucide `download`) |
| U+E002    | (PUA) | magnifier (Lucide `search`) |
| U+E003    | (PUA) | pin (Lucide `pin`) |
| U+E004    | (PUA) | copy / duplicate (Lucide `copy`) |
| U+E005    | (PUA) | circular alert (Lucide `circle-alert`) |
| U+E006    | (PUA) | network / hub (Lucide `network`) |
| U+E007    | (PUA) | zoom-in / magnifier with plus (Lucide `zoom-in`) |
| U+E008    | (PUA) | zoom-out / magnifier with minus (Lucide `zoom-out`) |
| U+E009    | (PUA) | power (Lucide `power`) |
| U+E00A    | (PUA) | trash / delete (Lucide `trash-2`) |
| U+E00B    | (PUA) | pencil / edit (Lucide `pencil`) |
| U+E00C    | (PUA) | plus / add (Lucide `plus`) |
| U+E00D    | (PUA) | terminal prompt (Lucide `terminal`) |
| U+E00E    | (PUA) | key (Lucide `key-round`) |
| U+E00F    | (PUA) | refresh / sync (Lucide `refresh-cw`) |
| U+E010    | (PUA) | arrow up (Lucide `arrow-up`) |
| U+E011    | (PUA) | arrow down (Lucide `arrow-down`) |
| U+E012    | (PUA) | info (Lucide `info`) |
| U+E013    | (PUA) | warning triangle (Lucide `triangle-alert`) |
| U+E014    | (PUA) | circular x / error (Lucide `circle-x`) |
| U+E015    | (PUA) | circular check / success (Lucide `circle-check`) |
| U+E016    | (PUA) | settings gear (Lucide `settings`) |
| U+E017    | (PUA) | menu / hamburger (Lucide `menu`) |
| U+E018    | (PUA) | arrow left (Lucide `arrow-left`) |
| U+E019    | (PUA) | arrow right (Lucide `arrow-right`) |
| U+E01A    | (PUA) | external link (Lucide `external-link`) |
| U+E01B    | (PUA) | chevron right (Lucide `chevron-right`) |
| U+E01C    | (PUA) | chevron down (Lucide `chevron-down`) |
| U+E01D    | (PUA) | filter (Lucide `funnel`) |
| U+E01E    | (PUA) | eye / show (Lucide `eye`) |
| U+E01F    | (PUA) | eye off / hide (Lucide `eye-off`) |
| U+E020    | (PUA) | home (Lucide `house`) |
| U+E021    | (PUA) | lock (Lucide `lock`) |
| U+E022    | (PUA) | save (Lucide `save`) |
| U+E023    | (PUA) | folder (Lucide `folder`) |
| U+E024    | (PUA) | open folder (Lucide `folder-open`) |
| U+E025    | (PUA) | play (Lucide `play`) |
| U+E026    | (PUA) | pause (Lucide `pause`) |
| U+E027    | (PUA) | stop (Lucide `square`) |
| U+E028    | (PUA) | file (Lucide `file`) |
| U+E029    | (PUA) | text file (Lucide `file-text`) |
| U+E02A    | (PUA) | server (Lucide `server`) |
| U+E02B    | (PUA) | monitor (Lucide `monitor`) |
| U+E02C    | (PUA) | two columns / split right (Lucide `columns-2`) |
| U+E02D    | (PUA) | two rows / split down (Lucide `rows-2`) |
| U+E02E    | (PUA) | image file (Lucide `file-image`) |

### Regenerating the subset

To add more glyphs or update the source font, re-run the subset step with
the extended codepoint list:

```sh
# Download source
curl -LO https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-fonts-ttf-2.37.tar.bz2
tar -xjf dejavu-fonts-ttf-2.37.tar.bz2

# Subset
pip install fonttools
pyftsubset dejavu-fonts-ttf-2.37/ttf/DejaVuSans.ttf \
    --output-file=assets/elegance-symbols.ttf \
    --unicodes='U+2190-2193,U+21A9,U+21B2,U+21B5,U+21E5,U+21E7,U+21EA,U+22EE,U+22EF,U+2303,U+2318,U+2325,U+2327,U+232B,U+2326,U+23CE,U+25B4,U+25B8,U+25BE,U+25C2,U+2713,U+2717' \
    --no-hinting --desubroutinize --name-IDs='*'

# Then rename via fontTools (see scripts/rename-symbol-font.py if present).

# Re-bake the Lucide icon set on top of the DejaVu subset.
pip install picosvg fonttools
python3 scripts/update_lucide_glyphs.py
```

Any new codepoints must also be added to the table above and any consumer-
facing documentation that lists the bundled glyphs.

### Imported (Lucide) glyphs

The U+2713 / U+2717 overrides and every Private Use Area entry in the
table above are baked in by `scripts/update_lucide_glyphs.py`, which
fetches each icon's SVG from a pinned [Lucide](https://lucide.dev)
release tag, flattens the stroked paths via `picosvg`, and writes the
result through `fontTools`. Lucide is dual-licensed (ISC + MIT for
Feather-derived icons); see `lucide-LICENSE.txt`. The list of glyphs
to bake is hard-coded near the top of the script — edit
`LUCIDE_GLYPHS` and re-run to add or update an icon.

The matching Rust constants in `src/lib.rs` (`pub mod glyphs`) must
stay in sync with the script's codepoint assignments.
