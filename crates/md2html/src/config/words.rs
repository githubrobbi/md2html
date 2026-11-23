// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The words of the page's own interface: one table per language, each in
//! a file of its own, and the one place that picks a table for a code.

mod de;
mod en;
mod es;
mod fr;
mod he;
mod id;
mod it;
mod ja;
mod ja_hira;
mod ja_latn;
mod jv;
mod nds;
mod nl;
mod yi;
mod yue;
mod zh_hans;
mod zh_hant;

use alloc::borrow::Cow;

use super::langs::FAMILY;
use crate::model::{Callout, Task};
use crate::text::{Date, put};

/// The words of the page's own interface, in one language.
#[derive(Debug)]
pub(crate) struct Ui {
    /// "Skip to content"
    pub(crate) skip: &'static str,
    /// The search button.
    pub(crate) search: &'static str,
    /// The search field's placeholder.
    pub(crate) search_page: &'static str,
    /// The keys named under the results.
    pub(crate) keys: SearchKeys,
    /// No result.
    none: &'static str,
    /// "3 results"; the page's script writes the number.
    results: Template<1>,
    /// "1 result"
    result: &'static str,
    /// The glossary chapter's title.
    pub(crate) glossary: &'static str,
    /// The line under it.
    pub(crate) glossary_intro: &'static str,
    /// Its addition when shared terms are listed, around the shared
    /// glossary's address.
    pub(crate) glossary_shared: Template<1>,
    /// "back"
    pub(crate) back: &'static str,
    /// "all terms"
    pub(crate) all_terms: &'static str,
    /// The outline's dock link.
    pub(crate) keep_outline: &'static str,
    /// The outline's name.
    pub(crate) outline: &'static str,
    /// The outline button's tips.
    outline_tips: OutlineTips,
    /// The footnotes' chapter.
    pub(crate) notes: &'static str,
    /// "Note 3", a footnote reference's label, around the note's number.
    pub(crate) note_n: Template<1>,
    /// The `#` beside a heading.
    pub(crate) link_section: &'static str,
    /// A code block's copy button.
    pub(crate) copy: CopyButton,
    /// The names of the callouts.
    pub(crate) callouts: Callouts,
    /// The foot of the page, around the source file's name.
    pub(crate) footer: Template<1>,
    /// The months.
    months: [&'static str; 12],
    /// How a date is written.
    date_fmt: DateFormat,
    /// The opening's name in the search.
    pub(crate) top: &'static str,
    /// The enlarged figure.
    figure: EnlargedFigure,
    /// The text size controls.
    pub(crate) size: TextSize,
    /// The theme switch.
    pub(crate) theme: ThemeSwitch,
    /// A table's scroll region.
    table: &'static str,
    /// The links to the previous and the next page.
    pub(crate) pages: PageLinks,
    /// A task's box.
    pub(crate) task: TaskBoxes,
    /// "How to read this", as an author writes it in this language.
    pub(crate) guide_heading: &'static str,
    /// "Glossary", as an author writes it.
    pub(crate) glossary_heading: &'static str,
    /// "Contents", as an author writes it.
    pub(crate) contents_heading: &'static str,
    /// The language switch.
    pub(crate) lang: LanguageSwitch,
    /// The export menu.
    pub(crate) export: ExportMenu,
    /// The name of the panel of reading settings (text size, theme, saving).
    pub(crate) prefs: &'static str,
    /// The credit at the foot, around md2html linked to its home.
    pub(crate) credit: Template<1>,
    /// Sharing a link to a part of the page.
    share: Sharing,
    /// After a live page reloaded with new content.
    live: &'static str,
}

/// The keys named under the search results.
#[derive(Debug)]
pub(crate) struct SearchKeys {
    /// "move": the arrows step through the results.
    pub(crate) step: &'static str,
    /// "open": Enter opens one.
    pub(crate) open: &'static str,
    /// "close": Escape closes the search.
    pub(crate) close: &'static str,
}

/// The outline button's tips.
#[derive(Debug)]
struct OutlineTips {
    /// "Hide the outline"
    hide: &'static str,
    /// "Show the outline"
    show: &'static str,
    /// The keys that do it, added to either.
    keys: &'static str,
}

impl OutlineTips {
    /// The words in the order the page's scripts index them.
    const fn in_script_order(&self) -> [&'static str; 3] {
        [self.hide, self.show, self.keys]
    }
}

/// A code block's copy button.
#[derive(Debug)]
pub(crate) struct CopyButton {
    /// "copy"
    pub(crate) label: &'static str,
    /// "copied", after a copy.
    copied: &'static str,
}

impl CopyButton {
    /// The words in the order the page's scripts index them.
    const fn in_script_order(&self) -> [&'static str; 2] {
        [self.label, self.copied]
    }
}

/// The name of each kind of callout.
#[derive(Debug)]
pub(crate) struct Callouts {
    /// "Note"
    pub(crate) note: &'static str,
    /// "Tip"
    tip: &'static str,
    /// "Important"
    important: &'static str,
    /// "Warning"
    warning: &'static str,
    /// "Caution"
    caution: &'static str,
}

impl Callouts {
    /// The name of a kind.
    pub(crate) const fn word(&self, kind: Callout) -> &'static str {
        match kind {
            Callout::Note => self.note,
            Callout::Tip => self.tip,
            Callout::Important => self.important,
            Callout::Warning => self.warning,
            Callout::Caution => self.caution,
        }
    }

