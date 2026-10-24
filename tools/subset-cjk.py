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
COMMON_COUNT = 800

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


def chinese_of_the_texts() -> str:
    """The characters of the Chinese translations and of the search words."""
    chars: set[str] = set()
    for path in sorted((ROOT / "tools" / "i18n").glob("zh_hk*.tsv")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or "\t" not in line:
                continue
            _en, zh = line.split("\t", 1)
            chars.update(zh)
    # (the Rust file too: some texts are not in a table row)
    rust = SRC / "lang" / "zh_hk.rs"
    if rust.exists():
        chars.update(rust.read_text(encoding="utf-8"))
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
    # (only the characters of the Basic Multilingual Plane: the font has the others too)
    return {c for c in chars if ord(c) < 0x10000}


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

    if not FULL.exists():
        print(f"{FULL.relative_to(ROOT)} is missing: fetching it", file=sys.stderr)
        try:
            fetch_full_font()
        except Exception as e:
            print(f"could not fetch the full font ({e}).", file=sys.stderr)
            print("download it by hand: " + FULL_URL, file=sys.stderr)
            return 1

    chars = wanted()
    # (the characters the full font does not have are egui's own: they are not needed)
    chars = in_the_full_font(chars)
    unicodes = sorted({ord(c) for c in chars})

    if args.check:
        if not SUBSET.exists():
            print(f"{SUBSET.relative_to(ROOT)} is missing: run tools/subset-cjk.py")
            return 1
        from fontTools.ttLib import TTFont

        font = TTFont(SUBSET, lazy=True)
        have = set(font.getBestCmap())
        font.close()
        missing = sorted(c for c in chars if ord(c) not in have)
        if missing:
            print(f"the subset is missing {len(missing)} characters: {''.join(missing[:40])}")
            return 1
        print(f"the subset has every character it needs ({len(chars)} characters).")
        return 0

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
