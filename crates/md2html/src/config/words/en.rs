// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `en`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `en`.
pub(super) const WORDS: Ui = Ui {
    skip: "Skip to content",
    search: "Search",
    search_page: "Search this page",
    keys: SearchKeys {
        step: "move",
        open: "open",
        close: "close",
    },
    none: "Nothing on this page matches that.",
    results: Template {
        lead: "",
        tails: [" results"],
    },
    result: "1 result",
    glossary: "Words used here",
    glossary_intro: "Each dotted word in the chapters shows its meaning when you rest on it and jumps here when you click it. Every entry has a <strong>back</strong> that returns you to the line you were reading.",
    glossary_shared: Template {
        lead: " The acronyms come from the <a href=\"",
        tails: ["\">shared glossary</a>; <strong>all terms</strong> opens it at the entry."],
    },
    back: "back",
    all_terms: "all terms",
    keep_outline: "&lsaquo; keep the outline open",
    outline: "Outline",
    outline_tips: OutlineTips {
        hide: "Hide the outline",
        show: "Show the outline",
        keys: " (⌘\\, Control+\\ on Windows)",
    },
    notes: "Notes",
    note_n: Template {
        lead: "Note ",
        tails: [""],
    },
    link_section: "Link to this section",
    copy: CopyButton {
        label: "copy",
        copied: "copied",
    },
    callouts: Callouts {
        note: "Note",
        tip: "Tip",
        important: "Important",
        warning: "Warning",
        caution: "Caution",
    },
    footer: Template {
        lead: "Rendered from <code>",
        tails: [
            "</code> by <code>md2html</code>. The Markdown file is the source; render again after editing it.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Top of the page",
    figure: EnlargedFigure {
        label: "The figure, enlarged",
        close: "Close",
        close_key: "Close (Escape)",
        position: Template {
            lead: "",
            tails: [" of ", ""],
        },
    },
    size: TextSize {
        group: "Text size",
        smaller: SizeStep {
            words: "Smaller text",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Larger text",
            joint: ", ",
            unit: "%",
        },
        as_written: "Text size: 100%, the page as written.",
        scaled: Template {
            lead: "Text size: ",
            tails: ["%. Click to go back to 100%."],
        },
        keys: " Command (Control on Windows) with + or - steps it here, 0 resets; the browser's own zoom is left alone.",
        smallest: "Smallest text already",
        largest: "Largest text already",
    },
    theme: ThemeSwitch {
        button: "Theme",
        tip: Template {
            lead: "Theme: ",
            tails: [". Click for ", ", then ", "."],
        },
        system_now: Template {
            lead: "System, ",
            tails: [" right now"],
        },
        system: "System",
        light: "Light",
        dark: "Dark",
        dark_mode: "dark",
        light_mode: "light",
    },
    table: "Table",
    pages: PageLinks {
        previous: "Previous",
        next: "Next",
        label: "Pages",
    },
    task: TaskBoxes {
        done: "Done",
        open: "Open",
    },
    guide_heading: "How to read this",
    glossary_heading: "Glossary",
    contents_heading: "Contents",
    lang: LanguageSwitch {
        label: "Language",
        original: "Original",
        notice: Template {
            lead: "This is a translation. The original (",
            tails: [") is the authoritative version."],
        },
        on_device: "On this device",
        machine: "A machine translation by this browser, on this device: nothing leaves it.",
        show_original: "Show the original",
        translating: "Translating…",
        cannot: Template {
            lead: "This browser cannot translate into ",
            tails: ["."],
        },
        more: "More languages",
    },
    export: ExportMenu {
        label: "Save or print",
        html: "This page, to read offline (HTML)",
        print: "Print, or save as PDF",
        pdf: "PDF",
        epub: "EPUB, for e-readers",
    },
    prefs: "Reading settings",
    credit: Template {
        lead: "Made with ",
        tails: [""],
    },
    share: Sharing {
        copied: "Link copied",
        passage: "Link to this passage",
        row: "Link to this row",
        thing: "Link to this",
    },
    live: "Updated to the latest version",
};