    /// The kind a lowercased word names, if it names one.
    pub(crate) fn kind_named(&self, lower: &str) -> Option<Callout> {
        Callout::ALL
            .into_iter()
            .find(|&kind| self.word(kind).to_lowercase() == lower)
    }
}

/// The enlarged figure.
#[derive(Debug)]
struct EnlargedFigure {
    /// Its label.
    label: &'static str,
    /// "Close"
    close: &'static str,
    /// "Close (Escape)"
    close_key: &'static str,
    /// "2 of 5": which figure, of how many.
    position: Template<2>,
}

impl EnlargedFigure {
    /// The words in the order the page's scripts index them.
    fn in_script_order(&self) -> [Cow<'static, str>; 4] {
        [
            self.label.into(),
            self.close.into(),
            self.close_key.into(),
            self.position.fill(["{i}", "{n}"]).into(),
        ]
    }
}

/// The text size controls.
#[derive(Debug)]
pub(crate) struct TextSize {
    /// The group's label.
    pub(crate) group: &'static str,
    /// "Smaller text, 90%"
    pub(crate) smaller: SizeStep,
    /// "Larger text, 110%"
    pub(crate) larger: SizeStep,
    /// The tip at 100%.
    as_written: &'static str,
    /// The tip at any other size, around the percentage.
    scaled: Template<1>,
    /// The keys, added to either tip.
    keys: &'static str,
    /// "Smallest text already"
    smallest: &'static str,
    /// "Largest text already"
    largest: &'static str,
}

impl TextSize {
    /// The words in the order the page's scripts index them.
    fn in_script_order(&self) -> [Cow<'static, str>; 8] {
        [
            self.group.into(),
            self.smaller.with_slot().into(),
            self.larger.with_slot().into(),
            self.as_written.into(),
            self.scaled.fill(["{n}"]).into(),
            self.keys.into(),
            self.smallest.into(),
            self.largest.into(),
        ]
    }
}

/// A text-size button's words: the step it takes, then the percentage it
/// leads to, which the page's script writes in.
#[derive(Debug)]
pub(crate) struct SizeStep {
    /// "Smaller text": the button's title before the script runs.
    pub(crate) words: &'static str,
    /// ", ": between the words and the number.
    joint: &'static str,
    /// "%": after the number.
    unit: &'static str,
}

impl SizeStep {
    /// The words with the number's slot, `{n}`, where the script fills it.
    fn with_slot(&self) -> String {
        format!("{}{}{{n}}{}", self.words, self.joint, self.unit)
    }
}

/// The theme switch.
#[derive(Debug)]
pub(crate) struct ThemeSwitch {
    /// The button.
    pub(crate) button: &'static str,
    /// Its tip, around the theme now, the next and the one after it.
    tip: Template<3>,
    /// The system's theme, around the mode it is in now.
    system_now: Template<1>,
    /// "System"
    pub(crate) system: &'static str,
    /// "Light"
    pub(crate) light: &'static str,
    /// "Dark"
    pub(crate) dark: &'static str,
    /// "dark", inside a sentence.
    dark_mode: &'static str,
    /// "light", inside a sentence.
    light_mode: &'static str,
}

impl ThemeSwitch {
    /// The words in the order the page's scripts index them.
    fn in_script_order(&self) -> [Cow<'static, str>; 8] {
        [
            self.button.into(),
            self.tip.fill(["{now}", "{next}", "{after}"]).into(),
            self.system_now.fill(["{mode}"]).into(),
            self.system.into(),
            self.light.into(),
            self.dark.into(),
            self.dark_mode.into(),
            self.light_mode.into(),
        ]
    }
}

