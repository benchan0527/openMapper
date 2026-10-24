#!/usr/bin/env python3
"""Every check of the Chinese translation, in one place.

Run this before pushing: it finds the translations that are missing, the ones that are
broken (a placeholder in the wrong place, a text written in the wrong encoding) and the
characters the font has not.

    python tools/check-i18n.py            # check everything
    python tools/check-i18n.py --fix     # write the Rust table from the tables (and check again)
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TOOLS = ROOT / "tools"

for _stream in (sys.stdout, sys.stderr):
    try:
        _stream.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, OSError):
        pass


def run(name: str, *args: str) -> bool:
    """Run a tool of the tools directory; True when it passed."""
    print(f"--- {name} {' '.join(args)}")
    r = subprocess.run([sys.executable, str(TOOLS / name), *args], cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = (r.stdout or "") + (r.stderr or "")
    for line in out.strip().splitlines():
        print(f"    {line}")
    if r.returncode != 0:
        print(f"    FAILED ({r.returncode})")
        return False
    return True


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fix", action="store_true", help="write the Rust table from the tables before checking")
    args = ap.parse_args()

    steps: list[tuple[str, list[str]]] = []
    if args.fix:
        steps.append(("generate-lang.py", []))
    else:
        steps.append(("generate-lang.py", ["--check"]))
    steps += [
        ("check-lang.py", []),
        ("extract-i18n.py", ["--check"]),
        ("subset-cjk.py", ["--check"]),
    ]

    failed = [name for name, a in steps if not run(name, *a)]
    if failed:
        print(f"\n{len(failed)} checks failed: {', '.join(failed)}")
        return 1
    print("\neverything is in order.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
