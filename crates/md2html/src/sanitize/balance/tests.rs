// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The balance of the author's tags, on the cases that matter.
//!
//! Markup kept as written, stray end tags, open elements, crossed tags,
//! what HTML ends without an end tag, and md2html's own elements kept
//! whole.

use super::{Balance, Change, Mend, Own};

/// The author's HTML `pieces`, each in a scope of its own when it is a
/// list (`[...]`) and written as it stands otherwise, balanced; and what
/// was mended.
fn balanced(pieces: &[&str]) -> (String, Vec<Mend>) {
    let mut balance = Balance::new();
    let mut out = String::new();
    let top = balance.enter(Own::NONE);
    for (line, piece) in (1..).zip(pieces) {
        match piece
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            Some(inside) => {
                let floor = balance.enter(Own::NONE);
                out.push('[');
                balance.write(inside, line, &mut out);
                balance.leave(floor, &mut out);
                out.push(']');
            }
            None => balance.write(piece, line, &mut out),
        }
    }
    balance.leave(top, &mut out);
    (out, balance.take_mends())
}

/// The balanced HTML alone.
fn html(pieces: &[&str]) -> String {
    balanced(pieces).0
}

/// Balanced markup is written as it stands, across pieces too.
///
/// # Panics
///
/// When anything of markup that is already balanced is changed or
/// mended: nested elements, a disclosure whose ends are pieces apart,
/// void elements, a drawing's self-closing shapes, text with `&lt;`.
#[test]
fn balanced_markup_is_kept_as_written() {
    for pieces in [
        &["<div class=\"a\"><p>x</p></div>"][..],
        &[
            "<details open><summary>More</summary>",
            "<p>inside</p>",
            "</details>",
        ],
        &["a<br>b<hr><img src=\"x\" alt=\"y\"><wbr>c"],
        &["<svg viewBox=\"0 0 1 1\"><circle r=\"1\"/><path d=\"M0\"></path></svg>"],
        &["1 &lt; 2 <b>bold</b> 3 > 2"],
    ] {
        let (out, mends) = balanced(pieces);
        assert_eq!(out, pieces.concat(), "{pieces:?}");
        assert!(mends.is_empty(), "{mends:?} for {pieces:?}");
    }
}

/// An end tag that closes nothing is left out, wherever it stands, and
/// the author is told on its line.
///
/// # Panics
///
/// When a stray `</div>`, `</details>` or `</p>` is written, at the top
/// or in a scope whose element was opened outside it, or when the
/// author is not told of each, on its line.
#[test]
fn an_end_tag_that_closes_nothing_is_left_out() {
    assert_eq!(html(&["a</div>b</details>c</p>"]), "abc", "at the top");
    assert_eq!(
        html(&["<div>", "[x</div>y]", "</div>"]),
        "<div>[xy]</div>",
        "the scope cannot close what was opened outside it"
    );
    let (_, mends) = balanced(&["</div>", "x", "</p>"]);
    assert_eq!(
        mends,
        [
            Mend {
                change: Change::LeftOut,
                name: "div",
                line: 1
            },
            Mend {
                change: Change::LeftOut,
                name: "p",
                line: 3
            }
        ],
        "told on its line"
    );
}

/// What the author leaves open is closed at the end of its scope, and
/// the author told of it, except where HTML allows the end to be left
/// out.
///
/// # Panics
///
/// When an open `div` or `details` is not closed at the end, or at the
/// end of the scope it was opened in, innermost first; when the author
/// is not told of them on the line they were opened on; or when a
/// paragraph or a list item left open is told.
#[test]
fn what_is_left_open_is_closed_at_the_end_of_its_scope() {
    assert_eq!(
        html(&["<div>a", "<details><summary>s</summary>b"]),
        "<div>a<details><summary>s</summary>b</details></div>"
    );
    assert_eq!(html(&["[<div>a]", "b"]), "[<div>a</div>]b");
    let (_, mends) = balanced(&["<div>", "<details>", "<p>", "<ul><li>"]);
    assert_eq!(mends, [
        Mend {
            change: Change::Closed,
            name: "ul",
            line: 4
        },
        Mend {
            change: Change::Closed,
            name: "details",
            line: 2
        },
        Mend {
            change: Change::Closed,
            name: "div",
            line: 1
        },
    ]);
}

