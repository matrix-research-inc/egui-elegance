#!/usr/bin/env python3
"""Bake Lucide icons into assets/elegance-symbols.ttf.

Each entry in ``LUCIDE_GLYPHS`` is mapped to a Unicode codepoint and
written into the font as a TrueType outline. Run from the repo root:

    python3 scripts/update_lucide_glyphs.py

The script fetches each icon's SVG from Lucide's GitHub release tag
``LUCIDE_TAG`` (so the bake is reproducible against a pinned version),
flattens the stroked paths to filled paths via ``picosvg``, then writes
TrueType outlines via ``fontTools``.

Lucide is dual-licensed: the icons derived from Cole Bemis's Feather
project are MIT, the rest are ISC. Both are compatible with elegance's
MIT/Apache-2.0 dual license. Full text in `assets/lucide-LICENSE.txt`.

Dependencies (``pip install picosvg fonttools``):

  * picosvg — flattens stroked SVG paths into filled paths.
  * fontTools — parses paths and writes TrueType outlines.
"""

import os
from http.client import HTTPException, HTTPSConnection
from pathlib import Path

from fontTools.pens.reverseContourPen import ReverseContourPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.svgLib.path import parse_path
from fontTools.ttLib import TTFont
from picosvg.svg import SVG

LUCIDE_TAG = "1.11.0"
LUCIDE_HOST = "raw.githubusercontent.com"
LUCIDE_PATH = f"/lucide-icons/lucide/{LUCIDE_TAG}/icons"

# Per-address connect/read timeout. Without one, a network where IPv6
# resolves but doesn't route waits out the OS connect timeout (~75 s) on
# each IPv6 address before falling back to IPv4.
FETCH_TIMEOUT_S = 10.0

# (lucide_name, codepoint). Codepoint conventions:
#   * U+2713 / U+2717 (check / x) overwrite DejaVu's character glyphs so
#     anyone typing ✓ ✗ in a RichText automatically gets the Lucide look.
#   * U+E000+ (Private Use Area) for icons with no standard codepoint.
#     Exposed publicly via the `glyphs` module in src/lib.rs — keep the
#     two lists in sync.
LUCIDE_GLYPHS = [
    ("upload", 0xE000),
    ("download", 0xE001),
    ("search", 0xE002),
    ("pin", 0xE003),
    ("copy", 0xE004),
    ("circle-alert", 0xE005),
    ("network", 0xE006),
    ("zoom-in", 0xE007),
    ("zoom-out", 0xE008),
    ("power", 0xE009),
    ("trash-2", 0xE00A),
    ("pencil", 0xE00B),
    ("plus", 0xE00C),
    ("terminal", 0xE00D),
    ("key-round", 0xE00E),
    ("refresh-cw", 0xE00F),
    ("arrow-up", 0xE010),
    ("arrow-down", 0xE011),
    # Status family — back CalloutTone's per-tone marks.
    ("info", 0xE012),
    ("triangle-alert", 0xE013),
    ("circle-x", 0xE014),
    ("circle-check", 0xE015),
    # Broadly useful actions / navigation.
    ("settings", 0xE016),
    ("menu", 0xE017),
    ("arrow-left", 0xE018),
    ("arrow-right", 0xE019),
    ("external-link", 0xE01A),
    ("chevron-right", 0xE01B),
    ("chevron-down", 0xE01C),
    # Situational. `funnel`/`house` are the Lucide names for the icons
    # exposed as FILTER / HOME (intent-named, like check / x).
    ("funnel", 0xE01D),
    ("eye", 0xE01E),
    ("eye-off", 0xE01F),
    ("house", 0xE020),
    ("lock", 0xE021),
    ("save", 0xE022),
    ("folder", 0xE023),
    ("folder-open", 0xE024),
    # Media transport. `square` is exposed as STOP (intent-named).
    ("play", 0xE025),
    ("pause", 0xE026),
    ("square", 0xE027),
    ("check", 0x2713),
    ("x", 0x2717),
]

FONT_PATH = Path(__file__).resolve().parent.parent / "assets" / "elegance-symbols.ttf"

# Icon design canvas. Cap-height matches the existing arrow glyphs so the
# icons sit on the same baseline; advance matches `arrowup` so callers
# can swap glyphs without re-measuring widths.
SVG_VIEWBOX = 24.0
CAP_HEIGHT = 1500.0
ADVANCE = 1716


