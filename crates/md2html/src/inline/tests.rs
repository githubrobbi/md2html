// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The inline parser on the cases that have gone wrong before.

use super::emphasis::{Flank, Run, Tok, emphasis};
use super::{Env, HtmlMode, Node, merge, parse, plain};
use crate::model::{Defs, Image, Link};
use crate::scan::Scan;

/// The nodes of `text` on a page with no definitions.
fn nodes(text: &str) -> Vec<Node> {
    let defs = Defs::default();
    parse(text, &Env {
        defs: &defs,
        html: HtmlMode::Safe,
        math: true,
    })
}

/// Square brackets that make no link stay as they were written.
///
/// # Panics
///
/// Panics when `Mark [V] and [M] here` does not come back word for word, or
/// when any part of it becomes a node other than text, such as a link or a
/// footnote reference.
#[test]
fn bare_brackets_stay_text() {
    let got = nodes("Mark [V] and [M] here");
    assert_eq!(plain(&got), "Mark [V] and [M] here");
    assert!(got.iter().all(|node| matches!(node, Node::Text(_))));
}

/// A link's address may hold balanced parentheses.
///
/// # Panics
///
/// Panics when the link to `https://en.wikipedia.org/wiki/Rust_(language)`
/// loses its closing `)`, for instance when the first `)` is taken as the end
/// of the address.
#[test]
fn a_url_may_hold_parentheses() {
    let got = nodes("[x](https://en.wikipedia.org/wiki/Rust_(language))");
    assert!(
        matches!(got.first(), Some(Node::Link(link)) if link.href == "https://en.wikipedia.org/wiki/Rust_(language)"),
        "{got:?}"
    );
}

/// Three stars on each side make strong text inside emphasis.
///
/// # Panics
///
/// Panics when `***both***` is not emphasis holding strong text, for instance
/// when it is read the other way round or as literal stars.
#[test]
fn bold_italic_nests() {
    let got = nodes("***both***");
    assert!(
        matches!(got.first(), Some(Node::Em(kids)) if matches!(kids.first(), Some(Node::Strong(_)))),
        "{got:?}"
    );
}

/// A dollar sign before a number does not open maths.
///
/// # Panics
///
/// Panics when `from $5 to $10, and $x^2$` does not give exactly one formula:
/// none when `$x^2$` is lost, more when `$5 to $` is read as maths.
#[test]
fn money_is_not_maths() {
    let got = nodes("from $5 to $10, and $x^2$");
    assert_eq!(
        got.iter()
            .filter(|node| matches!(node, Node::Math(_)))
            .count(),
        1
    );
}

/// A bare URL becomes a link that leaves the full stop after it out.
///
/// # Panics
///
/// Panics when `https://example.com/a.` is not the second node, a link to
/// `https://example.com/a` without the full stop.
#[test]
fn bare_urls_link_without_their_full_stop() {
    let got = nodes("See https://example.com/a. Done");
    assert!(
        matches!(got.get(1), Some(Node::Link(link)) if link.href == "https://example.com/a"),
        "{got:?}"
    );
}

/// Brackets that never close stay text in place, and emphasis around them still
/// works.
///
/// # Panics
///
/// Panics when `a [b *c [d] e* f` does not keep its words and brackets in their
/// order, or when `*c [d] e*` is not made emphasis.
#[test]
fn brackets_that_never_close_keep_their_order() {
    let got = nodes("a [b *c [d] e* f");
    assert_eq!(plain(&got), "a [b c [d] e f");
    assert!(
        got.iter().any(|node| matches!(node, Node::Em(_))),
        "{got:?}"
    );
}

/// The link and tag readers give what they read and the text after it.
///
/// # Panics
///
/// Panics when `<a b>` is not read as the destination `a b` with ` rest` after
/// it, when `'it\'s'` is not read as the title `it's` with ` rest` after it,
/// when `<a href='x'>` is not read as a tag up to its `>`, or when a tag whose
/// quoted value never closes is read as a tag.
#[test]
fn readers_give_the_text_after_what_they_read() {
    let mut scan = Scan::new("<a b> rest");
    assert_eq!(
        super::link_destination(&mut scan),
        Some(String::from("a b"))
    );
    assert_eq!(scan.rest(), " rest");
    let mut title = Scan::new("'it\\'s' rest");
    assert_eq!(super::link_title(&mut title), Some(String::from("it's")));
    assert_eq!(title.rest(), " rest");
    assert_eq!(
        crate::sanitize::html_tag("<a href='x'>text"),
        Some("<a href='x'>")
    );
    assert_eq!(crate::sanitize::html_tag("<a href='x>text"), None);
}

