// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `it`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `it`.
pub(super) static WORDS: Ui = Ui {
    skip: "Vai al contenuto",
    search: "Cerca",
    search_page: "Cerca in questa pagina",
    keys: SearchKeys {
        step: "naviga",
        open: "apri",
        close: "chiudi",
    },
    none: "Nessuna corrispondenza in questa pagina.",
    results: Template {
        lead: "",
        tails: [" risultati"],
    },
    result: "1 risultato",
    glossary: "Le parole di questa pagina",
    glossary_intro: "Ogni parola sottolineata a puntini nei capitoli mostra il suo significato al passaggio del puntatore e porta qui con un clic. Ogni voce ha un <strong>indietro</strong> che riporta alla riga che si stava leggendo.",
    glossary_shared: Template {
        lead: " Le sigle vengono dal <a href=\"",
        tails: ["\">glossario comune</a>; <strong>tutti i termini</strong> lo apre alla voce."],
    },
    back: "indietro",
    all_terms: "tutti i termini",
    keep_outline: "&lsaquo; tieni aperto l'indice",
    outline: "Indice",
    outline_tips: OutlineTips {
        hide: "Nascondi l'indice",
        show: "Mostra l'indice",
        keys: " (⌘\\, Ctrl+\\ su Windows)",
    },
    notes: "Note",
    note_n: Template {
        lead: "Nota ",
        tails: [""],
    },
    link_section: "Link a questa sezione",
    copy: CopyButton {
        label: "copia",
        copied: "copiato",
    },
    callouts: Callouts {
        note: "Nota",
        tip: "Suggerimento",
        important: "Importante",
        warning: "Avviso",
        caution: "Attenzione",
    },
    footer: Template {
        lead: "Generato da <code>",
        tails: [
            "</code> con <code>md2html</code>. Il file Markdown è la fonte; rigenerare dopo averlo modificato.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Inizio pagina",
    figure: EnlargedFigure {
        label: "Figura ingrandita",
        close: "Chiudi",
        close_key: "Chiudi (Esc)",
        position: Template {
            lead: "",
            tails: [" di ", ""],
        },
        enlarge: "Ingrandisci il diagramma",
    },
    size: TextSize {
        group: "Dimensione del testo",
        smaller: SizeStep {
            words: "Testo più piccolo",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Testo più grande",
            joint: ", ",
            unit: "%",
        },
        as_written: "Dimensione del testo: 100%, la pagina così com'è scritta.",
        scaled: Template {
            lead: "Dimensione del testo: ",
            tails: ["%. Clic per tornare al 100%."],
        },
        keys: " Comando (Ctrl su Windows) con + o - la cambia qui, 0 la ripristina; lo zoom del browser resta invariato.",
        smallest: "Testo già al minimo",
        largest: "Testo già al massimo",
    },
    theme: ThemeSwitch {
        button: "Tema",
        tip: Template {
            lead: "Tema: ",
            tails: [". Clic per passare a ", ", poi a ", "."],
        },
        system_now: Template {
            lead: "Sistema, ora ",
            tails: [""],
        },
        system: "Sistema",
        light: "Chiaro",
        dark: "Scuro",
        dark_mode: "scuro",
        light_mode: "chiaro",
    },
    table: "Tabella",
    pages: PageLinks {
        previous: "Precedente",
        next: "Successiva",
        label: "Pagine",
    },
    task: TaskBoxes {
        done: "Fatto",
        open: "Da fare",
    },
    guide_heading: "Come leggere questa pagina",
    glossary_heading: "Glossario",
    meaning_heads: &["Significato", "Definizione", "Descrizione"],
    contents_heading: "Indice",
    lang: LanguageSwitch {
        label: "Lingua",
        original: "Originale",
        notice: Template {
            lead: "Questa è una traduzione. Fa fede l'originale (",
            tails: [")."],
        },
        on_device: "Su questo dispositivo",
        machine: "Una traduzione automatica di questo browser, su questo dispositivo: nulla ne esce.",
        show_original: "Mostra l'originale",
        translating: "Traduzione in corso…",
        cannot: Template {
            lead: "Questo browser non può tradurre in ",
            tails: ["."],
        },
        more: "Altre lingue",
    },
    export: ExportMenu {
        label: "Salva o stampa",
        html: "Questa pagina, da leggere offline (HTML)",
        print: "Stampa, o salva come PDF",
        pdf: "PDF",
        epub: "EPUB, per e-reader",
    },
    prefs: "Impostazioni di lettura",
    credit: Template {
        lead: "Realizzato con ",
        tails: [""],
    },
    share: Sharing {
        copied: "Link copiato",
        passage: "Link a questo passaggio",
        row: "Link a questa riga",
        thing: "Link a questo elemento",
    },
    live: "Aggiornato all'ultima versione",
};
