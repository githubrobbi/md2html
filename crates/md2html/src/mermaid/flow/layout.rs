// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The layered layout of a flowchart, in phases: cycles broken, nodes
//! ranked, long edges split by invisible waypoints, each rank ordered to cut
//! crossings, nodes placed toward the middle of their neighbours, subgraphs
//! framed, the whole fitted to the canvas, edges routed.
//!
//! Every phase looks vertices up by their ids; a lookup that finds nothing
//! ends the layout with nothing, and the diagram is then shown as code.

mod frame;
mod order;
mod place;
mod route;

use alloc::collections::{BTreeMap, BTreeSet};

pub(in crate::mermaid) use route::Route;

use super::model::{ClusterId, EDGE_FONT, EdgeId, Graph, NODE_FONT, Node, NodeId, Shape};
use crate::mermaid::geom::{Len, px};
use crate::mermaid::label::Label;
use crate::mermaid::plane::{Bounds, Rect, Size};
use crate::mermaid::table::{Id, Table};

/// A flowchart laid out.
pub(in crate::mermaid) struct Layout {
    /// Each node's box, in the order of the graph's nodes.
    pub(in crate::mermaid) nodes: Vec<Bounds>,
    /// Each edge's way, in the order of the graph's edges.
    pub(in crate::mermaid) edges: Vec<Route>,
    /// Each subgraph's frame; a subgraph without nodes has none.
    pub(in crate::mermaid) frames: BTreeMap<ClusterId, Rect>,
    /// The size of the drawing.
    pub(in crate::mermaid) size: Size,
}

/// The layered layout of a flowchart; none when a phase looks up a vertex
/// it does not hold.
pub(in crate::mermaid) fn layout(graph: &Graph) -> Option<Layout> {
    let courses = courses(graph)?;
    let mut layered = Layered::new(graph);
    layered.rank(&courses)?;
    layered.split(&courses)?;
    layered.order()?;
    layered.place()?;
    layered.finish()
}

/// The number of a vertex. The first vertices are the graph's nodes, in
/// their order, so a node's number is its vertex's.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct VertexId(usize);

impl Id for VertexId {
    fn at(index: usize) -> Self {
        Self(index)
    }
    fn index(self) -> usize {
        self.0
    }
}

impl From<NodeId> for VertexId {
    fn from(node: NodeId) -> Self {
        Self(node.index())
    }
}

/// What a vertex stands for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// A node of the graph.
    Node,
    /// The waypoint of a long edge that carries its label.
    Label,
    /// Any other waypoint of a long edge.
    Bend,
}

impl Kind {
    /// Whether it is a waypoint, not a node.
    const fn is_waypoint(self) -> bool {
        matches!(self, Self::Label | Self::Bend)
    }

    /// Whether it needs room beside it: a node or a label.
    const fn takes_room(self) -> bool {
        matches!(self, Self::Node | Self::Label)
    }
}

/// One vertex of the layout, and all the layout knows of it.
#[derive(Clone, Debug)]
struct Vertex<'graph> {
    /// What it stands for.
    kind: Kind,
    /// Its size on the page.
    size: Size,
    /// The subgraphs holding it, outermost first.
    within: &'graph [ClusterId],
    /// Its rank: how far along the flow.
    rank: usize,
    /// The vertices with an edge to it.
    preds: Vec<VertexId>,
    /// The vertices it has an edge to.
    succs: Vec<VertexId>,
    /// Its place in its rank, counted from the start.
    slot: usize,
    /// Its centre across the flow.
    across: Len,
    /// Its centre along the flow.
    along: Len,
}

impl<'graph> Vertex<'graph> {
    /// A vertex at `rank`, without edges, not yet placed.
    const fn new(kind: Kind, size: Size, within: &'graph [ClusterId], rank: usize) -> Self {
        Self {
            kind,
            size,
            within,
            rank,
            preds: Vec::new(),
            succs: Vec::new(),
            slot: 0,
            across: px(0.0_f64),
            along: px(0.0_f64),
        }
    }
}

/// An edge of the layout between two nodes, pointing along the flow.
struct Link<'graph> {
    /// The node it leaves.
    from: VertexId,
    /// The node it reaches.
    to: VertexId,
    /// The ranks it spans at least.
    minlen: usize,
    /// The edge's label.
    label: Option<&'graph Label>,
    /// Whether the edge of the graph points the other way.
    backward: bool,
}

/// How an edge of the graph takes part in the layout.
enum Course<'graph> {
    /// A loop from a node back to itself: drawn beside the node, not laid
    /// out.
    Loop,
    /// An edge between two nodes.
    Link(Link<'graph>),
}

impl<'graph> Course<'graph> {
    /// The edge between two nodes, if it is one.
    const fn link(&self) -> Option<&Link<'graph>> {
        match self {
            Self::Loop => None,
            Self::Link(link) => Some(link),
        }
    }
}

