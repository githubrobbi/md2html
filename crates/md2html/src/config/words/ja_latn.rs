// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `ja-Latn`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `ja-Latn`.
pub(super) static WORDS: Ui = Ui {
    skip: "Honbun e idō",
    search: "Kensaku",
    search_page: "Kono pēji o kensaku",
    keys: SearchKeys {
        step: "idō",
        open: "hiraku",
        close: "tojiru",
    },
    none: "Kono pēji ni wa gaitō suru mono ga arimasen.",
    results: Template {
        lead: "",
        tails: [" ken"],
    },
    result: "1 ken",
    glossary: "Kono pēji no yōgo",
    glossary_intro: "Kaku shō no tensen no tsuita go ni pointā o noseru to imi ga hyōji sare, kurikku suru to koko e idō shimasu. Kaku kōmoku no <strong>modoru</strong> de, yonde ita gyō ni modoremasu.",
    glossary_shared: Template {
        lead: " Ryakugo wa <a href=\"",
        tails: [
            "\">kyōtsū no yōgoshū</a> ni yoru mono desu. <strong>Subete no yōgo</strong> de sono kōmoku o hirakimasu.",
        ],
    },
    back: "modoru",
    all_terms: "subete no yōgo",
    keep_outline: "&lsaquo; mokuji o hiraita mama ni suru",
    outline: "Mokuji",
    outline_tips: OutlineTips {
        hide: "Mokuji o kakusu",
        show: "Mokuji o hyōji",
        keys: " (⌘\\, Windows de wa Control+\\)",
    },
    notes: "Chū",
    note_n: Template {
        lead: "Chū ",
        tails: [""],
    },
    link_section: "Kono sekushon e no rinku",
    copy: CopyButton {
        label: "kopī",
        copied: "kopī shimashita",
    },
    callouts: Callouts {
        note: "Memo",
        tip: "Hinto",
        important: "Jūyō",
        warning: "Keikoku",
        caution: "Chūi",
    },
    footer: Template {
        lead: "<code>",
        tails: [
            "</code> kara <code>md2html</code> de seisei shimashita. Markdown fairu ga genpon desu. Henshū shitara sai-seisei shite kudasai.",
        ],
    },
    months: [
        "ichigatsu",
        "nigatsu",
        "sangatsu",
        "shigatsu",
        "gogatsu",
        "rokugatsu",
        "shichigatsu",
        "hachigatsu",
        "kugatsu",
        "jūgatsu",
        "jūichigatsu",
        "jūnigatsu",
    ],
    date_fmt: DateFormat::YearMonthDay(Template {
        lead: "",
        tails: ["-nen ", " ", "-nichi"],
    }),
    top: "Pēji no sentō",
    figure: EnlargedFigure {
        label: "Kakudai shita zu",
        close: "Tojiru",
        close_key: "Tojiru (Escape)",
        position: Template {
            lead: "",
            tails: [" / ", ""],
        },
    },
    size: TextSize {
        group: "Moji no ōkisa",
        smaller: SizeStep {
            words: "Moji o chiisaku",
            joint: ", ",
            unit: "%",
        },
        larger: SizeStep {
            words: "Moji o ōkiku",
            joint: ", ",
            unit: "%",
        },
        as_written: "Moji no ōkisa: 100% (moto no ōkisa).",
        scaled: Template {
            lead: "Moji no ōkisa: ",
            tails: ["%. Kurikku de 100% ni modorimasu."],
        },
        keys: " Command (Windows de wa Control) to + / - de henkō shi, 0 de moto ni modorimasu. Burauza no zūmu ni wa eikyō shimasen.",
        smallest: "Kore ijō chiisaku dekimasen",
        largest: "Kore ijō ōkiku dekimasen",
    },
    theme: ThemeSwitch {
        button: "Hyōji",
        tip: Template {
            lead: "Hyōji: ",
            tails: [". Kurikku de ", ", tsugi ni ", "."],
        },
        system_now: Template {
            lead: "Shisutemu (ima wa ",
            tails: [")"],
        },
        system: "Shisutemu",
        light: "Raito",
        dark: "Dāku",
        dark_mode: "dāku",
        light_mode: "raito",
    },
    table: "Hyō",
    pages: PageLinks {
        previous: "Mae e",
        next: "Tsugi e",
        label: "Pēji",
    },
    task: TaskBoxes {
        done: "Kanryō",
        open: "Mikanryō",
    },
    guide_heading: "Kono pēji no yomikata",
    glossary_heading: "Yōgoshū",
    contents_heading: "Mokuji",
    lang: LanguageSwitch {
        label: "Gengo",
        original: "Genbun",
        notice: Template {
            lead: "Kore wa hon'yaku desu. Seishiki na han wa genbun (",
            tails: [") desu."],
        },
        on_device: "Kono tanmatsu de",
        machine: "Kono burauza ni yoru, kono tanmatsu-jō de no kikai hon'yaku desu. Gaibu ni wa nani mo sōshin saremasen.",
        show_original: "Genbun o hyōji",
        translating: "Hon'yaku-chū…",
        cannot: Template {
            lead: "Kono burauza wa ",
            tails: [" ni hon'yaku dekimasen."],
        },
        more: "Sono hoka no gengo",
    },
    export: ExportMenu {
        label: "Hozon / insatsu",
        html: "Kono pēji (ofurain de yomeru HTML)",
        print: "Insatsu, mata wa PDF to shite hozon",
        pdf: "PDF",
        epub: "EPUB (denshi shoseki rīdā-yō)",
    },
    prefs: "Hyōji no settei",
    credit: Template {
        lead: "",
        tails: [" de sakusei"],
    },
    share: Sharing {
        copied: "Rinku o kopī shimashita",
        passage: "Kono kasho e no rinku",
        row: "Kono gyō e no rinku",
        thing: "Koko e no rinku",
    },
    live: "Saishin no han ni kōshin shimashita",
};