/// An end tag closes its element and everything opened after it; the
/// end tags after that close nothing.
///
/// # Panics
///
/// When `<div><p></div></p>`, `<div><span></div>` or `<b><i></b></i>`
/// does not become well nested, closing the later element first, or
/// when the `span` the author did not close is not told.
#[test]
fn crossed_tags_are_nested() {
    assert_eq!(html(&["<div><p>a</div>b</p>"]), "<div><p>a</p></div>b");
    assert_eq!(html(&["<b><i>a</b>b</i>"]), "<b><i>a</i></b>b");
    let (out, mends) = balanced(&["<div><span>a</div>"]);
    assert_eq!(out, "<div><span>a</span></div>");
    assert_eq!(mends, [Mend {
        change: Change::Closed,
        name: "span",
        line: 1
    }]);
}

/// A start tag ends what HTML lets the author leave open: a paragraph
/// before a block, a list item before the next, a cell before the next.
///
/// # Panics
///
/// When `<p>a<p>b`, `<p>a<div>`, `<li>` after `<li>`, `<dt>`/`<dd>`, or
/// cells and rows without their end tags nest inside one another instead
/// of following one another; when a list item in a nested list ends the
/// outer one, or a paragraph in a cell ends one outside the table; or
/// when any of it is told to the author.
#[test]
fn a_start_tag_ends_what_html_lets_be_left_open() {
    for (pieces, out) in [
        (&["<p>a<p>b"][..], "<p>a</p><p>b</p>"),
        (&["<p>a<div>c</div>"], "<p>a</p><div>c</div>"),
        (&["<ul><li>a<li>b</ul>"], "<ul><li>a</li><li>b</li></ul>"),
        (
            &["<ul><li>a<ul><li>b</ul></ul>"],
            "<ul><li>a<ul><li>b</li></ul></li></ul>",
        ),
        (
            &["<dl><dt>a<dd>b<dt>c</dl>"],
            "<dl><dt>a</dt><dd>b</dd><dt>c</dt></dl>",
        ),
        (
            &["<table><tr><td>a<td>b<tr><td>c</table>"],
            "<table><tr><td>a</td><td>b</td></tr><tr><td>c</td></tr></table>",
        ),
        (
            &["<p>a<table><tr><td><p>b<div>c</div></table>"],
            "<p>a</p><table><tr><td><p>b</p><div>c</div></td></tr></table>",
        ),
    ] {
        let (balanced_out, mends) = balanced(pieces);
        assert_eq!(balanced_out, out, "{pieces:?}");
        assert!(mends.is_empty(), "{mends:?} for {pieces:?}");
    }
}

/// md2html's own block ends an open paragraph of the author's, and
/// nothing else.
///
/// # Panics
///
/// When a `<p>` the author left open is not ended before md2html's
/// `<ul>` or `<p>`, is ended before md2html's emphasis, or when anything
/// but a paragraph is ended.
#[test]
fn an_own_block_ends_the_authors_paragraph() {
    let mut balance = Balance::new();
    let mut out = String::new();
    balance.write("<div><p>a", 1, &mut out);
    balance.own_start("em", &mut out);
    assert_eq!(out, "<div><p>a", "inline markup ends nothing");
    balance.own_start("ul", &mut out);
    assert_eq!(out, "<div><p>a</p>", "a block ends the paragraph");
    balance.own_start("p", &mut out);
    assert_eq!(out, "<div><p>a</p>", "and nothing more");
}

/// A void element and a drawing's shape close themselves; any other
/// element written to close itself is an empty one.
///
/// # Panics
///
/// When `<br>`, `<br/>` or `<img .../>` opens anything, when `<div/>` or
/// `<span class="x"/>` is not written as an empty element, or when a
/// shape inside `svg` or `math` is.
#[test]
fn void_and_self_closing_tags_open_nothing() {
    assert_eq!(
        html(&["<br><br/><img src=\"a\"/>x</br>"]),
        "<br><br/><img src=\"a\"/>x"
    );
    assert_eq!(
        html(&["<div/>a<span class=\"x\"/>b"]),
        "<div></div>a<span class=\"x\"></span>b"
    );
    assert_eq!(
        html(&["<svg><rect/></svg><math><mspace/></math>"]),
        "<svg><rect/></svg><math><mspace/></math>"
    );
}

