// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `jv`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `jv`.
pub(super) static WORDS: Ui = Ui {
    skip: "Langsung menyang isi",
    search: "Golèk",
    search_page: "Golèk ing kaca iki",
    keys: SearchKeys {
        step: "pindhah",
        open: "bukak",
        close: "tutup",
    },
    none: "Ora ana sing cocog ing kaca iki.",
    results: Template {
        lead: "",
        tails: [" asil"],
    },
    result: "1 asil",
    glossary: "Tembung ing kaca iki",
    glossary_intro: "Saben tembung kanthi garis titik ing bab nuduhaké tegesé yèn penunjuk ana ing ndhuwuré, lan mlumpat mréné yèn diklik. Saben entri duwé tombol <strong>bali</strong> menyang larik sing lagi diwaca.",
    glossary_shared: Template {
        lead: " Cekakan dijupuk saka <a href=\"",
        tails: [
            "\">glosarium sing dienggo bebarengan</a>; <strong>kabèh istilah</strong> mbukak glosarium kuwi ing entri kasebut.",
        ],
    },
    back: "bali",
    all_terms: "kabèh istilah",
    keep_outline: "&lsaquo; kerangka tetep dibukak",
    outline: "Kerangka",
    outline_tips: OutlineTips {
        hide: "Dhelikaké kerangka",
        show: "Tuduhaké kerangka",
        keys: " (⌘\\, Control+\\ ing Windows)",
    },
    notes: "Cathetan",
    note_n: Template {
        lead: "Cathetan ",
        tails: [""],
    },
    link_section: "Pranala menyang bagéan iki",
    copy: CopyButton {
        label: "salin",
        copied: "wis disalin",
    },
    callouts: Callouts {
        note: "Cathetan",
        tip: "Pitutur",
        important: "Wigati",
        warning: "Pènget",
        caution: "Ati-ati",
    },
    footer: Template {
        lead: "Digawé saka <code>",
        tails: [
            "</code> nganggo <code>md2html</code>. Berkas Markdown iku sumberé; gawé manèh sawisé diowahi.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Ndhuwur kaca",
    figure: EnlargedFigure {
        label: "Gambar sing digedhèkaké",
        close: "Tutup",
        close_key: "Tutup (Escape)",
        position: Template {
            lead: "",
            tails: [" saka ", ""],
        },
        enlarge: "Gedhèkaké diagram",
    },
    size: TextSize {
        group: "Ukuran teks",
        smaller: SizeStep {
            words: "Teks luwih cilik",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Teks luwih gedhé",
            joint: ", ",
            unit: "%",
        },
        as_written: "Ukuran teks: 100%, kaca kaya sing ditulis.",
        scaled: Template {
            lead: "Ukuran teks: ",
            tails: ["%. Klik kanggo bali menyang 100%."],
        },
        keys: " Command (Control ing Windows) karo + utawa - ngowahi ukurané ing kéné, 0 mbalèkaké kaya wiwitan; zoom peramban dhéwé ora diowahi.",
        smallest: "Teks wis paling cilik",
        largest: "Teks wis paling gedhé",
    },
    theme: ThemeSwitch {
        button: "Téma",
        tip: Template {
            lead: "Téma: ",
            tails: [". Klik kanggo ngalih menyang ", ", banjur menyang ", "."],
        },
        system_now: Template {
            lead: "Sistem, saiki ",
            tails: [""],
        },
        system: "Sistem",
        light: "Padhang",
        dark: "Peteng",
        dark_mode: "peteng",
        light_mode: "padhang",
    },
    table: "Tabel",
    pages: PageLinks {
        previous: "Sadurungé",
        next: "Sabanjuré",
        label: "Kaca",
    },
    task: TaskBoxes {
        done: "Rampung",
        open: "Durung",
    },
    guide_heading: "Carané maca kaca iki",
    glossary_heading: "Glosarium",
    meaning_heads: &["Teges", "Artine", "Definisi"],
    contents_heading: "Isi",
    lang: LanguageSwitch {
        label: "Basa",
        original: "Asli",
        notice: Template {
            lead: "Iki terjemahan. Vèrsi sing dadi pathokan yaiku asliné (",
            tails: [")."],
        },
        on_device: "Ing piranti iki",
        machine: "Terjemahan mesin déning peramban iki, ing piranti iki: ora ana sing metu saka piranti.",
        show_original: "Tuduhaké asliné",
        translating: "Lagi nerjemahaké…",
        cannot: Template {
            lead: "Peramban iki ora bisa nerjemahaké menyang ",
            tails: ["."],
        },
        more: "Basa liyané",
    },
    export: ExportMenu {
        label: "Simpen utawa cithak",
        html: "Kaca iki, kanggo diwaca tanpa internet (HTML)",
        print: "Cithak, utawa simpen dadi PDF",
        pdf: "PDF",
        epub: "EPUB, kanggo e-reader",
    },
    prefs: "Setèlan maca",
    credit: Template {
        lead: "Digawé nganggo ",
        tails: [""],
    },
    share: Sharing {
        copied: "Pranala wis disalin",
        passage: "Pranala menyang pethikan iki",
        row: "Pranala menyang larik iki",
        thing: "Pranala mréné",
    },
    live: "Dianyari dadi vèrsi paling anyar",
};
