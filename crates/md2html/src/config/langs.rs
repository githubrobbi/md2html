// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The languages md2html knows by name, and which of them run right to left.

/// How many of [`LANGS`] the language switch offers first, before "more".
pub(crate) const FAMILY: usize = 17;

/// Which way a language's lines run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// Left to right: most scripts.
    LeftToRight,
    /// Right to left: Hebrew, Yiddish, Arabic.
    RightToLeft,
}

impl Direction {
    /// The value of HTML's `dir` and EPUB's page progression: `ltr` or `rtl`.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::LeftToRight => "ltr",
            Self::RightToLeft => "rtl",
        }
    }
}

/// A language md2html knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lang {
    /// Its code: `de`, `zh-Hans`.
    pub(crate) code: &'static str,
    /// Its own name for itself.
    pub(crate) name: &'static str,
    /// Which way its lines run.
    pub(crate) direction: Direction,
}

impl Lang {
    /// A language written left to right.
    const fn left_to_right(code: &'static str, name: &'static str) -> Self {
        Self {
            code,
            name,
            direction: Direction::LeftToRight,
        }
    }

    /// A language written right to left.
    const fn right_to_left(code: &'static str, name: &'static str) -> Self {
        Self {
            code,
            name,
            direction: Direction::RightToLeft,
        }
    }
}

/// The languages md2html knows. The first [`FAMILY`] have interface words
/// of their own, in the order a page's language switch offers them; the
/// rest are offered for translation on the reader's device.
pub(crate) const LANGS: &[Lang] = &[
    Lang::left_to_right("en", "English"),
    Lang::left_to_right("de", "Deutsch"),
    Lang::right_to_left("he", "עברית"),
    Lang::left_to_right("id", "Bahasa Indonesia"),
    Lang::left_to_right("nl", "Nederlands"),
    Lang::left_to_right("jv", "Basa Jawa"),
    Lang::right_to_left("yi", "ייִדיש"),
    Lang::left_to_right("nds", "Plattdüütsch"),
    Lang::left_to_right("fr", "Français"),
    Lang::left_to_right("it", "Italiano"),
    Lang::left_to_right("es", "Español"),
    Lang::left_to_right("ja", "日本語"),
    Lang::left_to_right("ja-Hira", "にほんご（ひらがな）"),
    Lang::left_to_right("ja-Latn", "Nihongo (rōmaji)"),
    Lang::left_to_right("zh-Hans", "中文（简体）"),
    Lang::left_to_right("zh-Hant", "中文（繁體）"),
    Lang::left_to_right("yue", "粵語"),
    Lang::left_to_right("pt", "Português"),
    Lang::left_to_right("pl", "Polski"),
    Lang::left_to_right("tr", "Türkçe"),
    Lang::left_to_right("ru", "Русский"),
    Lang::left_to_right("uk", "Українська"),
    Lang::right_to_left("ar", "العربية"),
    Lang::left_to_right("ko", "한국어"),
    Lang::left_to_right("hi", "हिन्दी"),
    Lang::left_to_right("sv", "Svenska"),
    Lang::left_to_right("da", "Dansk"),
    Lang::left_to_right("nb", "Norsk"),
    Lang::left_to_right("fi", "Suomi"),
    Lang::left_to_right("cs", "Čeština"),
    Lang::left_to_right("el", "Ελληνικά"),
    Lang::left_to_right("ro", "Română"),
    Lang::left_to_right("hu", "Magyar"),
    Lang::left_to_right("vi", "Tiếng Việt"),
    Lang::left_to_right("th", "ไทย"),
];

/// A language's entry, by code (`de`, `de-CH`).
pub(crate) fn lang_info(code: &str) -> Option<&'static Lang> {
    let lower = code.to_ascii_lowercase().replace('_', "-");
    let primary = lower
        .split_once('-')
        .map_or(lower.as_str(), |(first, _)| first);
    let traditional = ["zh-tw", "zh-hk", "zh-mo"]
        .iter()
        .any(|region| lower.starts_with(region));
    let by_code = |wanted: &str| {
        LANGS
            .iter()
            .find(|known| known.code.eq_ignore_ascii_case(wanted))
    };
    by_code(&lower)
        .or_else(|| traditional.then(|| by_code("zh-Hant")).flatten())
        .or_else(|| {
            LANGS
                .iter()
                .find(|known| known.code.split('-').next() == Some(primary))
        })
}

/// Which way a language's lines run; left to right for a language this
/// tool does not know.
pub(crate) fn direction(code: &str) -> Direction {
    lang_info(code).map_or(Direction::LeftToRight, |lang| lang.direction)
}
