// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The interface words in `es`.

use super::{
    Callouts, CopyButton, CurrencySwitch, DateFormat, EnlargedFigure, ExportMenu, LanguageSwitch,
    OutlineTips, PageLinks, SearchKeys, Sharing, SizeStep, TaskBoxes, Template, TextSize,
    ThemeSwitch, Ui,
};

/// The interface in `es`.
pub(super) static WORDS: Ui = Ui {
    skip: "Ir al contenido",
    search: "Buscar",
    search_page: "Buscar en esta página",
    keys: SearchKeys {
        step: "navegar",
        open: "abrir",
        close: "cerrar",
    },
    none: "No hay coincidencias en esta página.",
    results: Template {
        lead: "",
        tails: [" resultados"],
    },
    result: "1 resultado",
    glossary: "Las palabras de esta página",
    glossary_intro: "Cada palabra subrayada con puntos en los capítulos muestra su significado al posar el puntero sobre ella y lleva aquí con un clic. Cada entrada tiene un <strong>volver</strong> que lleva de vuelta a la línea que estaba leyendo.",
    glossary_shared: Template {
        lead: " Las siglas vienen del <a href=\"",
        tails: [
            "\">glosario común</a>; <strong>todos los términos</strong> lo abre en la entrada.",
        ],
    },
    back: "volver",
    all_terms: "todos los términos",
    keep_outline: "&lsaquo; mantener el índice abierto",
    outline: "Índice",
    outline_tips: OutlineTips {
        hide: "Ocultar el índice",
        show: "Mostrar el índice",
        keys: " (⌘\\, Ctrl+\\ en Windows)",
    },
    notes: "Notas",
    note_n: Template {
        lead: "Nota ",
        tails: [""],
    },
    link_section: "Enlace a esta sección",
    copy: CopyButton {
        label: "copiar",
        copied: "copiado",
    },
    callouts: Callouts {
        note: "Nota",
        tip: "Consejo",
        important: "Importante",
        warning: "Advertencia",
        caution: "Precaución",
    },
    footer: Template {
        lead: "Generado a partir de <code>",
        tails: [
            "</code> con <code>md2html</code>. El archivo Markdown es la fuente; vuelva a generar la página después de editarlo.",
        ],
    },
    months: [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ],
    date_fmt: DateFormat::DayMonthYear(Template {
        lead: "",
        tails: [" de ", " de ", ""],
    }),
    top: "Inicio de la página",
    figure: EnlargedFigure {
        label: "La figura, ampliada",
        close: "Cerrar",
        close_key: "Cerrar (Esc)",
        position: Template {
            lead: "",
            tails: [" de ", ""],
        },
        enlarge: "Ampliar el diagrama",
    },
    size: TextSize {
        group: "Tamaño del texto",
        smaller: SizeStep {
            words: "Texto más pequeño",
            joint: ", ",
            unit: " %",
        },
        larger: SizeStep {
            words: "Texto más grande",
            joint: ", ",
            unit: " %",
        },
        as_written: "Tamaño del texto: 100 %, el tamaño original.",
        scaled: Template {
            lead: "Tamaño del texto: ",
            tails: [" %. Haga clic para volver al 100 %."],
        },
        keys: " Comando (Ctrl en Windows) con + o - lo cambia aquí, 0 lo restablece; el zoom del navegador no se toca.",
        smallest: "Ya es el texto más pequeño",
        largest: "Ya es el texto más grande",
    },
    theme: ThemeSwitch {
        button: "Tema",
        tip: Template {
            lead: "Tema: ",
            tails: [". Haga clic para pasar a ", " y luego a ", "."],
        },
        system_now: Template {
            lead: "Sistema, ahora ",
            tails: [""],
        },
        system: "Sistema",
        light: "Claro",
        dark: "Oscuro",
        dark_mode: "oscuro",
        light_mode: "claro",
    },
    table: "Tabla",
    pages: PageLinks {
        previous: "Anterior",
        next: "Siguiente",
        label: "Páginas",
    },
    task: TaskBoxes {
        done: "Hecho",
        open: "Pendiente",
    },
    guide_heading: "Cómo leer esto",
    glossary_heading: "Glosario",
    meaning_heads: &[
        "Significado",
        "Definición",
        "Definicion",
        "Descripción",
        "Descripcion",
    ],
    contents_heading: "Índice",
    lang: LanguageSwitch {
        label: "Idioma",
        original: "Original",
        notice: Template {
            lead: "Esta es una traducción. El original (",
            tails: [") es la versión de referencia."],
        },
        on_device: "En este dispositivo",
        machine: "Una traducción automática de este navegador, en este dispositivo: nada sale de él.",
        show_original: "Mostrar el original",
        translating: "Traduciendo…",
        cannot: Template {
            lead: "Este navegador no puede traducir al idioma ",
            tails: ["."],
        },
        more: "Más idiomas",
    },
    money: CurrencySwitch {
        label: "Moneda",
        rate: Template {
            lead: "Convertido al tipo de cambio del ",
            tails: [""],
        },
        more: "Más monedas",
    },
    export: ExportMenu {
        label: "Guardar o imprimir",
        html: "Esta página, para leer sin conexión (HTML)",
        print: "Imprimir o guardar como PDF",
        pdf: "PDF",
        epub: "EPUB, para lectores electrónicos",
    },
    prefs: "Ajustes de lectura",
    credit: Template {
        lead: "Hecho con ",
        tails: [""],
    },
    share: Sharing {
        copied: "Enlace copiado",
        passage: "Enlace a este pasaje",
        row: "Enlace a esta fila",
        thing: "Enlace a esto",
    },
    live: "Actualizado a la última versión",
};
