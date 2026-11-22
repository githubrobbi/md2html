// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! What a page is told about itself: the front matter at the top of the
//! Markdown, the command line on top of it, and the words of the page's
//! own interface in the page's language.

use alloc::collections::BTreeMap;
use std::path::PathBuf;

/// The `---` block at the very top of a document, as `key: value` pairs
/// (keys lowercased, dashes as underscores), the text after it, and the
/// number of the first line after it.
pub(crate) fn front_matter(text: &str) -> (BTreeMap<String, String>, &str, usize) {
    let none = (BTreeMap::new(), text, 1);
    let text_nobom = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text_nobom.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return none;
    };
    if first.trim_end() != "---" {
        return none;
    }
    let mut map = BTreeMap::new();
    let mut used = first.len();
    let mut count = 1usize;
    let mut last_key: Option<String> = None;
    for line in lines {
        used += line.len();
        count += 1;
        let bare = line.trim_end();
        if bare == "---" || bare == "..." {
            if map.is_empty() {
                return none;
            }
            return (map, &text_nobom[used..], count + 1);
        }
        if bare.trim().is_empty() || bare.trim_start().starts_with('#') {
            continue;
        }
        if let Some(item) = bare.trim_start().strip_prefix("- ") {
            // A list under the last key: kept as one comma-separated value.
            let Some(key) = &last_key else {
                return none;
            };
            let entry: &mut String = map.entry(key.clone()).or_default();
            if !entry.is_empty() {
                entry.push_str(", ");
            }
            entry.push_str(&unquote(item.trim()));
            continue;
        }
        if bare.starts_with([' ', '\t']) {
            // A folded continuation of the last value.
            let Some(key) = &last_key else {
                return none;
            };
            let entry: &mut String = map.entry(key.clone()).or_default();
            if !entry.is_empty() {
                entry.push(' ');
            }
            entry.push_str(bare.trim());
            continue;
        }
        let Some((key, value)) = bare.split_once(':') else {
            return none;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        {
            return none;
        }
        let key = key.to_ascii_lowercase().replace('-', "_");
        let value = value.trim();
        let value = if value == "|" || value == ">" {
            String::new()
        } else {
            unquote(value)
        };
        map.insert(key.clone(), value);
        last_key = Some(key);
        if count > 400 {
            return none;
        }
    }
    none
}

/// A value without the quotes around it.
fn unquote(value: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|v| v.strip_suffix(quote))
        {
            return if quote == '"' {
                inner.replace("\\\"", "\"")
            } else {
                inner.replace("''", "'")
            };
        }
    }
    value.to_owned()
}

/// Whether a value says yes.
pub(crate) fn truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1"
    )
}

/// Everything a page may be told, from front matter and the command line.
#[derive(Clone, Debug, Default)]
pub(crate) struct Settings {
    /// The page's title, instead of its first heading.
    pub(crate) title: Option<String>,
    /// The language: `lang` attribute, interface words, date.
    pub(crate) lang: Option<String>,
    /// The name in the bar instead of "md2html".
    pub(crate) brand: Option<String>,
    /// Where the name in the bar leads.
    pub(crate) brand_link: Option<String>,
    /// The tag beside the name, instead of the file name.
    pub(crate) kicker: Option<String>,
    /// The line above the title, instead of file name and date.
    pub(crate) eyebrow: Option<String>,
    /// The date shown, instead of today.
    pub(crate) date: Option<String>,
    /// A version shown with the date.
    pub(crate) version: Option<String>,
    /// The meta description, instead of the lead.
    pub(crate) description: Option<String>,
    /// The author, for the meta tags.
    pub(crate) author: Option<String>,
    /// The fine print at the foot, Markdown allowed.
    pub(crate) footer: Option<String>,
    /// Larger type.
    pub(crate) large: Option<bool>,
    /// Join the shared glossary.
    pub(crate) glossary: Option<bool>,
    /// Trust raw HTML.
    pub(crate) html: Option<bool>,
    /// Put the images inside the page.
    pub(crate) embed: Option<bool>,
    /// Add a Content-Security-Policy.
    pub(crate) csp: Option<bool>,
    /// Let search engines index the page.
    pub(crate) index: Option<bool>,
    /// Show the outline.
    pub(crate) toc: Option<bool>,
    /// Read `$...$` as maths.
    pub(crate) math: Option<bool>,
    /// Stylesheets to add, relative to the Markdown file.
    pub(crate) css: Vec<PathBuf>,
    /// HTML files to add to the head, relative to the Markdown file.
    pub(crate) head: Vec<PathBuf>,
}