def fetch_svg(conn: HTTPSConnection, name: str) -> str:
    """Fetch one icon over ``conn``, which is reused across icons so any
    slow address fallback is paid once per run rather than once per icon.
    A missing or renamed icon raises rather than baking nothing."""
    # Allow an offline cache for fast iteration: if LUCIDE_SVG_DIR points
    # to a directory containing `<name>.svg` files, use those; otherwise
    # fetch from the pinned Lucide release tag.
    cache = os.environ.get("LUCIDE_SVG_DIR")
    if cache:
        local = Path(cache) / f"{name}.svg"
        if local.exists():
            return local.read_text()

    path = f"{LUCIDE_PATH}/{name}.svg"
    try:
        body, status = _get(conn, path)
    except (HTTPException, ConnectionError):
        # The server may drop an idle kept-alive connection; reconnect once.
        conn.close()
        body, status = _get(conn, path)
    if status != 200:
        raise RuntimeError(
            f"fetch https://{LUCIDE_HOST}{path}: HTTP {status} "
            f"(is `{name}` in Lucide {LUCIDE_TAG}?)"
        )
    return body.decode("utf-8")


def _get(conn: HTTPSConnection, path: str) -> tuple[bytes, int]:
    conn.request("GET", path)
    resp = conn.getresponse()
    # Read the whole body even on error so the connection stays reusable.
    return resp.read(), resp.status


def build_glyph(svg_text: str):
    pico = SVG.fromstring(svg_text).topicosvg()

    scale = CAP_HEIGHT / SVG_VIEWBOX
    lsb = (ADVANCE - SVG_VIEWBOX * scale) / 2
    # SVG y-down → font y-up. ReverseContourPen flips contour direction
    # back to TT's clockwise-fill convention after the y-flip.
    transform = (scale, 0.0, 0.0, -scale, lsb, CAP_HEIGHT)

    tt_pen = TTGlyphPen(None)
    pen = TransformPen(ReverseContourPen(tt_pen), transform)

    for shape in pico.shapes():
        parse_path(shape.d, pen)

    return tt_pen.glyph()


def install_glyph(font, codepoint: int, glyph) -> str:
    """Install glyph at the given codepoint, reusing the existing glyph
    name if one is already mapped (so we replace in place rather than
    leave the old glyph orphaned). Returns the glyph name used."""
    existing_name = None
    for sub in font["cmap"].tables:
        if sub.isUnicode():
            existing_name = sub.cmap.get(codepoint)
            if existing_name:
                break

    name = existing_name or f"uni{codepoint:04X}"
    glyph_order = font.getGlyphOrder()
    if name not in glyph_order:
        glyph_order.append(name)
        font.setGlyphOrder(glyph_order)
    font["glyf"][name] = glyph
    font["hmtx"][name] = (ADVANCE, 0)

    for sub in font["cmap"].tables:
        if sub.isUnicode():
            sub.cmap[codepoint] = name

    return name


def main():
    font = TTFont(str(FONT_PATH))
    # Force lazy tables to decompile against the current glyph order
    # before we mutate it.
    _ = font["hmtx"].metrics
    _ = font["glyf"].glyphs

    has_pua = False
    # Connects lazily, so a fully cached run never touches the network.
    conn = HTTPSConnection(LUCIDE_HOST, timeout=FETCH_TIMEOUT_S)
    try:
        for icon_name, codepoint in LUCIDE_GLYPHS:
            svg = fetch_svg(conn, icon_name)
            glyph = build_glyph(svg)
            name = install_glyph(font, codepoint, glyph)
            print(f"  U+{codepoint:04X} → {name} (lucide:{icon_name})")
            if codepoint >= 0xE000:
                has_pua = True
    finally:
        conn.close()

    font["maxp"].numGlyphs = len(font.getGlyphOrder())
    if has_pua:
        os2 = font["OS/2"]
        ranges = set(os2.getUnicodeRanges())
        ranges.add(57)  # Private Use Area (U+E000–U+F8FF)
        os2.setUnicodeRanges(ranges)

    font.save(str(FONT_PATH))
    print(f"wrote {FONT_PATH} (Lucide tag {LUCIDE_TAG})")


if __name__ == "__main__":
    main()
