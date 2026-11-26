// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `zh-Hans`.

use super::{
    Callouts, CopyButton, CurrencySwitch, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch,
    OutlineTips, PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize,
    ThemeSwitch, Ui,
};

/// The interface in `zh-Hans`.
pub(super) static WORDS: Ui = Ui {
    skip: "跳到正文",
    search: "搜索",
    search_page: "搜索本页",
    keys: SearchKeys {
        step: "移动",
        open: "打开",
        close: "关闭",
    },
    none: "本页没有匹配的内容。",
    results: Template {
        lead: "",
        tails: [" 条结果"],
    },
    result: "1 条结果",
    glossary: "本页用语",
    glossary_intro: "各章中带虚线的词，鼠标停留时显示释义，点击则跳到这里。每个条目都有<strong>返回</strong>，可回到你正在读的那一行。",
    glossary_shared: Template {
        lead: " 缩写来自<a href=\"",
        tails: ["\">共享术语表</a>；<strong>全部术语</strong>会在该条目处打开它。"],
    },
    back: "返回",
    all_terms: "全部术语",
    keep_outline: "&lsaquo; 保持目录打开",
    outline: "目录",
    outline_tips: OutlineTips {
        hide: "隐藏目录",
        show: "显示目录",
        keys: "（⌘\\，Windows 上为 Control+\\）",
    },
    notes: "注释",
    note_n: Template {
        lead: "注释 ",
        tails: [""],
    },
    link_section: "链接到本节",
    copy: CopyButton {
        label: "复制",
        copied: "已复制",
    },
    callouts: Callouts {
        note: "说明",
        tip: "提示",
        important: "重要",
        warning: "警告",
        caution: "注意",
    },
    footer: Template {
        lead: "由 <code>md2html</code> 从 <code>",
        tails: ["</code> 生成。Markdown 文件是源文件；编辑后请重新生成。"],
    },
    months: [
        "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["年", "", "日"],
    }),
    top: "页面顶部",
    figure: EnlargedFigure {
        label: "放大的插图",
        close: "关闭",
        close_key: "关闭（Escape）",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
        enlarge: "放大图表",
    },
    size: TextSize {
        group: "字号",
        smaller: SizeStep {
            words: "缩小文字",
            joint: "，",
            unit: "%",
        },
        larger: SizeStep {
            words: "放大文字",
            joint: "，",
            unit: "%",
        },
        as_written: "字号：100%，按原样显示。",
        scaled: Template {
            lead: "字号：",
            tails: ["%。点击恢复为 100%。"],
        },
        keys: " 按 Command（Windows 上为 Control）加 + 或 - 在此调整，0 复原；不影响浏览器自身的缩放。",
        smallest: "已是最小字号",
        largest: "已是最大字号",
    },
    theme: ThemeSwitch {
        button: "外观",
        tip: Template {
            lead: "外观：",
            tails: ["。点击切换为", "，然后", "。"],
        },
        system_now: Template {
            lead: "跟随系统，当前为",
            tails: [""],
        },
        system: "跟随系统",
        light: "浅色",
        dark: "深色",
        dark_mode: "深色",
        light_mode: "浅色",
    },
    table: "表格",
    pages: PageLinks {
        previous: "上一页",
        next: "下一页",
        label: "页面",
    },
    task: TaskBoxes {
        done: "已完成",
        open: "未完成",
    },
    guide_heading: "如何阅读本页",
    glossary_heading: "术语表",
    meaning_heads: &["含义", "定义", "释义", "意思", "说明"],
    contents_heading: "目录",
    lang: LanguageSwitch {
        label: "语言",
        original: "原文",
        notice: Template {
            lead: "这是译文。以原文（",
            tails: ["）为准。"],
        },
        on_device: "在本设备上",
        machine: "由本浏览器在本设备上完成的机器翻译：任何内容都不会离开本设备。",
        show_original: "显示原文",
        translating: "正在翻译…",
        cannot: Template {
            lead: "此浏览器无法翻译成",
            tails: ["。"],
        },
        more: "更多语言",
    },
    money: CurrencySwitch {
        label: "货币",
        rate: Template {
            lead: "按",
            tails: ["的汇率换算"],
        },
        more: "更多货币",
    },
    export: ExportMenu {
        label: "保存或打印",
        html: "本页（可离线阅读的 HTML）",
        print: "打印，或另存为 PDF",
        pdf: "PDF",
        epub: "EPUB（适用于电子阅读器）",
    },
    prefs: "阅读设置",
    credit: Template {
        lead: "使用 ",
        tails: [" 制作"],
    },
    share: Sharing {
        copied: "链接已复制",
        passage: "链接到这段文字",
        row: "链接到这一行",
        thing: "链接到这里",
    },
    live: "已更新到最新版本",
};