/// An edge's way through the ranks between its two nodes.
struct Chain {
    /// The waypoints, from the node it leaves to the node it reaches.
    waypoints: Vec<VertexId>,
    /// The waypoint that carries its label.
    label: Option<VertexId>,
}

/// An edge of the graph as laid out.
enum Way {
    /// A loop from a node back to itself.
    Loop,
    /// An edge between two nodes, through its waypoints.
    Chain(Chain),
}

/// A flowchart being laid out.
struct Layered<'graph> {
    /// The flowchart.
    graph: &'graph Graph,
    /// Whether ranks run across the page.
    horizontal: bool,
    /// The nodes, then the waypoints.
    vertices: Table<VertexId, Vertex<'graph>>,
    /// Each edge's way, in the order of the graph's edges.
    ways: Vec<Way>,
    /// The vertices of each rank, in order.
    layers: Vec<Vec<VertexId>>,
}

/// How far a node is visited in the search for cycles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Visit {
    /// Not yet.
    New,
    /// On the current path.
    Open,
    /// Done with.
    Done,
}

/// The edges that close a cycle, found depth first: an edge back to a node
/// on the current path is laid out reversed.
fn backward_edges(graph: &Graph) -> Option<BTreeSet<EdgeId>> {
    let leaving: Table<NodeId, Vec<(EdgeId, NodeId)>> = graph.nodes.map_ids(|node| {
        graph
            .edges
            .iter()
            .filter(|(_, edge)| edge.from == node)
            .map(|(id, edge)| (id, edge.to))
            .collect()
    });
    let mut visit = graph.nodes.map(|_| Visit::New);
    let mut backward = BTreeSet::new();
    for root in graph.nodes.ids() {
        let seen = visit.get_mut(root)?;
        if *seen != Visit::New {
            continue;
        }
        *seen = Visit::Open;
        let mut path = vec![(root, leaving.get(root)?.iter())];
        while let Some((node, edges)) = path.last_mut() {
            let current = *node;
            let Some(&(edge, next)) = edges.next() else {
                *visit.get_mut(current)? = Visit::Done;
                path.pop();
                continue;
            };
            if next == current {
                continue;
            }
            let state = visit.get_mut(next)?;
            match *state {
                Visit::New => {
                    *state = Visit::Open;
                    path.push((next, leaving.get(next)?.iter()));
                }
                Visit::Open => {
                    backward.insert(edge);
                }
                Visit::Done => {}
            }
        }
    }
    Some(backward)
}

/// How each edge takes part in the layout, cycles broken.
fn courses(graph: &Graph) -> Option<Vec<Course<'_>>> {
    let backward = backward_edges(graph)?;
    Some(
        graph
            .edges
            .iter()
            .map(|(id, edge)| {
                if edge.is_loop() {
                    return Course::Loop;
                }
                let reversed = backward.contains(&id);
                let (from, to) = if reversed {
                    (edge.to, edge.from)
                } else {
                    (edge.from, edge.to)
                };
                Course::Link(Link {
                    from: from.into(),
                    to: to.into(),
                    minlen: edge.minlen.max(if edge.label.is_some() { 2 } else { 1 }),
                    label: edge.label.as_ref(),
                    backward: reversed,
                })
            })
            .collect(),
    )
}

/// The size of a node for its label and shape.
fn node_size(node: &Node) -> Size {
    let Size {
        width: text_width,
        height: text_height,
    } = node.label.size(NODE_FONT);
    let (width, height) = (text_width + px(30.0_f64), text_height + px(20.0_f64));
    match node.shape {
        Shape::Diamond => Size::new(
            text_width.mul_add(1.25_f64, text_height) + px(40.0_f64),
            text_height + px(34.0_f64) + (text_width * 0.22_f64).min(px(40.0_f64)),
        ),
        Shape::Circle | Shape::DoubleCircle => {
            let round = (text_width + px(24.0_f64)).max(text_height + px(24.0_f64));
            let across = if node.shape == Shape::DoubleCircle {
                round + px(10.0_f64)
            } else {
                round
            };
            Size::new(across, across)
        }
        Shape::Cylinder => Size::new(width, height + px(16.0_f64)),
        Shape::Lean | Shape::LeanBack | Shape::Trapezoid | Shape::TrapezoidDown => {
            Size::new(text_width + px(50.0_f64), height)
        }
        Shape::Stadium | Shape::Flag => Size::new(text_width + px(40.0_f64), height),
        Shape::Hexagon | Shape::Subroutine => Size::new(text_width + px(46.0_f64), height),
        Shape::Rect | Shape::Round => Size::new(width.max(px(60.0_f64)), height),
    }
}

/// The subgraphs holding both, outermost first: the longest start of
/// `first` that `second` starts with too, down to none.
fn shared<'graph>(first: &'graph [ClusterId], second: &[ClusterId]) -> &'graph [ClusterId] {
    let mut common = first;
    while let Some((_, outer)) = common.split_last().filter(|_| !second.starts_with(common)) {
        common = outer;
    }
    common
}

