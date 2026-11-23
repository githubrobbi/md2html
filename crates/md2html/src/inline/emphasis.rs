// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! `CommonMark`'s "process emphasis", rule of three included, as a stack.
//!
//! The tokens are read left to right onto a stack. A run that may close
//! takes items off the stack down to the nearest run it pairs with; what
//! lay between becomes the new node, which goes back on the stack. A run
//! with nothing to pair with stays on the stack, an opener for later
//! closers if it may open, text otherwise. Per kind of closer, the height
//! below which no opener was found is kept (`openers_bottom`), so no
//! closer looks below it again: the pass is linear in the tokens.

use super::{MAX_NEST, depth, merge};
use crate::model::Node;

/// A token of the inline pass.
pub(super) enum Tok {
    /// A finished node.
    Node(Node),
    /// A run of `*`, `_` or `~`, its part not known yet.
    Run(Run),
}

/// A run of `*`, `_` or `~`, as written.
#[derive(Clone, Copy)]
pub(super) struct Run {
    /// The mark.
    pub(super) mark: char,
    /// How many there are.
    pub(super) len: usize,
    /// Which way it may work.
    pub(super) flank: Flank,
}

/// Whether a run may open emphasis, close it, or both.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Flank {
    /// It may only open.
    Opens,
    /// It may only close.
    Closes,
    /// It may do either (the rule of three looks at these).
    Both,
    /// It may do neither: it is text.
    Neither,
}

impl Flank {
    /// What a run of `mark` may do between the characters `before` and
    /// `after`: `CommonMark`'s left- and right-flanking rules, with the
    /// stricter ones for `_`.
    pub(super) fn of(mark: char, before: char, after: char) -> Self {
        let left = !after.is_whitespace()
            && (!is_punct(after) || before.is_whitespace() || is_punct(before));
        let right = !before.is_whitespace()
            && (!is_punct(before) || after.is_whitespace() || is_punct(after));
        let (opens, closes) = if mark == '_' {
            (
                left && (!right || is_punct(before)),
                right && (!left || is_punct(after)),
            )
        } else {
            (left, right)
        };
        match (opens, closes) {
            (true, true) => Self::Both,
            (true, false) => Self::Opens,
            (false, true) => Self::Closes,
            (false, false) => Self::Neither,
        }
    }

    /// Whether the run may open emphasis.
    const fn opens(self) -> bool {
        matches!(self, Self::Opens | Self::Both)
    }

    /// Whether the run may close emphasis.
    const fn closes(self) -> bool {
        matches!(self, Self::Closes | Self::Both)
    }
}

/// Whether a character counts as punctuation for emphasis (`CommonMark`
/// 0.31 counts symbols too).
fn is_punct(ch: char) -> bool {
    ch.is_ascii_punctuation() || (!ch.is_alphanumeric() && !ch.is_whitespace() && !ch.is_ascii())
}

/// A run on the stack, with how many of its marks are not used yet:
/// never none.
struct Delim {
    /// The run as written.
    run: Run,
    /// How many marks are left.
    left: usize,
}

impl Delim {
    /// Whether this opener and `closer` make emphasis together.
    fn pairs_with(&self, closer: &Self) -> bool {
        let (opener, closing) = (self.run, closer.run);
        let three = (opener.flank == Flank::Both || closing.flank == Flank::Both)
            && (opener.len + closing.len) % 3 == 0
            && !(opener.len % 3 == 0 && closing.len % 3 == 0);
        let tilde = closing.mark == '~' && (self.left < 2 || closer.left < 2);
        opener.flank.opens() && opener.mark == closing.mark && !three && !tilde
    }
}

/// A finished node on the stack, with how deep it nests.
struct Nested {
    /// The node.
    node: Node,
    /// Its [`depth`].
    depth: usize,
}

/// One item of the stack.
enum Item {
    /// A finished node.
    Node(Nested),
    /// A run, an opener if it may open.
    Run(Delim),
}

impl Item {
    /// The item as a node: a run left over is its marks, as text.
    fn into_node(self) -> Node {
        match self {
            Self::Node(nested) => nested.node,
            Self::Run(delim) => {
                Node::Text(core::iter::repeat_n(delim.run.mark, delim.left).collect())
            }
        }
    }
}

/// What the closers of one kind have in common for the search: the mark,
/// whether the closer could also open, and its length modulo three.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Kind {
    /// The mark.
    mark: char,
    /// The closer may also open.
    both: bool,
    /// Its length, modulo three.
    rest: usize,
}

impl Kind {
    /// The kind of `closer`.
    const fn of(closer: &Delim) -> Self {
        Self {
            mark: closer.run.mark,
            both: matches!(closer.run.flank, Flank::Both),
            rest: closer.run.len % 3,
        }
    }
}

/// An opener taken off the stack, with what lay above it.
struct Pair {
    /// The opener.
    opener: Delim,
    /// The items between it and the closer, in order.
    between: Vec<Item>,
    /// How deep the deepest of them nests.
    deepest: usize,
}

