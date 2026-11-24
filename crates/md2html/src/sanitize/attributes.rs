// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The attributes HTML defines, element by element: one table for the
//! page and the book.
//!
//! With `--html` an author's attribute stands only where HTML defines it:
//! for every element (the global attributes, `data-*`, `aria-*`, the XML
//! namespace's own), or for the element it stands on. An attribute HTML
//! has never had (`<a b="c">`) is left out of the page, where a browser
//! would pass over it, and of the book, whose XHTML is validated and
//! refuses it; the author is told once of each kind.
//!
//! The two read the table as their reader does ([`Reader`]): a browser
//! still honours HTML 4's presentational attributes (`align`, `bgcolor`,
//! `<a name>`), which EPUB's XHTML does not have; the book writes them as
//! its style sheet says them (`export`), or leaves them out. Being defined
//! makes an attribute no safer: the filter refuses what could run or
//! fetch anything first, defined or not. Inline SVG and `MathML` keep
//! theirs; this table does not judge their elements.

/// Whose rules an attribute is judged by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reader {
    /// A browser reading the page: HTML, HTML 4's presentational
    /// attributes among it.
    Browser,
    /// An EPUB reader: EPUB's XHTML, as its validator takes it.
    Epub,
}

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

/// The attributes HTML defines on every element that EPUB's validator
/// does not take; a page keeps them.
const GLOBAL_PAGE: [&str; 6] = [
    "autocorrect",
    "exportparts",
    "inert",
    "part",
    "slot",
    "writingsuggestions",
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
    /// Its own attributes, in a page and in a book.
    own: &'static [&'static str],
    /// Its own attributes a browser honours and EPUB's XHTML does not
    /// have: HTML 4's presentational ones, and a few HTML has that EPUB's
    /// validator does not take.
    page: &'static [&'static str],
}

impl Element {
    /// An element with its own attributes, the same in a page and a book.
    const fn of(name: &'static str, own: &'static [&'static str]) -> Self {
        Self {
            name,
            own,
            page: NONE,
        }
    }

    /// The element with the attributes a page keeps beyond those.
    const fn and_page(self, page: &'static [&'static str]) -> Self {
        Self { page, ..self }
    }

    /// Whether `attr` is its own, for `reader`.
    fn has(&self, reader: Reader, attr: &str) -> bool {
        self.own.contains(&attr) || (reader == Reader::Browser && self.page.contains(&attr))
    }
}

/// What an element that takes only the global attributes has of its own.
const NONE: &[&str] = &[];

/// HTML 4's alignment of a block's text.
const ALIGN: &[&str] = &["align"];

/// HTML 4's of a table's row group: how its cells align.
const SECTION: &[&str] = &["align", "valign"];

/// HTML 4's of a table's column.
const COLUMN: &[&str] = &["align", "valign", "width"];

/// HTML 4's of a table's cell.
const OLD_CELL: &[&str] = &["align", "bgcolor", "height", "nowrap", "valign", "width"];

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

/// Every element the page's and the book's article may hold outside SVG
/// and `MathML`, with its own attributes.
///
/// They are those the filter lets stand and those md2html writes; HTML
/// 4's `center` and `strike` last, which the book writes as a current
/// element. The page's `a` keeps SVG's `xlink:` link too: the filter
/// reads one tag at a time and cannot tell a drawing's link from a
/// page's.
const ELEMENTS: [Element; 79] = [
    Element::of("a", LINK).and_page(&["name", "xlink:href", "xlink:title"]),
    Element::of("abbr", NONE),
    Element::of("address", NONE),
    Element::of("article", NONE),
    Element::of("aside", NONE),
    Element::of("audio", MEDIA),
    Element::of("b", NONE),
    Element::of("bdi", NONE),
    Element::of("bdo", NONE),
    Element::of("blockquote", &["cite"]),
    Element::of("br", NONE).and_page(&["clear"]),
    Element::of("button", CONTROL),
    Element::of("caption", NONE).and_page(ALIGN),
    Element::of("cite", NONE),
    Element::of("code", NONE),
    Element::of("col", &["span"]).and_page(COLUMN),
    Element::of("colgroup", &["span"]).and_page(COLUMN),
    Element::of("dd", NONE),
    Element::of("del", CITED),
    Element::of("details", &["open", "name"]),
    Element::of("dfn", NONE),
    Element::of("div", NONE).and_page(ALIGN),
    Element::of("dl", NONE).and_page(&["compact"]),
    Element::of("dt", NONE),
    Element::of("em", NONE),
    Element::of("figcaption", NONE),
    Element::of("figure", NONE),
    Element::of("footer", NONE),
    Element::of("h1", NONE).and_page(ALIGN),
    Element::of("h2", NONE).and_page(ALIGN),
    Element::of("h3", NONE).and_page(ALIGN),
    Element::of("h4", NONE).and_page(ALIGN),
    Element::of("h5", NONE).and_page(ALIGN),
    Element::of("h6", NONE).and_page(ALIGN),
    Element::of("header", NONE),
    Element::of("hr", NONE).and_page(&["align", "color", "noshade", "size", "width"]),
    Element::of("i", NONE),
    Element::of("img", IMAGE).and_page(&["align", "border", "hspace", "vspace"]),
    Element::of("input", CONTROL),
    Element::of("ins", CITED),
    Element::of("kbd", NONE),
    Element::of("li", &["value"]).and_page(&["type"]),
    Element::of("mark", NONE),
    Element::of("nav", NONE),
    Element::of("ol", &["reversed", "start", "type"]).and_page(&["compact"]),
    Element::of("p", NONE).and_page(ALIGN),
    Element::of("picture", NONE),
    Element::of("pre", NONE).and_page(&["width"]),
    Element::of("q", &["cite"]),
    Element::of("rp", NONE),
    Element::of("rt", NONE),
    Element::of("rtc", NONE),
    Element::of("ruby", NONE),
    Element::of("s", NONE),
    Element::of("samp", NONE),
    Element::of("section", NONE),
    Element::of("small", NONE),
    Element::of("source", &[
        "type", "media", "src", "srcset", "sizes", "width", "height",
    ]),
    Element::of("span", NONE),
    Element::of("strong", NONE),
    Element::of("sub", NONE),
    Element::of("summary", NONE),
    Element::of("sup", NONE),
    Element::of("table", NONE).and_page(&[
        "align",
        "bgcolor",
        "border",
        "cellpadding",
        "cellspacing",
        "frame",
        "height",
        "rules",
        "width",
    ]),
    Element::of("tbody", NONE).and_page(SECTION),
    Element::of("td", CELL).and_page(OLD_CELL),
    Element::of("tfoot", NONE).and_page(SECTION),
    // HTML gives a header cell an `abbr` too; EPUB's validator does not.
    Element::of("th", &["colspan", "rowspan", "headers", "scope"]).and_page(&[
        "abbr", "align", "bgcolor", "height", "nowrap", "valign", "width",
    ]),
    Element::of("thead", NONE).and_page(SECTION),
    Element::of("time", &["datetime"]),
    Element::of("tr", NONE).and_page(&["align", "bgcolor", "height", "valign"]),
    Element::of("track", &["default", "kind", "label", "src", "srclang"]),
    Element::of("u", NONE),
    Element::of("ul", NONE).and_page(&["compact", "type"]),
    Element::of("var", NONE),
    Element::of("video", VIDEO),
    Element::of("wbr", NONE),
    Element::of("center", NONE),
    Element::of("strike", NONE),
];