impl Settings {
    /// Sets one key; an unknown key is reported.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let key = key.trim().to_ascii_lowercase().replace('-', "_");
        let text = || Some(value.to_owned());
        let flag = || Some(truthy(value));
        match key.as_str() {
            "title" => self.title = text(),
            "lang" | "language" => self.lang = text(),
            "brand" | "site" => self.brand = text(),
            "brand_link" | "brand_href" | "home" => self.brand_link = text(),
            "kicker" => self.kicker = text(),
            "eyebrow" => self.eyebrow = text(),
            "date" => self.date = text(),
            "version" => self.version = text(),
            "description" | "summary" => self.description = text(),
            "author" | "authors" => self.author = text(),
            "footer" => self.footer = text(),
            "large" => self.large = flag(),
            "glossary" | "shared_glossary" => self.glossary = flag(),
            "html" | "raw_html" => self.html = flag(),
            "embed" | "embed_images" | "self_contained" => self.embed = flag(),
            "csp" => self.csp = flag(),
            "index" => self.index = flag(),
            "noindex" => self.index = Some(!truthy(value)),
            "toc" | "outline" => self.toc = flag(),
            "math" => self.math = flag(),
            "css" | "stylesheet" => self.css.extend(
                value
                    .split(',')
                    .map(|p| PathBuf::from(p.trim()))
                    .filter(|p| !p.as_os_str().is_empty()),
            ),
            "head" => self.head.extend(
                value
                    .split(',')
                    .map(|p| PathBuf::from(p.trim()))
                    .filter(|p| !p.as_os_str().is_empty()),
            ),
            // Common front matter that is not the page's business.
            "tags" | "categories" | "layout" | "slug" | "permalink" | "draft" | "keywords"
            | "status" | "updated" | "created" => {}
            other => return Err(format!("unknown setting `{other}`")),
        }
        Ok(())
    }

    /// `over` on top of `self`: what `over` says wins.
    pub(crate) fn overlay(&self, over: &Self) -> Self {
        fn pick<T: Clone>(base: Option<&T>, top: Option<&T>) -> Option<T> {
            top.or(base).cloned()
        }
        let mut css = self.css.clone();
        css.extend(over.css.iter().cloned());
        let mut head = self.head.clone();
        head.extend(over.head.iter().cloned());
        Self {
            title: pick(self.title.as_ref(), over.title.as_ref()),
            lang: pick(self.lang.as_ref(), over.lang.as_ref()),
            brand: pick(self.brand.as_ref(), over.brand.as_ref()),
            brand_link: pick(self.brand_link.as_ref(), over.brand_link.as_ref()),
            kicker: pick(self.kicker.as_ref(), over.kicker.as_ref()),
            eyebrow: pick(self.eyebrow.as_ref(), over.eyebrow.as_ref()),
            date: pick(self.date.as_ref(), over.date.as_ref()),
            version: pick(self.version.as_ref(), over.version.as_ref()),
            description: pick(self.description.as_ref(), over.description.as_ref()),
            author: pick(self.author.as_ref(), over.author.as_ref()),
            footer: pick(self.footer.as_ref(), over.footer.as_ref()),
            large: pick(self.large.as_ref(), over.large.as_ref()),
            glossary: pick(self.glossary.as_ref(), over.glossary.as_ref()),
            html: pick(self.html.as_ref(), over.html.as_ref()),
            embed: pick(self.embed.as_ref(), over.embed.as_ref()),
            csp: pick(self.csp.as_ref(), over.csp.as_ref()),
            index: pick(self.index.as_ref(), over.index.as_ref()),
            toc: pick(self.toc.as_ref(), over.toc.as_ref()),
            math: pick(self.math.as_ref(), over.math.as_ref()),
            css,
            head,
        }
    }
}

