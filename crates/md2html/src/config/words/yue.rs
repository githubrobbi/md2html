// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `yue`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `yue`.
pub(super) static WORDS: Ui = Ui {
    skip: "跳去正文",
    search: "搵",
    search_page: "喺呢頁搵",
    keys: SearchKeys {
        step: "移動",
        open: "打開",
        close: "閂",
    },
    none: "呢頁冇相符嘅內容。",
    results: Template {
        lead: "",
        tails: [" 個結果"],
    },
    result: "1 個結果",
    glossary: "呢頁用到嘅詞",
    glossary_intro: "每章入面有虛線嘅詞，滑鼠停喺上面就會顯示意思，撳一下就會跳嚟呢度。每個條目都有<strong>返回</strong>，可以返去你啱啱睇緊嗰行。",
    glossary_shared: Template {
        lead: " 縮寫嚟自<a href=\"",
        tails: ["\">共用詞彙表</a>；撳<strong>全部詞彙</strong>會喺嗰個條目度打開佢。"],
    },
    back: "返回",
    all_terms: "全部詞彙",
    keep_outline: "&lsaquo; 保持目錄打開",
    outline: "目錄",
    outline_tips: OutlineTips {
        hide: "收埋目錄",
        show: "顯示目錄",
        keys: "（⌘\\，Windows 用 Control+\\）",
    },
    notes: "註釋",
    note_n: Template {
        lead: "註釋 ",
        tails: [""],
    },
    link_section: "連結去呢節",
    copy: CopyButton {
        label: "複製",
        copied: "複製咗",
    },
    callouts: Callouts {
        note: "說明",
        tip: "貼士",
        important: "重要",
        warning: "警告",
        caution: "注意",
    },
    footer: Template {
        lead: "由 <code>md2html</code> 用 <code>",
        tails: ["</code> 整出嚟。Markdown 檔案先係正本；改完要再整過。"],
    },
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["年", "", "日"],
    }),
    top: "頁頂",
    figure: EnlargedFigure {
        label: "放大咗嘅插圖",
        close: "閂",
        close_key: "閂（Escape）",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
        enlarge: "放大圖表",
    },
    size: TextSize {
        group: "字體大小",
        smaller: SizeStep {
            words: "細啲",
            joint: "，",
            unit: "%",
        },
        larger: SizeStep {
            words: "大啲",
            joint: "，",
            unit: "%",
        },
        as_written: "字體大小：100%，即係原本咁。",
        scaled: Template {
            lead: "字體大小：",
            tails: ["%。撳一下返去 100%。"],
        },
        keys: " 用 Command（Windows 用 Control）加 + 或 - 喺度調，0 還原；唔會郁到瀏覽器本身嘅縮放。",
        smallest: "已經係最細",
        largest: "已經係最大",
    },
    theme: ThemeSwitch {
        button: "外觀",
        tip: Template {
            lead: "外觀：",
            tails: ["。撳一下轉做", "，再轉做", "。"],
        },
        system_now: Template {
            lead: "跟系統，而家係",
            tails: [""],
        },
        system: "跟系統",
        light: "淺色",
        dark: "深色",
        dark_mode: "深色",
        light_mode: "淺色",
    },
    table: "表格",
    pages: PageLinks {
        previous: "上一頁",
        next: "下一頁",
        label: "頁面",
    },
    task: TaskBoxes {
        done: "搞掂",
        open: "未搞掂",
    },
    guide_heading: "點樣睇呢一頁",
    glossary_heading: "詞彙表",
    meaning_heads: &["意思", "定義", "解釋", "含義"],
    contents_heading: "目錄",
    lang: LanguageSwitch {
        label: "語言",
        original: "原文",
        notice: Template {
            lead: "呢個係譯本。以原文（",
            tails: ["）為準。"],
        },
        on_device: "喺呢部機度",
        machine: "由呢個瀏覽器喺呢部機入面做嘅機器翻譯：乜都唔會傳出去。",
        show_original: "睇原文",
        translating: "翻譯緊…",
        cannot: Template {
            lead: "呢個瀏覽器唔識譯做",
            tails: ["。"],
        },
        more: "更多語言",
    },
    export: ExportMenu {
        label: "儲存或者列印",
        html: "呢一頁（離線都睇得嘅 HTML）",
        print: "列印，或者存做 PDF",
        pdf: "PDF",
        epub: "EPUB（電子書閱讀器用）",
    },
    prefs: "閱讀設定",
    credit: Template {
        lead: "用 ",
        tails: [" 整嘅"],
    },
    share: Sharing {
        copied: "連結複製咗",
        passage: "連結去呢段文字",
        row: "連結去呢一行",
        thing: "連結去呢度",
    },
    live: "已經更新到最新版本",
};