/// Whether HTML, as `reader` reads it, defines `attr` on `element`.
///
/// Both names are in lower case. An element this table does not know (a
/// drawing's, a formula's) is not judged: its attributes are taken as
/// defined.
pub(crate) fn defined(reader: Reader, element: &str, attr: &str) -> bool {
    if GLOBAL.contains(&attr)
        || PREFIXES.iter().any(|prefix| attr.starts_with(prefix))
        || (reader == Reader::Browser && GLOBAL_PAGE.contains(&attr))
    {
        return true;
    }
    ELEMENTS
        .iter()
        .find(|known| known.name == element)
        .is_none_or(|known| known.has(reader, attr))
}

#[cfg(test)]
mod tests {
    use super::{ELEMENTS, GLOBAL, GLOBAL_PAGE, Reader, defined};

    /// An attribute stands where HTML defines it, and nowhere else.
    ///
    /// # Panics
    ///
    /// When a global attribute, a `data-` or `aria-` one, `xml:lang` or
    /// `epub:type`, or an element's own (`href` on a link, `alt` on an
    /// image, `colspan` on a cell) is not defined for either reader; when a
    /// made-up name (`b`, `hre`) or one of another element (`href` on a
    /// `div`, `colspan` on a `p`, `target` on a `span`, `width` on a `div`)
    /// is defined for either; or when an element the table does not know
    /// is judged.
    #[test]
    fn an_attribute_stands_where_html_defines_it() {
        for reader in [Reader::Browser, Reader::Epub] {
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
                ("circle", "cx"),
            ] {
                assert!(defined(reader, element, attr), "{attr} on {element}");
            }
            for (element, attr) in [
                ("a", "b"),
                ("a", "hre"),
                ("div", "href"),
                ("p", "colspan"),
                ("img", "onerror"),
                ("span", "target"),
                ("div", "width"),
                ("span", "align"),
                ("p", "bgcolor"),
            ] {
                assert!(!defined(reader, element, attr), "{attr} on {element}");
            }
        }
    }

    /// A browser honours HTML 4's presentational attributes and a few
    /// others that EPUB's validator does not take; the book has none of
    /// them.
    ///
    /// # Panics
    ///
    /// When `align` on a paragraph, `name` on a link, `bgcolor` on a cell,
    /// `abbr` on a header cell, `clear` on a line break, `type` on a list
    /// or `inert` anywhere is not defined for a browser, or is for an EPUB
    /// reader.
    #[test]
    fn a_page_keeps_what_a_browser_honours_and_a_book_lacks() {
        for (element, attr) in [
            ("p", "align"),
            ("h1", "align"),
            ("a", "name"),
            ("a", "xlink:href"),
            ("table", "cellpadding"),
            ("td", "bgcolor"),
            ("th", "abbr"),
            ("br", "clear"),
            ("ul", "type"),
            ("img", "hspace"),
            ("div", "inert"),
        ] {
            assert!(
                defined(Reader::Browser, element, attr),
                "{attr} on {element} in a page"
            );
            assert!(
                !defined(Reader::Epub, element, attr),
                "{attr} on {element} in a book"
            );
        }
    }

    /// The table names each element once, in lower case, and each of its
    /// attributes once.
    ///
    /// # Panics
    ///
    /// When an element is in the table twice, its name or one of its
    /// attributes is not in lower case, or an attribute is in its page's
    /// list as well as its own or the global ones.
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
            for attr in element.own.iter().chain(element.page) {
                assert_eq!(*attr, attr.to_ascii_lowercase(), "{}", element.name);
            }
            for attr in element.page {
                assert!(
                    !element.own.contains(attr) && !GLOBAL.contains(attr),
                    "{attr} on {} twice",
                    element.name
                );
            }
        }
        for attr in GLOBAL_PAGE {
            assert!(!GLOBAL.contains(&attr), "{attr} twice");
        }
    }
}
