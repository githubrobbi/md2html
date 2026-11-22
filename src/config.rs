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
    /// Print a PDF beside the page (with Chrome).
    pub(crate) pdf: Option<bool>,
    /// Write an EPUB beside the page.
    pub(crate) epub: Option<bool>,
    /// Let search engines index the page.
    pub(crate) index: Option<bool>,
    /// Show the outline.
    pub(crate) toc: Option<bool>,
    /// Read `$...$` as maths.
    pub(crate) math: Option<bool>,
    /// How Mermaid diagrams are drawn: native, js or code.
    pub(crate) mermaid: Option<String>,
    /// A local copy of mermaid.js, for the diagrams drawn in the browser.
    pub(crate) mermaid_js: Option<PathBuf>,
    /// The file this page translates (for a translation).
    pub(crate) translated_from: Option<String>,
    /// The SHA-256 of that file when it was translated.
    pub(crate) source_sha256: Option<String>,
    /// Offer a language switch: `false` never, `device` even without translations.
    pub(crate) translations: Option<String>,
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
            "pdf" => self.pdf = flag(),
            "epub" => self.epub = flag(),
            "index" => self.index = flag(),
            "noindex" => self.index = Some(!truthy(value)),
            "toc" | "outline" => self.toc = flag(),
            "math" => self.math = flag(),
            "mermaid" | "diagrams" => {
                let mode = value.trim().to_ascii_lowercase();
                if !["native", "js", "code"].contains(&mode.as_str()) {
                    return Err(format!("mermaid: `{value}` is not native, js or code"));
                }
                self.mermaid = Some(mode);
            }
            "mermaid_js" => self.mermaid_js = Some(PathBuf::from(value.trim())),
            "translated_from" | "translation_of" => self.translated_from = text(),
            "source_sha256" => self.source_sha256 = Some(value.trim().to_ascii_lowercase()),
            "translations" | "language_switch" => {
                self.translations = Some(value.trim().to_ascii_lowercase());
            }
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
            | "status" | "updated" | "created" | "translated_by" | "translator" => {}
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
            pdf: pick(self.pdf.as_ref(), over.pdf.as_ref()),
            epub: pick(self.epub.as_ref(), over.epub.as_ref()),
            index: pick(self.index.as_ref(), over.index.as_ref()),
            toc: pick(self.toc.as_ref(), over.toc.as_ref()),
            math: pick(self.math.as_ref(), over.math.as_ref()),
            mermaid: pick(self.mermaid.as_ref(), over.mermaid.as_ref()),
            mermaid_js: pick(self.mermaid_js.as_ref(), over.mermaid_js.as_ref()),
            translated_from: pick(self.translated_from.as_ref(), over.translated_from.as_ref()),
            source_sha256: pick(self.source_sha256.as_ref(), over.source_sha256.as_ref()),
            translations: pick(self.translations.as_ref(), over.translations.as_ref()),
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
    /// How a date is written: `{d}`, `{m}`, `{y}`.
    pub(crate) date_fmt: &'static str,
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
    /// "How to read this", as an author writes it in this language.
    pub(crate) guide_heading: &'static str,
    /// "Glossary", as an author writes it.
    pub(crate) glossary_heading: &'static str,
    /// "Contents", as an author writes it.
    pub(crate) contents_heading: &'static str,
    /// The language switch: its name, "original", the translation notice
    /// (`{orig}`), on this device, the machine-translation note, show the
    /// original, translating, cannot translate (`{lang}`), more languages.
    pub(crate) lang: [&'static str; 9],
    /// The export menu: its name, this page as HTML, print or PDF, the
    /// PDF, the EPUB.
    pub(crate) export: [&'static str; 5],
    /// The name of the panel of reading settings (text size, theme, saving).
    pub(crate) prefs: &'static str,
    /// The credit at the foot: `{link}` is md2html, linked to its home.
    pub(crate) credit: &'static str,
}

/// The interface in `en`.
const EN: Ui = Ui {
    skip: "Skip to content",
    search: "Search",
    search_page: "Search this page",
    keys: ["move", "open", "close"],
    none: "Nothing on this page matches that.",
    results: "{n} results",
    result: "1 result",
    glossary: "Words used here",
    glossary_intro: "Each dotted word in the chapters shows its meaning when you rest on it and jumps here when you click it. Every entry has a <strong>back</strong> that returns you to the line you were reading.",
    glossary_shared: " The acronyms come from the <a href=\"{url}\">shared glossary</a>; <strong>all terms</strong> opens it at the entry.",
    back: "back",
    all_terms: "all terms",
    keep_outline: "&lsaquo; keep the outline open",
    outline: "Outline",
    outline_tips: [
        "Hide the outline",
        "Show the outline",
        " (⌘\\, Control+\\ on Windows)",
    ],
    notes: "Notes",
    note_n: "Note {n}",
    link_section: "Link to this section",
    copy: ["copy", "copied"],
    callouts: ["Note", "Tip", "Important", "Warning", "Caution"],
    footer: "Rendered from <code>{source}</code> by <code>md2html</code>. The Markdown file is the source; render again after editing it.",
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
    date_fmt: "{d} {m} {y}",
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
    guide_heading: "How to read this",
    glossary_heading: "Glossary",
    contents_heading: "Contents",
    lang: [
        "Language",
        "Original",
        "This is a translation. The original ({orig}) is the authoritative version.",
        "On this device",
        "A machine translation by this browser, on this device: nothing leaves it.",
        "Show the original",
        "Translating…",
        "This browser cannot translate into {lang}.",
        "More languages",
    ],
    export: [
        "Save or print",
        "This page, to read offline (HTML)",
        "Print, or save as PDF",
        "PDF",
        "EPUB, for e-readers",
    ],
    prefs: "Reading settings",
    credit: "Made with {link}",
};

/// The interface in `de`.
const DE: Ui = Ui {
    skip: "Zum Inhalt springen",
    search: "Suchen",
    search_page: "Diese Seite durchsuchen",
    keys: ["wählen", "öffnen", "schließen"],
    none: "Nichts auf dieser Seite passt dazu.",
    results: "{n} Treffer",
    result: "1 Treffer",
    glossary: "Begriffe",
    glossary_intro: "Jedes gepunktet unterstrichene Wort in den Kapiteln zeigt seine Bedeutung, wenn der Zeiger darauf ruht, und springt hierher, wenn man es anklickt. Jeder Eintrag hat ein <strong>zurück</strong>, das zu der Zeile führt, an der man gerade gelesen hat.",
    glossary_shared: " Die Kürzel stammen aus dem <a href=\"{url}\">gemeinsamen Glossar</a>; <strong>alle Begriffe</strong> öffnet es beim Eintrag.",
    back: "zurück",
    all_terms: "alle Begriffe",
    keep_outline: "&lsaquo; Gliederung offen halten",
    outline: "Gliederung",
    outline_tips: [
        "Gliederung ausblenden",
        "Gliederung einblenden",
        " (⌘\\, Strg+\\ unter Windows)",
    ],
    notes: "Anmerkungen",
    note_n: "Anmerkung {n}",
    link_section: "Link zu diesem Abschnitt",
    copy: ["kopieren", "kopiert"],
    callouts: ["Hinweis", "Tipp", "Wichtig", "Achtung", "Vorsicht"],
    footer: "Erstellt aus <code>{source}</code> mit <code>md2html</code>. Die Markdown-Datei ist die Quelle; nach dem Bearbeiten neu erzeugen.",
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
    date_fmt: "{d}. {m} {y}",
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
    guide_heading: "Wie diese Seite zu lesen ist",
    glossary_heading: "Glossar",
    contents_heading: "Inhalt",
    lang: [
        "Sprache",
        "Original",
        "Dies ist eine Übersetzung. Maßgeblich ist das Original ({orig}).",
        "Auf diesem Gerät",
        "Eine maschinelle Übersetzung durch diesen Browser, auf diesem Gerät: nichts verlässt es.",
        "Original zeigen",
        "Wird übersetzt…",
        "Dieser Browser kann nicht ins {lang} übersetzen.",
        "Weitere Sprachen",
    ],
    export: [
        "Speichern oder drucken",
        "Diese Seite, offline zu lesen (HTML)",
        "Drucken oder als PDF sichern",
        "PDF",
        "EPUB, für E-Reader",
    ],
    prefs: "Leseeinstellungen",
    credit: "Erstellt mit {link}",
};

/// The interface in `he`.
const HE: Ui = Ui {
    skip: "דילוג לתוכן",
    search: "חיפוש",
    search_page: "חיפוש בדף זה",
    keys: ["מעבר", "פתיחה", "סגירה"],
    none: "שום דבר בדף זה אינו תואם.",
    results: "{n} תוצאות",
    result: "תוצאה אחת",
    glossary: "מילים שבדף",
    glossary_intro: "כל מילה מנוקדת בפרקים מציגה את פירושה כשמצביעים עליה, ולחיצה עליה מובילה לכאן. בכל ערך יש <strong>חזרה</strong> שמחזירה לשורה שבה קראתם.",
    glossary_shared: " הקיצורים לקוחים מ<a href=\"{url}\">המילון המשותף</a>; <strong>כל המונחים</strong> פותח אותו בערך.",
    back: "חזרה",
    all_terms: "כל המונחים",
    keep_outline: "&rsaquo; להשאיר את תוכן העניינים פתוח",
    outline: "תוכן העניינים",
    outline_tips: [
        "הסתרת תוכן העניינים",
        "הצגת תוכן העניינים",
        " (⌘\\, Control+\\ ב־Windows)",
    ],
    notes: "הערות",
    note_n: "הערה {n}",
    link_section: "קישור לסעיף זה",
    copy: ["העתקה", "הועתק"],
    callouts: ["הערה", "עצה", "חשוב", "אזהרה", "זהירות"],
    footer: "נוצר מ־<code>{source}</code> באמצעות <code>md2html</code>. קובץ ה־Markdown הוא המקור; יש ליצור מחדש לאחר עריכה.",
    months: [
        "ינואר",
        "פברואר",
        "מרץ",
        "אפריל",
        "מאי",
        "יוני",
        "יולי",
        "אוגוסט",
        "ספטמבר",
        "אוקטובר",
        "נובמבר",
        "דצמבר",
    ],
    date_fmt: "{d} ב{m} {y}",
    top: "ראש הדף",
    figure: ["האיור, מוגדל", "סגירה", "סגירה (Escape)", "{i} מתוך {n}"],
    size: [
        "גודל טקסט",
        "טקסט קטן יותר, {n}%",
        "טקסט גדול יותר, {n}%",
        "גודל טקסט: 100%, הדף כפי שנכתב.",
        "גודל טקסט: {n}%. לחיצה מחזירה ל־100%.",
        " Command (Control ב־Windows) עם + או - משנה אותו כאן, 0 מאפס; הזום של הדפדפן אינו משתנה.",
        "הטקסט כבר בגודל הקטן ביותר",
        "הטקסט כבר בגודל הגדול ביותר",
    ],
    theme: [
        "ערכת צבעים",
        "ערכת צבעים: {now}. לחיצה עוברת ל{next}, ואחר כך ל{after}.",
        "מערכת, כרגע {mode}",
        "מערכת",
        "בהירה",
        "כהה",
        "כהה",
        "בהירה",
    ],
    table: "טבלה",
    pages: ["הדף הקודם", "הדף הבא", "דפים"],
    task: ["בוצע", "פתוח"],
    guide_heading: "איך לקרוא את זה",
    glossary_heading: "מילון מונחים",
    contents_heading: "תוכן עניינים",
    lang: [
        "שפה",
        "מקור",
        "זהו תרגום. הנוסח המחייב הוא המקור ({orig}).",
        "במכשיר זה",
        "תרגום מכונה של הדפדפן, במכשיר זה: דבר אינו יוצא ממנו.",
        "הצגת המקור",
        "מתרגם…",
        "הדפדפן אינו יכול לתרגם ל{lang}.",
        "שפות נוספות",
    ],
    export: [
        "שמירה או הדפסה",
        "הדף הזה, לקריאה ללא חיבור (HTML)",
        "הדפסה או שמירה כ־PDF",
        "PDF",
        "EPUB, לקוראים אלקטרוניים",
    ],
    prefs: "הגדרות קריאה",
    credit: "נוצר באמצעות {link}",
};

/// The interface in `id`.
const ID: Ui = Ui {
    skip: "Langsung ke isi",
    search: "Cari",
    search_page: "Cari di halaman ini",
    keys: ["pindah", "buka", "tutup"],
    none: "Tidak ada yang cocok di halaman ini.",
    results: "{n} hasil",
    result: "1 hasil",
    glossary: "Istilah di halaman ini",
    glossary_intro: "Setiap kata bergaris titik di dalam bab menampilkan artinya saat penunjuk berada di atasnya, dan melompat ke sini saat diklik. Setiap entri memiliki tombol <strong>kembali</strong> ke baris yang sedang Anda baca.",
    glossary_shared: " Singkatan diambil dari <a href=\"{url}\">glosarium bersama</a>; <strong>semua istilah</strong> membukanya pada entri tersebut.",
    back: "kembali",
    all_terms: "semua istilah",
    keep_outline: "&lsaquo; biarkan kerangka tetap terbuka",
    outline: "Kerangka",
    outline_tips: [
        "Sembunyikan kerangka",
        "Tampilkan kerangka",
        " (⌘\\, Control+\\ di Windows)",
    ],
    notes: "Catatan",
    note_n: "Catatan {n}",
    link_section: "Tautan ke bagian ini",
    copy: ["salin", "tersalin"],
    callouts: ["Catatan", "Tips", "Penting", "Peringatan", "Awas"],
    footer: "Dibuat dari <code>{source}</code> oleh <code>md2html</code>. Berkas Markdown adalah sumbernya; buat ulang setelah menyuntingnya.",
    months: [
        "Januari",
        "Februari",
        "Maret",
        "April",
        "Mei",
        "Juni",
        "Juli",
        "Agustus",
        "September",
        "Oktober",
        "November",
        "Desember",
    ],
    date_fmt: "{d} {m} {y}",
    top: "Bagian atas halaman",
    figure: [
        "Gambar, diperbesar",
        "Tutup",
        "Tutup (Escape)",
        "{i} dari {n}",
    ],
    size: [
        "Ukuran teks",
        "Teks lebih kecil, {n}%",
        "Teks lebih besar, {n}%",
        "Ukuran teks: 100%, halaman seperti ditulis.",
        "Ukuran teks: {n}%. Klik untuk kembali ke 100%.",
        " Command (Control di Windows) dengan + atau - mengubahnya di sini, 0 mengembalikan; zoom peramban tidak diubah.",
        "Sudah teks terkecil",
        "Sudah teks terbesar",
    ],
    theme: [
        "Tema",
        "Tema: {now}. Klik untuk {next}, lalu {after}.",
        "Sistem, saat ini {mode}",
        "Sistem",
        "Terang",
        "Gelap",
        "gelap",
        "terang",
    ],
    table: "Tabel",
    pages: ["Sebelumnya", "Berikutnya", "Halaman"],
    task: ["Selesai", "Belum"],
    guide_heading: "Cara membaca halaman ini",
    glossary_heading: "Glosarium",
    contents_heading: "Daftar isi",
    lang: [
        "Bahasa",
        "Asli",
        "Ini adalah terjemahan. Versi yang berlaku adalah aslinya ({orig}).",
        "Di perangkat ini",
        "Terjemahan mesin oleh peramban ini, di perangkat ini: tidak ada yang keluar darinya.",
        "Tampilkan aslinya",
        "Menerjemahkan…",
        "Peramban ini tidak dapat menerjemahkan ke {lang}.",
        "Bahasa lainnya",
    ],
    export: [
        "Simpan atau cetak",
        "Halaman ini, untuk dibaca offline (HTML)",
        "Cetak, atau simpan sebagai PDF",
        "PDF",
        "EPUB, untuk pembaca e-book",
    ],
    prefs: "Pengaturan baca",
    credit: "Dibuat dengan {link}",
};

/// The interface in `nl`.
const NL: Ui = Ui {
    skip: "Naar de inhoud",
    search: "Zoeken",
    search_page: "Deze pagina doorzoeken",
    keys: ["kiezen", "openen", "sluiten"],
    none: "Niets op deze pagina komt overeen.",
    results: "{n} resultaten",
    result: "1 resultaat",
    glossary: "Woorden op deze pagina",
    glossary_intro: "Elk gestippeld woord in de hoofdstukken toont zijn betekenis als u erop rust en springt hierheen als u erop klikt. Elk item heeft een <strong>terug</strong> naar de regel die u aan het lezen was.",
    glossary_shared: " De afkortingen komen uit de <a href=\"{url}\">gedeelde woordenlijst</a>; <strong>alle termen</strong> opent die bij het item.",
    back: "terug",
    all_terms: "alle termen",
    keep_outline: "&lsaquo; overzicht open houden",
    outline: "Overzicht",
    outline_tips: [
        "Overzicht verbergen",
        "Overzicht tonen",
        " (⌘\\, Ctrl+\\ op Windows)",
    ],
    notes: "Noten",
    note_n: "Noot {n}",
    link_section: "Link naar dit gedeelte",
    copy: ["kopiëren", "gekopieerd"],
    callouts: ["Opmerking", "Tip", "Belangrijk", "Waarschuwing", "Let op"],
    footer: "Gemaakt van <code>{source}</code> met <code>md2html</code>. Het Markdown-bestand is de bron; maak de pagina opnieuw na het bewerken.",
    months: [
        "januari",
        "februari",
        "maart",
        "april",
        "mei",
        "juni",
        "juli",
        "augustus",
        "september",
        "oktober",
        "november",
        "december",
    ],
    date_fmt: "{d} {m} {y}",
    top: "Begin van de pagina",
    figure: [
        "De afbeelding, vergroot",
        "Sluiten",
        "Sluiten (Escape)",
        "{i} van {n}",
    ],
    size: [
        "Tekstgrootte",
        "Kleinere tekst, {n}%",
        "Grotere tekst, {n}%",
        "Tekstgrootte: 100%, de pagina zoals geschreven.",
        "Tekstgrootte: {n}%. Klik om terug te gaan naar 100%.",
        " Command (Ctrl op Windows) met + of - verandert hem hier, 0 zet terug; de zoom van de browser blijft ongemoeid.",
        "Al de kleinste tekst",
        "Al de grootste tekst",
    ],
    theme: [
        "Thema",
        "Thema: {now}. Klik voor {next}, daarna {after}.",
        "Systeem, nu {mode}",
        "Systeem",
        "Licht",
        "Donker",
        "donker",
        "licht",
    ],
    table: "Tabel",
    pages: ["Vorige", "Volgende", "Pagina's"],
    task: ["Klaar", "Open"],
    guide_heading: "Hoe u dit leest",
    glossary_heading: "Woordenlijst",
    contents_heading: "Inhoud",
    lang: [
        "Taal",
        "Origineel",
        "Dit is een vertaling. Het origineel ({orig}) is de geldende versie.",
        "Op dit apparaat",
        "Een machinevertaling door deze browser, op dit apparaat: er verlaat niets het apparaat.",
        "Origineel tonen",
        "Bezig met vertalen…",
        "Deze browser kan niet naar het {lang} vertalen.",
        "Meer talen",
    ],
    export: [
        "Bewaren of afdrukken",
        "Deze pagina, om offline te lezen (HTML)",
        "Afdrukken of als PDF bewaren",
        "PDF",
        "EPUB, voor e-readers",
    ],
    prefs: "Leesinstellingen",
    credit: "Gemaakt met {link}",
};

/// The interface in `jv`.
const JV: Ui = Ui {
    skip: "Langsung menyang isi",
    search: "Golèk",
    search_page: "Golèk ing kaca iki",
    keys: ["pindhah", "bukak", "tutup"],
    none: "Ora ana sing cocog ing kaca iki.",
    results: "{n} asil",
    result: "1 asil",
    glossary: "Tembung ing kaca iki",
    glossary_intro: "Saben tembung kanthi garis titik ing bab nuduhaké tegesé yèn penunjuk ana ing ndhuwuré, lan mlumpat mréné yèn diklik. Saben entri duwé <strong>bali</strong> menyang larik sing lagi diwaca.",
    glossary_shared: " Cekakan dijupuk saka <a href=\"{url}\">glosarium bebarengan</a>; <strong>kabèh istilah</strong> mbukak ing entri kasebut.",
    back: "bali",
    all_terms: "kabèh istilah",
    keep_outline: "&lsaquo; kerangka tetep dibukak",
    outline: "Kerangka",
    outline_tips: [
        "Dhelikaké kerangka",
        "Tuduhaké kerangka",
        " (⌘\\, Control+\\ ing Windows)",
    ],
    notes: "Cathetan",
    note_n: "Cathetan {n}",
    link_section: "Pranala menyang bagéan iki",
    copy: ["salin", "wis disalin"],
    callouts: ["Cathetan", "Pitutur", "Wigati", "Pènget", "Ati-ati"],
    footer: "Digawé saka <code>{source}</code> nganggo <code>md2html</code>. Berkas Markdown iku sumberé; gawé manèh sawisé diowahi.",
    months: [
        "Januari",
        "Fèbruari",
        "Maret",
        "April",
        "Mèi",
        "Juni",
        "Juli",
        "Agustus",
        "Sèptèmber",
        "Oktober",
        "Nopèmber",
        "Désèmber",
    ],
    date_fmt: "{d} {m} {y}",
    top: "Pucuk kaca",
    figure: [
        "Gambar, digedhèkaké",
        "Tutup",
        "Tutup (Escape)",
        "{i} saka {n}",
    ],
    size: [
        "Ukuran teks",
        "Teks luwih cilik, {n}%",
        "Teks luwih gedhé, {n}%",
        "Ukuran teks: 100%, kaca kaya ditulis.",
        "Ukuran teks: {n}%. Klik kanggo bali menyang 100%.",
        " Command (Control ing Windows) karo + utawa - ngowahi ing kéné, 0 mbalèkaké; zoom peramban ora diowahi.",
        "Wis teks paling cilik",
        "Wis teks paling gedhé",
    ],
    theme: [
        "Téma",
        "Téma: {now}. Klik kanggo {next}, banjur {after}.",
        "Sistem, saiki {mode}",
        "Sistem",
        "Padhang",
        "Peteng",
        "peteng",
        "padhang",
    ],
    table: "Tabel",
    pages: ["Sadurungé", "Sabanjuré", "Kaca"],
    task: ["Rampung", "Durung"],
    guide_heading: "Carané maca kaca iki",
    glossary_heading: "Glosarium",
    contents_heading: "Isi",
    lang: [
        "Basa",
        "Asli",
        "Iki terjemahan. Vèrsi sing dadi pathokan yaiku asliné ({orig}).",
        "Ing piranti iki",
        "Terjemahan mesin déning peramban iki, ing piranti iki: ora ana sing metu saka piranti.",
        "Tuduhaké asliné",
        "Lagi nerjemahaké…",
        "Peramban iki ora bisa nerjemahaké menyang {lang}.",
        "Basa liyané",
    ],
    export: [
        "Simpen utawa cithak",
        "Kaca iki, kanggo diwaca tanpa internet (HTML)",
        "Cithak, utawa simpen dadi PDF",
        "PDF",
        "EPUB, kanggo e-reader",
    ],
    prefs: "Setelan maca",
    credit: "Digawe nganggo {link}",
};

/// The interface in `yi`.
const YI: Ui = Ui {
    skip: "שפּרינג צום אינהאַלט",
    search: "זוכן",
    search_page: "זוכן אויף דעם בלאַט",
    keys: ["רירן", "עפֿענען", "פֿאַרמאַכן"],
    none: "גאָרנישט אויף דעם בלאַט שטימט.",
    results: "{n} רעזולטאַטן",
    result: "איין רעזולטאַט",
    glossary: "ווערטער אויף דעם בלאַט",
    glossary_intro: "יעדעס פּינטלדיק וואָרט אין די קאַפּיטלען ווײַזט זײַן באַטײַט ווען מע שטעלט אויף אים דעם טײַטל, און שפּרינגט אַהער ווען מע קליקט. יעדער אַרײַנטראָג האָט אַ <strong>צוריק</strong> צו דער שורה וואָס מע האָט געלייענט.",
    glossary_shared: " די ראַשי־תּיבֿות קומען פֿונעם <a href=\"{url}\">געמיינזאַמען ווערטערבוך</a>; <strong>אַלע טערמינען</strong> עפֿנט עס בײַם אַרײַנטראָג.",
    back: "צוריק",
    all_terms: "אַלע טערמינען",
    keep_outline: "&rsaquo; האַלטן דעם אינהאַלט אָפֿן",
    outline: "אינהאַלט",
    outline_tips: [
        "באַהאַלטן דעם אינהאַלט",
        "ווײַזן דעם אינהאַלט",
        " (⌘\\, Control+\\ אויף Windows)",
    ],
    notes: "הערות",
    note_n: "הערה {n}",
    link_section: "לינק צו דעם אָפּטייל",
    copy: ["קאָפּירן", "קאָפּירט"],
    callouts: ["באַמערקונג", "עצה", "וויכטיק", "וואָרענונג", "אָפּגעהיט"],
    footer: "געמאַכט פֿון <code>{source}</code> מיט <code>md2html</code>. די Markdown־טעקע איז דער מקור; מאַכט אויף ס׳נײַ נאָכן רעדאַגירן.",
    months: [
        "יאַנואַר",
        "פֿעברואַר",
        "מערץ",
        "אַפּריל",
        "מיי",
        "יוני",
        "יולי",
        "אויגוסט",
        "סעפּטעמבער",
        "אָקטאָבער",
        "נאָוועמבער",
        "דעצעמבער",
    ],
    date_fmt: "{d} {m} {y}",
    top: "אָנהייב פֿונעם בלאַט",
    figure: [
        "דאָס בילד, פֿאַרגרעסערט",
        "פֿאַרמאַכן",
        "פֿאַרמאַכן (Escape)",
        "{i} פֿון {n}",
    ],
    size: [
        "גרייס פֿונעם טעקסט",
        "קלענערער טעקסט, {n}%",
        "גרעסערער טעקסט, {n}%",
        "גרייס פֿונעם טעקסט: 100%, דער בלאַט ווי געשריבן.",
        "גרייס פֿונעם טעקסט: {n}%. קליקט צוריק צו 100%.",
        " Command (Control אויף Windows) מיט + אָדער - ענדערט עס דאָ, 0 שטעלט צוריק; דער זום פֿונעם בלעטערער בלײַבט אומגערירט.",
        "שוין דער קלענסטער טעקסט",
        "שוין דער גרעסטער טעקסט",
    ],
    theme: [
        "קאָלירן",
        "קאָלירן: {now}. קליקט פֿאַר {next}, דערנאָך {after}.",
        "סיסטעם, איצט {mode}",
        "סיסטעם",
        "ליכטיק",
        "טונקל",
        "טונקל",
        "ליכטיק",
    ],
    table: "טאַבעלע",
    pages: ["פֿריִערדיקער בלאַט", "קומענדיקער בלאַט", "בלעטער"],
    task: ["געטאָן", "אָפֿן"],
    guide_heading: "ווי צו לייענען דאָס",
    glossary_heading: "ווערטערבוך",
    contents_heading: "אינהאַלט",
    lang: [
        "שפּראַך",
        "אָריגינאַל",
        "דאָס איז אַן איבערזעצונג. דער גילטיקער נוסח איז דער אָריגינאַל ({orig}).",
        "אויף דעם אַפּאַראַט",
        "אַ מאַשין־איבערזעצונג פֿונעם בלעטערער, אויף דעם אַפּאַראַט: גאָרנישט גייט אַרויס.",
        "ווײַזן דעם אָריגינאַל",
        "מע זעצט איבער…",
        "דער בלעטערער קען נישט איבערזעצן אויף {lang}.",
        "מער שפּראַכן",
    ],
    export: [
        "אָפּהיטן אָדער דרוקן",
        "דער בלאַט, צו לייענען אָן אינטערנעץ (HTML)",
        "דרוקן, אָדער אָפּהיטן ווי PDF",
        "PDF",
        "EPUB, פֿאַר עלעקטראָנישע לייענערס",
    ],
    prefs: "לייען־איינשטעלונגען",
    credit: "געמאַכט מיט {link}",
};

/// The interface in `nds`.
const NDS: Ui = Ui {
    skip: "Na'n Inholt springen",
    search: "Söken",
    search_page: "Disse Siet dörsöken",
    keys: ["wählen", "opmaken", "dichtmaken"],
    none: "Nix op disse Siet passt dorto.",
    results: "{n} Funnen",
    result: "1 Fund",
    glossary: "Wöör op disse Siet",
    glossary_intro: "Elk Woort mit Pünkten ünner in de Kapitels wiest sien Bedüden, wenn de Wieser dor op liggt, un springt hierher, wenn een dor op klickt. Elk Indrag hett en <strong>trüch</strong> na de Reeg, de een jüst leest hett.",
    glossary_shared: " De Afkörten kaamt ut de <a href=\"{url}\">gemeensame Woortlist</a>; <strong>all Wöör</strong> maakt de bi'n Indrag op.",
    back: "trüch",
    all_terms: "all Wöör",
    keep_outline: "&lsaquo; Översicht open laten",
    outline: "Översicht",
    outline_tips: [
        "Översicht wegdoon",
        "Översicht wiesen",
        " (⌘\\, Strg+\\ ünner Windows)",
    ],
    notes: "Anmarken",
    note_n: "Anmarken {n}",
    link_section: "Link na dissen Afsnitt",
    copy: ["koperen", "kopeert"],
    callouts: ["Henwies", "Tipp", "Wichtig", "Acht geven", "Vörsicht"],
    footer: "Maakt ut <code>{source}</code> mit <code>md2html</code>. De Markdown-Datei is de Born; na't Ännern nee maken.",
    months: [
        "Januaar",
        "Februaar",
        "März",
        "April",
        "Mai",
        "Juni",
        "Juli",
        "August",
        "September",
        "Oktover",
        "November",
        "Dezember",
    ],
    date_fmt: "{d}. {m} {y}",
    top: "Baven op de Siet",
    figure: [
        "Dat Bild, grötter maakt",
        "Dichtmaken",
        "Dichtmaken (Escape)",
        "{i} vun {n}",
    ],
    size: [
        "Schriftgrött",
        "Lüttere Schrift, {n} %",
        "Gröttere Schrift, {n} %",
        "Schriftgrött: 100 %, de Siet as schreven.",
        "Schriftgrött: {n} %. Klick för 100 %.",
        " Befehlstast (Strg ünner Windows) mit + oder - ännert ehr hier, 0 sett trüch; de Zoom vun'n Browser blifft as he is.",
        "Al de lüttste Schrift",
        "Al de gröttste Schrift",
    ],
    theme: [
        "Utsehn",
        "Utsehn: {now}. Klick för {next}, denn {after}.",
        "System, jüst {mode}",
        "System",
        "Hell",
        "Düster",
        "düster",
        "hell",
    ],
    table: "Tabell",
    pages: ["Vörige Siet", "Neegste Siet", "Sieden"],
    task: ["Daan", "Open"],
    guide_heading: "So leest een dat",
    glossary_heading: "Woortlist",
    contents_heading: "Inholt",
    lang: [
        "Spraak",
        "Original",
        "Dit is en Översetten. Dat gellt, wat in't Original ({orig}) steiht.",
        "Op dit Reedschap",
        "En Maschien-Översetten vun dissen Browser, op dit Reedschap: nix geiht dor rut.",
        "Original wiesen",
        "Warrt översett…",
        "Disse Browser kann nich op {lang} översetten.",
        "Mehr Spraken",
    ],
    export: [
        "Spiekern oder drucken",
        "Disse Siet, ahn Internet to lesen (HTML)",
        "Drucken oder as PDF spiekern",
        "PDF",
        "EPUB, för E-Reader",
    ],
    prefs: "Leesinstellen",
    credit: "Maakt mit {link}",
};

/// The interface in `fr`.
const FR: Ui = Ui {
    skip: "Aller au contenu",
    search: "Rechercher",
    search_page: "Rechercher dans cette page",
    keys: ["choisir", "ouvrir", "fermer"],
    none: "Rien sur cette page ne correspond.",
    results: "{n} résultats",
    result: "1 résultat",
    glossary: "Les mots de cette page",
    glossary_intro: "Chaque mot souligné de pointillés dans les chapitres affiche son sens au survol et mène ici d'un clic. Chaque entrée a un <strong>retour</strong> vers la ligne que vous lisiez.",
    glossary_shared: " Les sigles viennent du <a href=\"{url}\">glossaire commun</a> ; <strong>tous les termes</strong> l'ouvre à l'entrée.",
    back: "retour",
    all_terms: "tous les termes",
    keep_outline: "&lsaquo; garder le plan ouvert",
    outline: "Plan",
    outline_tips: [
        "Masquer le plan",
        "Afficher le plan",
        " (⌘\\, Ctrl+\\ sous Windows)",
    ],
    notes: "Notes",
    note_n: "Note {n}",
    link_section: "Lien vers cette section",
    copy: ["copier", "copié"],
    callouts: [
        "Remarque",
        "Conseil",
        "Important",
        "Avertissement",
        "Attention",
    ],
    footer: "Produit à partir de <code>{source}</code> par <code>md2html</code>. Le fichier Markdown est la source ; régénérez la page après l'avoir modifié.",
    months: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    date_fmt: "{d} {m} {y}",
    top: "Haut de la page",
    figure: [
        "La figure, agrandie",
        "Fermer",
        "Fermer (Échap)",
        "{i} sur {n}",
    ],
    size: [
        "Taille du texte",
        "Texte plus petit, {n} %",
        "Texte plus grand, {n} %",
        "Taille du texte : 100 %, la page telle qu'écrite.",
        "Taille du texte : {n} %. Cliquez pour revenir à 100 %.",
        " Commande (Ctrl sous Windows) avec + ou - la modifie ici, 0 la rétablit ; le zoom du navigateur n'est pas touché.",
        "Déjà le plus petit texte",
        "Déjà le plus grand texte",
    ],
    theme: [
        "Thème",
        "Thème : {now}. Cliquez pour {next}, puis {after}.",
        "Système, actuellement {mode}",
        "Système",
        "Clair",
        "Sombre",
        "sombre",
        "clair",
    ],
    table: "Tableau",
    pages: ["Précédent", "Suivant", "Pages"],
    task: ["Fait", "À faire"],
    guide_heading: "Comment lire ceci",
    glossary_heading: "Glossaire",
    contents_heading: "Sommaire",
    lang: [
        "Langue",
        "Original",
        "Ceci est une traduction. La version qui fait foi est l'original ({orig}).",
        "Sur cet appareil",
        "Une traduction automatique par ce navigateur, sur cet appareil : rien n'en sort.",
        "Afficher l'original",
        "Traduction…",
        "Ce navigateur ne peut pas traduire en {lang}.",
        "Autres langues",
    ],
    export: [
        "Enregistrer ou imprimer",
        "Cette page, à lire hors ligne (HTML)",
        "Imprimer, ou enregistrer en PDF",
        "PDF",
        "EPUB, pour liseuses",
    ],
    prefs: "Réglages de lecture",
    credit: "Réalisé avec {link}",
};

/// The interface in `it`.
const IT: Ui = Ui {
    skip: "Vai al contenuto",
    search: "Cerca",
    search_page: "Cerca in questa pagina",
    keys: ["scegli", "apri", "chiudi"],
    none: "Niente in questa pagina corrisponde.",
    results: "{n} risultati",
    result: "1 risultato",
    glossary: "Le parole di questa pagina",
    glossary_intro: "Ogni parola sottolineata a puntini nei capitoli mostra il suo significato quando ci si posa sopra e porta qui con un clic. Ogni voce ha un <strong>indietro</strong> che riporta alla riga che stavate leggendo.",
    glossary_shared: " Le sigle vengono dal <a href=\"{url}\">glossario comune</a>; <strong>tutti i termini</strong> lo apre alla voce.",
    back: "indietro",
    all_terms: "tutti i termini",
    keep_outline: "&lsaquo; tieni aperto l'indice",
    outline: "Indice",
    outline_tips: [
        "Nascondi l'indice",
        "Mostra l'indice",
        " (⌘\\, Ctrl+\\ su Windows)",
    ],
    notes: "Note",
    note_n: "Nota {n}",
    link_section: "Link a questa sezione",
    copy: ["copia", "copiato"],
    callouts: ["Nota", "Suggerimento", "Importante", "Avviso", "Attenzione"],
    footer: "Generato da <code>{source}</code> con <code>md2html</code>. Il file Markdown è la fonte; rigenerare dopo averlo modificato.",
    months: [
        "gennaio",
        "febbraio",
        "marzo",
        "aprile",
        "maggio",
        "giugno",
        "luglio",
        "agosto",
        "settembre",
        "ottobre",
        "novembre",
        "dicembre",
    ],
    date_fmt: "{d} {m} {y}",
    top: "Inizio pagina",
    figure: [
        "La figura, ingrandita",
        "Chiudi",
        "Chiudi (Esc)",
        "{i} di {n}",
    ],
    size: [
        "Dimensione del testo",
        "Testo più piccolo, {n}%",
        "Testo più grande, {n}%",
        "Dimensione del testo: 100%, la pagina come scritta.",
        "Dimensione del testo: {n}%. Clic per tornare al 100%.",
        " Command (Ctrl su Windows) con + o - la cambia qui, 0 la ripristina; lo zoom del browser resta invariato.",
        "Già il testo più piccolo",
        "Già il testo più grande",
    ],
    theme: [
        "Tema",
        "Tema: {now}. Clic per {next}, poi {after}.",
        "Sistema, ora {mode}",
        "Sistema",
        "Chiaro",
        "Scuro",
        "scuro",
        "chiaro",
    ],
    table: "Tabella",
    pages: ["Precedente", "Successiva", "Pagine"],
    task: ["Fatto", "Da fare"],
    guide_heading: "Come leggere questa pagina",
    glossary_heading: "Glossario",
    contents_heading: "Indice",
    lang: [
        "Lingua",
        "Originale",
        "Questa è una traduzione. Fa fede l'originale ({orig}).",
        "Su questo dispositivo",
        "Una traduzione automatica di questo browser, su questo dispositivo: nulla ne esce.",
        "Mostra l'originale",
        "Traduzione in corso…",
        "Questo browser non può tradurre in {lang}.",
        "Altre lingue",
    ],
    export: [
        "Salva o stampa",
        "Questa pagina, da leggere offline (HTML)",
        "Stampa, o salva come PDF",
        "PDF",
        "EPUB, per e-reader",
    ],
    prefs: "Impostazioni di lettura",
    credit: "Realizzato con {link}",
};

/// The interface in `es`.
const ES: Ui = Ui {
    skip: "Ir al contenido",
    search: "Buscar",
    search_page: "Buscar en esta página",
    keys: ["elegir", "abrir", "cerrar"],
    none: "Nada en esta página coincide.",
    results: "{n} resultados",
    result: "1 resultado",
    glossary: "Las palabras de esta página",
    glossary_intro: "Cada palabra subrayada con puntos en los capítulos muestra su significado al posarse sobre ella y lleva aquí con un clic. Cada entrada tiene un <strong>volver</strong> a la línea que estabais leyendo.",
    glossary_shared: " Las siglas vienen del <a href=\"{url}\">glosario común</a>; <strong>todos los términos</strong> lo abre en la entrada.",
    back: "volver",
    all_terms: "todos los términos",
    keep_outline: "&lsaquo; mantener el índice abierto",
    outline: "Índice",
    outline_tips: [
        "Ocultar el índice",
        "Mostrar el índice",
        " (⌘\\, Ctrl+\\ en Windows)",
    ],
    notes: "Notas",
    note_n: "Nota {n}",
    link_section: "Enlace a esta sección",
    copy: ["copiar", "copiado"],
    callouts: ["Nota", "Consejo", "Importante", "Advertencia", "Precaución"],
    footer: "Generado a partir de <code>{source}</code> con <code>md2html</code>. El archivo Markdown es la fuente; vuelva a generarlo después de editarlo.",
    months: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    date_fmt: "{d} de {m} de {y}",
    top: "Inicio de la página",
    figure: [
        "La figura, ampliada",
        "Cerrar",
        "Cerrar (Esc)",
        "{i} de {n}",
    ],
    size: [
        "Tamaño del texto",
        "Texto más pequeño, {n} %",
        "Texto más grande, {n} %",
        "Tamaño del texto: 100 %, la página tal como se escribió.",
        "Tamaño del texto: {n} %. Haga clic para volver al 100 %.",
        " Comando (Ctrl en Windows) con + o - lo cambia aquí, 0 lo restablece; el zoom del navegador no se toca.",
        "Ya es el texto más pequeño",
        "Ya es el texto más grande",
    ],
    theme: [
        "Tema",
        "Tema: {now}. Haga clic para {next} y luego {after}.",
        "Sistema, ahora {mode}",
        "Sistema",
        "Claro",
        "Oscuro",
        "oscuro",
        "claro",
    ],
    table: "Tabla",
    pages: ["Anterior", "Siguiente", "Páginas"],
    task: ["Hecho", "Pendiente"],
    guide_heading: "Cómo leer esto",
    glossary_heading: "Glosario",
    contents_heading: "Índice",
    lang: [
        "Idioma",
        "Original",
        "Esto es una traducción. La versión que da fe es el original ({orig}).",
        "En este dispositivo",
        "Una traducción automática de este navegador, en este dispositivo: nada sale de él.",
        "Mostrar el original",
        "Traduciendo…",
        "Este navegador no puede traducir al {lang}.",
        "Más idiomas",
    ],
    export: [
        "Guardar o imprimir",
        "Esta página, para leer sin conexión (HTML)",
        "Imprimir, o guardar como PDF",
        "PDF",
        "EPUB, para lectores electrónicos",
    ],
    prefs: "Ajustes de lectura",
    credit: "Hecho con {link}",
};

/// The interface in `ja`.
const JA: Ui = Ui {
    skip: "本文へ移動",
    search: "検索",
    search_page: "このページを検索",
    keys: ["移動", "開く", "閉じる"],
    none: "このページに一致するものはありません。",
    results: "{n} 件",
    result: "1 件",
    glossary: "このページの用語",
    glossary_intro: "各章の点線の付いた語にポインタを置くと意味が表示され、クリックするとここへ移動します。各項目の<strong>戻る</strong>で、読んでいた行に戻れます。",
    glossary_shared: " 略語は<a href=\"{url}\">共通の用語集</a>によるものです。<strong>すべての用語</strong>で該当する項目を開きます。",
    back: "戻る",
    all_terms: "すべての用語",
    keep_outline: "&lsaquo; 目次を開いたままにする",
    outline: "目次",
    outline_tips: [
        "目次を隠す",
        "目次を表示",
        "（⌘\\、Windows では Control+\\）",
    ],
    notes: "注",
    note_n: "注 {n}",
    link_section: "このセクションへのリンク",
    copy: ["コピー", "コピーしました"],
    callouts: ["メモ", "ヒント", "重要", "警告", "注意"],
    footer: "<code>{source}</code> から <code>md2html</code> で生成しました。Markdown ファイルが原本です。編集したら生成し直してください。",
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: "{y}年{m}{d}日",
    top: "ページの先頭",
    figure: ["拡大した図", "閉じる", "閉じる（Escape）", "{i} / {n}"],
    size: [
        "文字サイズ",
        "文字を小さく、{n}%",
        "文字を大きく、{n}%",
        "文字サイズ：100%（元の大きさ）。",
        "文字サイズ：{n}%。クリックで 100% に戻ります。",
        " Command（Windows では Control）と + / - でここで変更し、0 で元に戻ります。ブラウザのズームには影響しません。",
        "これ以上小さくできません",
        "これ以上大きくできません",
    ],
    theme: [
        "表示",
        "表示：{now}。クリックで{next}、次に{after}。",
        "システム（現在は{mode}）",
        "システム",
        "ライト",
        "ダーク",
        "ダーク",
        "ライト",
    ],
    table: "表",
    pages: ["前へ", "次へ", "ページ"],
    task: ["完了", "未完了"],
    guide_heading: "このページの読み方",
    glossary_heading: "用語集",
    contents_heading: "目次",
    lang: [
        "言語",
        "原文",
        "これは翻訳です。正式な版は原文（{orig}）です。",
        "この端末で",
        "このブラウザによる、この端末上での機械翻訳です。外部には何も送信されません。",
        "原文を表示",
        "翻訳中…",
        "このブラウザは{lang}に翻訳できません。",
        "その他の言語",
    ],
    export: [
        "保存・印刷",
        "このページ（オフラインで読める HTML）",
        "印刷、または PDF として保存",
        "PDF",
        "EPUB（電子書籍リーダー用）",
    ],
    prefs: "表示の設定",
    credit: "{link} で作成",
};

/// The interface in `ja-Hira`.
const JA_HIRA: Ui = Ui {
    skip: "ほんぶんへ すすむ",
    search: "さがす",
    search_page: "この ページを さがす",
    keys: ["うごく", "ひらく", "とじる"],
    none: "この ページには あてはまる ものが ありません。",
    results: "{n} けん",
    result: "1 けん",
    glossary: "この ページの ことば",
    glossary_intro: "しょうの なかで てんせんの ついた ことばに ポインターを のせると いみが でて、クリックすると ここへ きます。どの こうもくにも <strong>もどる</strong> が あり、よんで いた ぎょうに もどれます。",
    glossary_shared: " りゃくごは <a href=\"{url}\">きょうつうの ことばの いみ</a>から きて います。<strong>すべての ことば</strong>で その こうもくを ひらきます。",
    back: "もどる",
    all_terms: "すべての ことば",
    keep_outline: "&lsaquo; もくじを ひらいた ままに する",
    outline: "もくじ",
    outline_tips: [
        "もくじを かくす",
        "もくじを だす",
        "（⌘\\、Windows では Control+\\）",
    ],
    notes: "ちゅう",
    note_n: "ちゅう {n}",
    link_section: "この ぶぶんへの リンク",
    copy: ["コピー", "コピーしました"],
    callouts: ["メモ", "ヒント", "たいせつ", "けいこく", "ちゅうい"],
    footer: "<code>{source}</code> から <code>md2html</code> で つくりました。もとは Markdown ファイルです。なおしたら もう いちど つくって ください。",
    months: [
        "1がつ", "2がつ", "3がつ", "4がつ", "5がつ", "6がつ", "7がつ", "8がつ", "9がつ", "10がつ",
        "11がつ", "12がつ",
    ],
    date_fmt: "{y}ねん {m} {d}にち",
    top: "ページの いちばん うえ",
    figure: [
        "おおきく した ず",
        "とじる",
        "とじる（Escape）",
        "{i} / {n}",
    ],
    size: [
        "もじの おおきさ",
        "もじを ちいさく、{n}%",
        "もじを おおきく、{n}%",
        "もじの おおきさ：100%（もとの おおきさ）。",
        "もじの おおきさ：{n}%。クリックで 100% に もどります。",
        " Command（Windows では Control）と + / - で かえられ、0 で もとに もどります。ブラウザの ズームは かわりません。",
        "これより ちいさく できません",
        "これより おおきく できません",
    ],
    theme: [
        "みため",
        "みため：{now}。クリックで {next}、つぎに {after}。",
        "システム（いまは {mode}）",
        "システム",
        "あかるい",
        "くらい",
        "くらい",
        "あかるい",
    ],
    table: "ひょう",
    pages: ["まえへ", "つぎへ", "ページ"],
    task: ["おわり", "まだ"],
    guide_heading: "この ページの よみかた",
    glossary_heading: "ことばの いみ",
    contents_heading: "もくじ",
    lang: [
        "ことば",
        "もとの ぶん",
        "これは ほんやくです。せいしきな ものは もとの ぶん（{orig}）です。",
        "この きかいで",
        "この ブラウザが この きかいの なかで した きかい ほんやくです。そとへは なにも おくりません。",
        "もとの ぶんを みる",
        "ほんやく ちゅう…",
        "この ブラウザは {lang} に ほんやく できません。",
        "ほかの ことば",
    ],
    export: [
        "ほぞん・いんさつ",
        "この ページ（インターネット なしで よめる HTML）",
        "いんさつ、または PDF で ほぞん",
        "PDF",
        "EPUB（でんし しょせき よう）",
    ],
    prefs: "よみやすさの せってい",
    credit: "{link} で つくりました",
};

/// The interface in `ja-Latn`.
const JA_LATN: Ui = Ui {
    skip: "Honbun e idō",
    search: "Kensaku",
    search_page: "Kono pēji o kensaku",
    keys: ["idō", "hiraku", "tojiru"],
    none: "Kono pēji ni wa gaitō suru mono ga arimasen.",
    results: "{n} ken",
    result: "1 ken",
    glossary: "Kono pēji no yōgo",
    glossary_intro: "Kaku shō no tensen no tsuita go ni pointā o noseru to imi ga hyōji sare, kurikku suru to koko e idō shimasu. Kaku kōmoku no <strong>modoru</strong> de, yonde ita gyō ni modoremasu.",
    glossary_shared: " Ryakugo wa <a href=\"{url}\">kyōtsū no yōgoshū</a> ni yoru mono desu. <strong>Subete no yōgo</strong> de sono kōmoku o hirakimasu.",
    back: "modoru",
    all_terms: "subete no yōgo",
    keep_outline: "&lsaquo; mokuji o hiraita mama ni suru",
    outline: "Mokuji",
    outline_tips: [
        "Mokuji o kakusu",
        "Mokuji o hyōji",
        " (⌘\\, Windows de wa Control+\\)",
    ],
    notes: "Chū",
    note_n: "Chū {n}",
    link_section: "Kono sekushon e no rinku",
    copy: ["kopī", "kopī shimashita"],
    callouts: ["Memo", "Hinto", "Jūyō", "Keikoku", "Chūi"],
    footer: "<code>{source}</code> kara <code>md2html</code> de seisei shimashita. Markdown fairu ga genpon desu. Henshū shitara sai-seisei shite kudasai.",
    months: [
        "ichigatsu",
        "nigatsu",
        "sangatsu",
        "shigatsu",
        "gogatsu",
        "rokugatsu",
        "shichigatsu",
        "hachigatsu",
        "kugatsu",
        "jūgatsu",
        "jūichigatsu",
        "jūnigatsu",
    ],
    date_fmt: "{y}-nen {m} {d}-nichi",
    top: "Pēji no sentō",
    figure: ["Kakudai shita zu", "Tojiru", "Tojiru (Escape)", "{i} / {n}"],
    size: [
        "Moji no ōkisa",
        "Moji o chiisaku, {n}%",
        "Moji o ōkiku, {n}%",
        "Moji no ōkisa: 100% (moto no ōkisa).",
        "Moji no ōkisa: {n}%. Kurikku de 100% ni modorimasu.",
        " Command (Windows de wa Control) to + / - de henkō shi, 0 de moto ni modorimasu. Burauza no zūmu ni wa eikyō shimasen.",
        "Kore ijō chiisaku dekimasen",
        "Kore ijō ōkiku dekimasen",
    ],
    theme: [
        "Hyōji",
        "Hyōji: {now}. Kurikku de {next}, tsugi ni {after}.",
        "Shisutemu (ima wa {mode})",
        "Shisutemu",
        "Raito",
        "Dāku",
        "dāku",
        "raito",
    ],
    table: "Hyō",
    pages: ["Mae e", "Tsugi e", "Pēji"],
    task: ["Kanryō", "Mikanryō"],
    guide_heading: "Kono pēji no yomikata",
    glossary_heading: "Yōgoshū",
    contents_heading: "Mokuji",
    lang: [
        "Gengo",
        "Genbun",
        "Kore wa hon'yaku desu. Seishiki na han wa genbun ({orig}) desu.",
        "Kono tanmatsu de",
        "Kono burauza ni yoru, kono tanmatsu-jō de no kikai hon'yaku desu. Gaibu ni wa nani mo sōshin saremasen.",
        "Genbun o hyōji",
        "Hon'yaku-chū…",
        "Kono burauza wa {lang} ni hon'yaku dekimasen.",
        "Sono hoka no gengo",
    ],
    export: [
        "Hozon / insatsu",
        "Kono pēji (ofurain de yomeru HTML)",
        "Insatsu, mata wa PDF to shite hozon",
        "PDF",
        "EPUB (denshi shoseki rīdā-yō)",
    ],
    prefs: "Hyōji no settei",
    credit: "{link} de sakusei",
};

/// The interface in `zh-Hans`.
const ZH_HANS: Ui = Ui {
    skip: "跳到正文",
    search: "搜索",
    search_page: "搜索本页",
    keys: ["移动", "打开", "关闭"],
    none: "本页没有匹配的内容。",
    results: "{n} 条结果",
    result: "1 条结果",
    glossary: "本页用语",
    glossary_intro: "各章中带虚线的词，鼠标停留时显示释义，点击则跳到这里。每个条目都有<strong>返回</strong>，可回到你正在读的那一行。",
    glossary_shared: " 缩写来自<a href=\"{url}\">共享术语表</a>；<strong>全部术语</strong>会在该条目处打开它。",
    back: "返回",
    all_terms: "全部术语",
    keep_outline: "&lsaquo; 保持目录打开",
    outline: "目录",
    outline_tips: ["隐藏目录", "显示目录", "（⌘\\，Windows 上为 Control+\\）"],
    notes: "注释",
    note_n: "注释 {n}",
    link_section: "链接到本节",
    copy: ["复制", "已复制"],
    callouts: ["说明", "提示", "重要", "警告", "注意"],
    footer: "由 <code>md2html</code> 从 <code>{source}</code> 生成。Markdown 文件是原件；编辑后请重新生成。",
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: "{y}年{m}{d}日",
    top: "页面顶部",
    figure: ["放大的图", "关闭", "关闭（Escape）", "{i} / {n}"],
    size: [
        "字号",
        "缩小文字，{n}%",
        "放大文字，{n}%",
        "字号：100%，按原样显示。",
        "字号：{n}%。点击恢复为 100%。",
        " 按 Command（Windows 上为 Control）加 + 或 - 在此调整，0 复原；不影响浏览器自身的缩放。",
        "已是最小字号",
        "已是最大字号",
    ],
    theme: [
        "外观",
        "外观：{now}。点击切换为{next}，然后{after}。",
        "跟随系统，当前为{mode}",
        "跟随系统",
        "浅色",
        "深色",
        "深色",
        "浅色",
    ],
    table: "表格",
    pages: ["上一页", "下一页", "页面"],
    task: ["已完成", "未完成"],
    guide_heading: "如何阅读本页",
    glossary_heading: "术语表",
    contents_heading: "目录",
    lang: [
        "语言",
        "原文",
        "这是译文。以原文（{orig}）为准。",
        "在本设备上",
        "由本浏览器在本设备上完成的机器翻译：任何内容都不会离开本设备。",
        "显示原文",
        "正在翻译…",
        "此浏览器无法翻译成{lang}。",
        "更多语言",
    ],
    export: [
        "保存或打印",
        "本页（可离线阅读的 HTML）",
        "打印，或另存为 PDF",
        "PDF",
        "EPUB（适用于电子阅读器）",
    ],
    prefs: "阅读设置",
    credit: "使用 {link} 制作",
};

/// The interface in `zh-Hant`.
const ZH_HANT: Ui = Ui {
    skip: "跳至正文",
    search: "搜尋",
    search_page: "搜尋本頁",
    keys: ["移動", "開啟", "關閉"],
    none: "本頁沒有符合的內容。",
    results: "{n} 筆結果",
    result: "1 筆結果",
    glossary: "本頁用語",
    glossary_intro: "各章中帶虛線的詞，滑鼠停留時顯示釋義，點擊則跳到這裡。每個條目都有<strong>返回</strong>，可回到你正在讀的那一行。",
    glossary_shared: " 縮寫來自<a href=\"{url}\">共用術語表</a>；<strong>全部術語</strong>會在該條目處開啟它。",
    back: "返回",
    all_terms: "全部術語",
    keep_outline: "&lsaquo; 保持目錄開啟",
    outline: "目錄",
    outline_tips: ["隱藏目錄", "顯示目錄", "（⌘\\，Windows 上為 Control+\\）"],
    notes: "註釋",
    note_n: "註釋 {n}",
    link_section: "連結到本節",
    copy: ["複製", "已複製"],
    callouts: ["說明", "提示", "重要", "警告", "注意"],
    footer: "由 <code>md2html</code> 從 <code>{source}</code> 產生。Markdown 檔案是原件；編輯後請重新產生。",
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: "{y}年{m}{d}日",
    top: "頁面頂端",
    figure: ["放大的圖", "關閉", "關閉（Escape）", "{i} / {n}"],
    size: [
        "字級",
        "縮小文字，{n}%",
        "放大文字，{n}%",
        "字級：100%，依原樣顯示。",
        "字級：{n}%。點擊恢復為 100%。",
        " 按 Command（Windows 上為 Control）加 + 或 - 在此調整，0 復原；不影響瀏覽器本身的縮放。",
        "已是最小字級",
        "已是最大字級",
    ],
    theme: [
        "外觀",
        "外觀：{now}。點擊切換為{next}，然後{after}。",
        "跟隨系統，目前為{mode}",
        "跟隨系統",
        "淺色",
        "深色",
        "深色",
        "淺色",
    ],
    table: "表格",
    pages: ["上一頁", "下一頁", "頁面"],
    task: ["已完成", "未完成"],
    guide_heading: "如何閱讀本頁",
    glossary_heading: "術語表",
    contents_heading: "目錄",
    lang: [
        "語言",
        "原文",
        "這是譯文。以原文（{orig}）為準。",
        "在本裝置上",
        "由本瀏覽器在本裝置上完成的機器翻譯：任何內容都不會離開本裝置。",
        "顯示原文",
        "正在翻譯…",
        "此瀏覽器無法翻譯成{lang}。",
        "更多語言",
    ],
    export: [
        "儲存或列印",
        "本頁（可離線閱讀的 HTML）",
        "列印，或另存為 PDF",
        "PDF",
        "EPUB（適用於電子閱讀器）",
    ],
    prefs: "閱讀設定",
    credit: "使用 {link} 製作",
};

/// The interface in `yue`.
const YUE: Ui = Ui {
    skip: "跳去正文",
    search: "搵",
    search_page: "喺呢頁搵",
    keys: ["移動", "打開", "閂"],
    none: "呢頁冇相符嘅內容。",
    results: "{n} 個結果",
    result: "1 個結果",
    glossary: "呢頁用到嘅詞",
    glossary_intro: "每章入面有虛線嘅詞，滑鼠停喺上面就會顯示意思，撳一下就會跳嚟呢度。每個條目都有<strong>返回</strong>，可以返去你啱啱睇緊嗰行。",
    glossary_shared: " 縮寫嚟自<a href=\"{url}\">共用詞彙表</a>；撳<strong>全部詞彙</strong>會喺嗰個條目度打開佢。",
    back: "返回",
    all_terms: "全部詞彙",
    keep_outline: "&lsaquo; 保持目錄打開",
    outline: "目錄",
    outline_tips: ["收埋目錄", "顯示目錄", "（⌘\\，Windows 用 Control+\\）"],
    notes: "註釋",
    note_n: "註釋 {n}",
    link_section: "連結去呢節",
    copy: ["複製", "複製咗"],
    callouts: ["說明", "貼士", "重要", "警告", "注意"],
    footer: "由 <code>md2html</code> 用 <code>{source}</code> 整出嚟。Markdown 檔案先係正本；改完要再整過。",
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: "{y}年{m}{d}日",
    top: "頁頂",
    figure: ["放大咗嘅圖", "閂", "閂（Escape）", "{i} / {n}"],
    size: [
        "字體大小",
        "細啲，{n}%",
        "大啲，{n}%",
        "字體大小：100%，即係原本咁。",
        "字體大小：{n}%。撳一下返去 100%。",
        " 用 Command（Windows 用 Control）加 + 或 - 喺度調，0 還原；唔會郁到瀏覽器本身嘅縮放。",
        "已經係最細",
        "已經係最大",
    ],
    theme: [
        "外觀",
        "外觀：{now}。撳一下轉做{next}，再轉做{after}。",
        "跟系統，而家係{mode}",
        "跟系統",
        "淺色",
        "深色",
        "深色",
        "淺色",
    ],
    table: "表格",
    pages: ["上一頁", "下一頁", "頁面"],
    task: ["搞掂", "未搞掂"],
    guide_heading: "點樣睇呢一頁",
    glossary_heading: "詞彙表",
    contents_heading: "目錄",
    lang: [
        "語言",
        "原文",
        "呢個係譯本。以原文（{orig}）為準。",
        "喺呢部機度",
        "由呢個瀏覽器喺呢部機入面做嘅機器翻譯：乜都唔會傳出去。",
        "睇原文",
        "翻譯緊…",
        "呢個瀏覽器唔識譯做{lang}。",
        "更多語言",
    ],
    export: [
        "儲存或者列印",
        "呢一頁（離線都睇得嘅 HTML）",
        "列印，或者存做 PDF",
        "PDF",
        "EPUB（電子書閱讀器用）",
    ],
    prefs: "閱讀設定",
    credit: "用 {link} 整嘅",
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
        "he" | "iw" => &HE,
        "id" => &ID,
        "nl" => &NL,
        "jv" => &JV,
        "yi" | "ji" => &YI,
        "nds" => &NDS,
        "fr" => &FR,
        "it" => &IT,
        "es" => &ES,
        "ja" => match lang.to_ascii_lowercase().as_str() {
            "ja-hira" | "ja-kana" => &JA_HIRA,
            "ja-latn" => &JA_LATN,
            _ => &JA,
        },
        "zh" => match lang.to_ascii_lowercase().as_str() {
            "zh-hant" | "zh-tw" | "zh-hk" | "zh-mo" | "zh-hant-tw" | "zh-hant-hk" => &ZH_HANT,
            _ => &ZH_HANS,
        },
        "yue" => &YUE,
        _ => &EN,
    }
}

