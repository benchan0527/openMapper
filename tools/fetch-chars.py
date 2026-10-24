#!/usr/bin/env python3
"""Refresh the character list the Chinese font subset is made of.

The subset keeps the characters openMapper shows: the Chinese of the translations, the
common characters of a Traditional Chinese text and the ones Hong Kong adds (HKSCS).
The lists come from the internet, so they are cached in `tools/i18n/chars.txt` and only
refreshed when this script is run.

    python tools/fetch-chars.py            # refresh tools/i18n/chars.txt
    python tools/fetch-chars.py --check    # only the exit code (0: the list is there)
"""

from __future__ import annotations

import argparse
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tools" / "i18n" / "chars.txt"

# The common characters of a Chinese (Cantonese) text, by how often they are used.
CANTONESE = "https://raw.githubusercontent.com/hardmaru/cantonese-list/master/cantonese.txt"
# The characters Hong Kong adds to the Big5 ones (HKSCS: the Hong Kong supplementary
# character set).
HKSCS = "https://raw.githubusercontent.com/rime-aca/character_set/master/HKSCS.txt"


def get(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "openMapper/tools"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read().decode("utf-8")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="only the exit code (0: the list is there)")
    args = ap.parse_args()

    if args.check:
        if not OUT.exists():
            print(f"{OUT.relative_to(ROOT)} is missing: run tools/fetch-chars.py")
            return 1
        print(f"{OUT.relative_to(ROOT)} holds {len(OUT.read_text(encoding='utf-8').strip())} characters.")
        return 0

    chars: list[str] = []
    seen: set[str] = set()
    # (the common characters first, in the order of how often they are used: the font
    # subset keeps the first ones of them)
    for url in (CANTONESE, HKSCS):
        try:
            text = get(url)
        except Exception as e:  # (the network may be away: the cached list stays)
            print(f"could not fetch {url}: {e}", file=sys.stderr)
            continue
        for line in text.splitlines():
            for c in line.strip():
                if not c.isspace() and c not in seen:
                    seen.add(c)
                    chars.append(c)
    if not chars:
        print("nothing was fetched: the list was not written.", file=sys.stderr)
        return 1
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("".join(chars) + "\n", encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)} ({len(chars)} characters)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
