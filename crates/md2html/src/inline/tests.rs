// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The inline parser on the cases that have gone wrong before.

use super::emphasis::{Flank, Run, Tok, emphasis};
use super::{Env, HtmlMode, Node, merge, parse, plain};
use crate::model::Defs;

/// The nodes of `text` on a page with no definitions.
fn nodes(text: &str) -> Vec<Node> {
    let defs = Defs::default();
    parse(text, &Env {
        defs: &defs,
        html: HtmlMode::Safe,
        math: true,
    })
}

#[test]
fn bare_brackets_stay_text() {
    let got = nodes("Mark [V] and [M] here");
    assert_eq!(plain(&got), "Mark [V] and [M] here");
    assert!(got.iter().all(|node| matches!(node, Node::Text(_))));
}

#[test]
fn a_url_may_hold_parentheses() {
    let got = nodes("[x](https://en.wikipedia.org/wiki/Rust_(language))");
    assert!(
        matches!(got.first(), Some(Node::Link(link)) if link.href == "https://en.wikipedia.org/wiki/Rust_(language)"),
        "{got:?}"
    );
}

#[test]
fn bold_italic_nests() {
    let got = nodes("***both***");
    assert!(
        matches!(got.first(), Some(Node::Em(kids)) if matches!(kids.first(), Some(Node::Strong(_)))),
        "{got:?}"
    );
}

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

#[test]
fn bare_urls_link_without_their_full_stop() {
    let got = nodes("See https://example.com/a. Done");
    assert!(
        matches!(got.get(1), Some(Node::Link(link)) if link.href == "https://example.com/a"),
        "{got:?}"
    );
}

#[test]
fn brackets_that_never_close_keep_their_order() {
    let got = nodes("a [b *c [d] e* f");
    assert_eq!(plain(&got), "a [b c [d] e f");
    assert!(
        got.iter().any(|node| matches!(node, Node::Em(_))),
        "{got:?}"
    );
}

#[test]
fn readers_give_the_text_after_what_they_read() {
    assert_eq!(
        super::link_destination("<a b> rest"),
        Some((String::from("a b"), " rest"))
    );
    assert_eq!(
        super::link_title("'it\\'s' rest"),
        Some((String::from("it's"), " rest"))
    );
    assert_eq!(super::html_tag("<a href='x'>text"), Some("<a href='x'>"));
    assert_eq!(super::html_tag("<a href='x>text"), None);
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