/// Every interface, for recognising an author's headings and callout words
/// in any of these languages.
pub(crate) fn all_ui() -> [&'static Ui; 17] {
    [
        &EN, &DE, &HE, &ID, &NL, &JV, &YI, &NDS, &FR, &IT, &ES, &JA, &JA_HIRA, &JA_LATN, &ZH_HANS,
        &ZH_HANT, &YUE,
    ]
}

/// How many of [`LANGS`] the language switch offers first, before "more".
pub(crate) const FAMILY: usize = 17;

/// A language md2html knows: its code, its own name for itself, and
/// whether it is written right to left. The first eleven have interface
/// words of their own, in the order a page's language switch offers them;
/// the rest are offered for translation on the reader's device.
pub(crate) const LANGS: &[(&str, &str, bool)] = &[
    ("en", "English", false),
    ("de", "Deutsch", false),
    ("he", "עברית", true),
    ("id", "Bahasa Indonesia", false),
    ("nl", "Nederlands", false),
    ("jv", "Basa Jawa", false),
    ("yi", "ייִדיש", true),
    ("nds", "Plattdüütsch", false),
    ("fr", "Français", false),
    ("it", "Italiano", false),
    ("es", "Español", false),
    ("ja", "日本語", false),
    ("ja-Hira", "にほんご（ひらがな）", false),
    ("ja-Latn", "Nihongo (rōmaji)", false),
    ("zh-Hans", "中文（简体）", false),
    ("zh-Hant", "中文（繁體）", false),
    ("yue", "粵語", false),
    ("pt", "Português", false),
    ("pl", "Polski", false),
    ("tr", "Türkçe", false),
    ("ru", "Русский", false),
    ("uk", "Українська", false),
    ("ar", "العربية", true),
    ("ko", "한국어", false),
    ("hi", "हिन्दी", false),
    ("sv", "Svenska", false),
    ("da", "Dansk", false),
    ("nb", "Norsk", false),
    ("fi", "Suomi", false),
    ("cs", "Čeština", false),
    ("el", "Ελληνικά", false),
    ("ro", "Română", false),
    ("hu", "Magyar", false),
    ("vi", "Tiếng Việt", false),
    ("th", "ไทย", false),
];

