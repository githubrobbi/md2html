// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! What a page is told about itself: the front matter at the top of the
//! Markdown, the command line on top of it, and the words of the page's
//! own interface in the page's language.

mod front;
mod langs;
mod settings;
mod values;
mod words;

pub(crate) use front::front_matter;
pub(crate) use langs::{Direction, FAMILY, LANGS, Lang, direction, lang_info};
pub(crate) use settings::Settings;
pub(crate) use values::{Diagrams, Fingerprint, Translations, Ttl};
pub(crate) use words::{SearchKeys, Ui, all_ui, ui};

#[cfg(test)]
mod tests {
    use super::{LANGS, all_ui, ui};

    /// Each language of the switch finds its own interface words by its code.
    ///
    /// # Panics
    ///
    /// When [`ui`] gives a language's code another table of words than the one
    /// [`all_ui`] lists at that language's place in [`LANGS`], for example
    /// English words for a language whose table falls through to English.
    #[test]
    fn each_language_of_the_family_has_its_own_words() {
        for (lang, words) in LANGS.iter().zip(all_ui()) {
            assert!(core::ptr::eq(ui(lang.code), words), "{}", lang.code);
        }
    }
}
