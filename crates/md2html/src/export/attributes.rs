// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The attributes HTML defines, element by element, as a book reads them.
//!
//! The filter lets an author's attribute through by its spelling, so with
//! `--html` an attribute HTML has never had (`<a b="c">`) reaches the page,
//! where a browser passes over it. EPUB's XHTML is validated, and refuses
//! it. The book keeps an attribute only when HTML defines it: for every
//! element (the global attributes, `data-*`, `aria-*`, the XML and EPUB
//! namespaces' own), or for the element it stands on. Inline SVG and
//! `MathML` keep theirs; the rewrite does not look inside them.

/// The attributes HTML defines on every element, as EPUB's validator
/// takes them.
const GLOBAL: [&str; 23] = [
    "accesskey",
    "autocapitalize",
    "class",
    "contenteditable",
    "dir",
    "draggable",
    "enterkeyhint",
    "hidden",
    "id",
    "inputmode",
    "itemid",
    "itemprop",
    "itemref",
    "itemscope",
    "itemtype",
    "lang",
    "nonce",
    "role",
    "spellcheck",
    "style",
    "tabindex",
    "title",
    "translate",
];

/// Prefixes that name a family of attributes valid on every element:
/// custom data, accessibility, and the XML and EPUB namespaces'
/// (`xml:lang`, `epub:type`).
const PREFIXES: [&str; 4] = ["data-", "aria-", "xml:", "epub:"];

/// One element and the attributes HTML defines for it beyond the global
/// ones.
#[derive(Clone, Copy, Debug)]
struct Element {
    /// Its name, in lower case.
    name: &'static str,
    /// Its own attributes.
    own: &'static [&'static str],
}

/// What an element that takes only the global attributes has of its own.
const NONE: &[&str] = &[];

/// The attributes of a link to a document or a file.
const LINK: &[&str] = &[
    "href",
    "target",
    "download",
    "ping",
    "rel",
    "hreflang",
    "type",
    "referrerpolicy",
];

/// The attributes of a cited change or quotation.
const CITED: &[&str] = &["cite", "datetime"];

/// The attributes every playing medium has.
const MEDIA: &[&str] = &[
    "src",
    "crossorigin",
    "preload",
    "autoplay",
    "loop",
    "muted",
    "controls",
];

/// A video's: a medium's, and its picture's.
const VIDEO: &[&str] = &[
    "src",
    "crossorigin",
    "preload",
    "autoplay",
    "loop",
    "muted",
    "controls",
    "poster",
    "playsinline",
    "width",
    "height",
];

/// An image's.
const IMAGE: &[&str] = &[
    "alt",
    "src",
    "srcset",
    "sizes",
    "crossorigin",
    "usemap",
    "ismap",
    "width",
    "height",
    "referrerpolicy",
    "decoding",
    "loading",
    "fetchpriority",
];

/// A table cell's.
const CELL: &[&str] = &["colspan", "rowspan", "headers"];

/// A form control's that md2html writes (a task's box, a copy button).
const CONTROL: &[&str] = &[
    "autocomplete",
    "checked",
    "disabled",
    "form",
    "name",
    "readonly",
    "required",
    "type",
    "value",
];

