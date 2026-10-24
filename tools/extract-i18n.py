#!/usr/bin/env python3
"""The texts openMapper shows, and what is missing in the Chinese table.

The English is the key: every `tr("…", "…")` in the sources is a text the program shows,
and the Chinese table (`src/lang/zh_hk.rs`) holds its translation. This tool lists them,
so a translator can work through them and a build can check none is left out.

    python tools/extract-i18n.py --missing     # the texts without a Chinese translation
    python tools/extract-i18n.py --template    # a table to fill in
    python tools/extract-i18n.py --check      # only the exit code (0: all translated)
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"
TABLE = SRC / "lang" / "zh_hk.rs"

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", '"': '"', "\\": "\\"}

# (the texts have arrows and dashes the Windows console's code page cannot show)
for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError):
        pass


def literal_at(text: str, i: int) -> tuple[str, int] | None:
    """The string literal at `i` (a quote), as (value, the index after it)."""
    if i >= len(text) or text[i] != '"':
        return None
    j = i + 1
    buf: list[str] = []
    while j < len(text):
        c = text[j]
        if c == "\\" and j + 1 < len(text):
            buf.append(ESCAPES.get(text[j + 1], text[j + 1]))
            j += 2
            continue
        if c == '"':
            return "".join(buf), j + 1
        buf.append(c)
        j += 1
    return None


def calls(text: str) -> list[tuple[str, str]]:
    """Every `tr("…", "…")` in the text, as (english, russian)."""
    found: list[tuple[str, str]] = []
    for m in re.finditer(r"\btr\s*\(", text):
        # the first two string literals of the call, at its own level
        i = m.end()
        depth = 1
        lits: list[str] = []
        while i < len(text) and depth > 0 and len(lits) < 2:
            c = text[i]
            if c == '"':
                got = literal_at(text, i)
                if got is None:
                    break
                lits.append(got[0])
                i = got[1]
                continue
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        if len(lits) == 2:
            found.append((lits[0], lits[1]))
    return found


def sources() -> list[Path]:
    # (the language module itself holds the texts of its own tests: they are not shown)
    return sorted(p for p in SRC.rglob("*.rs") if "lang" not in p.parts and p.name != "i18n.rs")


def table_entries(text: str) -> dict[str, str]:
    """The (english, chinese) pairs the Chinese table holds."""
    out: dict[str, str] = {}
    for m in re.finditer(r'\(\s*("(?:[^"\\]|\\.)*")\s*,\s*("(?:[^"\\]|\\.)*")\s*\)', text):
        # (the texts may hold control characters, which JSON does not allow raw)
        out[json.loads(m.group(1), strict=False)] = json.loads(m.group(2), strict=False)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--missing", action="store_true", help="the texts without a Chinese translation")
    ap.add_argument("--template", action="store_true", help="a Chinese table to fill in")
    ap.add_argument("--check", action="store_true", help="only the exit code (0: every text is translated)")
    ap.add_argument("--json", action="store_true", help="the texts and their translations as JSON")
    ap.add_argument("--by-file", action="store_true", help="the texts of each file, with the line they are on")
    args = ap.parse_args()

    if args.by_file:
        for path in sources():
            lines = path.read_text(encoding="utf-8").splitlines()
            found: list[tuple[int, str]] = []
            for no, line in enumerate(lines, 1):
                for en, _ru in calls(line):
                    found.append((no, en))
            if not found:
                continue
            print(f"### {path.relative_to(ROOT)} ({len(found)})")
            for no, en in found:
                print(f"{no}\t{en}")
            print()
        return 0

    pairs: dict[str, str] = {}
    for path in sources():
        for en, ru in calls(path.read_text(encoding="utf-8")):
            pairs.setdefault(en, ru)

    table = table_entries(TABLE.read_text(encoding="utf-8")) if TABLE.exists() else {}
    missing = sorted(en for en in pairs if en not in table)
    extra = sorted(en for en in table if en not in pairs)

    if args.json:
        # (the texts may hold control characters, which JSON does not allow raw)
        json.dump({"total": len(pairs), "missing": missing, "extra": extra}, sys.stdout, ensure_ascii=False, indent=2, default=str)
        print()
        return 0
    if args.check:
        for en in missing:
            print(f"missing: {en}")
        if missing:
            print(f"\n{len(missing)} of {len(pairs)} texts are not translated.")
            return 1
        print(f"all {len(pairs)} texts are translated.")
        return 0
    if args.template:
        print("//! The Chinese (Hong Kong) translations of openMapper's texts.")
        print("//!")
        print("//! The English is the key: the same text the program shows in English.")
        print()
        print("/// (english, chinese)")
        print("pub const T: &[(&str, &str)] = &[")
        for en in sorted(pairs):
            zh = table.get(en, "")
            print(f"    ({json.dumps(en, ensure_ascii=False)}, {json.dumps(zh, ensure_ascii=False)}),")
        print("];")
        return 0
    for en in missing:
        print(en)
    print(f"\n{len(missing)} of {len(pairs)} texts are not translated ({len(table)} in the table).", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
