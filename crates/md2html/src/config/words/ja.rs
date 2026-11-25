// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `ja`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `ja`.
pub(super) static WORDS: Ui = Ui {
    skip: "本文へ移動",
    search: "検索",
    search_page: "このページを検索",
    keys: SearchKeys {
        step: "移動",
        open: "開く",
        close: "閉じる",
    },
    none: "このページに一致するものはありません。",
    results: Template {
        lead: "",
        tails: [" 件"],
    },
    result: "1 件",
    glossary: "このページの用語",
    glossary_intro: "各章の点線の付いた語にポインタを置くと意味が表示され、クリックするとここへ移動します。各項目の<strong>戻る</strong>で、読んでいた行に戻れます。",
    glossary_shared: Template {
        lead: " 略語は<a href=\"",
        tails: [
            "\">共通の用語集</a>によるものです。<strong>すべての用語</strong>で該当する項目を開きます。",
        ],
    },
    back: "戻る",
    all_terms: "すべての用語",
    keep_outline: "&lsaquo; 目次を開いたままにする",
    outline: "目次",
    outline_tips: OutlineTips {
        hide: "目次を隠す",
        show: "目次を表示",
        keys: "（⌘\\、Windows では Control+\\）",
    },
    notes: "注",
    note_n: Template {
        lead: "注 ",
        tails: [""],
    },
    link_section: "このセクションへのリンク",
    copy: CopyButton {
        label: "コピー",
        copied: "コピーしました",
    },
    callouts: Callouts {
        note: "メモ",
        tip: "ヒント",
        important: "重要",
        warning: "警告",
        caution: "注意",
    },
    footer: Template {
        lead: "<code>",
        tails: [
            "</code> から <code>md2html</code> で生成しました。Markdown ファイルが原本です。編集したら生成し直してください。",
        ],
    },
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["年", "", "日"],
    }),
    top: "ページの先頭",
    figure: EnlargedFigure {
        label: "拡大した図",
        close: "閉じる",
        close_key: "閉じる（Escape）",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
        enlarge: "ダイアグラムを拡大する",
    },
    size: TextSize {
        group: "文字サイズ",
        smaller: SizeStep {
            words: "文字を小さく",
            joint: "、",
            unit: "%",
        },
        larger: SizeStep {
            words: "文字を大きく",
            joint: "、",
            unit: "%",
        },
        as_written: "文字サイズ：100%（元の大きさ）。",
        scaled: Template {
            lead: "文字サイズ：",
            tails: ["%。クリックで 100% に戻ります。"],
        },
        keys: " Command（Windows では Control）と + / - でここで変更し、0 で元に戻ります。ブラウザのズームには影響しません。",
        smallest: "これ以上小さくできません",
        largest: "これ以上大きくできません",
    },
    theme: ThemeSwitch {
        button: "表示",
        tip: Template {
            lead: "表示：",
            tails: ["。クリックで", "、次に", "。"],
        },
        system_now: Template {
            lead: "システム（現在は",
            tails: ["）"],
        },
        system: "システム",
        light: "ライト",
        dark: "ダーク",
        dark_mode: "ダーク",
        light_mode: "ライト",
    },
    table: "表",
    pages: PageLinks {
        previous: "前へ",
        next: "次へ",
        label: "ページ",
    },
    task: TaskBoxes {
        done: "完了",
        open: "未完了",
    },
    guide_heading: "このページの読み方",
    glossary_heading: "用語集",
    meaning_heads: &["意味", "定義", "説明"],
    contents_heading: "目次",
    lang: LanguageSwitch {
        label: "言語",
        original: "原文",
        notice: Template {
            lead: "これは翻訳です。正式な版は原文（",
            tails: ["）です。"],
        },
        on_device: "この端末で",
        machine: "このブラウザによる、この端末上での機械翻訳です。外部には何も送信されません。",
        show_original: "原文を表示",
        translating: "翻訳中…",
        cannot: Template {
            lead: "このブラウザは",
            tails: ["に翻訳できません。"],
        },
        more: "その他の言語",
    },
    export: ExportMenu {
        label: "保存・印刷",
        html: "このページ（オフラインで読める HTML）",
        print: "印刷、または PDF として保存",
        pdf: "PDF",
        epub: "EPUB（電子書籍リーダー用）",
    },
    prefs: "表示の設定",
    credit: Template {
        lead: "",
        tails: [" で作成"],
    },
    share: Sharing {
        copied: "リンクをコピーしました",
        passage: "この箇所へのリンク",
        row: "この行へのリンク",
        thing: "ここへのリンク",
    },
    live: "最新の版に更新しました",
};
