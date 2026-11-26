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
    direction: Direction,
    /// The currency its readers count in most (ISO 4217): what a page's
    /// amounts are converted to first, for a reader in this language.
    pub(crate) currency: &'static str,
}

impl Lang {
    /// A language written left to right.
    const fn left_to_right(code: &'static str, name: &'static str, currency: &'static str) -> Self {
        Self {
            code,
            name,
            direction: Direction::LeftToRight,
            currency,
        }
    }

    /// A language written right to left.
    const fn right_to_left(code: &'static str, name: &'static str, currency: &'static str) -> Self {
        Self {
            code,
            name,
            direction: Direction::RightToLeft,
            currency,
        }
    }
}

/// The languages md2html knows. The first [`FAMILY`] have interface words
/// of their own, in the order a page's language switch offers them; the
/// rest are offered for translation on the reader's device.
pub(crate) const LANGS: &[Lang] = &[
    Lang::left_to_right("en", "English", "USD"),
    Lang::left_to_right("de", "Deutsch", "EUR"),
    Lang::right_to_left("he", "עברית", "ILS"),
    Lang::left_to_right("id", "Bahasa Indonesia", "IDR"),
    Lang::left_to_right("nl", "Nederlands", "EUR"),
    Lang::left_to_right("jv", "Basa Jawa", "IDR"),
    Lang::right_to_left("yi", "ייִדיש", "USD"),
    Lang::left_to_right("nds", "Plattdüütsch", "EUR"),
    Lang::left_to_right("fr", "Français", "EUR"),
    Lang::left_to_right("it", "Italiano", "EUR"),
    Lang::left_to_right("es", "Español", "EUR"),
    Lang::left_to_right("ja", "日本語", "JPY"),
    Lang::left_to_right("ja-Hira", "にほんご（ひらがな）", "JPY"),
    Lang::left_to_right("ja-Latn", "Nihongo (rōmaji)", "JPY"),
    Lang::left_to_right("zh-Hans", "中文（简体）", "CNY"),
    Lang::left_to_right("zh-Hant", "中文（繁體）", "TWD"),
    Lang::left_to_right("yue", "粵語", "HKD"),
    Lang::left_to_right("pt", "Português", "BRL"),
    Lang::left_to_right("pl", "Polski", "PLN"),
    Lang::left_to_right("tr", "Türkçe", "TRY"),
    Lang::left_to_right("ru", "Русский", "RUB"),
    Lang::left_to_right("uk", "Українська", "UAH"),
    Lang::right_to_left("ar", "العربية", "SAR"),
    Lang::left_to_right("ko", "한국어", "KRW"),
    Lang::left_to_right("hi", "हिन्दी", "INR"),
    Lang::left_to_right("sv", "Svenska", "SEK"),
    Lang::left_to_right("da", "Dansk", "DKK"),
    Lang::left_to_right("nb", "Norsk", "NOK"),
    Lang::left_to_right("fi", "Suomi", "EUR"),
    Lang::left_to_right("cs", "Čeština", "CZK"),
    Lang::left_to_right("el", "Ελληνικά", "EUR"),
    Lang::left_to_right("ro", "Română", "RON"),
    Lang::left_to_right("hu", "Magyar", "HUF"),
    Lang::left_to_right("vi", "Tiếng Việt", "VND"),
    Lang::left_to_right("th", "ไทย", "THB"),
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