/// An HTML start tag leaves a drawing or a formula as HTML's parser
/// leaves it, so what the author writes after it is balanced as HTML.
///
/// # Panics
///
/// When `<b/>`, `<p>` or `<div/>` inside an `svg` does not close the
/// drawing before it (the parser would have closed it there and kept the
/// self-closed HTML element open over the rest of the page), when an
/// element written to close itself after it is not written as an empty
/// one, or when HTML inside an SVG `title` or a `MathML` `mi`, where HTML
/// is let in, closes the drawing or the formula.
#[test]
fn an_html_tag_leaves_a_drawing_as_the_parser_does() {
    assert_eq!(
        html(&["<svg><b class=\"leak\"/> after"]),
        "<svg></svg><b class=\"leak\"></b> after"
    );
    assert_eq!(
        html(&["<svg><g><p></p><a href=\"https://e.org/\"/>x"]),
        "<svg><g></g></svg><p></p><a href=\"https://e.org/\"></a>x"
    );
    assert_eq!(
        html(&["<svg><p>x</p><div class=\"hide\"/>rest"]),
        "<svg></svg><p>x</p><div class=\"hide\"></div>rest"
    );
    assert_eq!(
        html(&["<svg><title><b>t</b></title><rect/></svg><math><mi><b>i</b></mi></math>"]),
        "<svg><title><b>t</b></title><rect/></svg><math><mi><b>i</b></mi></math>"
    );
}

/// A tag no list of the sanitizer names is not written; written as
/// written, nothing is balanced.
///
/// # Panics
///
/// When `<script>` or a comment passes the balanced record, or when the
/// record for the `CommonMark` examples changes anything.
#[test]
fn only_known_elements_pass_and_as_written_changes_nothing() {
    assert_eq!(html(&["1<script>2</script><!-- 3 -->4"]), "124");
    let mut balance = Balance::as_written();
    let mut out = String::new();
    let floor = balance.enter(Own::PARAGRAPH);
    balance.write("</div><div><p>", 1, &mut out);
    balance.leave(floor, &mut out);
    assert_eq!(out, "</div><div><p>");
}

/// Finding the element an end tag closes takes no walk down the stack.
///
/// # Panics
///
/// When 50,000 open `div`s and 50,000 stray `</span>`s, then as many
/// `</div>`s, do not come out balanced: every `</span>` left out,
/// every `div` closed once.
#[test]
fn many_open_elements_stay_cheap() {
    let text = format!(
        "{}{}{}",
        "<div>".repeat(50_000),
        "</span>".repeat(50_000),
        "</div>".repeat(50_000)
    );
    let (out, mends) = balanced(&[&text]);
    assert_eq!(
        out,
        format!("{}{}", "<div>".repeat(50_000), "</div>".repeat(50_000))
    );
    assert_eq!(mends.len(), 50_000, "each stray end tag told");
}

/// The author's HTML `html` written inside md2html's element `own` (in the
/// article), balanced; and the names of the start tags left out because
/// they would end an element they stand in.
fn inside(own: Own, html: &str) -> (String, Vec<&'static str>) {
    let mut balance = Balance::new();
    let mut out = String::new();
    let article = balance.enter(Own::ARTICLE);
    let floor = balance.enter(own);
    balance.write(html, 1, &mut out);
    balance.leave(floor, &mut out);
    balance.leave(article, &mut out);
    let refused = balance
        .take_mends()
        .into_iter()
        .filter(|mend| matches!(mend.change, Change::WouldEnd(_) | Change::Outside(_)))
        .map(|mend| mend.name)
        .collect();
    (out, refused)
}

/// A start tag that would end md2html's paragraph is left out with its end
/// tag, its contents kept; one that may stand in a paragraph is kept.
///
/// # Panics
///
/// When a block, a paragraph, a list, a table, a rule or a disclosure in
/// md2html's paragraph is written, its contents are lost, or it is not
/// told; or when emphasis or a span is changed.
#[test]
fn a_block_does_not_end_md2htmls_paragraph() {
    let (out, refused) = inside(
        Own::PARAGRAPH,
        "a<div>b</div>c<p>d</p>e<ul><li>f</li></ul>g<hr>h<details><summary>i</summary>j</details>",
    );
    assert_eq!(out, "abcdefghij");
    assert_eq!(refused, [
        "div", "p", "ul", "li", "hr", "details", "summary"
    ]);
    let (kept, none) = inside(Own::PARAGRAPH, "<em>a</em><span class=\"x\">b</span><br>");
    assert_eq!(kept, "<em>a</em><span class=\"x\">b</span><br>");
    assert!(none.is_empty(), "{none:?}");
}

