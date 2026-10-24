#!/usr/bin/env python3
"""Show some of the translations (to read them without hunting in the files)."""

from __future__ import annotations

import argparse
import pathlib
import random
import sys

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
ROOT = pathlib.Path(__file__).resolve().parent.parent


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--count", type=int, default=20, help="how many to show")
    ap.add_argument("--seed", type=int, default=0, help="the seed of the choice")
    ap.add_argument("--grep", default="", help="only the texts with this in them")
    args = ap.parse_args()

    rows: list[tuple[str, str]] = []
    for path in sorted((ROOT / "tools" / "i18n").glob("zh_hk*.tsv")):
        if path.name.endswith("_search.tsv"):
            continue
        for line in path.read_text(encoding="utf-8").splitlines():
            if "\t" in line:
                en, zh = line.split("\t", 1)
                rows.append((en, zh))
    if args.grep:
        rows = [r for r in rows if args.grep.lower() in r[0].lower()]
    random.Random(args.seed).shuffle(rows)
    for en, zh in rows[: args.count]:
        print(f"{en}\n  → {zh}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
