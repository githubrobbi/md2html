// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `zh-Hant`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `zh-Hant`.
pub(super) static WORDS: Ui = Ui {
    skip: "跳至正文",
    search: "搜尋",
    search_page: "搜尋本頁",
    keys: SearchKeys {
        step: "移動",
        open: "開啟",
        close: "關閉",
    },
    none: "本頁沒有符合的內容。",
    results: Template {
        lead: "",
        tails: [" 筆結果"],
    },
    result: "1 筆結果",
    glossary: "本頁用語",
    glossary_intro: "各章中帶虛線的詞，滑鼠停留時顯示釋義，點擊則跳到這裡。每個條目都有<strong>返回</strong>，可回到你正在讀的那一行。",
    glossary_shared: Template {
        lead: " 縮寫來自<a href=\"",
        tails: ["\">共用術語表</a>；<strong>全部術語</strong>會在該條目處開啟它。"],
    },
    back: "返回",
    all_terms: "全部術語",
    keep_outline: "&lsaquo; 保持目錄開啟",
    outline: "目錄",
    outline_tips: OutlineTips {
        hide: "隱藏目錄",
        show: "顯示目錄",
        keys: "（⌘\\，Windows 上為 Control+\\）",
    },
    notes: "註釋",
    note_n: Template {
        lead: "註釋 ",
        tails: [""],
    },
    link_section: "連結到本節",
    copy: CopyButton {
        label: "複製",
        copied: "已複製",
    },
    callouts: Callouts {
        note: "說明",
        tip: "提示",
        important: "重要",
        warning: "警告",
        caution: "注意",
    },
    footer: Template {
        lead: "由 <code>md2html</code> 從 <code>",
        tails: ["</code> 產生。Markdown 檔案是原件；編輯後請重新產生。"],
    },
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["年", "", "日"],
    }),
    top: "頁面頂端",
    figure: EnlargedFigure {
        label: "放大的圖",
        close: "關閉",
        close_key: "關閉（Escape）",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
        enlarge: "放大圖表",
    },
    size: TextSize {
        group: "字級",
        smaller: SizeStep {
            words: "縮小文字",
            joint: "，",
            unit: "%",
        },
        larger: SizeStep {
            words: "放大文字",
            joint: "，",
            unit: "%",
        },
        as_written: "字級：100%，依原樣顯示。",
        scaled: Template {
            lead: "字級：",
            tails: ["%。點擊恢復為 100%。"],
        },
        keys: " 按 Command（Windows 上為 Control）加 + 或 - 在此調整，0 復原；不影響瀏覽器本身的縮放。",
        smallest: "已是最小字級",
        largest: "已是最大字級",
    },
    theme: ThemeSwitch {
        button: "外觀",
        tip: Template {
            lead: "外觀：",
            tails: ["。點擊切換為", "，然後", "。"],
        },
        system_now: Template {
            lead: "跟隨系統，目前為",
            tails: [""],
        },
        system: "跟隨系統",
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
        done: "已完成",
        open: "未完成",
    },
    guide_heading: "如何閱讀本頁",
    glossary_heading: "術語表",
    meaning_heads: &["含義", "定義", "釋義", "意思", "說明"],
    contents_heading: "目錄",
    lang: LanguageSwitch {
        label: "語言",
        original: "原文",
        notice: Template {
            lead: "這是譯文。以原文（",
            tails: ["）為準。"],
        },
        on_device: "在本裝置上",
        machine: "由本瀏覽器在本裝置上完成的機器翻譯：任何內容都不會離開本裝置。",
        show_original: "顯示原文",
        translating: "正在翻譯…",
        cannot: Template {
            lead: "此瀏覽器無法翻譯成",
            tails: ["。"],
        },
        more: "更多語言",
    },
    export: ExportMenu {
        label: "儲存或列印",
        html: "本頁（可離線閱讀的 HTML）",
        print: "列印，或另存為 PDF",
        pdf: "PDF",
        epub: "EPUB（適用於電子閱讀器）",
    },
    prefs: "閱讀設定",
    credit: Template {
        lead: "使用 ",
        tails: [" 製作"],
    },
    share: Sharing {
        copied: "連結已複製",
        passage: "連結到這段文字",
        row: "連結到這一列",
        thing: "連結到這裡",
    },
    live: "已更新為最新版本",
};