/// A start tag left out ends with its own end tag, and an end tag of its
/// closes what was opened inside it, as for a tag written.
///
/// # Panics
///
/// When nested `div`s left out do not each take their end tag, or a span
/// opened inside one is not closed at its end tag.
#[test]
fn a_tag_left_out_takes_its_end_tag() {
    let (out, refused) = inside(Own::PARAGRAPH, "<div><div>a</div>b</div>c</div>");
    assert_eq!(out, "abc");
    assert_eq!(refused, ["div", "div"]);
    let mut balance = Balance::new();
    let mut written = String::new();
    let floor = balance.enter(Own::PARAGRAPH);
    balance.write("<div>x<span>y</div>z", 3, &mut written);
    balance.leave(floor, &mut written);
    assert_eq!(written, "x<span>y</span>z");
    let mends = balance.take_mends();
    assert!(
        mends.contains(&Mend {
            change: Change::Closed,
            name: "span",
            line: 3
        }),
        "{mends:?}"
    );
    assert!(
        !mends.iter().any(|mend| mend.change == Change::LeftOut),
        "{mends:?}"
    );
}

/// md2html's list item is ended by no list item of the author's, up to
/// the author's list or another special element.
///
/// # Panics
///
/// When an author's `li` straight in md2html's item, or inside a `div`
/// there, is written; or when one in the author's own list, quote or
/// definition list is changed.
#[test]
fn an_item_does_not_end_md2htmls_item() {
    assert_eq!(inside(Own::ITEM, "a<li>b</li>c").0, "abc");
    assert_eq!(inside(Own::ITEM, "<div><li>b</li></div>").0, "<div>b</div>");
    for kept in [
        "<ul><li>a</li><li>b</li></ul>",
        "<blockquote><li>a</li></blockquote>",
        "<dl><dt>a</dt><dd>b</dd></dl>",
    ] {
        assert_eq!(inside(Own::ITEM, kept), (kept.to_owned(), vec![]), "{kept}");
    }
}

/// md2html's table cell is ended by no part of a table of the author's,
/// but the author's own table in it stands.
///
/// # Panics
///
/// When a row, a cell, a head cell, a body, a caption or a column in
/// md2html's cell is written, or a table of the author's in it changed.
#[test]
fn a_table_part_does_not_end_md2htmls_cell() {
    for own in [Own::CELL, Own::HEAD_CELL] {
        let (out, refused) = inside(own, "a<tr>b<td>c<th>d<tbody>e<caption>f</caption><col>g");
        assert_eq!(out, "abcdefg");
        assert_eq!(refused, ["tr", "td", "th", "tbody", "caption", "col"]);
    }
    let table = "<table><tbody><tr><td>a</td><td>b</td></tr></tbody></table>";
    assert_eq!(inside(Own::CELL, table).0, table);
}

/// md2html's link is ended by no link of the author's, and its heading by
/// no heading written straight in it.
///
/// # Panics
///
/// When an author's `a` in md2html's link, in any depth of the author's
/// inline elements, is written; when a heading straight in md2html's
/// heading is, or one inside the author's span is not.
#[test]
fn a_link_or_a_heading_does_not_end_md2htmls() {
    assert_eq!(
        inside(Own::LINK, "a<a href=\"x\">b</a>c<b><a href=\"y\">d</a></b>").0,
        "abc<b>d</b>"
    );
    let level = crate::model::Level::new(3).unwrap();
    assert_eq!(inside(Own::heading(level), "a<h4>b</h4>c").0, "abc");
    assert_eq!(
        inside(Own::heading(level), "<span><h4>b</h4></span>").0,
        "<span><h4>b</h4></span>"
    );
}

