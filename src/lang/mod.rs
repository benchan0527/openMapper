//! The translations that are not written side by side with the English.
//!
//! The English stays the key: a translation is found by the text it translates, so the
//! call sites keep their two languages and a new language only needs a table.

pub mod zh_hk;

use ab_glyph::Font;

/// The font the Chinese is written with (the subset of Noto Sans CJK TC).
pub const CJK_FONT: &[u8] = include_bytes!("../../assets/fonts/NotoSansCJKtc-Subset.otf");

/// The characters a language's texts are written with, that the font has not.
///
/// The Chinese font is a subset (made by `tools/subset-cjk.py`), so a text may hold a
/// character the font has not: the text would show a square. This finds them.
pub fn missing_glyphs(lang: crate::i18n::Lang) -> Vec<char> {
    let texts: Vec<&str> = match lang {
        crate::i18n::Lang::ZhHk => zh_hk::TABLE.iter().map(|(_, zh)| *zh).collect(),
        // (the English and the Russian are in the fonts of the system already)
        _ => return Vec::new(),
    };
    let Ok(font) = ab_glyph::FontRef::try_from_slice(CJK_FONT) else {
        return Vec::new();
    };
    let mut missing: Vec<char> = texts
        .iter()
        .flat_map(|t| t.chars())
        .filter(|c| !c.is_ascii() && font.glyph_id(*c) == ab_glyph::GlyphId::default())
        .collect();
    missing.sort_unstable();
    missing.dedup();
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chinese_font_has_every_character_of_the_translations() {
        let missing = missing_glyphs(crate::i18n::Lang::ZhHk);
        assert!(missing.is_empty(), "the Chinese font is missing: {missing:?}");
    }

    #[test]
    fn the_font_file_is_read_as_it_should() {
        // (the reading of the font has to find the Latin letters and the Chinese)
        use ab_glyph::{Font, FontRef};
        let font = FontRef::try_from_slice(CJK_FONT).expect("the Chinese font is not a font");
        let has = |c: char| font.glyph_id(c) != ab_glyph::GlyphId::default();
        assert!(has('A'));
        assert!(has('0'));
        assert!(has('儲'), "the font has no Chinese in it");
    }

    #[test]
    fn egui_shows_every_character_of_the_translations() {
        // (the whole way the program shows the text: the fonts of the program, the family
        // the text is written in, and the fallback to the Chinese font behind them)
        let ctx = egui::Context::default();
        ctx.set_fonts(crate::icons::fonts());
        // (the fonts are only there after the first frame)
        ctx.begin_pass(egui::RawInput::default());
        let _ = ctx.end_pass();
        let text = "儲存檔案 — 選單、工具、地圖";
        let id = egui::FontId::proportional(14.0);
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&id, text)), "egui cannot show the Chinese");
        // (the Latin is still there too)
        assert!(ctx.fonts_mut(|f| f.has_glyphs(&id, "Save")));
    }
}
