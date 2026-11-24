// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! HTML 4's presentational markup, rewritten for the book.

use super::modernize;

/// The chapter `xhtml` rewritten, with no problem reported.
///
/// # Panics
///
/// Panics when the rewrite reports anything left out.
fn clean(xhtml: &str) -> String {
    let (out, problems) = modernize(xhtml);
    assert_eq!(problems, Vec::<String>::new());
    out
}

/// `<center>` and `<strike>` become the elements XHTML has for them.
///
/// # Panics
///
/// Panics when `<center>` is not a `div` with the centring class (its end
/// tag too), or a `span` inside a paragraph; when an author's `div` inside
/// a heading is not a `span`, or one outside text changes; or when
/// `<strike>` is not an `s`.
#[test]
fn center_and_strike_become_current_elements() {
    assert_eq!(
        clean("<center><b>a</b></center><p><strike>b</strike></p>"),
        "<div class=\"align-center\"><b>a</b></div><p><s>b</s></p>"
    );
    // A `div` may not stand in a paragraph; a `span` may.
    assert_eq!(
        clean("<p>a <center>b <center>c</center></center> d</p><center>e</center>"),
        "<p>a <span class=\"align-center\">b <span class=\"align-center\">c</span></span> d</p><div class=\"align-center\">e</div>"
    );
    // The author's `div` in a heading too; one outside text stays as it is.
    assert_eq!(
        clean("<h1>a <div style=\"x:y\">b <div>c</div></div></h1><div id=\"d\">e</div>"),
        "<h1>a <span style=\"x:y\">b <span>c</span></span></h1><div id=\"d\">e</div>"
    );
}

/// `align` on a block becomes a class of the book's style sheet.
///
/// # Panics
///
/// Panics when a paragraph, a heading, a division or a cell keeps `align`,
/// gets a class other than the one for its side, or loses its own class,
/// or when a tag with nothing to change is written differently.
#[test]
fn align_on_a_block_becomes_a_class() {
    assert_eq!(
        clean(
            "<p align=\"center\">a</p><h1 id=\"t\" align=\"RIGHT\">b</h1><div class=\"x\" align=\"left\">c</div><td align=\"justify\">d</td><p id=\"q\">e</p>"
        ),
        "<p class=\"align-center\">a</p><h1 id=\"t\" class=\"align-right\">b</h1><div class=\"x align-left\">c</div><td class=\"align-justify\">d</td><p id=\"q\">e</p>"
    );
}

/// `align` on an image floats it or aligns it on the line.
///
/// # Panics
///
/// Panics when `left` or `right` is not a float class, when `middle` is
/// not the vertical class, or when the image's other attributes change.
#[test]
fn align_on_an_image_floats_it() {
    assert_eq!(
        clean(
            "<img src=\"a.png\" align=\"right\" alt=\"a\"/><img align=\"left\" src=\"b.png\"/><img src=\"c.png\" align=\"middle\"/>"
        ),
        "<img src=\"a.png\" alt=\"a\" class=\"float-right\"/><img src=\"b.png\" class=\"float-left\"/><img src=\"c.png\" class=\"valign-middle\"/>"
    );
}

/// A table's and an image's sizes, colours, spacing and margins become
/// styles, and `border` takes a value XHTML allows.
///
/// # Panics
///
/// Panics when a percentage or pixel width is not a style where XHTML has
/// no `width`, when an image's whole-pixel size is not kept, when a table
/// border is not `1` (or gone for `0`), or when `bgcolor`, `cellspacing`,
/// `nowrap`, `valign`, `hspace` and `vspace` are not their styles.
#[test]
fn sizes_colours_and_spacing_become_styles() {
    assert_eq!(
        clean(
            "<table width=\"100%\" border=\"2\" cellspacing=\"0\" align=\"center\" bgcolor=\"#eee\"><tr valign=\"top\">\
<td width=\"50\" height=\"20.5\" nowrap=\"nowrap\" style=\"color:red;\">a</td></tr></table><table border=\"0\"></table>\
<img src=\"a.png\" width=\"120\" height=\"80\" hspace=\"10\" vspace=\"5px\" border=\"2\"/><img src=\"b.png\" width=\"50%\"/>"
        ),
        "<table border=\"1\" class=\"box-center\" style=\"width:100%;border-spacing:0px;background-color:#eee\"><tr class=\"valign-top\">\
<td style=\"color:red;width:50px;height:20.5px;white-space:nowrap\">a</td></tr></table><table></table>\
<img src=\"a.png\" width=\"120\" height=\"80\" style=\"margin-left:10px;margin-right:10px;margin-top:5px;margin-bottom:5px;border:2px solid\"/>\
<img src=\"b.png\" style=\"width:50%\"/>"
    );
}