/// The stack, and how far down each kind of closer still has to look.
///
/// Why a floor hides no opener a search without floors would find: a
/// floor of `h` for a kind says that no item below height `h` pairs with
/// a closer of that kind (closely enough for [`MAX_NEST`]).
/// - It is true when set: the failed search looked at every item down to the
///   old floor, below which it held already. A search that stopped at an opener
///   nested too deep stopped above every other one, and what lies between only
///   grows, so those nest too deep as well.
/// - It stays true. Whether an item pairs with a closer of a kind depends on
///   what the kind fixes (mark, may-also-open, length modulo three), on the
///   item's run as written, and on how many marks it has left; a closer's own
///   count matters only for `~`, whose closers always search with both marks.
///   Marks left only shrink, and fewer never make a pair that more did not. And
///   an item below `h` is still the one that was there when the floor was set:
///   the stack loses items below `h` only when a closer takes them, and
///   [`Stack::lower_floors`] then brings every floor down to the new height.
///
/// The test `the_floors_hide_no_opener` in `inline/tests.rs` compares the
/// pass with one that searches the whole stack every time.
#[derive(Default)]
struct Stack {
    /// The items, last on top.
    items: Vec<Item>,
    /// Per kind of closer: below this height no opener for it was found.
    floors: Vec<(Kind, usize)>,
}

impl Stack {
    /// How far down a closer of `kind` looks.
    fn floor(&self, kind: Kind) -> usize {
        self.floors
            .iter()
            .find(|(known, _)| *known == kind)
            .map_or(0, |(_, height)| *height)
    }

    /// Closers of `kind` need not look below `height` again.
    fn raise_floor(&mut self, kind: Kind, height: usize) {
        match self.floors.iter_mut().find(|(known, _)| *known == kind) {
            Some(floor) => floor.1 = height,
            None => self.floors.push((kind, height)),
        }
    }

    /// The stack was cut down to its present height: no floor stands above
    /// it, so what is put on it from here on is looked at.
    fn lower_floors(&mut self) {
        let height = self.items.len();
        for floor in &mut self.floors {
            floor.1 = floor.1.min(height);
        }
    }

    /// Takes the items down to the nearest opener `closer` pairs with, not
    /// below `floor`. When there is none, or what lies between already
    /// nests as deep as it may, everything goes back and the answer is
    /// None.
    fn take_opener(&mut self, closer: &Delim, floor: usize) -> Option<Pair> {
        let mut above = Vec::new();
        let mut deepest = 0;
        let mut found = None;
        while let Some(item) = self.items.pop() {
            if self.items.len() < floor {
                self.items.push(item);
                break;
            }
            match item {
                Item::Run(delim) if delim.pairs_with(closer) => {
                    if deepest < MAX_NEST {
                        found = Some(delim);
                    } else {
                        self.items.push(Item::Run(delim));
                    }
                    break;
                }
                Item::Node(nested) => {
                    deepest = deepest.max(nested.depth);
                    above.push(Item::Node(nested));
                }
                Item::Run(delim) => above.push(Item::Run(delim)),
            }
        }
        above.reverse();
        let Some(opener) = found else {
            self.items.extend(above);
            return None;
        };
        Some(Pair {
            opener,
            between: above,
            deepest,
        })
    }

    /// Puts a finished node on the stack.
    fn push_node(&mut self, node: Node) {
        let nests = depth(&node);
        self.items.push(Item::Node(Nested { node, depth: nests }));
    }

    /// Puts a run on the stack, closing what it can first.
    fn push_run(&mut self, run: Run) {
        let mut closer = Delim { run, left: run.len };
        if run.flank.closes() {
            let kind = Kind::of(&closer);
            while closer.left > 0 {
                let Some(pair) = self.take_opener(&closer, self.floor(kind)) else {
                    self.raise_floor(kind, self.items.len());
                    break;
                };
                self.close(pair, &mut closer);
            }
        }
        if closer.left > 0 {
            self.items.push(Item::Run(closer));
        }
    }

    /// Emphasis from `pair` and `closer`: two marks of each make strong
    /// emphasis (and strike-through), one makes emphasis.
    fn close(&mut self, pair: Pair, closer: &mut Delim) {
        let Pair {
            mut opener,
            between,
            deepest,
        } = pair;
        let strong = closer.run.mark == '~' || (opener.left >= 2 && closer.left >= 2);
        let used = if strong { 2 } else { 1 };
        let kids = merge(between.into_iter().map(Item::into_node).collect());
        let node = match (closer.run.mark, strong) {
            ('~', _) => Node::Del(kids),
            (_, true) => Node::Strong(kids),
            (_, false) => Node::Em(kids),
        };
        opener.left = opener.left.saturating_sub(used);
        closer.left = closer.left.saturating_sub(used);
        if opener.left > 0 {
            self.items.push(Item::Run(opener));
        }
        self.lower_floors();
        self.items.push(Item::Node(Nested {
            node,
            depth: deepest + 1,
        }));
    }
}

/// Emphasis from the delimiter runs among `toks`; the runs left over are
/// text.
pub(super) fn emphasis<Toks>(toks: Toks) -> Vec<Node>
where
    Toks: IntoIterator<Item = Tok>,
{
    let mut stack = Stack::default();
    for tok in toks {
        match tok {
            Tok::Node(node) => stack.push_node(node),
            Tok::Run(run) => stack.push_run(run),
        }
    }
    stack.items.into_iter().map(Item::into_node).collect()
}