/// The links to the previous and the next page.
#[derive(Debug)]
pub(crate) struct PageLinks {
    /// "Previous"
    pub(crate) previous: &'static str,
    /// "Next"
    pub(crate) next: &'static str,
    /// The links' label.
    pub(crate) label: &'static str,
}

/// A task's box.
#[derive(Debug)]
pub(crate) struct TaskBoxes {
    /// "Done"
    done: &'static str,
    /// "Open"
    open: &'static str,
}

impl TaskBoxes {
    /// The label of a box.
    pub(crate) const fn word(&self, task: Task) -> &'static str {
        match task {
            Task::Done => self.done,
            Task::Open => self.open,
        }
    }
}

/// The language switch.
#[derive(Debug)]
pub(crate) struct LanguageSwitch {
    /// Its name.
    pub(crate) label: &'static str,
    /// "Original", beside the original version.
    pub(crate) original: &'static str,
    /// The notice on a translation, around the original's language.
    pub(crate) notice: Template<1>,
    /// "On this device"
    pub(crate) on_device: &'static str,
    /// The note on a machine translation.
    pub(crate) machine: &'static str,
    /// "Show the original"
    pub(crate) show_original: &'static str,
    /// "Translating…"
    translating: &'static str,
    /// The browser cannot translate, around the language asked for.
    cannot: Template<1>,
    /// "More languages"
    pub(crate) more: &'static str,
}

impl LanguageSwitch {
    /// The words in the order the page's scripts index them.
    fn in_script_order(&self) -> [Cow<'static, str>; 9] {
        [
            self.label.into(),
            self.original.into(),
            self.notice.fill(["{orig}"]).into(),
            self.on_device.into(),
            self.machine.into(),
            self.show_original.into(),
            self.translating.into(),
            self.cannot.fill(["{lang}"]).into(),
            self.more.into(),
        ]
    }
}

/// The export menu.
#[derive(Debug)]
pub(crate) struct ExportMenu {
    /// Its name.
    pub(crate) label: &'static str,
    /// This page as HTML, to read offline.
    pub(crate) html: &'static str,
    /// Print, or save as PDF.
    pub(crate) print: &'static str,
    /// The PDF.
    pub(crate) pdf: &'static str,
    /// The EPUB.
    pub(crate) epub: &'static str,
}

/// Sharing a link to a part of the page.
#[derive(Debug)]
struct Sharing {
    /// "Link copied"
    copied: &'static str,
    /// "Link to this passage"
    passage: &'static str,
    /// "Link to this row"
    row: &'static str,
    /// "Link to this": a figure, a diagram, a code block.
    thing: &'static str,
}

impl Sharing {
    /// The words in the order the page's scripts index them.
    const fn in_script_order(&self) -> [&'static str; 4] {
        [self.copied, self.passage, self.row, self.thing]
    }
}

/// The interface words for a language code (`de`, `de-CH`, ...);
/// English for any language this tool has no words for.
pub(crate) fn ui(lang: &str) -> &'static Ui {
    let primary = lang
        .split_once(['-', '_'])
        .map_or(lang, |(first, _)| first)
        .to_ascii_lowercase();
    match primary.as_str() {
        "de" => &de::WORDS,
        "he" | "iw" => &he::WORDS,
        "id" => &id::WORDS,
        "nl" => &nl::WORDS,
        "jv" => &jv::WORDS,
        "yi" | "ji" => &yi::WORDS,
        "nds" => &nds::WORDS,
        "fr" => &fr::WORDS,
        "it" => &it::WORDS,
        "es" => &es::WORDS,
        "ja" => match lang.to_ascii_lowercase().as_str() {
            "ja-hira" | "ja-kana" => &ja_hira::WORDS,
            "ja-latn" => &ja_latn::WORDS,
            _ => &ja::WORDS,
        },
        "zh" => match lang.to_ascii_lowercase().as_str() {
            "zh-hant" | "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant-tw" | "zh-hant-hk" => {
                &zh_hant::WORDS
            }
            _ => &zh_hans::WORDS,
        },
        "yue" => &yue::WORDS,
        _ => &en::WORDS,
    }
}

/// Every interface, in the order of the language switch, for recognising
/// an author's headings and callout words in any of these languages.
pub(crate) const fn all_ui() -> [&'static Ui; FAMILY] {
    [
        &en::WORDS,
        &de::WORDS,
        &he::WORDS,
        &id::WORDS,
        &nl::WORDS,
        &jv::WORDS,
        &yi::WORDS,
        &nds::WORDS,
        &fr::WORDS,
        &it::WORDS,
        &es::WORDS,
        &ja::WORDS,
        &ja_hira::WORDS,
        &ja_latn::WORDS,
        &zh_hans::WORDS,
        &zh_hant::WORDS,
        &yue::WORDS,
    ]
}