/// The words of the page's own interface, in one language.
#[derive(Debug)]
pub(crate) struct Ui {
    /// "Skip to content"
    pub(crate) skip: &'static str,
    /// The search button.
    pub(crate) search: &'static str,
    /// The search field's placeholder.
    pub(crate) search_page: &'static str,
    /// Under the results: move, open, close.
    pub(crate) keys: [&'static str; 3],
    /// No result.
    pub(crate) none: &'static str,
    /// "{n} results"
    pub(crate) results: &'static str,
    /// "1 result"
    pub(crate) result: &'static str,
    /// The glossary chapter's title.
    pub(crate) glossary: &'static str,
    /// The line under it.
    pub(crate) glossary_intro: &'static str,
    /// Its addition when shared terms are listed; `{url}` is replaced.
    pub(crate) glossary_shared: &'static str,
    /// "back"
    pub(crate) back: &'static str,
    /// "all terms"
    pub(crate) all_terms: &'static str,
    /// The outline's dock link.
    pub(crate) keep_outline: &'static str,
    /// The outline's name.
    pub(crate) outline: &'static str,
    /// Hide / show the outline, and the keys.
    pub(crate) outline_tips: [&'static str; 3],
    /// The footnotes' chapter.
    pub(crate) notes: &'static str,
    /// "Note 3", a footnote reference's label; `{n}` is replaced.
    pub(crate) note_n: &'static str,
    /// The `#` beside a heading.
    pub(crate) link_section: &'static str,
    /// The copy button, and after a copy.
    pub(crate) copy: [&'static str; 2],
    /// Note, Tip, Important, Warning, Caution.
    pub(crate) callouts: [&'static str; 5],
    /// The foot of the page; `{source}` is replaced.
    pub(crate) footer: &'static str,
    /// The months.
    pub(crate) months: [&'static str; 12],
    /// Day first with a dot ("21. November 2026").
    pub(crate) day_dot: bool,
    /// The opening's name in the search.
    pub(crate) top: &'static str,
    /// The enlarged figure: its label, close, close with the key, "{i} of {n}".
    pub(crate) figure: [&'static str; 4],
    /// Text size: group label, smaller, larger, the 100% tip, the other tip, the keys tip, smallest, largest.
    pub(crate) size: [&'static str; 8],
    /// Theme: the button, its tip, system now, System, Light, Dark, dark, light.
    pub(crate) theme: [&'static str; 8],
    /// A table's scroll region.
    pub(crate) table: &'static str,
    /// Previous page, next page, the pages.
    pub(crate) pages: [&'static str; 3],
    /// A task's box: done, open.
    pub(crate) task: [&'static str; 2],
}

/// English.
const EN: Ui = Ui {
    skip: "Skip to content",
    search: "Search",
    search_page: "Search this page",
    keys: ["move", "open", "close"],
    none: "Nothing on this page matches that.",
    results: "{n} results",
    result: "1 result",
    glossary: "Words used here",
    glossary_intro: "Each dotted word in the chapters shows its meaning when you rest on it and jumps here when \
you click it. Every entry has a <strong>back</strong> that returns you to the line you were reading.",
    glossary_shared: " The acronyms come from the <a href=\"{url}\">shared glossary</a>; \
<strong>all terms</strong> opens it at the entry.",
    back: "back",
    all_terms: "all terms",
    keep_outline: "&lsaquo; keep the outline open",
    outline: "Outline",
    outline_tips: [
        "Hide the outline",
        "Show the outline",
        " (\u{2318}\\, Control+\\ on Windows)",
    ],
    notes: "Notes",
    note_n: "Note {n}",
    link_section: "Link to this section",
    copy: ["copy", "copied"],
    callouts: ["Note", "Tip", "Important", "Warning", "Caution"],
    footer: "Rendered from <code>{source}</code> by <code>md2html</code>. The Markdown file is the source; \
render again after editing it.",
    months: [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ],
    day_dot: false,
    top: "Top of the page",
    figure: [
        "The figure, enlarged",
        "Close",
        "Close (Escape)",
        "{i} of {n}",
    ],
    size: [
        "Text size",
        "Smaller text, {n}%",
        "Larger text, {n}%",
        "Text size: 100%, the page as written.",
        "Text size: {n}%. Click to go back to 100%.",
        " Command (Control on Windows) with + or - steps it here, 0 resets; the browser's own zoom is left alone.",
        "Smallest text already",
        "Largest text already",
    ],
    theme: [
        "Theme",
        "Theme: {now}. Click for {next}, then {after}.",
        "System, {mode} right now",
        "System",
        "Light",
        "Dark",
        "dark",
        "light",
    ],
    table: "Table",
    pages: ["Previous", "Next", "Pages"],
    task: ["Done", "Open"],
};

/// German.
const DE: Ui = Ui {
    skip: "Zum Inhalt springen",
    search: "Suchen",
    search_page: "Diese Seite durchsuchen",
    keys: ["wählen", "öffnen", "schließen"],
    none: "Nichts auf dieser Seite passt dazu.",
    results: "{n} Treffer",
    result: "1 Treffer",
    glossary: "Begriffe",
    glossary_intro: "Jedes gepunktet unterstrichene Wort in den Kapiteln zeigt seine Bedeutung, wenn der Zeiger \
darauf ruht, und springt hierher, wenn man es anklickt. Jeder Eintrag hat ein <strong>zurück</strong>, das zu der \
Zeile führt, an der man gerade gelesen hat.",
    glossary_shared: " Die Kürzel stammen aus dem <a href=\"{url}\">gemeinsamen Glossar</a>; \
<strong>alle Begriffe</strong> öffnet es beim Eintrag.",
    back: "zurück",
    all_terms: "alle Begriffe",
    keep_outline: "&lsaquo; Gliederung offen halten",
    outline: "Gliederung",
    outline_tips: [
        "Gliederung ausblenden",
        "Gliederung einblenden",
        " (\u{2318}\\, Strg+\\ unter Windows)",
    ],
    notes: "Anmerkungen",
    note_n: "Anmerkung {n}",
    link_section: "Link zu diesem Abschnitt",
    copy: ["kopieren", "kopiert"],
    callouts: ["Hinweis", "Tipp", "Wichtig", "Achtung", "Vorsicht"],
    footer: "Erstellt aus <code>{source}</code> mit <code>md2html</code>. Die Markdown-Datei ist die Quelle; \
nach dem Bearbeiten neu erzeugen.",
    months: [
        "Januar",
        "Februar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktober",
        "November",
        "Dezember",
    ],
    day_dot: true,
    top: "Seitenanfang",
    figure: [
        "Die Abbildung, vergrößert",
        "Schließen",
        "Schließen (Escape)",
        "{i} von {n}",
    ],
    size: [
        "Textgröße",
        "Kleinere Schrift, {n} %",
        "Größere Schrift, {n} %",
        "Textgröße: 100 %, die Seite wie geschrieben.",
        "Textgröße: {n} %. Klick für 100 %.",
        " Befehlstaste (Strg unter Windows) mit + oder - ändert sie hier, 0 setzt zurück; der Zoom des Browsers bleibt unberührt.",
        "Schon die kleinste Schrift",
        "Schon die größte Schrift",
    ],
    theme: [
        "Darstellung",
        "Darstellung: {now}. Klick für {next}, dann {after}.",
        "System, gerade {mode}",
        "System",
        "Hell",
        "Dunkel",
        "dunkel",
        "hell",
    ],
    table: "Tabelle",
    pages: ["Vorherige Seite", "Nächste Seite", "Seiten"],
    task: ["Erledigt", "Offen"],
};

/// The interface words for a language code (`de`, `de-CH`, ...);
/// English for any language this tool has no words for.
pub(crate) fn ui(lang: &str) -> &'static Ui {
    let primary = lang
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match primary.as_str() {
        "de" => &DE,
        _ => &EN,
    }
}

impl Ui {
    /// A date in this language: "21 November 2026", "21. November 2026".
    pub(crate) fn date(&self, (year, month, day): (i64, u32, u32)) -> String {
        let name = self
            .months
            .get(usize::try_from(month).unwrap_or(1).saturating_sub(1))
            .copied()
            .unwrap_or_default();
        if self.day_dot {
            format!("{day}. {name} {year}")
        } else {
            format!("{day} {name} {year}")
        }
    }

    /// The words the page's scripts need, as JSON.
    pub(crate) fn json(&self) -> String {
        let q = crate::page::json_str;
        format!(
            "{{\"table\":{},\"none\":{},\"results\":{},\"result\":{},\"outline\":[{},{},{}],\"copy\":[{},{}],\
\"figure\":[{},{},{},{}],\"size\":[{},{},{},{},{},{},{},{}],\"theme\":[{},{},{},{},{},{},{},{}]}}",
            q(self.table),
            q(self.none),
            q(self.results),
            q(self.result),
            q(self.outline_tips[0]),
            q(self.outline_tips[1]),
            q(self.outline_tips[2]),
            q(self.copy[0]),
            q(self.copy[1]),
            q(self.figure[0]),
            q(self.figure[1]),
            q(self.figure[2]),
            q(self.figure[3]),
            q(self.size[0]),
            q(self.size[1]),
            q(self.size[2]),
            q(self.size[3]),
            q(self.size[4]),
            q(self.size[5]),
            q(self.size[6]),
            q(self.size[7]),
            q(self.theme[0]),
            q(self.theme[1]),
            q(self.theme[2]),
            q(self.theme[3]),
            q(self.theme[4]),
            q(self.theme[5]),
            q(self.theme[6]),
            q(self.theme[7]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, front_matter};

    #[test]
    fn front_matter_is_read_and_taken_off() {
        let (map, rest, line) =
            front_matter("---\ntitle: \"A: B\"\nlang: de\ntags:\n  - x\n  - y\n---\n# Hi\n");
        assert_eq!(map.get("title").map(String::as_str), Some("A: B"));
        assert_eq!(map.get("tags").map(String::as_str), Some("x, y"));
        assert_eq!(rest, "# Hi\n");
        assert_eq!(line, 8);
    }

    #[test]
    fn a_rule_at_the_top_is_not_front_matter() {
        let (map, rest, _) = front_matter("---\n\nJust text, then a rule.\n");
        assert!(map.is_empty());
        assert!(rest.starts_with("---"));
    }

    #[test]
    fn unknown_settings_are_named() {
        let mut settings = Settings::default();
        assert!(settings.set("brand", "Thomas").is_ok());
        assert!(settings.set("colour", "red").is_err());
    }
}
