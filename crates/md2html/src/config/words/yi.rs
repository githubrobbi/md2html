// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `yi`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `yi`.
pub(super) static WORDS: Ui = Ui {
    skip: "שפּרינג צום אינהאַלט",
    search: "זוכן",
    search_page: "זוכן אויף דעם בלאַט",
    keys: SearchKeys {
        step: "רירן",
        open: "עפֿענען",
        close: "פֿאַרמאַכן",
    },
    none: "גאָרנישט אויף דעם בלאַט שטימט.",
    results: Template {
        lead: "",
        tails: [" רעזולטאַטן"],
    },
    result: "איין רעזולטאַט",
    glossary: "ווערטער אויף דעם בלאַט",
    glossary_intro: "יעדעס פּינטלדיק וואָרט אין די קאַפּיטלען ווײַזט זײַן באַטײַט ווען מע שטעלט אויף אים דעם טײַטל, און שפּרינגט אַהער ווען מע קליקט. יעדער אַרײַנטראָג האָט אַ <strong>צוריק</strong> צו דער שורה וואָס מע האָט געלייענט.",
    glossary_shared: Template {
        lead: " די ראַשי־תּיבֿות קומען פֿונעם <a href=\"",
        tails: [
            "\">געמיינזאַמען ווערטערבוך</a>; <strong>אַלע טערמינען</strong> עפֿנט עס בײַם אַרײַנטראָג.",
        ],
    },
    back: "צוריק",
    all_terms: "אַלע טערמינען",
    keep_outline: "&rsaquo; האַלטן דעם אינהאַלט אָפֿן",
    outline: "אינהאַלט",
    outline_tips: OutlineTips {
        hide: "באַהאַלטן דעם אינהאַלט",
        show: "ווײַזן דעם אינהאַלט",
        keys: " (⌘\\, Control+\\ אויף Windows)",
    },
    notes: "הערות",
    note_n: Template {
        lead: "הערה ",
        tails: [""],
    },
    link_section: "לינק צו דעם אָפּטייל",
    copy: CopyButton {
        label: "קאָפּירן",
        copied: "קאָפּירט",
    },
    callouts: Callouts {
        note: "באַמערקונג",
        tip: "עצה",
        important: "וויכטיק",
        warning: "וואָרענונג",
        caution: "אָפּגעהיט",
    },
    footer: Template {
        lead: "געמאַכט פֿון <code>",
        tails: [
            "</code> מיט <code>md2html</code>. די Markdown־טעקע איז דער מקור; מאַכט אויף ס׳נײַ נאָכן רעדאַגירן.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "אָנהייב פֿונעם בלאַט",
    figure: EnlargedFigure {
        label: "דאָס בילד, פֿאַרגרעסערט",
        close: "פֿאַרמאַכן",
        close_key: "פֿאַרמאַכן (Escape)",
        position: Template {
            lead: "",
            tails: [" פֿון ", ""],
        },
    },
    size: TextSize {
        group: "גרייס פֿונעם טעקסט",
        smaller: SizeStep {
            words: "קלענערער טעקסט",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "גרעסערער טעקסט",
            joint: ", ",
            unit: "%",
        },
        as_written: "גרייס פֿונעם טעקסט: 100%, דער בלאַט ווי געשריבן.",
        scaled: Template {
            lead: "גרייס פֿונעם טעקסט: ",
            tails: ["%. קליקט צוריק צו 100%."],
        },
        keys: " Command (Control אויף Windows) מיט + אָדער - ענדערט עס דאָ, 0 שטעלט צוריק; דער זום פֿונעם בלעטערער בלײַבט אומגערירט.",
        smallest: "שוין דער קלענסטער טעקסט",
        largest: "שוין דער גרעסטער טעקסט",
    },
    theme: ThemeSwitch {
        button: "קאָלירן",
        tip: Template {
            lead: "קאָלירן: ",
            tails: [". קליקט פֿאַר ", ", דערנאָך ", "."],
        },
        system_now: Template {
            lead: "סיסטעם, איצט ",
            tails: [""],
        },
        system: "סיסטעם",
        light: "ליכטיק",
        dark: "טונקל",
        dark_mode: "טונקל",
        light_mode: "ליכטיק",
    },
    table: "טאַבעלע",
    pages: PageLinks {
        previous: "פֿריִערדיקער בלאַט",
        next: "קומענדיקער בלאַט",
        label: "בלעטער",
    },
    task: TaskBoxes {
        done: "געטאָן",
        open: "אָפֿן",
    },
    guide_heading: "ווי צו לייענען דאָס",
    glossary_heading: "ווערטערבוך",
    contents_heading: "אינהאַלט",
    lang: LanguageSwitch {
        label: "שפּראַך",
        original: "אָריגינאַל",
        notice: Template {
            lead: "דאָס איז אַן איבערזעצונג. דער גילטיקער נוסח איז דער אָריגינאַל (",
            tails: [")."],
        },
        on_device: "אויף דעם אַפּאַראַט",
        machine: "אַ מאַשין־איבערזעצונג פֿונעם בלעטערער, אויף דעם אַפּאַראַט: גאָרנישט גייט אַרויס.",
        show_original: "ווײַזן דעם אָריגינאַל",
        translating: "מע זעצט איבער…",
        cannot: Template {
            lead: "דער בלעטערער קען נישט איבערזעצן אויף ",
            tails: ["."],
        },
        more: "מער שפּראַכן",
    },
    export: ExportMenu {
        label: "אָפּהיטן אָדער דרוקן",
        html: "דער בלאַט, צו לייענען אָן אינטערנעץ (HTML)",
        print: "דרוקן, אָדער אָפּהיטן ווי PDF",
        pdf: "PDF",
        epub: "EPUB, פֿאַר עלעקטראָנישע לייענערס",
    },
    prefs: "לייען־איינשטעלונגען",
    credit: Template {
        lead: "געמאַכט מיט ",
        tails: [""],
    },
    share: Sharing {
        copied: "דער לינק איז קאָפּירט",
        passage: "אַ לינק צו דעם אָפּשניט",
        row: "אַ לינק צו דער שורה",
        thing: "אַ לינק אַהער",
    },
    live: "דערהײַנטיקט צו דער לעצטער ווערסיע",
};