/// A language's entry, by code (`de`, `de-CH`).
pub(crate) fn lang_info(code: &str) -> Option<(&'static str, &'static str, bool)> {
    let lower = code.to_ascii_lowercase().replace('_', "-");
    let primary = lower.split('-').next().unwrap_or_default();
    let traditional = ["zh-tw", "zh-hk", "zh-mo"]
        .iter()
        .any(|r| lower.starts_with(r));
    LANGS
        .iter()
        .copied()
        .find(|(c, _, _)| c.eq_ignore_ascii_case(&lower))
        .or_else(|| {
            traditional
                .then(|| LANGS.iter().copied().find(|(c, _, _)| *c == "zh-Hant"))
                .flatten()
        })
        .or_else(|| {
            LANGS
                .iter()
                .copied()
                .find(|(c, _, _)| c.split('-').next() == Some(primary))
        })
}

/// Whether a language is written right to left.
pub(crate) fn rtl(code: &str) -> bool {
    lang_info(code).is_some_and(|(_, _, r)| r)
}

impl Ui {
    /// A date in this language: "21 November 2026", "21. November 2026".
    pub(crate) fn date(&self, (year, month, day): (i64, u32, u32)) -> String {
        let name = self
            .months
            .get(usize::try_from(month).unwrap_or(1).saturating_sub(1))
            .copied()
            .unwrap_or_default();
        self.date_fmt
            .replace("{d}", &day.to_string())
            .replace("{m}", name)
            .replace("{y}", &year.to_string())
    }

    /// The words the page's scripts need, as JSON.
    pub(crate) fn json(&self) -> String {
        let q = crate::page::json_str;
        format!(
            "{{\"lang\":[{},{},{},{},{},{},{},{},{}],\"table\":{},\"none\":{},\"results\":{},\"result\":{},\"outline\":[{},{},{}],\"copy\":[{},{}],\
\"figure\":[{},{},{},{}],\"size\":[{},{},{},{},{},{},{},{}],\"theme\":[{},{},{},{},{},{},{},{}]}}",
            q(self.lang[0]),
            q(self.lang[1]),
            q(self.lang[2]),
            q(self.lang[3]),
            q(self.lang[4]),
            q(self.lang[5]),
            q(self.lang[6]),
            q(self.lang[7]),
            q(self.lang[8]),
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