/// A part of a table outside a table is left out, as HTML's parser drops
/// it; inside the author's table it stands.
///
/// # Panics
///
/// When a row or a cell outside a table is written, or one inside the
/// author's table is not.
#[test]
fn a_table_part_outside_a_table_is_left_out() {
    let (out, refused) = inside(Own::DIV, "<tr><td>a</td></tr>b");
    assert_eq!(out, "ab");
    assert_eq!(refused, ["tr", "td"]);
    let table = "<table><tr><td>a</td></tr></table>";
    assert_eq!(inside(Own::DIV, table).0, table);
}

/// Ruby text ends what HTML ends before it when a ruby is open: the
/// author's ruby text, never md2html's paragraph.
///
/// # Panics
///
/// When ruby text inside the author's ruby does not end the one before
/// it, or ruby text in md2html's paragraph under the author's ruby is
/// written (the ruby, closed with none, is given an empty one).
#[test]
fn ruby_text_ends_ruby_text_only() {
    assert_eq!(
        inside(Own::DIV, "<ruby>a<rt>b<rt>c</ruby>").0,
        "<ruby>a<rt>b</rt><rt>c</rt></ruby>"
    );
    let mut balance = Balance::new();
    let mut out = String::new();
    let top = balance.enter(Own::ARTICLE);
    balance.write("<ruby>", 1, &mut out);
    let paragraph = balance.enter(Own::PARAGRAPH);
    balance.write("a<rt>b</rt>c", 2, &mut out);
    balance.leave(paragraph, &mut out);
    balance.leave(top, &mut out);
    assert_eq!(out, "<ruby>abc<rt></rt></ruby>");
}

/// XHTML lets ruby text stand only in a ruby, and a ruby only with ruby
/// text: ruby text outside a ruby is left out, and a ruby md2html has to
/// close before any ruby text is given an empty one.
///
/// # Panics
///
/// When `rt` or `rp` outside a ruby is written, or not told; when a ruby
/// md2html closes with no ruby text is not given an empty one, or one
/// that holds some is given another; or when the author's own `</ruby>`
/// is changed.
#[test]
fn ruby_text_stands_in_a_ruby_and_a_ruby_holds_some() {
    let (out, refused) = inside(Own::PARAGRAPH, "a<rt>b</rt><rp>(</rp>c");
    assert_eq!(out, "ab(c");
    assert_eq!(refused, ["rt", "rp"]);
    let told = Mend {
        change: Change::Outside("ruby"),
        name: "rt",
        line: 1,
    };
    assert_eq!(
        told.message(),
        "`<rt>` stands in no ruby; it is left out, its contents kept"
    );
    assert_eq!(
        inside(Own::PARAGRAPH, "<ruby>a").0,
        "<ruby>a<rt></rt></ruby>"
    );
    assert_eq!(
        inside(Own::PARAGRAPH, "<ruby>a<rt>b").0,
        "<ruby>a<rt>b</rt></ruby>"
    );
    assert_eq!(inside(Own::PARAGRAPH, "<ruby>a</ruby>").0, "<ruby>a</ruby>");
}

/// md2html's own start tag first closes what of the author's cannot hold
/// it: a drawing, a table outside its cells, a paragraph before a block,
/// a heading before a heading.
///
/// # Panics
///
/// When md2html's emphasis is written inside the author's open `svg` or
/// `table`, its paragraph inside the author's `p`, its heading inside the
/// author's heading; or when its emphasis is kept out of the author's
/// cell or an SVG `title`, where HTML stands.
#[test]
fn md2htmls_tags_stand_where_html_holds_them() {
    for (open, own, closed) in [
        ("<svg><g>", "em", "<svg><g></g></svg>"),
        ("<table><tr>", "em", "<table><tr></tr></table>"),
        ("<div><p>", "p", "<div><p></p>"),
        ("<h3>", "h2", "<h3></h3>"),
        ("<table><tr><td>", "em", "<table><tr><td>"),
        ("<svg><title>", "em", "<svg><title>"),
    ] {
        let mut balance = Balance::new();
        let mut out = String::new();
        let top = balance.enter(Own::ARTICLE);
        balance.write(open, 1, &mut out);
        balance.own_start(own, &mut out);
        assert_eq!(out, closed, "{open} then md2html's {own}");
        balance.leave(top, &mut out);
    }
}

