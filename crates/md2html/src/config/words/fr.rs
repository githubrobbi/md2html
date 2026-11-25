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
        step: "choisir",
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
    glossary_intro: "Chaque mot souligné de pointillés dans les chapitres affiche son sens au survol et mène ici d'un clic. Chaque entrée a un <strong>retour</strong> vers la ligne que vous lisiez.",
    glossary_shared: Template {
        lead: " Les sigles viennent du <a href=\"",
        tails: ["\">glossaire commun</a> ; <strong>tous les termes</strong> l'ouvre à l'entrée."],
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
            "</code> par <code>md2html</code>. Le fichier Markdown est la source ; régénérez la page après l'avoir modifié.",
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
        label: "La figure, agrandie",
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
            unit: " %",
        },
        larger: SizeStep {
            words: "Texte plus grand",
            joint: ", ",
            unit: " %",
        },
        as_written: "Taille du texte : 100 %, la page telle qu'écrite.",
        scaled: Template {
            lead: "Taille du texte : ",
            tails: [" %. Cliquez pour revenir à 100 %."],
        },
        keys: " Commande (Ctrl sous Windows) avec + ou - la modifie ici, 0 la rétablit ; le zoom du navigateur n'est pas touché.",
        smallest: "Déjà le plus petit texte",
        largest: "Déjà le plus grand texte",
    },
    theme: ThemeSwitch {
        button: "Thème",
        tip: Template {
            lead: "Thème : ",
            tails: [". Cliquez pour ", ", puis ", "."],
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
    guide_heading: "Comment lire ceci",
    glossary_heading: "Glossaire",
    contents_heading: "Sommaire",
    lang: LanguageSwitch {
        label: "Langue",
        original: "Original",
        notice: Template {
            lead: "Ceci est une traduction. La version qui fait foi est l'original (",
            tails: [")."],
        },
        on_device: "Sur cet appareil",
        machine: "Une traduction automatique par ce navigateur, sur cet appareil : rien n'en sort.",
        show_original: "Afficher l'original",
        translating: "Traduction…",
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