impl<'graph> Layered<'graph> {
    /// A vertex for each node, nothing ranked yet.
    fn new(graph: &'graph Graph) -> Self {
        Self {
            graph,
            horizontal: graph.dir.horizontal(),
            vertices: graph
                .nodes
                .values()
                .map(|node| Vertex::new(Kind::Node, node_size(node), &node.within, 0))
                .collect(),
            ways: Vec::new(),
            layers: Vec::new(),
        }
    }

    /// Ranks the nodes: the longest path from the sources, then each source
    /// pulled down to just before its nearest successor.
    fn rank(&mut self, courses: &[Course<'graph>]) -> Option<()> {
        let links = || courses.iter().filter_map(Course::link);
        let mut pending: Table<VertexId, usize> = self.vertices.map(|_| 0);
        for link in links() {
            *pending.get_mut(link.to)? += 1;
        }
        let mut queue: Vec<VertexId> = pending
            .iter()
            .filter(|(_, waiting)| **waiting == 0)
            .map(|(id, _)| id)
            .collect();
        let mut done = Vec::with_capacity(self.vertices.len());
        while let Some(current) = queue.pop() {
            done.push(current);
            let base = self.vertices.get(current)?.rank;
            for link in links().filter(|link| link.from == current) {
                let next = self.vertices.get_mut(link.to)?;
                next.rank = next.rank.max(base + link.minlen);
                let waiting = pending.get_mut(link.to)?;
                *waiting -= 1;
                if *waiting == 0 {
                    queue.push(link.to);
                }
            }
        }
        for &current in done.iter().rev() {
            if links().any(|link| link.to == current) {
                continue;
            }
            let reachable = links()
                .filter(|link| link.from == current)
                .map(|link| {
                    let rank = self.vertices.get(link.to)?.rank;
                    Some(rank.saturating_sub(link.minlen))
                })
                .collect::<Option<Vec<usize>>>()?;
            if let Some(&nearest) = reachable.iter().min() {
                self.vertices.get_mut(current)?.rank = nearest;
            }
        }
        Some(())
    }

    /// Splits every edge longer than one rank into a chain of waypoints,
    /// then gathers the ranks.
    fn split(&mut self, courses: &[Course<'graph>]) -> Option<()> {
        let mut ways = Vec::with_capacity(courses.len());
        for course in courses {
            ways.push(match course {
                Course::Loop => Way::Loop,
                Course::Link(link) => Way::Chain(self.chain(link)?),
            });
        }
        self.ways = ways;
        let ranks = self
            .vertices
            .values()
            .map(|vertex| vertex.rank)
            .max()
            .map_or(1, |last| last + 1);
        self.layers = (0..ranks)
            .map(|rank| {
                self.vertices
                    .iter()
                    .filter(|(_, vertex)| vertex.rank == rank)
                    .map(|(id, _)| id)
                    .collect()
            })
            .collect();
        Some(())
    }

    /// The waypoints of one edge, one per rank it crosses; the middle one of
    /// a labelled edge carries its label's size.
    fn chain(&mut self, link: &Link<'graph>) -> Option<Chain> {
        let (start, end) = (self.vertices.get(link.from)?, self.vertices.get(link.to)?);
        let (first_rank, span) = (start.rank, end.rank.saturating_sub(start.rank));
        let within = shared(start.within, end.within);
        let mut waypoints = Vec::new();
        let mut label = None;
        let mut prev = link.from;
        for step in 1..span {
            let carried = link.label.filter(|_| step == span / 2);
            let (kind, size) = carried.map_or_else(
                || (Kind::Bend, Size::new(px(1.0_f64), px(1.0_f64))),
                |text| (Kind::Label, self.label_room(text)),
            );
            let waypoint = self
                .vertices
                .push(Vertex::new(kind, size, within, first_rank + step));
            if carried.is_some() {
                label = Some(waypoint);
            }
            self.join(prev, waypoint)?;
            waypoints.push(waypoint);
            prev = waypoint;
        }
        self.join(prev, link.to)?;
        if link.backward {
            waypoints.reverse();
        }
        Some(Chain { waypoints, label })
    }

    /// The room a label takes on its waypoint, turned with the flow.
    fn label_room(&self, label: &Label) -> Size {
        let Size { width, height } = label.size(EDGE_FONT);
        if self.horizontal {
            Size::new(height + px(8.0_f64), width + px(12.0_f64))
        } else {
            Size::new(width + px(12.0_f64), height + px(8.0_f64))
        }
    }

    /// An edge from `from` to `to`, one rank apart.
    fn join(&mut self, from: VertexId, to: VertexId) -> Option<()> {
        self.vertices.get_mut(from)?.succs.push(to);
        self.vertices.get_mut(to)?.preds.push(from);
        Some(())
    }

    /// The extent of `size` across the flow.
    const fn across(&self, size: Size) -> Len {
        if self.horizontal {
            size.height
        } else {
            size.width
        }
    }

    /// The extent of `size` along the flow.
    const fn along(&self, size: Size) -> Len {
        if self.horizontal {
            size.width
        } else {
            size.height
        }
    }
}
