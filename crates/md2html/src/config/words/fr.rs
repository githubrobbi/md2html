// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `fr`.

use super::{
    Callouts, CopyButton, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch, OutlineTips,
    PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize, ThemeSwitch, Ui,
};

/// The interface in `fr`.
pub(super) static WORDS: Ui = Ui {
    skip: "Aller au contenu",
    search: "Rechercher",
    search_page: "Rechercher dans cette page",
    keys: SearchKeys {
        step: "naviguer",
        open: "ouvrir",
        close: "fermer",
    },
    none: "Rien sur cette page ne correspond.",
    results: Template {
        lead: "",
        tails: [" résultats"],
    },
    result: "1 résultat",
    glossary: "Les mots de cette page",
    glossary_intro: "Chaque mot souligné de pointillés dans les chapitres affiche son sens au survol et mène ici d'un clic. Chaque entrée comporte un lien <strong>retour</strong> vers la ligne que vous lisiez.",
    glossary_shared: Template {
        lead: " Les sigles viennent du <a href=\"",
        tails: [
            "\">glossaire commun</a>\u{a0}; <strong>tous les termes</strong> l'ouvre à l'entrée correspondante.",
        ],
    },
    back: "retour",
    all_terms: "tous les termes",
    keep_outline: "&lsaquo; garder le plan ouvert",
    outline: "Plan",
    outline_tips: OutlineTips {
        hide: "Masquer le plan",
        show: "Afficher le plan",
        keys: " (⌘\\, Ctrl+\\ sous Windows)",
    },
    notes: "Notes",
    note_n: Template {
        lead: "Note ",
        tails: [""],
    },
    link_section: "Lien vers cette section",
    copy: CopyButton {
        label: "copier",
        copied: "copié",
    },
    callouts: Callouts {
        note: "Remarque",
        tip: "Conseil",
        important: "Important",
        warning: "Avertissement",
        caution: "Attention",
    },
    footer: Template {
        lead: "Produit à partir de <code>",
        tails: [
            "</code> par <code>md2html</code>. Le fichier Markdown est la source\u{a0}; régénérez la page après l'avoir modifié.",
        ],
    },
    months: [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ],
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" ", " ", ""],
    }),
    top: "Haut de la page",
    figure: EnlargedFigure {
        label: "Figure agrandie",
        close: "Fermer",
        close_key: "Fermer (Échap)",
        position: Template {
            lead: "",
            tails: [" sur ", ""],
        },
        enlarge: "Agrandir le diagramme",
    },
    size: TextSize {
        group: "Taille du texte",
        smaller: SizeStep {
            words: "Texte plus petit",
            joint: ", ",
            unit: "\u{a0}%",
        },
        larger: SizeStep {
            words: "Texte plus grand",
            joint: ", ",
            unit: "\u{a0}%",
        },
        as_written: "Taille du texte\u{a0}: 100\u{a0}%, la page telle qu'écrite.",
        scaled: Template {
            lead: "Taille du texte\u{a0}: ",
            tails: ["\u{a0}%. Cliquez pour revenir à 100\u{a0}%."],
        },
        keys: " Commande (Ctrl sous Windows) avec + ou - la modifie ici, 0 la rétablit\u{a0}; le zoom du navigateur n'est pas touché.",
        smallest: "Taille minimale atteinte",
        largest: "Taille maximale atteinte",
    },
    theme: ThemeSwitch {
        button: "Thème",
        tip: Template {
            lead: "Thème\u{a0}: ",
            tails: [". Cliquez pour passer à ", ", puis à ", "."],
        },
        system_now: Template {
            lead: "Système, actuellement ",
            tails: [""],
        },
        system: "Système",
        light: "Clair",
        dark: "Sombre",
        dark_mode: "sombre",
        light_mode: "clair",
    },
    table: "Tableau",
    pages: PageLinks {
        previous: "Précédent",
        next: "Suivant",
        label: "Pages",
    },
    task: TaskBoxes {
        done: "Fait",
        open: "À faire",
    },
    guide_heading: "Comment lire cette page",
    glossary_heading: "Glossaire",
    meaning_heads: &["Sens", "Signification", "Définition", "Description"],
    contents_heading: "Sommaire",
    lang: LanguageSwitch {
        label: "Langue",
        original: "Original",
        notice: Template {
            lead: "Ceci est une traduction. La version qui fait foi est l'original (",
            tails: [")."],
        },
        on_device: "Sur cet appareil",
        machine: "Traduction automatique effectuée par ce navigateur, sur cet appareil\u{a0}: rien n'en sort.",
        show_original: "Afficher l'original",
        translating: "Traduction en cours…",
        cannot: Template {
            lead: "Ce navigateur ne peut pas traduire en ",
            tails: ["."],
        },
        more: "Autres langues",
    },
    export: ExportMenu {
        label: "Enregistrer ou imprimer",
        html: "Cette page, à lire hors ligne (HTML)",
        print: "Imprimer, ou enregistrer en PDF",
        pdf: "PDF",
        epub: "EPUB, pour liseuses",
    },
    prefs: "Réglages de lecture",
    credit: Template {
        lead: "Réalisé avec ",
        tails: [""],
    },
    share: Sharing {
        copied: "Lien copié",
        passage: "Lien vers ce passage",
        row: "Lien vers cette ligne",
        thing: "Lien vers ceci",
    },
    live: "Mis à jour avec la dernière version",
};
