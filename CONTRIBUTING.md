# Contributing to openMapper

Thank you for helping. A few rules keep the project easy to work on.

## Issues

* Write in English; one problem per issue; use the latest release.
* For a bug say what you did, what you expected and what happened, and attach the map's `global.cfg`
  path and the end of the log if the program stopped. A screenshot helps.
* Security problems are reported privately, see [SECURITY.md](SECURITY.md).

## Pull requests

* One change per pull request, based on the latest `main`.
* `cargo build --release` and `cargo test --release` pass (CI runs them).
* Keep large files out of the repository: link screenshots and videos in the pull request and keep
  test data small.
* Write code that reads like the code around it: its naming, its comment density, its idiom. The
  interface texts come in English and Russian (`tr("English", "Русский")`); a third language is a
  table beside them (see [Translations](#translations)).
* New code goes into new files where it can: shared files (`src/ui.rs`, `src/editor.rs`,
  `src/app.rs`, `src/main.rs`) only get small hooks. See [docs/ROADMAP.md](docs/ROADMAP.md).
* Commit messages say what changed for the user, in plain words.

## Building

```bash
cargo run --release
```

Linux needs `libgtk-3-dev`, `libxkbcommon-dev` and `libwayland-dev`. The logo and icons are made by
`python3 tools/make-icons.py` (Pillow, fontTools, skia-pathops).

## Translations

A text is written where it is used, in English and Russian:

```rust
tr("Save", "Сохранить")
```

Another language does not touch those lines: it is a table found by the English, so a text without
an entry is shown in English. The Chinese (Hong Kong) one is `src/lang/zh_hk.rs`, made from
`tools/i18n/zh_hk*.tsv` — one pair a line, the English, a tab, the translation:

    Save	儲存

To change a translation, edit the `.tsv`, then:

```bash
python3 tools/generate-lang.py     # write src/lang/zh_hk.rs from the tables
python3 tools/check-i18n.py        # every text translated, placeholders and characters in order
```

`tools/check-i18n.py` checks that no `tr()` text is left without a translation, that a translation
keeps the `{…}` placeholders of its English, and that the Chinese font has every character of the
texts. CI runs it. A new language needs a table of its own, a `Lang` in `src/i18n.rs`, and a line in
`lang_menu()` of `src/ui.rs`.

The Chinese font (`assets/fonts/NotoSansCJKtc-Subset.otf`) is a subset made by
`tools/subset-cjk.py`: the full Noto Sans CJK TC is 16 MB and stays out of the repository. The
subset is kept under a megabyte, so when many new characters are added, `COMMON_COUNT` in that
script has to come down (`--check` says when it is out of date).
