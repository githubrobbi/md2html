// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `nds`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `nds`.
pub(super) static WORDS: Ui = Ui {
    skip: "Na'n Inholt springen",
    search: "Söken",
    search_page: "Disse Siet dörsöken",
    keys: SearchKeys {
        step: "wählen",
        open: "opmaken",
        close: "dichtmaken",
    },
    none: "Nix op disse Siet passt dorto.",
    results: Template {
        lead: "",
        tails: [" Funnen"],
    },
    result: "1 Fund",
    glossary: "Wöör op disse Siet",
    glossary_intro: "Elk Woort mit Pünkten ünner in de Kapitels wiest sien Bedüden, wenn de Wieser dor op liggt, un springt hierher, wenn een dor op klickt. Elk Indrag hett en <strong>trüch</strong> na de Reeg, de een jüst leest hett.",
    glossary_shared: Template {
        lead: " De Afkörten kaamt ut de <a href=\"",
        tails: ["\">gemeensame Woortlist</a>; <strong>all Wöör</strong> maakt de bi'n Indrag op."],
    },
    back: "trüch",
    all_terms: "all Wöör",
    keep_outline: "&lsaquo; Översicht open laten",
    outline: "Översicht",
    outline_tips: OutlineTips {
        hide: "Översicht wegdoon",
        show: "Översicht wiesen",
        keys: " (⌘\\, Strg+\\ ünner Windows)",
    },
    notes: "Anmarken",
    note_n: Template {
        lead: "Anmarken ",
        tails: [""],
    },
    link_section: "Link na dissen Afsnitt",
    copy: CopyButton {
        label: "koperen",
        copied: "kopeert",
    },
    callouts: Callouts {
        note: "Henwies",
        tip: "Tipp",
        important: "Wichtig",
        warning: "Acht geven",
        caution: "Vörsicht",
    },
    footer: Template {
        lead: "Maakt ut <code>",
        tails: [
            "</code> mit <code>md2html</code>. De Markdown-Datei is de Born; na't Ännern nee maken.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [". ", " ", ""],
    }),
    top: "Baven op de Siet",
    figure: EnlargedFigure {
        label: "Dat Bild, grötter maakt",
        close: "Dichtmaken",
        close_key: "Dichtmaken (Escape)",
        position: Template {
            lead: "",
            tails: [" vun ", ""],
        },
        enlarge: "Diagramm grötter maken",
    },
    size: TextSize {
        group: "Schriftgrött",
        smaller: SizeStep {
            words: "Lüttere Schrift",
            joint: ", ",
            unit: " %",
        },
        larger: SizeStep {
            words: "Gröttere Schrift",
            joint: ", ",
            unit: " %",
        },
        as_written: "Schriftgrött: 100 %, de Siet as schreven.",
        scaled: Template {
            lead: "Schriftgrött: ",
            tails: [" %. Klick för 100 %."],
        },
        keys: " Befehlstast (Strg ünner Windows) mit + oder - ännert ehr hier, 0 sett trüch; de Zoom vun'n Browser blifft as he is.",
        smallest: "Al de lüttste Schrift",
        largest: "Al de gröttste Schrift",
    },
    theme: ThemeSwitch {
        button: "Utsehn",
        tip: Template {
            lead: "Utsehn: ",
            tails: [". Klick för ", ", denn ", "."],
        },
        system_now: Template {
            lead: "System, jüst ",
            tails: [""],
        },
        system: "System",
        light: "Hell",
        dark: "Düster",
        dark_mode: "düster",
        light_mode: "hell",
    },
    table: "Tabell",
    pages: PageLinks {
        previous: "Vörige Siet",
        next: "Neegste Siet",
        label: "Sieden",
    },
    task: TaskBoxes {
        done: "Daan",
        open: "Open",
    },
    guide_heading: "So leest een dat",
    glossary_heading: "Woortlist",
    meaning_heads: &["Bedüden", "Bedüüdnis"],
    contents_heading: "Inholt",
    lang: LanguageSwitch {
        label: "Spraak",
        original: "Original",
        notice: Template {
            lead: "Dit is en Översetten. Dat gellt, wat in't Original (",
            tails: [") steiht."],
        },
        on_device: "Op dit Reedschap",
        machine: "En Maschien-Översetten vun dissen Browser, op dit Reedschap: nix geiht dor rut.",
        show_original: "Original wiesen",
        translating: "Warrt översett…",
        cannot: Template {
            lead: "Disse Browser kann nich op ",
            tails: [" översetten."],
        },
        more: "Mehr Spraken",
    },
    export: ExportMenu {
        label: "Spiekern oder drucken",
        html: "Disse Siet, ahn Internet to lesen (HTML)",
        print: "Drucken oder as PDF spiekern",
        pdf: "PDF",
        epub: "EPUB, för E-Reader",
    },
    prefs: "Leesinstellen",
    credit: Template {
        lead: "Maakt mit ",
        tails: [""],
    },
    share: Sharing {
        copied: "Link kopeert",
        passage: "Link to disse Stee",
        row: "Link to disse Reeg",
        thing: "Link hierhen",
    },
    live: "Op'n neesten Stand brocht",
};
