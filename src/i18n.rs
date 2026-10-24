//! The languages the editor knows, and the text in the current one.
//!
//! Two languages are written side by side where the text is used: `tr("Save", "Сохранить")`.
//! A third language (Traditional Chinese, Hong Kong) is kept in its own table
//! (`lang::zh_hk`), so the English stays the key and no call site has to change.

use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ru,
    /// Traditional Chinese, as written in Hong Kong.
    ZhHk,
}

pub fn set(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn get() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::Ru,
        2 => Lang::ZhHk,
        _ => Lang::En,
    }
}

pub fn code(l: Lang) -> &'static str {
    match l {
        Lang::En => "en",
        Lang::Ru => "ru",
        Lang::ZhHk => "zh-HK",
    }
}

/// The language a settings value names, else the system's.
pub fn from_code(c: &str) -> Lang {
    let c = c.trim().to_lowercase().replace('_', "-");
    match c.as_str() {
        "ru" => Lang::Ru,
        "en" => Lang::En,
        // Traditional Chinese (any spelling of it the user may have).
        "zh-hk" | "zh-hant-hk" | "zh-hant" | "zh-tw" | "zh-hk-hant" | "cht" | "zhmo" | "zh-mo" => Lang::ZhHk,
        _ => from_system(),
    }
}

/// The language the system asks for, else English.
fn from_system() -> Lang {
    let sys = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|v| std::env::var(v).ok()).find(|v| !v.is_empty()).unwrap_or_default();
    let mac = if sys.is_empty() && cfg!(target_os = "macos") {
        std::process::Command::new("defaults").args(["read", "-g", "AppleLanguages"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default()
    } else {
        String::new()
    };
    let s = format!("{sys}{mac}").to_lowercase().replace('_', "-");
    // (the first language the system names, with its region: "zh-hk", "zh-tw", "ru-ru"…)
    let first = s.split(|c: char| !c.is_ascii_alphabetic() && c != '-').find(|w| w.len() >= 2).unwrap_or("");
    let (lang, region) = match first.split_once('-') {
        Some((l, r)) => (l, r),
        None => (first, ""),
    };
    match lang {
        "ru" | "uk" | "be" => Lang::Ru,
        // Traditional Chinese: Hong Kong, Macau, Taiwan and the explicit "hant".
        "zh" => match region {
            "hk" | "mo" | "tw" | "hant" => Lang::ZhHk,
            // (simplified Chinese is not translated yet: English it is)
            _ => Lang::En,
        },
        _ => Lang::En,
    }
}

/// The text in the current language.
pub fn tr(en: &'static str, ru: &'static str) -> &'static str {
    match get() {
        Lang::En => en,
        Lang::Ru => ru,
        Lang::ZhHk => crate::lang::zh_hk::lookup(en).unwrap_or(en),
    }
}

/// The words a command is found by, in every language: its names and the extra words
/// beside them.
pub fn tr_words(en: &'static str, ru: &'static str, words: &'static str) -> String {
    let mut s = String::with_capacity(en.len() + ru.len() + words.len() + 8);
    s.push_str(en);
    s.push(' ');
    s.push_str(ru);
    s.push(' ');
    s.push_str(words);
    if get() == Lang::ZhHk {
        let zh = crate::lang::zh_hk::lookup(en).unwrap_or("");
        if !zh.is_empty() {
            s.push(' ');
            s.push_str(zh);
        }
        let extra = crate::lang::zh_hk::search_words(en);
        if !extra.is_empty() {
            s.push(' ');
            s.push_str(extra);
        }
    }
    s
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    /// The language is a global, so the tests that change it take this lock (and the
    /// tests that read it take it too, so they do not see a language of their own).
    pub(crate) static LANG_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_language_is_chosen_by_its_code() {
        let _g = lock();
        assert_eq!(from_code("en"), Lang::En);
        assert_eq!(from_code("ru"), Lang::Ru);
        assert_eq!(from_code("zh-HK"), Lang::ZhHk);
        assert_eq!(from_code("zh_HK"), Lang::ZhHk);
        assert_eq!(from_code("zh-TW"), Lang::ZhHk);
        assert_eq!(from_code("zh-Hant"), Lang::ZhHk);
        assert_eq!(from_code("cht"), Lang::ZhHk);
        // (simplified Chinese is not translated yet)
        assert_eq!(from_code("zh-CN"), Lang::En);
        assert_eq!(from_code("zh"), Lang::En);
    }

    #[test]
    fn the_language_is_kept_where_it_was_set() {
        let _g = lock();
        set(Lang::ZhHk);
        assert_eq!(get(), Lang::ZhHk);
        assert_eq!(code(Lang::ZhHk), "zh-HK");
        assert_eq!(tr("Save", "Сохранить"), "儲存");
        set(Lang::En);
        assert_eq!(tr("Save", "Сохранить"), "Save");
        set(Lang::Ru);
        assert_eq!(tr("Save", "Сохранить"), "Сохранить");
        set(Lang::En);
    }

    #[test]
    fn a_text_without_a_translation_stays_in_english() {
        let _g = lock();
        set(Lang::ZhHk);
        assert_eq!(tr("A text nobody translated", "Текст"), "A text nobody translated");
        set(Lang::En);
    }

    #[test]
    fn a_command_is_found_by_its_chinese_words() {
        let _g = lock();
        set(Lang::ZhHk);
        let words = tr_words("Save", "Сохранить", "write");
        assert!(words.contains("儲存"), "the Chinese name is not in the words: {words}");
        assert!(words.contains("write"), "the English words are not in the words: {words}");
        set(Lang::En);
    }
}