/// md2html writes no markup of its own inside the author's drawing, table
/// outside a cell, or link, nor inside its own link.
///
/// # Panics
///
/// When markup is taken in the author's `svg`, `table`, `tr` or `a`, or in
/// md2html's link; or when it is refused in a paragraph, the author's cell
/// or an SVG `title`.
#[test]
fn md2html_adds_no_markup_where_it_would_not_stand() {
    for (own, open, takes) in [
        (Own::PARAGRAPH, "", true),
        (Own::PARAGRAPH, "<svg>", false),
        (Own::PARAGRAPH, "<svg><title>", true),
        (Own::DIV, "<table>", false),
        (Own::DIV, "<table><tr>", false),
        (Own::DIV, "<table><tr><td>", true),
        (Own::PARAGRAPH, "<a href=\"x\">", false),
        (Own::PARAGRAPH, "<a href=\"x\"><b>", false),
        (Own::LINK, "", false),
    ] {
        let mut balance = Balance::new();
        let mut out = String::new();
        let floor = balance.enter(own);
        balance.write(open, 1, &mut out);
        assert_eq!(balance.takes_markup(), takes, "{own:?} {open}");
        balance.leave(floor, &mut out);
    }
}

/// Tags left out cost no walk: many of them, and their end tags, stay
/// linear.
///
/// # Panics
///
/// When 50,000 `div`s left out in md2html's paragraph and their 50,000 end
/// tags leave anything but the text, or a stray end tag is told.
#[test]
fn many_tags_left_out_stay_cheap() {
    let text = format!("{}x{}", "<div>".repeat(50_000), "</div>".repeat(50_000));
    let mut balance = Balance::new();
    let mut out = String::new();
    let floor = balance.enter(Own::PARAGRAPH);
    balance.write(&text, 1, &mut out);
    balance.leave(floor, &mut out);
    assert_eq!(out, "x");
    let mends = balance.take_mends();
    assert_eq!(mends.len(), 50_000, "each start tag told once");
    assert!(
        mends
            .iter()
            .all(|mend| mend.change == Change::WouldEnd("p"))
    );
}

/// Before a block of its own, md2html closes the author's elements that
/// hold phrasing only, past the transparent ones, and tells each.
///
/// A block may still stand in the author's link, division or cell, a
/// heading in a summary, and md2html's emphasis in the author's `span`.
///
/// # Panics
///
/// When md2html's paragraph, division or list is written inside the
/// author's open `ruby`, `span`, `em` (through an `a`) or `summary`; when
/// it closes the author's `a`, `div` or cell, a `summary` before a
/// heading, or a `span` before its emphasis; or when an element closed is
/// not told, or one kept is.
#[test]
fn md2htmls_block_closes_the_authors_phrasing() {
    for (open, own, closed, told) in [
        ("<ruby>", "p", "<ruby><rt></rt></ruby>", &["ruby"][..]),
        ("<ruby>a<rt>", "p", "<ruby>a<rt></rt></ruby>", &["ruby"]),
        ("<div><span><em>", "div", "<div><span><em></em></span>", &[
            "em", "span",
        ]),
        (
            "<em><a href=\"x\">",
            "ul",
            "<em><a href=\"x\"></a></em>",
            &["a", "em"],
        ),
        ("<a href=\"x\">", "p", "<a href=\"x\">", &[]),
        (
            "<table><tr><td><span>",
            "p",
            "<table><tr><td><span></span>",
            &["span"],
        ),
        ("<details><summary>", "h2", "<details><summary>", &[]),
        (
            "<details><summary>",
            "p",
            "<details><summary></summary>",
            &["summary"],
        ),
        ("<span>", "em", "<span>", &[]),
    ] {
        let mut balance = Balance::new();
        let mut out = String::new();
        let top = balance.enter(Own::ARTICLE);
        balance.write(open, 1, &mut out);
        balance.own_start(own, &mut out);
        assert_eq!(out, closed, "{open} then md2html's {own}");
        let named: Vec<&str> = balance
            .take_mends()
            .iter()
            .filter(|mend| mend.change == Change::Closed)
            .map(|mend| mend.name)
            .collect();
        assert_eq!(named, told, "{open} then md2html's {own}");
        balance.leave(top, &mut out);
    }
}