/// Words with `N` slots for values: the words before the first slot,
/// then the words after each one. The values fill the slots in order, so
/// none can be missing or out of place.
#[derive(Debug)]
pub(crate) struct Template<const N: usize> {
    /// The words before the first slot.
    lead: &'static str,
    /// The words after each slot.
    tails: [&'static str; N],
}

impl<const N: usize> Template<N> {
    /// The words with the values in their slots.
    pub(crate) fn fill(&self, values: [&str; N]) -> String {
        self.fill_around(values, str::to_owned)
    }

    /// The words, each piece passed through `words` (`esc` when the words
    /// are text and the values markup), with the values in their slots
    /// as they are.
    pub(crate) fn fill_around(&self, values: [&str; N], words: impl Fn(&str) -> String) -> String {
        let mut out = words(self.lead);
        for (value, tail) in values.into_iter().zip(self.tails) {
            out.push_str(value);
            out.push_str(&words(tail));
        }
        out
    }
}

/// How a language writes a date: the order of the day, the month's name
/// and the year, and the words around them.
#[derive(Debug)]
enum DateFormat {
    /// "21 November 2026"
    DayMonthYear(Template<3>),
    /// "2026年11月21日"
    YearMonthDay(Template<3>),
}

impl Ui {
    /// A date in this language: "21 November 2026", "21. November 2026".
    pub(crate) fn date(&self, date: Date) -> String {
        let day = date.day.to_string();
        let month = date.month.of(&self.months);
        let year = date.year.to_string();
        match &self.date_fmt {
            DateFormat::DayMonthYear(words) => words.fill([&day, month, &year]),
            DateFormat::YearMonthDay(words) => words.fill([&year, month, &day]),
        }
    }

    /// The words the page's scripts need, as JSON, under the names the
    /// scripts read them by; a slot is written as the scripts fill it,
    /// `{n}`, `{lang}`, ...
    pub(crate) fn json(&self) -> String {
        let entries = [
            ("live", json_word(self.live)),
            ("share", json_list(&self.share.in_script_order())),
            ("lang", json_list(&self.lang.in_script_order())),
            ("table", json_word(self.table)),
            ("none", json_word(self.none)),
            ("results", json_word(&self.results.fill(["{n}"]))),
            ("result", json_word(self.result)),
            ("outline", json_list(&self.outline_tips.in_script_order())),
            ("copy", json_list(&self.copy.in_script_order())),
            ("figure", json_list(&self.figure.in_script_order())),
            ("size", json_list(&self.size.in_script_order())),
            ("theme", json_list(&self.theme.in_script_order())),
        ];
        let mut out = String::from("{");
        for (place, (name, value)) in entries.iter().enumerate() {
            if place > 0 {
                out.push(',');
            }
            put!(out, "\"{name}\":{value}");
        }
        out.push('}');
        out
    }
}

/// One of the scripts' words, as a JSON string.
fn json_word(word: &str) -> String {
    crate::text::json_str(word)
}

/// The scripts' words in a fixed order, as a JSON array.
fn json_list(words: &[impl AsRef<str>]) -> String {
    let mut out = String::from("[");
    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&json_word(word.as_ref()));
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::all_ui;
    use crate::text::iso_date;

    /// Every language writes a date and keeps the slots its script fills.
    ///
    /// # Panics
    ///
    /// When a language's date of 22 November 2026 lacks the day, the year or
    /// its word for November; when its footnote phrase does not hold the number
    /// exactly once; when its words for the page's script lack one of the
    /// slots (`{n}`, `{now}`, `{lang}`, ...) that the script fills in; or when
    /// the date in the test cannot be read.
    #[test]
    fn every_language_writes_a_date_and_its_slots() {
        let date = iso_date("2026-11-22").unwrap();
        for words in all_ui() {
            let written = words.date(date);
            let [.., november, _] = words.months;
            assert!(
                written.contains("22") && written.contains("2026") && written.contains(november),
                "{written}"
            );
            assert_eq!(words.note_n.fill(["7"]).matches('7').count(), 1);
            let json = words.json();
            for slot in [
                "{n}", "{i}", "{now}", "{next}", "{after}", "{mode}", "{orig}", "{lang}",
            ] {
                assert!(json.contains(slot), "{slot} in {json}");
            }
        }
    }
}
