#!/usr/bin/env python3
"""Write the Chinese translation table into the Rust source.

The translations live in `tools/i18n/zh_hk.tsv` (english, a tab, chinese): a translator
can work in a text editor, and the Rust file is made from it, so nothing has to be
escaped by hand.

    python tools/generate-lang.py            # write src/lang/zh_hk.rs
    python tools/generate-lang.py --check    # only the exit code (0: the file is current)
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TSV = ROOT / "tools" / "i18n" / "zh_hk.tsv"
RUST = ROOT / "src" / "lang" / "zh_hk.rs"

HEADER = '''//! The Chinese (Hong Kong) translations of openMapper's texts.
//!
//! The English is the key: the text the program shows in English is how a translation is
//! found, so the call sites keep their two languages and a new language needs only a table.
//! A text without a translation is shown in English.
//!
//! Made by `tools/generate-lang.py` from `tools/i18n/zh_hk.tsv`. Do not edit it here.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The translations, by the English they translate.
fn map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| TABLE.iter().copied().collect())
}

/// The Chinese of the English text, if it has one.
pub fn lookup(en: &str) -> Option<&'static str> {
    map().get(en).copied()
}

/// The extra words a command is found by in Chinese (besides its Chinese name).
pub fn search_words(en: &str) -> &'static str {
    SEARCH_WORDS.iter().find(|(k, _)| *k == en).map(|(_, v)| *v).unwrap_or("")
}

/// The texts, by the English they translate: (english, chinese).
pub const TABLE: &[(&str, &str)] = &[
'''

FOOTER = '''];

/// The extra words a command is found by in Chinese, besides its Chinese name.
///
/// The command panel (Ctrl+K) looks for what is typed in the names of the commands and
/// the words beside them. A user types Chinese, so every command needs Chinese words of
/// its own to be found by.
pub const SEARCH_WORDS: &[(&str, &str)] = &[
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_translated_text_is_found_by_its_english() {
        assert_eq!(lookup("Save"), Some("儲存"));
        assert_eq!(lookup("Nothing here"), None);
    }

    #[test]
    fn the_table_has_no_text_twice() {
        let mut seen = std::collections::HashSet::new();
        for (en, _) in TABLE {
            assert!(seen.insert(*en), "the text is in the table twice: {en}");
        }
    }

    #[test]
    fn every_translation_is_not_empty() {
        for (en, zh) in TABLE {
            assert!(!zh.trim().is_empty(), "the translation of {en} is empty");
        }
    }

    #[test]
    fn the_placeholders_of_a_translation_are_the_ones_of_its_english() {
        fn names(s: &str) -> Vec<&str> {
            let mut v = Vec::new();
            let mut rest = s;
            while let Some(a) = rest.find('{') {
                let b = rest[a..].find('}').map(|i| a + i).unwrap_or(rest.len());
                v.push(&rest[a..=b]);
                rest = &rest[b.min(rest.len())..];
                if rest.len() >= b {
                    rest = &rest[1..];
                }
            }
            v.sort_unstable();
            v
        }
        for (en, zh) in TABLE {
            assert_eq!(names(en), names(zh), "the placeholders of {en} differ");
        }
    }

    #[test]
    fn every_search_word_is_not_empty() {
        for (en, words) in SEARCH_WORDS {
            assert!(!words.trim().is_empty(), "the search words of {en} are empty");
        }
    }
}
'''


def read_tsv() -> list[tuple[str, str]]:
    """The (english, chinese) pairs of the table, each text once."""
    out: list[tuple[str, str]] = []
    seen: set[str] = set()
    files = sorted((ROOT / "tools" / "i18n").glob("zh_hk*.tsv"))
    if not files:
        return out
    for path in files:
        # (the search words are not translations: they are in their own file)
        if path.name.endswith("_search.tsv"):
            continue
        for no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if "\t" not in line:
                print(f"{path.relative_to(ROOT)}:{no}: the line has no tab", file=sys.stderr)
                continue
            en, zh = line.split("\t", 1)
            # (the same text may be in several files: it is translated once)
            if en in seen:
                continue
            seen.add(en)
            out.append((en, zh))
    # (the texts that hold a line break are kept apart: a table row cannot hold one)
    out.extend(MULTILINE)
    return out


def read_search_words() -> list[tuple[str, str]]:
    """The (english, chinese words) pairs of the search words."""
    path = ROOT / "tools" / "i18n" / "zh_hk_search.tsv"
    out: list[tuple[str, str]] = []
    if not path.exists():
        return out
    for no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if "\t" not in line:
            print(f"{path.relative_to(ROOT)}:{no}: the line has no tab", file=sys.stderr)
            continue
        en, words = line.split("\t", 1)
        out.append((en, words))
    return out


def rust_str(s: str) -> str:
    """The Rust literal of the text."""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n") + '"'


# The texts that hold a line break: a table row cannot hold one, so they are here.
MULTILINE: list[tuple[str, str]] = [
    (
        "A depot file tells the buses of the map their destinations, the names their displays and announcements give the stops, and the routes the IBIS knows (the line, the route's code, its stops). OMSI 2 finds it in the map's folder, openOMSI in the content's HOFs folder: saving puts it in both.",
        "車廠檔案為地圖的巴士提供終點、顯示屏與廣播所用的站名，以及 IBIS 所知的路線（路線、路線代碼、其車站）。OMSI 2 在地圖的資料夾中尋找它，openOMSI 則在內容的 HOFs 資料夾中尋找：儲存時會同時放入兩處。",
    ),
    (
        "Load a big map 100 tiles at a time, as OMSI 2's own editor does?\n\nOnly the 100 tiles round the camera are kept in graphics memory; the others are hidden until you click on the empty zone, where the next 100 are loaded. It keeps very big maps light. You can change it any time in View > Load 100 tiles at a time.",
        "像 OMSI 2 自身的編輯器那樣，每次載入 100 個 Tile 來載入大地圖？\n\n只有相機周圍的 100 個 Tile 會留在圖形記憶體中；其餘會隱藏，直到你點擊空白區域，那時會載入下 100 個。這令非常大的地圖保持輕快。你隨時可在「檢視 > 每次載入 100 個 Tile」更改。",
    ),
    (
        "Pick a line on the left, or make a new one.\n\nA line has trips (a way from stop to stop) and tours (one bus's day: which trip it drives when).",
        "在左側選擇一條路線，或新增一條。\n\n路線有班次（由站到站的路徑）及行車週期（一架巴士的一天：何時執行哪個班次）。",
    ),
]


def build() -> str:
    pairs = read_tsv()
    body = "".join(f"    ({rust_str(en)}, {rust_str(zh)}),\n" for en, zh in pairs)
    words = "".join(f"    ({rust_str(en)}, {rust_str(w)}),\n" for en, w in read_search_words())
    return HEADER + body + FOOTER.replace("pub const SEARCH_WORDS: &[(&str, &str)] = &[\n];", f"pub const SEARCH_WORDS: &[(&str, &str)] = &[\n{words}];")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="only the exit code (0: the file is current)")
    args = ap.parse_args()
    text = build()
    if args.check:
        if not RUST.exists() or RUST.read_text(encoding="utf-8") != text:
            print("src/lang/zh_hk.rs is not what the table says: run tools/generate-lang.py")
            return 1
        print("src/lang/zh_hk.rs is current.")
        return 0
    RUST.parent.mkdir(parents=True, exist_ok=True)
    RUST.write_text(text, encoding="utf-8")
    print(f"wrote {RUST.relative_to(ROOT)} ({len(read_tsv())} texts, {len(read_search_words())} search words)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