/// A backslash in a link destination escapes only ASCII punctuation.
///
/// # Panics
///
/// Panics when `\\` before a space or a line break is read as escaping it,
/// so that the destination runs on past the space or onto the next line, or
/// when `\\)` does not keep its `)` in the destination.
#[test]
fn a_backslash_escapes_only_punctuation_in_a_destination() {
    let mut spaced = Scan::new("/u\\ b");
    assert_eq!(
        super::link_destination(&mut spaced),
        Some(String::from("/u\\"))
    );
    assert_eq!(spaced.rest(), " b");
    let mut broken = Scan::new("/u\\\nb");
    assert_eq!(
        super::link_destination(&mut broken),
        Some(String::from("/u\\"))
    );
    assert_eq!(broken.rest(), "\nb");
    assert_eq!(super::link_destination(&mut Scan::new("<a\\\nb>")), None);
    assert_eq!(
        super::link_destination(&mut Scan::new("/u\\)b")),
        Some(String::from("/u)b"))
    );
}

/// The tokens of `text`, written with `*`, `_`, `~` and other characters,
/// as the inline pass makes them.
fn runs_of(text: &str) -> Vec<Tok> {
    let chars: Vec<char> = text.chars().collect();
    let mut toks = Vec::new();
    let mut before = ' ';
    let mut rest = chars.as_slice();
    while let Some((&first, _)) = rest.split_first() {
        let same = rest.iter().take_while(|ch| **ch == first).count();
        let (run, after_run) = rest.split_at(same);
        let after = after_run.first().copied().unwrap_or(' ');
        let is_mark = matches!(first, '*' | '_') || (first == '~' && same == 2);
        if is_mark {
            toks.push(Tok::Run(Run {
                mark: first,
                len: same,
                flank: Flank::of(first, before, after),
            }));
        } else {
            toks.push(Tok::Node(Node::Text(run.iter().collect())));
        }
        before = first;
        rest = after_run;
    }
    toks
}

/// A token of the reference pass.
enum Plain {
    /// A finished node.
    Node(Node),
    /// A run, with how many marks it has left.
    Run(Run, usize),
}

/// The item as a node: a run left over is its marks.
fn plain_node(item: Plain) -> Node {
    match item {
        Plain::Node(node) => node,
        Plain::Run(run, left) => Node::Text(core::iter::repeat_n(run.mark, left).collect()),
    }
}

/// Whether `opener` (with `opener_left` marks) and `closer` (with
/// `closer_left`) make emphasis: `CommonMark`'s rules as the
/// specification states them, the rule of three included.
fn spec_pairs(opener: &Run, opener_left: usize, closer: &Run, closer_left: usize) -> bool {
    let may_open = matches!(opener.flank, Flank::Opens | Flank::Both);
    let either_both = opener.flank == Flank::Both || closer.flank == Flank::Both;
    let three = either_both
        && (opener.len + closer.len) % 3 == 0
        && (opener.len % 3 != 0 || closer.len % 3 != 0);
    let tilde_short = closer.mark == '~' && (opener_left < 2 || closer_left < 2);
    may_open && opener.mark == closer.mark && !three && !tilde_short
}

/// "Process emphasis" with no `openers_bottom`: every closer searches the
/// whole stack.
///
/// # Panics
///
/// Panics when the opener a closer found is not on top of the stack once what
/// lies above it is split off; `split_off` just after the opener's place keeps
/// it there.
fn spec_emphasis(toks: Vec<Tok>) -> Vec<Node> {
    let mut items: Vec<Plain> = Vec::new();
    for tok in toks {
        let run = match tok {
            Tok::Node(node) => {
                items.push(Plain::Node(node));
                continue;
            }
            Tok::Run(run) => run,
        };
        let mut left = run.len;
        let closes = matches!(run.flank, Flank::Closes | Flank::Both);
        while closes && left > 0 {
            let Some(at) = items.iter().rposition(|item| {
                matches!(item, Plain::Run(opener, opener_left)
                    if spec_pairs(opener, *opener_left, &run, left))
            }) else {
                break;
            };
            let between = items.split_off(at + 1);
            let Some(Plain::Run(_, opener_left)) = items.last_mut() else {
                panic!("the opener is on top");
            };
            let strong = run.mark == '~' || (*opener_left >= 2 && left >= 2);
            let used = if strong { 2 } else { 1 };
            *opener_left -= used;
            left -= used;
            if *opener_left == 0 {
                items.pop();
            }
            let kids = merge(between.into_iter().map(plain_node).collect());
            items.push(Plain::Node(match (run.mark, strong) {
                ('~', _) => Node::Del(kids),
                (_, true) => Node::Strong(kids),
                (_, false) => Node::Em(kids),
            }));
        }
        if left > 0 {
            items.push(Plain::Run(run, left));
        }
    }
    merge(items.into_iter().map(plain_node).collect())
}

