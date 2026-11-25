// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `ja-Hira`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `ja-Hira`.
pub(super) static WORDS: Ui = Ui {
    skip: "ほんぶんへ すすむ",
    search: "さがす",
    search_page: "この ページを さがす",
    keys: SearchKeys {
        step: "うごく",
        open: "ひらく",
        close: "とじる",
    },
    none: "この ページには あてはまる ものが ありません。",
    results: Template {
        lead: "",
        tails: [" けん"],
    },
    result: "1 けん",
    glossary: "この ページの ことば",
    glossary_intro: "しょうの なかで てんせんの ついた ことばに ポインターを のせると いみが でて、クリックすると ここへ きます。どの こうもくにも <strong>もどる</strong> が あり、よんで いた ぎょうに もどれます。",
    glossary_shared: Template {
        lead: " りゃくごは <a href=\"",
        tails: [
            "\">きょうつうの ことばの いみ</a>から きて います。<strong>すべての ことば</strong>で その こうもくを ひらきます。",
        ],
    },
    back: "もどる",
    all_terms: "すべての ことば",
    keep_outline: "&lsaquo; もくじを ひらいた ままに する",
    outline: "もくじ",
    outline_tips: OutlineTips {
        hide: "もくじを かくす",
        show: "もくじを だす",
        keys: "（⌘\\、Windows では Control+\\）",
    },
    notes: "ちゅう",
    note_n: Template {
        lead: "ちゅう ",
        tails: [""],
    },
    link_section: "この ぶぶんへの リンク",
    copy: CopyButton {
        label: "コピー",
        copied: "コピーしました",
    },
    callouts: Callouts {
        note: "メモ",
        tip: "ヒント",
        important: "たいせつ",
        warning: "けいこく",
        caution: "ちゅうい",
    },
    footer: Template {
        lead: "<code>",
        tails: [
            "</code> から <code>md2html</code> で つくりました。もとは Markdown ファイルです。なおしたら もう いちど つくって ください。",
        ],
    },
    months: [
        "1がつ", "2がつ", "3がつ", "4がつ", "5がつ", "6がつ", "7がつ", "8がつ", "9がつ", "10がつ",
        "11がつ", "12がつ",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["ねん ", " ", "にち"],
    }),
    top: "ページの いちばん うえ",
    figure: EnlargedFigure {
        label: "おおきく した ず",
        close: "とじる",
        close_key: "とじる（Escape）",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
        enlarge: "ずを おおきく する",
    },
    size: TextSize {
        group: "もじの おおきさ",
        smaller: SizeStep {
            words: "もじを ちいさく",
            joint: "、",
            unit: "%",
        },
        larger: SizeStep {
            words: "もじを おおきく",
            joint: "、",
            unit: "%",
        },
        as_written: "もじの おおきさ：100%（もとの おおきさ）。",
        scaled: Template {
            lead: "もじの おおきさ：",
            tails: ["%。クリックで 100% に もどります。"],
        },
        keys: " Command（Windows では Control）と + / - で かえられ、0 で もとに もどります。ブラウザの ズームは かわりません。",
        smallest: "これより ちいさく できません",
        largest: "これより おおきく できません",
    },
    theme: ThemeSwitch {
        button: "みため",
        tip: Template {
            lead: "みため：",
            tails: ["。クリックで ", "、つぎに ", "。"],
        },
        system_now: Template {
            lead: "システム（いまは ",
            tails: ["）"],
        },
        system: "システム",
        light: "あかるい",
        dark: "くらい",
        dark_mode: "くらい",
        light_mode: "あかるい",
    },
    table: "ひょう",
    pages: PageLinks {
        previous: "まえへ",
        next: "つぎへ",
        label: "ページ",
    },
    task: TaskBoxes {
        done: "おわり",
        open: "まだ",
    },
    guide_heading: "この ページの よみかた",
    glossary_heading: "ことばの いみ",
    meaning_heads: &["いみ", "ていぎ", "せつめい"],
    contents_heading: "もくじ",
    lang: LanguageSwitch {
        label: "ことば",
        original: "もとの ぶん",
        notice: Template {
            lead: "これは ほんやくです。せいしきな ものは もとの ぶん（",
            tails: ["）です。"],
        },
        on_device: "この きかいで",
        machine: "この ブラウザが この きかいの なかで した きかい ほんやくです。そとへは なにも おくりません。",
        show_original: "もとの ぶんを みる",
        translating: "ほんやく ちゅう…",
        cannot: Template {
            lead: "この ブラウザは ",
            tails: [" に ほんやく できません。"],
        },
        more: "ほかの ことば",
    },
    export: ExportMenu {
        label: "ほぞん・いんさつ",
        html: "この ページ（インターネット なしで よめる HTML）",
        print: "いんさつ、または PDF で ほぞん",
        pdf: "PDF",
        epub: "EPUB（でんし しょせき よう）",
    },
    prefs: "よみやすさの せってい",
    credit: Template {
        lead: "",
        tails: [" で つくりました"],
    },
    share: Sharing {
        copied: "リンクを コピーしました",
        passage: "この ぶぶんへの リンク",
        row: "この ぎょうへの リンク",
        thing: "ここへの リンク",
    },
    live: "あたらしい ものに しました",
};
