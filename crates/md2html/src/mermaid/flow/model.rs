// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A flowchart as read: nodes, the edges between them, the subgraphs that
//! hold them.

use crate::mermaid::geom::{Len, px};
use crate::mermaid::label::Label;
use crate::mermaid::table::{Id, Table};
use crate::mermaid::title::Caption;

/// The size of a node's words.
pub(super) const NODE_FONT: Len = px(14.0_f64);
/// The size of the words on an edge.
pub(super) const EDGE_FONT: Len = px(13.0_f64);
/// The size of a subgraph's title.
pub(super) const TITLE_FONT: Len = px(13.0_f64);

/// A flowchart's direction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::mermaid) enum Dir {
    /// Top to bottom.
    TopDown,
    /// Bottom to top.
    BottomUp,
    /// Left to right.
    LeftRight,
    /// Right to left.
    RightLeft,
}

impl Dir {
    /// A direction from its keyword (`TD`, `LR`, ...).
    pub(in crate::mermaid) fn parse(word: &str) -> Self {
        match word.trim_end_matches(';').to_ascii_uppercase().as_str() {
            "BT" => Self::BottomUp,
            "LR" => Self::LeftRight,
            "RL" => Self::RightLeft,
            _ => Self::TopDown,
        }
    }

    /// Whether ranks run across the page (left to right or back).
    pub(super) const fn horizontal(self) -> bool {
        matches!(self, Self::LeftRight | Self::RightLeft)
    }
}

/// The number of a node.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(in crate::mermaid) struct NodeId(usize);

impl Id for NodeId {
    fn at(index: usize) -> Self {
        Self(index)
    }
    fn index(self) -> usize {
        self.0
    }
}

/// The number of an edge.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) struct EdgeId(usize);

impl Id for EdgeId {
    fn at(index: usize) -> Self {
        Self(index)
    }
    fn index(self) -> usize {
        self.0
    }
}

/// The number of a subgraph.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) struct ClusterId(usize);

impl Id for ClusterId {
    fn at(index: usize) -> Self {
        Self(index)
    }
    fn index(self) -> usize {
        self.0
    }
}

/// A node's outline.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Shape {
    /// `[text]`
    Rect,
    /// `(text)`
    Round,
    /// `([text])`
    Stadium,
    /// `[[text]]`
    Subroutine,
    /// `[(text)]`
    Cylinder,
    /// `((text))`
    Circle,
    /// `(((text)))`
    DoubleCircle,
    /// `{text}`
    Diamond,
    /// `{{text}}`
    Hexagon,
    /// `>text]`
    Flag,
    /// `[/text/]`
    Lean,
    /// `[\text\]`
    LeanBack,
    /// `[/text\]`
    Trapezoid,
    /// `[\text/]`
    TrapezoidDown,
}

/// A node.
#[derive(Clone, Debug)]
pub(in crate::mermaid) struct Node {
    /// Its id in the source.
    pub(in crate::mermaid) id: String,
    /// Its label.
    pub(super) label: Label,
    /// Its outline.
    pub(super) shape: Shape,
    /// The subgraphs holding it, outermost first.
    pub(super) within: Vec<ClusterId>,
}

/// How an edge is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Stroke {
    /// `--`
    Solid,
    /// `-.-`
    Dotted,
    /// `==`
    Thick,
    /// `~~~`, laid out but not drawn.
    Invisible,
}

/// An end of an edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Head {
    /// None.
    None,
    /// `>`
    Arrow,
    /// `o`
    Circle,
    /// `x`
    Cross,
}

/// An edge.
#[derive(Clone, Debug)]
pub(super) struct Edge {
    /// From this node.
    pub(super) from: NodeId,
    /// To this node.
    pub(super) to: NodeId,
    /// Its label.
    pub(super) label: Option<Label>,
    /// Its line.
    pub(super) stroke: Stroke,
    /// At the start.
    pub(super) tail: Head,
    /// At the end.
    pub(super) head: Head,
    /// Ranks it spans at least (`--->` is two).
    pub(super) minlen: usize,
}

impl Edge {
    /// Whether it leads from a node back to the same node.
    pub(super) fn is_loop(&self) -> bool {
        self.from == self.to
    }
}

/// A subgraph.
#[derive(Clone, Debug)]
pub(super) struct Cluster {
    /// Its title.
    pub(super) title: Label,
    /// The subgraphs holding it, outermost first, and itself last.
    pub(super) within: Vec<ClusterId>,
}

impl Cluster {
    /// The subgraph it sits in.
    pub(super) fn parent(&self) -> Option<ClusterId> {
        self.within.iter().rev().nth(1).copied()
    }
}

/// A flowchart as read.
#[derive(Debug)]
pub(in crate::mermaid) struct Graph {
    /// Which way the graph flows.
    pub(super) dir: Dir,
    /// The nodes, in the order they were first named.
    pub(in crate::mermaid) nodes: Table<NodeId, Node>,
    /// The edges between them.
    pub(super) edges: Table<EdgeId, Edge>,
    /// The subgraphs.
    pub(super) clusters: Table<ClusterId, Cluster>,
    /// The author's title, shown under it: its front matter's `title`.
    pub(super) caption: Option<Caption>,
    /// The author's name for it for readers who cannot see it
    /// (`accTitle:`), if given.
    pub(super) acc_title: Option<String>,
    /// The author's description, if given.
    pub(super) desc: Option<String>,
}

impl Graph {
    /// How many edges it has.
    pub(in crate::mermaid) fn edge_count(&self) -> usize {
        self.edges.len()
    }
}
