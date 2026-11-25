// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `he`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `he`.
pub(super) static WORDS: Ui = Ui {
    skip: "דילוג לתוכן",
    search: "חיפוש",
    search_page: "חיפוש בדף זה",
    keys: SearchKeys {
        step: "מעבר",
        open: "פתיחה",
        close: "סגירה",
    },
    none: "שום דבר בדף זה אינו תואם.",
    results: Template {
        lead: "",
        tails: [" תוצאות"],
    },
    result: "תוצאה אחת",
    glossary: "מילים שבדף",
    glossary_intro: "כל מילה מנוקדת בפרקים מציגה את פירושה כשמצביעים עליה, ולחיצה עליה מובילה לכאן. בכל ערך יש <strong>חזרה</strong> שמחזירה לשורה שבה קראתם.",
    glossary_shared: Template {
        lead: " הקיצורים לקוחים מ<a href=\"",
        tails: ["\">המילון המשותף</a>; <strong>כל המונחים</strong> פותח אותו בערך."],
    },
    back: "חזרה",
    all_terms: "כל המונחים",
    keep_outline: "&rsaquo; להשאיר את תוכן העניינים פתוח",
    outline: "תוכן העניינים",
    outline_tips: OutlineTips {
        hide: "הסתרת תוכן העניינים",
        show: "הצגת תוכן העניינים",
        keys: " (⌘\\, Control+\\ ב־Windows)",
    },
    notes: "הערות",
    note_n: Template {
        lead: "הערה ",
        tails: [""],
    },
    link_section: "קישור לסעיף זה",
    copy: CopyButton {
        label: "העתקה",
        copied: "הועתק",
    },
    callouts: Callouts {
        note: "הערה",
        tip: "עצה",
        important: "חשוב",
        warning: "אזהרה",
        caution: "זהירות",
    },
    footer: Template {
        lead: "נוצר מ־<code>",
        tails: [
            "</code> באמצעות <code>md2html</code>. קובץ ה־Markdown הוא המקור; יש ליצור מחדש לאחר עריכה.",
        ],
    },
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
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ב", " ", ""],
    }),
    top: "ראש הדף",
    figure: EnlargedFigure {
        label: "האיור, מוגדל",
        close: "סגירה",
        close_key: "סגירה (Escape)",
        position: Template {
            lead: "",
            tails: [" מתוך ", ""],
        },
        enlarge: "הגדלת התרשים",
    },
    size: TextSize {
        group: "גודל טקסט",
        smaller: SizeStep {
            words: "טקסט קטן יותר",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "טקסט גדול יותר",
            joint: ", ",
            unit: "%",
        },
        as_written: "גודל טקסט: 100%, הדף כפי שנכתב.",
        scaled: Template {
            lead: "גודל טקסט: ",
            tails: ["%. לחיצה מחזירה ל־100%."],
        },
        keys: " Command (Control ב־Windows) עם + או - משנה אותו כאן, 0 מאפס; הזום של הדפדפן אינו משתנה.",
        smallest: "הטקסט כבר בגודל הקטן ביותר",
        largest: "הטקסט כבר בגודל הגדול ביותר",
    },
    theme: ThemeSwitch {
        button: "ערכת צבעים",
        tip: Template {
            lead: "ערכת צבעים: ",
            tails: [". לחיצה עוברת ל", ", ואחר כך ל", "."],
        },
        system_now: Template {
            lead: "מערכת, כרגע ",
            tails: [""],
        },
        system: "מערכת",
        light: "בהירה",
        dark: "כהה",
        dark_mode: "כהה",
        light_mode: "בהירה",
    },
    table: "טבלה",
    pages: PageLinks {
        previous: "הדף הקודם",
        next: "הדף הבא",
        label: "דפים",
    },
    task: TaskBoxes {
        done: "בוצע",
        open: "פתוח",
    },
    guide_heading: "איך לקרוא את זה",
    glossary_heading: "מילון מונחים",
    meaning_heads: &["משמעות", "הגדרה", "פירוש", "הסבר"],
    contents_heading: "תוכן עניינים",
    lang: LanguageSwitch {
        label: "שפה",
        original: "מקור",
        notice: Template {
            lead: "זהו תרגום. הנוסח המחייב הוא המקור (",
            tails: [")."],
        },
        on_device: "במכשיר זה",
        machine: "תרגום מכונה של הדפדפן, במכשיר זה: דבר אינו יוצא ממנו.",
        show_original: "הצגת המקור",
        translating: "מתרגם…",
        cannot: Template {
            lead: "הדפדפן אינו יכול לתרגם ל",
            tails: ["."],
        },
        more: "שפות נוספות",
    },
    export: ExportMenu {
        label: "שמירה או הדפסה",
        html: "הדף הזה, לקריאה ללא חיבור (HTML)",
        print: "הדפסה או שמירה כ־PDF",
        pdf: "PDF",
        epub: "EPUB, לקוראים אלקטרוניים",
    },
    prefs: "הגדרות קריאה",
    credit: Template {
        lead: "נוצר באמצעות ",
        tails: [""],
    },
    share: Sharing {
        copied: "הקישור הועתק",
        passage: "קישור לקטע הזה",
        row: "קישור לשורה הזו",
        thing: "קישור לכאן",
    },
    live: "עודכן לגרסה האחרונה",
};