/// Lists, line breaks, rules and captions say with styles and classes what
/// their attributes said.
///
/// # Panics
///
/// Panics when a list's or an item's `type` is not its list style, when an
/// ordered list loses its own `type`, when `clear` is not a style, when a
/// rule's `align` is not its margin class, or when a caption's side is not
/// a style.
#[test]
fn lists_breaks_rules_and_captions() {
    assert_eq!(
        clean(
            "<ul type=\"square\"><li type=\"A\">a</li></ul><ol type=\"i\"><li>b</li></ol><br clear=\"all\"/><br clear=\"none\"/>\
<hr align=\"right\"/><caption align=\"bottom\">c</caption>"
        ),
        "<ul style=\"list-style-type:square\"><li style=\"list-style-type:upper-alpha\">a</li></ul><ol type=\"i\"><li>b</li></ol>\
<br style=\"clear:both\"/><br/><hr class=\"box-right\"/><caption style=\"caption-side:bottom\">c</caption>"
    );
}

/// What has no equivalent in a book is left out and named once per kind.
///
/// # Panics
///
/// Panics when `cellpadding`, `summary`, a rule's `size` and `noshade`, a
/// cell's `axis` or a preformatted block's `width` stays, when an `align`
/// with a value no side has stays, or when a kind is named other than
/// once.
#[test]
fn what_has_no_equivalent_is_left_out_once() {
    let (out, problems) = modernize(
        "<table cellpadding=\"4\" summary=\"s\"><tr><td axis=\"a\">x</td></tr></table><table cellpadding=\"2\"></table>\
<hr size=\"2\" noshade=\"noshade\"/><pre width=\"80\">p</pre><p align=\"sideways\">q</p>",
    );
    assert_eq!(
        out,
        "<table><tr><td>x</td></tr></table><table></table><hr/><pre>p</pre><p>q</p>"
    );
    assert_eq!(problems, [
        "epub: <table cellpadding> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <table summary> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <td axis> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <hr size> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <hr noshade> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <pre width> is left out: EPUB's XHTML has neither it nor an equivalent",
        "epub: <p align> is left out: EPUB's XHTML has neither it nor an equivalent",
    ]);
}

/// An attribute HTML does not define for its element is left out of the
/// book and named once; those it defines stay.
///
/// # Panics
///
/// When `b` or `hre` on a link, `href` on a `div` or `colspan` on a
/// paragraph stays, when a kind is named other than once, or when a
/// global, `data-`, `aria-` or `epub:` attribute, or an element's own,
/// is lost.
#[test]
fn an_attribute_html_does_not_define_is_left_out() {
    let (out, problems) = modernize(
        "<p colspan=\"2\" lang=\"en\" data-x=\"1\"><a b=\"c\" href=\"#x\" aria-label=\"l\">a</a><a hre=\"h\" b=\"d\">b</a></p>\
<div href=\"y\" id=\"x\">c</div><a epub:type=\"noteref\" href=\"#x\" title=\"t\">d</a><td colspan=\"2\" headers=\"x\">e</td>",
    );
    assert_eq!(
        out,
        "<p lang=\"en\" data-x=\"1\"><a href=\"#x\" aria-label=\"l\">a</a><a>b</a></p>\
<div id=\"x\">c</div><a epub:type=\"noteref\" href=\"#x\" title=\"t\">d</a><td colspan=\"2\" headers=\"x\">e</td>"
    );
    assert_eq!(problems, [
        "epub: <p colspan> is left out: EPUB's XHTML defines no such attribute for it",
        "epub: <a b> is left out: EPUB's XHTML defines no such attribute for it",
        "epub: <a hre> is left out: EPUB's XHTML defines no such attribute for it",
        "epub: <div href> is left out: EPUB's XHTML defines no such attribute for it",
    ]);
}

/// A value that could carry more CSS never reaches a style.
///
/// # Panics
///
/// Panics when a `width`, `bgcolor`, `hspace` or `cellspacing` holding
/// anything but a length or a colour is written into a style instead of
/// being left out.
#[test]
fn a_value_never_carries_more_css() {
    let (out, problems) = modernize(
        "<td width=\"1;position:fixed\" bgcolor=\"red;background:url(x)\" hspace=\"expression(1)\">a</td><table cellspacing=\"1.2.3\"></table>",
    );
    assert_eq!(out, "<td>a</td><table></table>");
    assert_eq!(problems.len(), 4);
}

/// Inline SVG and `MathML` keep their own attributes.
///
/// # Panics
///
/// Panics when an attribute inside an `svg` or a `math` element is
/// rewritten, or when markup after them is not.
#[test]
fn drawings_and_maths_keep_their_attributes() {
    assert_eq!(
        clean(
            "<svg width=\"10%\" align=\"x\"><svg width=\"5\"></svg><rect width=\"1.5\" align=\"left\"/></svg>\
<math><mtable align=\"top\"></mtable></math><p align=\"center\">a</p>"
        ),
        "<svg width=\"10%\" align=\"x\"><svg width=\"5\"></svg><rect width=\"1.5\" align=\"left\"/></svg>\
<math><mtable align=\"top\"></mtable></math><p class=\"align-center\">a</p>"
    );
}
