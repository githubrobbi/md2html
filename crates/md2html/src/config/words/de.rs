// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `de`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `de`.
pub(super) const WORDS: Ui = Ui {
    skip: "Zum Inhalt springen",
    search: "Suchen",
    search_page: "Diese Seite durchsuchen",
    keys: SearchKeys {
        step: "wählen",
        open: "öffnen",
        close: "schließen",
    },
    none: "Nichts auf dieser Seite passt dazu.",
    results: Template {
        lead: "",
        tails: [" Treffer"],
    },
    result: "1 Treffer",
    glossary: "Begriffe",
    glossary_intro: "Jedes gepunktet unterstrichene Wort in den Kapiteln zeigt seine Bedeutung, wenn der Zeiger darauf ruht, und springt hierher, wenn man es anklickt. Jeder Eintrag hat ein <strong>zurück</strong>, das zu der Zeile führt, an der man gerade gelesen hat.",
    glossary_shared: Template {
        lead: " Die Kürzel stammen aus dem <a href=\"",
        tails: [
            "\">gemeinsamen Glossar</a>; <strong>alle Begriffe</strong> öffnet es beim Eintrag.",
        ],
    },
    back: "zurück",
    all_terms: "alle Begriffe",
    keep_outline: "&lsaquo; Gliederung offen halten",
    outline: "Gliederung",
    outline_tips: OutlineTips {
        hide: "Gliederung ausblenden",
        show: "Gliederung einblenden",
        keys: " (⌘\\, Strg+\\ unter Windows)",
    },
    notes: "Anmerkungen",
    note_n: Template {
        lead: "Anmerkung ",
        tails: [""],
    },
    link_section: "Link zu diesem Abschnitt",
    copy: CopyButton {
        label: "kopieren",
        copied: "kopiert",
    },
    callouts: Callouts {
        note: "Hinweis",
        tip: "Tipp",
        important: "Wichtig",
        warning: "Achtung",
        caution: "Vorsicht",
    },
    footer: Template {
        lead: "Erstellt aus <code>",
        tails: [
            "</code> mit <code>md2html</code>. Die Markdown-Datei ist die Quelle; nach dem Bearbeiten neu erzeugen.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [". ", " ", ""],
    }),
    top: "Seitenanfang",
    figure: EnlargedFigure {
        label: "Die Abbildung, vergrößert",
        close: "Schließen",
        close_key: "Schließen (Escape)",
        position: Template {
            lead: "",
            tails: [" von ", ""],
        },
    },
    size: TextSize {
        group: "Textgröße",
        smaller: SizeStep {
            words: "Kleinere Schrift",
            joint: ", ",
            unit: " %",
        },
        larger: SizeStep {
            words: "Größere Schrift",
            joint: ", ",
            unit: " %",
        },
        as_written: "Textgröße: 100 %, die Seite wie geschrieben.",
        scaled: Template {
            lead: "Textgröße: ",
            tails: [" %. Klick für 100 %."],
        },
        keys: " Befehlstaste (Strg unter Windows) mit + oder - ändert sie hier, 0 setzt zurück; der Zoom des Browsers bleibt unberührt.",
        smallest: "Schon die kleinste Schrift",
        largest: "Schon die größte Schrift",
    },
    theme: ThemeSwitch {
        button: "Darstellung",
        tip: Template {
            lead: "Darstellung: ",
            tails: [". Klick für ", ", dann ", "."],
        },
        system_now: Template {
            lead: "System, gerade ",
            tails: [""],
        },
        system: "System",
        light: "Hell",
        dark: "Dunkel",
        dark_mode: "dunkel",
        light_mode: "hell",
    },
    table: "Tabelle",
    pages: PageLinks {
        previous: "Vorherige Seite",
        next: "Nächste Seite",
        label: "Seiten",
    },
    task: TaskBoxes {
        done: "Erledigt",
        open: "Offen",
    },
    guide_heading: "Wie diese Seite zu lesen ist",
    glossary_heading: "Glossar",
    contents_heading: "Inhalt",
    lang: LanguageSwitch {
        label: "Sprache",
        original: "Original",
        notice: Template {
            lead: "Dies ist eine Übersetzung. Maßgeblich ist das Original (",
            tails: [")."],
        },
        on_device: "Auf diesem Gerät",
        machine: "Eine maschinelle Übersetzung durch diesen Browser, auf diesem Gerät: nichts verlässt es.",
        show_original: "Original zeigen",
        translating: "Wird übersetzt…",
        cannot: Template {
            lead: "Dieser Browser kann nicht ins ",
            tails: [" übersetzen."],
        },
        more: "Weitere Sprachen",
    },
    export: ExportMenu {
        label: "Speichern oder drucken",
        html: "Diese Seite, offline zu lesen (HTML)",
        print: "Drucken oder als PDF sichern",
        pdf: "PDF",
        epub: "EPUB, für E-Reader",
    },
    prefs: "Leseeinstellungen",
    credit: Template {
        lead: "Erstellt mit ",
        tails: [""],
    },
    share: Sharing {
        copied: "Link kopiert",
        passage: "Link zu dieser Stelle",
        row: "Link zu dieser Zeile",
        thing: "Link hierhin",
    },
    live: "Auf den neuesten Stand gebracht",
};
