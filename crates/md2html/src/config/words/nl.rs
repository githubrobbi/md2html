// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `nl`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `nl`.
pub(super) static WORDS: Ui = Ui {
    skip: "Naar de inhoud",
    search: "Zoeken",
    search_page: "Deze pagina doorzoeken",
    keys: SearchKeys {
        step: "kiezen",
        open: "openen",
        close: "sluiten",
    },
    none: "Niets op deze pagina komt overeen.",
    results: Template {
        lead: "",
        tails: [" resultaten"],
    },
    result: "1 resultaat",
    glossary: "Woorden op deze pagina",
    glossary_intro: "Elk gestippeld woord in de hoofdstukken toont zijn betekenis als u erop rust en springt hierheen als u erop klikt. Elk item heeft een <strong>terug</strong> naar de regel die u aan het lezen was.",
    glossary_shared: Template {
        lead: " De afkortingen komen uit de <a href=\"",
        tails: [
            "\">gedeelde woordenlijst</a>; <strong>alle termen</strong> opent die bij het item.",
        ],
    },
    back: "terug",
    all_terms: "alle termen",
    keep_outline: "&lsaquo; overzicht open houden",
    outline: "Overzicht",
    outline_tips: OutlineTips {
        hide: "Overzicht verbergen",
        show: "Overzicht tonen",
        keys: " (⌘\\, Ctrl+\\ op Windows)",
    },
    notes: "Noten",
    note_n: Template {
        lead: "Noot ",
        tails: [""],
    },
    link_section: "Link naar dit gedeelte",
    copy: CopyButton {
        label: "kopiëren",
        copied: "gekopieerd",
    },
    callouts: Callouts {
        note: "Opmerking",
        tip: "Tip",
        important: "Belangrijk",
        warning: "Waarschuwing",
        caution: "Let op",
    },
    footer: Template {
        lead: "Gemaakt van <code>",
        tails: [
            "</code> met <code>md2html</code>. Het Markdown-bestand is de bron; maak de pagina opnieuw na het bewerken.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Begin van de pagina",
    figure: EnlargedFigure {
        label: "De afbeelding, vergroot",
        close: "Sluiten",
        close_key: "Sluiten (Escape)",
        position: Template {
            lead: "",
            tails: [" van ", ""],
        },
    },
    size: TextSize {
        group: "Tekstgrootte",
        smaller: SizeStep {
            words: "Kleinere tekst",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Grotere tekst",
            joint: ", ",
            unit: "%",
        },
        as_written: "Tekstgrootte: 100%, de pagina zoals geschreven.",
        scaled: Template {
            lead: "Tekstgrootte: ",
            tails: ["%. Klik om terug te gaan naar 100%."],
        },
        keys: " Command (Ctrl op Windows) met + of - verandert hem hier, 0 zet terug; de zoom van de browser blijft ongemoeid.",
        smallest: "Al de kleinste tekst",
        largest: "Al de grootste tekst",
    },
    theme: ThemeSwitch {
        button: "Thema",
        tip: Template {
            lead: "Thema: ",
            tails: [". Klik voor ", ", daarna ", "."],
        },
        system_now: Template {
            lead: "Systeem, nu ",
            tails: [""],
        },
        system: "Systeem",
        light: "Licht",
        dark: "Donker",
        dark_mode: "donker",
        light_mode: "licht",
    },
    table: "Tabel",
    pages: PageLinks {
        previous: "Vorige",
        next: "Volgende",
        label: "Pagina's",
    },
    task: TaskBoxes {
        done: "Klaar",
        open: "Open",
    },
    guide_heading: "Hoe u dit leest",
    glossary_heading: "Woordenlijst",
    contents_heading: "Inhoud",
    lang: LanguageSwitch {
        label: "Taal",
        original: "Origineel",
        notice: Template {
            lead: "Dit is een vertaling. Het origineel (",
            tails: [") is de geldende versie."],
        },
        on_device: "Op dit apparaat",
        machine: "Een machinevertaling door deze browser, op dit apparaat: er verlaat niets het apparaat.",
        show_original: "Origineel tonen",
        translating: "Bezig met vertalen…",
        cannot: Template {
            lead: "Deze browser kan niet naar het ",
            tails: [" vertalen."],
        },
        more: "Meer talen",
    },
    export: ExportMenu {
        label: "Bewaren of afdrukken",
        html: "Deze pagina, om offline te lezen (HTML)",
        print: "Afdrukken of als PDF bewaren",
        pdf: "PDF",
        epub: "EPUB, voor e-readers",
    },
    prefs: "Leesinstellingen",
    credit: Template {
        lead: "Gemaakt met ",
        tails: [""],
    },
    share: Sharing {
        copied: "Link gekopieerd",
        passage: "Link naar deze passage",
        row: "Link naar deze rij",
        thing: "Link hierheen",
    },
    live: "Bijgewerkt naar de nieuwste versie",
};
