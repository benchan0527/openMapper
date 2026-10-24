#!/usr/bin/env python3
"""Check the translation table for the mistakes that are easy to make.

- a text with a replacement character in it (the file was written in the wrong encoding)
- a text whose placeholders differ from the English's
- a text twice in the table
- a translation that is still empty
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TSV_DIR = ROOT / "tools" / "i18n"


def placeholders(s: str) -> list[str]:
    return sorted(re.findall(r"\{[^{}]*\}", s))


def main() -> int:
    bad = 0
    dup = 0
    seen: dict[str, tuple[str, str]] = {}
    for path in sorted(TSV_DIR.glob("zh_hk*.tsv")):
        rel = path.relative_to(ROOT)
        # (the search words are not translations: they are the words a command is found by)
        if path.name.endswith("_search.tsv"):
            continue
        for no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if "\t" not in line:
                print(f"{rel}:{no}: no tab", file=sys.stderr)
                bad += 1
                continue
            en, zh = line.split("\t", 1)
            # a replacement character: the file was written in the wrong encoding
            if "\ufffd" in en or "\ufffd" in zh:
                print(f"{rel}:{no}: a replacement character: {en[:60]}", file=sys.stderr)
                bad += 1
            # the placeholders have to be the same
            if placeholders(en) != placeholders(zh):
                print(f"{rel}:{no}: the placeholders differ: {en[:60]}", file=sys.stderr)
                print(f"    {placeholders(en)} vs {placeholders(zh)}", file=sys.stderr)
                bad += 1
            # an empty translation
            if not zh.strip():
                print(f"{rel}:{no}: the translation is empty: {en[:60]}", file=sys.stderr)
                bad += 1
            # the same text twice: only a problem if the two translations differ
            if en in seen:
                if seen[en][1] != zh:
                    print(f"{rel}:{no}: the text is translated twice, differently: {en[:60]}", file=sys.stderr)
                    print(f"    first in {seen[en][0]}: {seen[en][1]}", file=sys.stderr)
                    print(f"    here: {zh}", file=sys.stderr)
                    bad += 1
                else:
                    dup += 1
            else:
                seen[en] = (f"{rel}:{no}", zh)
    if bad:
        print(f"\n{bad} problems in the translation table.", file=sys.stderr)
        return 1
    print(f"the translation table is clean ({len(seen)} texts, {dup} repeated).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