/// Every element the book's chapter may hold outside SVG and `MathML`,
/// with its own attributes.
///
/// They are those the filter lets stand and those md2html writes; HTML
/// 4's `center` and `strike` last, which the book writes as a current
/// element.
const ELEMENTS: [Element; 79] = [
    Element {
        name: "a",
        own: LINK,
    },
    Element {
        name: "abbr",
        own: NONE,
    },
    Element {
        name: "address",
        own: NONE,
    },
    Element {
        name: "article",
        own: NONE,
    },
    Element {
        name: "aside",
        own: NONE,
    },
    Element {
        name: "audio",
        own: MEDIA,
    },
    Element {
        name: "b",
        own: NONE,
    },
    Element {
        name: "bdi",
        own: NONE,
    },
    Element {
        name: "bdo",
        own: NONE,
    },
    Element {
        name: "blockquote",
        own: &["cite"],
    },
    Element {
        name: "br",
        own: NONE,
    },
    Element {
        name: "button",
        own: CONTROL,
    },
    Element {
        name: "caption",
        own: NONE,
    },
    Element {
        name: "cite",
        own: NONE,
    },
    Element {
        name: "code",
        own: NONE,
    },
    Element {
        name: "col",
        own: &["span"],
    },
    Element {
        name: "colgroup",
        own: &["span"],
    },
    Element {
        name: "dd",
        own: NONE,
    },
    Element {
        name: "del",
        own: CITED,
    },
    Element {
        name: "details",
        own: &["open", "name"],
    },
    Element {
        name: "dfn",
        own: NONE,
    },
    Element {
        name: "div",
        own: NONE,
    },
    Element {
        name: "dl",
        own: NONE,
    },
    Element {
        name: "dt",
        own: NONE,
    },
    Element {
        name: "em",
        own: NONE,
    },
    Element {
        name: "figcaption",
        own: NONE,
    },
    Element {
        name: "figure",
        own: NONE,
    },
    Element {
        name: "footer",
        own: NONE,
    },
    Element {
        name: "h1",
        own: NONE,
    },
    Element {
        name: "h2",
        own: NONE,
    },
    Element {
        name: "h3",
        own: NONE,
    },
    Element {
        name: "h4",
        own: NONE,
    },
    Element {
        name: "h5",
        own: NONE,
    },
    Element {
        name: "h6",
        own: NONE,
    },
    Element {
        name: "header",
        own: NONE,
    },
    Element {
        name: "hr",
        own: NONE,
    },
    Element {
        name: "i",
        own: NONE,
    },
    Element {
        name: "img",
        own: IMAGE,
    },
    Element {
        name: "input",
        own: CONTROL,
    },
    Element {
        name: "ins",
        own: CITED,
    },
    Element {
        name: "kbd",
        own: NONE,
    },
    Element {
        name: "li",
        own: &["value"],
    },
    Element {
        name: "mark",
        own: NONE,
    },
    Element {
        name: "nav",
        own: NONE,
    },
    Element {
        name: "ol",
        own: &["reversed", "start", "type"],
    },
    Element {
        name: "p",
        own: NONE,
    },
    Element {
        name: "picture",
        own: NONE,
    },
    Element {
        name: "pre",
        own: NONE,
    },
    Element {
        name: "q",
        own: &["cite"],
    },
    Element {
        name: "rp",
        own: NONE,
    },
    Element {
        name: "rt",
        own: NONE,
    },
    Element {
        name: "rtc",
        own: NONE,
    },
    Element {
        name: "ruby",
        own: NONE,
    },
    Element {
        name: "s",
        own: NONE,
    },
    Element {
        name: "samp",
        own: NONE,
    },
    Element {
        name: "section",
        own: NONE,
    },
    Element {
        name: "small",
        own: NONE,
    },
    Element {
        name: "source",
        own: &["type", "media", "src", "srcset", "sizes", "width", "height"],
    },
    Element {
        name: "span",
        own: NONE,
    },
    Element {
        name: "strong",
        own: NONE,
    },
    Element {
        name: "sub",
        own: NONE,
    },
    Element {
        name: "summary",
        own: NONE,
    },
    Element {
        name: "sup",
        own: NONE,
    },
    Element {
        name: "table",
        own: NONE,
    },
    Element {
        name: "tbody",
        own: NONE,
    },
    Element {
        name: "td",
        own: CELL,
    },
    Element {
        name: "tfoot",
        own: NONE,
    },
    // HTML gives a header cell an `abbr` too; EPUB's validator does not.
    Element {
        name: "th",
        own: &["colspan", "rowspan", "headers", "scope"],
    },
    Element {
        name: "thead",
        own: NONE,
    },
    Element {
        name: "time",
        own: &["datetime"],
    },
    Element {
        name: "tr",
        own: NONE,
    },
    Element {
        name: "track",
        own: &["default", "kind", "label", "src", "srclang"],
    },
    Element {
        name: "u",
        own: NONE,
    },
    Element {
        name: "ul",
        own: NONE,
    },
    Element {
        name: "var",
        own: NONE,
    },
    Element {
        name: "video",
        own: VIDEO,
    },
    Element {
        name: "wbr",
        own: NONE,
    },
    Element {
        name: "center",
        own: NONE,
    },
    Element {
        name: "strike",
        own: NONE,
    },
];

/// Whether HTML defines `attr` (in lower case) on `element` (in lower
/// case). An element this table does not know is not judged: its
/// attributes are taken as defined.
pub(super) fn defined(element: &str, attr: &str) -> bool {
    if GLOBAL.contains(&attr) || PREFIXES.iter().any(|prefix| attr.starts_with(prefix)) {
        return true;
    }
    ELEMENTS
        .iter()
        .find(|known| known.name == element)
        .is_none_or(|known| known.own.contains(&attr))
}

#[cfg(test)]
mod tests {
    use super::{ELEMENTS, defined};

    /// An attribute stands where HTML defines it, and nowhere else.
    ///
    /// # Panics
    ///
    /// When a global attribute, a `data-` or `aria-` one, `xml:lang` or
    /// `epub:type`, or an element's own (`href` on a link, `alt` on an
    /// image, `colspan` on a cell) is not defined; when a made-up name
    /// (`b`, `hre`) or one of another element (`href` on a `div`,
    /// `colspan` on a `p`) is; or when an element the table does not know
    /// is judged.
    #[test]
    fn an_attribute_stands_where_html_defines_it() {
        for (element, attr) in [
            ("div", "id"),
            ("span", "class"),
            ("p", "lang"),
            ("em", "data-x"),
            ("nav", "aria-label"),
            ("p", "xml:lang"),
            ("a", "epub:type"),
            ("a", "href"),
            ("img", "alt"),
            ("td", "colspan"),
            ("input", "checked"),
            ("svg", "viewbox"),
        ] {
            assert!(defined(element, attr), "{attr} on {element}");
        }
        for (element, attr) in [
            ("a", "b"),
            ("a", "hre"),
            ("div", "href"),
            ("p", "colspan"),
            ("img", "onerror"),
        ] {
            assert!(!defined(element, attr), "{attr} on {element}");
        }
    }

    /// The table names each element once, in lower case.
    ///
    /// # Panics
    ///
    /// When an element is in the table twice, or its name or one of its
    /// attributes is not in lower case.
    #[test]
    fn each_element_is_named_once() {
        for (at, element) in ELEMENTS.iter().enumerate() {
            assert!(
                ELEMENTS
                    .iter()
                    .skip(at + 1)
                    .all(|other| other.name != element.name),
                "{} twice",
                element.name
            );
            assert_eq!(element.name, element.name.to_ascii_lowercase());
            for attr in element.own {
                assert_eq!(*attr, attr.to_ascii_lowercase(), "{}", element.name);
            }
        }
    }
}
