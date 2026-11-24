// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The lists the filter decides by: what may stand, what goes with its
//! contents, which attributes never pass, which carry a URL, and the ids
//! the page keeps for itself.

/// Elements that may stand in the article. Not `main`: the page has its
/// own, and HTML allows one.
pub(super) const ELEMENTS: &[&str] = &[
    // text and structure
    "a",
    "abbr",
    "address",
    "article",
    "aside",
    "b",
    "bdi",
    "bdo",
    "blockquote",
    "br",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "dfn",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "i",
    "ins",
    "kbd",
    "li",
    "mark",
    "nav",
    "ol",
    "p",
    "pre",
    "q",
    "rp",
    "rt",
    "ruby",
    "s",
    "samp",
    "section",
    "small",
    "span",
    "strike",
    "strong",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "time",
    "tr",
    "u",
    "ul",
    "var",
    "wbr",
    // media
    "audio",
    "img",
    "picture",
    "source",
    "track",
    "video",
    // SVG drawing (no script, no foreignObject, no animation that could set a
    // link)
    "svg",
    "g",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "text",
    "tspan",
    "textpath",
    "defs",
    "use",
    "symbol",
    "marker",
    "lineargradient",
    "radialgradient",
    "stop",
    "clippath",
    "mask",
    "pattern",
    "title",
    "desc",
    "image",
    // MathML
    "math",
    "mi",
    "mo",
    "mn",
    "ms",
    "mtext",
    "mrow",
    "msup",
    "msub",
    "msubsup",
    "mfrac",
    "msqrt",
    "mroot",
    "mtable",
    "mtr",
    "mtd",
    "mover",
    "munder",
    "munderover",
    "mspace",
    "mstyle",
    "semantics",
    "annotation",
];

/// Elements that hold nothing: written without an end tag, never open.
pub(super) const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements dropped together with everything inside them.
pub(super) const DROP_WITH_CONTENT: &[&str] = &[
    "script",
    "style",
    "iframe",
    "frame",
    "frameset",
    "object",
    "embed",
    "applet",
    "template",
    "noscript",
    "noembed",
    "noframes",
    "textarea",
    "xmp",
    "plaintext",
    "select",
    "option",
    "foreignobject",
    "head",
];

/// Attributes that never pass, whatever the element.
pub(super) const DENIED: &[&str] = &[
    "srcdoc",
    "formaction",
    "action",
    "ping",
    "background",
    "lowsrc",
    "dynsrc",
    "is",
    "nonce",
    "http-equiv",
    "popover",
    "popovertarget",
    "popovertargetaction",
    "command",
    "commandfor",
    "autofocus",
    "attributename",
    "xlink:actuate",
    "xml:base",
];

/// The start tags that end a drawing or a formula in HTML's parser.
///
/// They are WHATWG HTML's ("rules for parsing tokens in foreign content"):
/// inside SVG or `MathML` each one closes what is open of it, unless it
/// stands where HTML is let in ([`LETS_HTML_IN`]).
pub(super) const BREAKS_OUT: &[&str] = &[
    "b",
    "big",
    "blockquote",
    "br",
    "center",
    "code",
    "dd",
    "div",
    "dl",
    "dt",
    "em",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "i",
    "img",
    "li",
    "menu",
    "ol",
    "p",
    "pre",
    "ruby",
    "s",
    "small",
    "span",
    "strike",
    "strong",
    "sub",
    "sup",
    "table",
    "tt",
    "u",
    "ul",
    "var",
];

/// The elements of a drawing or a formula that let HTML in: SVG's
/// `title` and `desc` (HTML integration points) and `MathML`'s token
/// elements (text integration points).
pub(super) const LETS_HTML_IN: &[&str] = &["title", "desc", "mi", "mo", "mn", "ms", "mtext"];

/// The only attribute names with a prefix that may stand.
///
/// Their prefix is one XML binds itself (`xml:`) or SVG uses by its fixed
/// name (`xlink:`), each with a meaning the filter knows; `xlink:href` is
/// checked as a URL.
pub(super) const PREFIXED: &[&str] = &["xlink:href", "xlink:title", "xml:lang", "xml:space"];

/// Attributes that carry a URL.
pub(super) const URL_ATTRS: &[&str] = &[
    "href",
    "src",
    "xlink:href",
    "poster",
    "cite",
    "longdesc",
    "codebase",
    "data",
    "srcset",
    "usemap",
];

/// The ids the page's own script and layout use: an author's element
/// must not take one.
pub(super) const RESERVED_IDS: &[&str] = &[
    "top",
    "main",
    "toc",
    "find",
    "q",
    "hits",
    "hits-n",
    "idx",
    "md2html-ui",
    "bar",
    "lang",
    "langs",
    "prefs",
    "aa",
    "print",
    "size",
    "size-up",
    "size-down",
    "toggle",
    "edge",
    "dock",
    "crumb",
    "open-find",
    "theme",
    "glossary",
    "notes",
    "zoom",
    "save",
    "keep",
    "toast",
    "passage",
];

/// What an inline style may not hold, once its white space is squeezed
/// out and it is lowercased: each could fetch or run something.
pub(super) const STYLE_DENIED: &[&str] = &[
    "expression",
    "javascript:",
    "vbscript:",
    "url(",
    "@import",
    "behavior",
    "-moz-binding",
    "\\",
    "/*",
    "image-set(",
    "src(",
];

/// The attributes a `meta` or `link` of the `head` file keeps.
pub(super) const HEAD_ATTRS: &[&str] = &[
    "name",
    "content",
    "property",
    "charset",
    "rel",
    "href",
    "type",
    "media",
    "sizes",
    "hreflang",
    "title",
    "crossorigin",
    "itemprop",
];

/// The relations a `link` of the `head` file may name: none of them
/// loads a script.
pub(super) const LINK_RELS: &[&str] = &[
    "stylesheet",
    "icon",
    "apple-touch-icon",
    "alternate",
    "canonical",
    "author",
    "license",
    "preconnect",
    "dns-prefetch",
];