/// Every text of up to `longest` characters from `alphabet`.
fn every_text(alphabet: &[char], longest: u32) -> Vec<String> {
    let base = alphabet.len();
    let mut out = Vec::new();
    for length in 1..=longest {
        let count = base.pow(length);
        for mut code in 0..count {
            let mut text = String::new();
            for _ in 0..length {
                text.push(alphabet.get(code % base).copied().unwrap_or(' '));
                code /= base;
            }
            out.push(text);
        }
    }
    out
}

/// The emphasis pass, with its search floors, gives what the specification's
/// plain algorithm gives.
///
/// # Panics
///
/// Panics when, for one of the hand-picked texts or any text of up to eight
/// characters from `*`, `_`, `a` and space (six from `~`, `*`, `a` and space),
/// the emphasis pass pairs delimiters differently from the specification's
/// algorithm without floors: a floor that skipped an opener that should have
/// closed.
#[test]
fn the_floors_hide_no_opener() {
    // The first ones remove the delimiter a floor was set at (`_` fails,
    // then `*` closes around it); the sweep covers every short shape.
    let mut texts: Vec<String> = [
        "*a b_ c* d_",
        "*a b_ c* _d e_",
        "_a *b c_ d* e*",
        "**a _b c** d_ e_",
        "*a **b*** c_ d_ *e*",
        "~~a *b~~ c* d~~ e~~",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    texts.extend(every_text(&['*', '_', 'a', ' '], 8));
    texts.extend(every_text(&['~', '*', 'a', ' '], 6));
    for text in &texts {
        let ours = format!("{:?}", merge(emphasis(runs_of(text))));
        let spec = format!("{:?}", spec_emphasis(runs_of(text)));
        assert_eq!(ours, spec, "{text:?}");
    }
}

/// The bytes `nodes` hold, `room` slots of them: the slots, the text each
/// node owns, and the nodes inside them with their own room.
fn held(nodes: &[Node], room: usize) -> usize {
    let inside: usize = nodes
        .iter()
        .map(|node| match node {
            Node::Text(text) | Node::Code(text) | Node::Html(text) | Node::Note(text) => {
                text.capacity()
            }
            Node::Em(kids) | Node::Strong(kids) | Node::Del(kids) => held(kids, kids.capacity()),
            Node::Link(link) => {
                size_of::<Link>()
                    + link.href.capacity()
                    + link.title.as_ref().map_or(0, String::capacity)
                    + held(&link.kids, link.kids.capacity())
            }
            Node::Image(image) => {
                size_of::<Image>() + image.src.capacity() + held(&image.alt, image.alt.capacity())
            }
            Node::Math(math) => math.tex.capacity(),
            Node::HardBreak => 0,
        })
        .sum();
    room * size_of::<Node>() + inside
}

/// A text of many short lines holds memory for its text, not a node for
/// each line, and a node is small; measured by size, at two sizes.
///
/// # Panics
///
/// When a node takes more than 32 bytes; when a paragraph of one-letter
/// lines is read into more than one node, or its nodes hold more than
/// twice its bytes; or when lines of a code span, emphasis or a link each
/// hold more than their bound per byte of text, at 1,000 lines or at
/// 100,000.
#[test]
fn many_short_lines_hold_memory_for_their_text() {
    assert!(size_of::<Node>() <= 32, "{}", size_of::<Node>());
    for lines in [1_000, 100_000] {
        let letters = "a\n".repeat(lines);
        let one = nodes(&letters);
        assert_eq!(one.len(), 1, "{lines} lines: one text node");
        let one_holds = held(&one, one.capacity());
        assert!(
            one_holds <= 2 * letters.len(),
            "{lines} lines: {one_holds} bytes"
        );
        // A node a line is the least these can cost: 18, 28 and 27 bytes
        // a byte of text, whatever the number of lines.
        for (line, per_byte) in [("`a`\n", 20), ("*a*\n", 30), ("[a](b)\n", 30)] {
            let text = line.repeat(lines);
            let got = nodes(&text);
            let bytes = held(&got, got.capacity());
            assert!(
                bytes <= per_byte * text.len(),
                "{line:?} x {lines}: {bytes} bytes for {}",
                text.len()
            );
        }
    }
}
