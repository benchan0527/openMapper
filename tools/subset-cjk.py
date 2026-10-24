#!/usr/bin/env python3
"""Make a smaller Chinese font out of the big one, with the glyphs the program needs.

The full Noto Sans CJK TC is 16 MB, which makes the installer that much bigger for the
few thousand characters openMapper actually shows. This script keeps only:

- the characters of the Chinese translations and of the Chinese search words,
- the common Traditional Chinese characters (the list of `tools/i18n/chars.txt`, made by
  `tools/fetch-chars.py`), so a text written by hand (a map name, a bus line) still shows,
- the Latin, the punctuation and the symbols of the program's own texts.

    python tools/subset-cjk.py            # write assets/fonts/NotoSansCJKtc-Subset.otf
    python tools/subset-cjk.py --check    # only the exit code (0: the subset is current)

The subset is used by src/icons.rs (see the CJK constant there), so the program has to
be told about it: this script prints what to do.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# (the texts have characters the Windows console's code page cannot show)
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError):
        pass

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
FONTS = ROOT / "assets" / "fonts"
FULL = FONTS / "NotoSansCJKtc-Regular.otf"
SUBSET = FONTS / "NotoSansCJKtc-Subset.otf"
CHARS = ROOT / "tools" / "i18n" / "chars.txt"

# The full font is 16 MB and does not go into the repository (the CI keeps the files of a
# megabyte and more out); it is fetched when the subset is made.
FULL_URL = "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/TraditionalChinese/NotoSansCJKtc-Regular.otf"

# How many of the common characters to keep besides the ones of the translations: the
# more, the bigger the font and the more of a text written by hand it shows.
# (1400 is the most that fits: the subset is then 996600 bytes, just under the megabyte
# the CI keeps out of the repository, so a few more characters of new translations still
# fit. The full list of tools/i18n/chars.txt has 6298 characters, so a bigger subset has
# to be made another way: a handful of more characters need COMMON_COUNT brought down.)
COMMON_COUNT = 1400

# The Latin letters, the digits, the punctuation and the symbols the program uses.
ASCII = "".join(chr(c) for c in range(0x20, 0x7F))
EXTRA = (
    "…—–·×÷±°§¶©®™‰′″ "
    "←↑→↓↔⇄⇒★☆■□▲△▼▽●○◆◇✔✘✓✕"
    "“”‘’„‟«»‹›「」『』《》〈〉【】〔〕"
    "、。，．：；？！～￥＄€£¥"
    "№ℹ⚠✔"
    "─│┌┐└┘├┤┬┴┼═║╔╗╚╝"
    "①②③④⑤⑥⑦⑧⑨⑩"
    "㊣㊤"
    "０１２３４５６７８９"
)

# The symbols of EXTRA that Noto Sans CJK TC has no glyph for: egui shows them with
# another of its fonts (its emoji one), so they are not asked of the subset.
FONT_LACKS = "‟ℹ✔✕✘"

# The ranges a Chinese text is written with (the ideographs and the CJK punctuation):
# the characters outside them are the Latin of the code and of the tests, not the text.
CJK_RANGES = ((0x3400, 0x4DBF), (0x4E00, 0x9FFF), (0x3000, 0x303F), (0xFF00, 0xFFEF))


def is_cjk(c: str) -> bool:
    return any(lo <= ord(c) <= hi for lo, hi in CJK_RANGES)


def chinese_of_the_texts() -> str:
    """The characters of the Chinese translations and of the search words."""
    chars: set[str] = set()
    for path in sorted((ROOT / "tools" / "i18n").glob("zh_hk*.tsv")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or "\t" not in line:
                continue
            _en, zh = line.split("\t", 1)
            chars.update(zh)
    # (the Rust file too: a few texts are built from `tools/generate-lang.py` instead of a
    # table row, and are only in the file it makes. Only its Chinese is taken: the Latin of
    # the code beside it is not text, and the line breaks are not characters.)
    rust = SRC / "lang" / "zh_hk.rs"
    if rust.exists():
        chars.update(c for c in rust.read_text(encoding="utf-8") if is_cjk(c))
    return "".join(sorted(chars))


def common() -> list[str]:
    """The common Traditional Chinese characters, the most used ones first."""
    if not CHARS.exists():
        print(f"{CHARS.relative_to(ROOT)} is missing: run tools/fetch-chars.py", file=sys.stderr)
        return []
    return [c for c in CHARS.read_text(encoding="utf-8").strip() if "一" <= c <= "鿿"]


def wanted() -> set[str]:
    chars = set(chinese_of_the_texts()) | set(ASCII) | set(EXTRA)
    chars.update(common()[:COMMON_COUNT])
    chars -= set(FONT_LACKS)
    # (only the characters of the Basic Multilingual Plane: the font has the others too)
    return {c for c in chars if ord(c) < 0x10000}


def characters_of(font_bytes: bytes) -> set[str]:
    """The characters a font file has, from its cmap (no font library needed).

    Only the segments of the format 4 subtable are read: the characters of a font made
    by `fontTools.subset` are all in them, and a check has to run on a machine without
    `fonttools` installed.
    """
    def u16(b: bytes, i: int) -> int:
        return int.from_bytes(b[i : i + 2], "big")

    def u32(b: bytes, i: int) -> int:
        return int.from_bytes(b[i : i + 4], "big")

    out: set[str] = set()
    if len(font_bytes) < 12:
        return out
    for t in range(u16(font_bytes, 4)):
        rec = 12 + t * 16
        if rec + 16 > len(font_bytes):
            break
        if font_bytes[rec : rec + 4] != b"cmap":
            continue
        table = u32(font_bytes, rec + 8)
        if table + 4 > len(font_bytes):
            continue
        for s in range(u16(font_bytes, table + 2)):
            sub = table + 4 + s * 8
            if sub + 8 > len(font_bytes):
                break
            offset = table + u32(font_bytes, sub + 4)
            if offset + 2 > len(font_bytes) or u16(font_bytes, offset) != 4:
                continue
            seg_x2 = u16(font_bytes, offset + 6)
            ends = offset + 14
            starts = ends + seg_x2 + 2
            for seg in range(seg_x2 // 2):
                if starts + seg * 2 + 2 > len(font_bytes):
                    break
                start = u16(font_bytes, starts + seg * 2)
                end = u16(font_bytes, ends + seg * 2)
                if start == 0xFFFF:
                    continue
                for code in range(start, end + 1):
                    if (ch := chr(code)) not in ("\ufffe", "\uffff"):
                        out.add(ch)
    return out


def in_the_full_font(chars: set[str]) -> set[str]:
    """The characters the full font has: the others egui shows with another font."""
    from fontTools.ttLib import TTFont

    font = TTFont(FULL, lazy=True)
    have = set(font.getBestCmap())
    font.close()
    return {c for c in chars if ord(c) in have}


def fetch_full_font() -> None:
    """Fetch the full Chinese font (it is too big for the repository)."""
    import urllib.request

    req = urllib.request.Request(FULL_URL, headers={"User-Agent": "openMapper/tools"})
    with urllib.request.urlopen(req, timeout=120) as r:
        data = r.read()
    FULL.write_bytes(data)
    print(f"wrote {FULL.relative_to(ROOT)} ({len(data) // 1024} kB)")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="only the exit code (0: the subset is current)")
    args = ap.parse_args()

    chars = wanted()

    if args.check:
        # (a check needs neither the full font nor fonttools: it reads the subset itself)
        if not SUBSET.exists():
            print(f"{SUBSET.relative_to(ROOT)} is missing: run tools/subset-cjk.py")
            return 1
        have = characters_of(SUBSET.read_bytes())
        missing = sorted(c for c in chars if c not in have)
        if missing:
            print(f"the subset is missing {len(missing)} characters: {''.join(missing[:40])}")
            print("run tools/subset-cjk.py to make it again")
            return 1
        print(f"the subset has every character it needs ({len(chars)} characters).")
        return 0

    if not FULL.exists():
        print(f"{FULL.relative_to(ROOT)} is missing: fetching it", file=sys.stderr)
        try:
            fetch_full_font()
        except Exception as e:
            print(f"could not fetch the full font ({e}).", file=sys.stderr)
            print("download it by hand: " + FULL_URL, file=sys.stderr)
            return 1

    # (the characters the full font does not have are egui's own: they are not needed)
    chars = in_the_full_font(chars)
    unicodes = sorted({ord(c) for c in chars})

    from fontTools import subset as ftsubset  # (only needed when the subset is written)

    options = ftsubset.Options()
    options.layout_features = ["*"]
    options.name_IDs = ["*"]
    options.name_legacy = True
    options.notdef_outline = True
    options.recalc_bounds = True
    options.drop_tables = []
    font = ftsubset.load_font(str(FULL), options)
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=unicodes)
    subsetter.subset(font)
    ftsubset.save_font(font, str(SUBSET), options)
    print(f"wrote {SUBSET.relative_to(ROOT)} ({len(unicodes)} characters, {SUBSET.stat().st_size // 1024} kB)")
    print("tell src/icons.rs to use it (the include_bytes! of the subset)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
