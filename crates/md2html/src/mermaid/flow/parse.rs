// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading a flowchart: statements, subgraphs, nodes with their shapes, and
//! the edges that join them.

use alloc::collections::BTreeMap;

use super::arrow::{at_edge, edge_op};
use super::model::{
    Cluster, ClusterId, Dir, EDGE_FONT, Edge, Graph, NODE_FONT, Node, NodeId, Shape,
};
use crate::mermaid::geom::px;
use crate::mermaid::label::Label;
use crate::mermaid::missing;
use crate::mermaid::table::Table;
use crate::scan::Scan;

/// The flowchart in a block's lines after its header.
///
/// # Errors
///
/// Returns the first error a statement gives (see [`Reader::node`]):
/// "cannot read `<rest>`" where no edge follows a node, "a node without an
/// id" where an id is expected, and "cannot read the shape at `<rest>`"
/// where a shape opens but never closes. The diagram is then shown as
/// code.
pub(in crate::mermaid) fn parse(lines: &[&str], dir: Dir) -> Result<Graph, String> {
    let mut reader = Reader {
        graph: Graph {
            dir,
            nodes: Table::default(),
            edges: Table::default(),
            clusters: Table::default(),
            title: None,
            desc: None,
        },
        ids: BTreeMap::new(),
        open: Vec::new(),
        cluster_ids: BTreeMap::new(),
    };
    for line in lines {
        for statement in split_statements(line) {
            reader.statement(statement.trim())?;
        }
    }
    Ok(reader.graph)
}

/// A flowchart being read.
struct Reader {
    /// What has been read so far.
    graph: Graph,
    /// The nodes by their ids in the source.
    ids: BTreeMap<String, NodeId>,
    /// The subgraphs open where the reading is, outermost first.
    open: Vec<ClusterId>,
    /// The subgraphs by their ids, which an edge may name as if they were
    /// nodes.
    cluster_ids: BTreeMap<String, ClusterId>,
}

impl Reader {
    /// One statement.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Reader::chain`] for a statement that is not a
    /// subgraph, an `end`, a styling statement, a comment or an accessible
    /// title or description.
    fn statement(&mut self, statement: &str) -> Result<(), String> {
        if statement.is_empty() || statement.starts_with("%%") {
            return Ok(());
        }
        let (word, rest) = statement
            .split_once(char::is_whitespace)
            .unwrap_or((statement, ""));
        match word {
            "subgraph" => self.subgraph(rest.trim()),
            "end" if rest.is_empty() => {
                self.open.pop();
            }
            "direction" | "classDef" | "class" | "style" | "linkStyle" | "click" | "callback" => {}
            _ => {
                if let Some(title) = statement.strip_prefix("accTitle:") {
                    self.graph.title = Some(title.trim().to_owned());
                } else if let Some(desc) = statement.strip_prefix("accDescr:") {
                    self.graph.desc = Some(desc.trim().to_owned());
                } else {
                    self.chain(statement)?;
                }
            }
        }
        Ok(())
    }

    /// `subgraph` and what follows it: a subgraph opens.
    fn subgraph(&mut self, head: &str) {
        let (id, title) = subgraph_head(head);
        let outer = self.open.clone();
        let cluster = self.graph.clusters.push_with(|cluster| {
            let mut within = outer;
            within.push(cluster);
            Cluster {
                title: Label::parse(&title),
                within,
            }
        });
        self.cluster_ids.insert(id, cluster);
        self.open.push(cluster);
    }

    /// Nodes joined by edges: `A --> B & C -- yes --> D`.
    ///
    /// # Errors
    ///
    /// Returns "cannot read `<rest>`" when what follows a node is not an edge,
    /// and the errors of [`Reader::group`] for each group of nodes.
    fn chain(&mut self, statement: &str) -> Result<(), String> {
        let mut scan = Scan::new(statement);
        let mut prev = self.group(&mut scan)?;
        loop {
            skip_space(&mut scan);
            if scan.is_done() {
                return Ok(());
            }
            let Some(op) = edge_op(&mut scan) else {
                return Err(format!("cannot read `{}`", scan.rest()));
            };
            skip_space(&mut scan);
            let next = self.group(&mut scan)?;
            for &from in &prev {
                for &to in &next {
                    self.graph.edges.push(Edge {
                        from,
                        to,
                        label: op
                            .label
                            .clone()
                            .map(|label| label.wrapped(px(170.0_f64), EDGE_FONT)),
                        stroke: op.stroke,
                        tail: op.tail,
                        head: op.head,
                        minlen: op.minlen,
                    });
                }
            }
            prev = next;
        }
    }

    /// Nodes joined by `&`: at least one.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`Reader::node`] for any node of the group,
    /// including one after a trailing `&`.
    fn group(&mut self, scan: &mut Scan<'_>) -> Result<Vec<NodeId>, String> {
        let mut out = Vec::new();
        loop {
            skip_space(scan);
            out.push(self.node(scan)?);
            skip_space(scan);
            if !scan.eat('&') {
                return Ok(out);
            }
        }
    }

    /// One node: its id, and a shape with its label the first time.
    ///
    /// # Errors
    ///
    /// Returns "a node without an id" when no id stands where `scan` is (an
    /// edge or `&` with nothing after it, or a shape with no id before it,
    /// as in `A --> [x]`), the errors of [`shape_label`] when a shape opens
    /// after the id, and "the diagram refers to a part it does not hold"
    /// when a node the reader has recorded is not in the graph, which the
    /// reader's own bookkeeping prevents.
    fn node(&mut self, scan: &mut Scan<'_>) -> Result<NodeId, String> {
        let start = scan.mark();
        if scan.eat_if(|ch| !ends_id(ch)).is_some() {
            while !at_edge(scan.rest()) && scan.eat_if(|ch| !ends_id(ch)).is_some() {}
        }
        let written = scan.since(start);
        if written.is_empty() {
            return Err(String::from("a node without an id"));
        }
        let shaped = if scan.peek().is_some_and(|ch| "[({>".contains(ch)) {
            Some(shape_label(scan)?)
        } else {
            None
        };
        // `node:::className`
        if scan.eat_str(":::") {
            while !at_edge(scan.rest())
                && scan.eat_if(|ch| !ch.is_whitespace() && ch != '&').is_some()
            {}
        }
        let id = written.split_once(":::").map_or(written, |(id, _)| id);
        if shaped.is_none() && !self.ids.contains_key(id) {
            if let Some(first) = self.first_in_subgraph(id) {
                return Ok(first);
            }
        }
        if let Some(&known) = self.ids.get(id) {
            if let Some((shape, label)) = shaped {
                let node = self.graph.nodes.get_mut(known).ok_or_else(missing)?;
                node.shape = shape;
                node.label = node_label(label);
            }
            return Ok(known);
        }
        let node = self.graph.nodes.push(Node {
            id: id.to_owned(),
            label: node_label(shaped.map_or(id, |(_, label)| label)),
            shape: shaped.map_or(Shape::Rect, |(shape, _)| shape),
            within: self.open.clone(),
        });
        self.ids.insert(id.to_owned(), node);
        Ok(node)
    }

    /// An edge to a subgraph lands on its first node, or an anchor in it:
    /// that node, when `id` names a subgraph with one.
    fn first_in_subgraph(&self, id: &str) -> Option<NodeId> {
        let cluster = self.cluster_ids.get(id)?;
        self.graph
            .nodes
            .iter()
            .find(|(_, node)| node.within.last() == Some(cluster))
            .map(|(first, _)| first)
    }
}

/// A node's label, broken to fit.
fn node_label(text: &str) -> Label {
    Label::parse(text).wrapped(px(230.0_f64), NODE_FONT)
}

/// Whether `ch` ends a node's id.
fn ends_id(ch: char) -> bool {
    ch.is_whitespace() || "[({>&;|".contains(ch)
}

/// Passes over white space.
fn skip_space(scan: &mut Scan<'_>) {
    scan.take_while(char::is_whitespace);
}

/// A line split at `;` outside quotes and brackets.
fn split_statements(line: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut scan = Scan::new(line);
    let mut start = scan.mark();
    let mut depth = 0_i32;
    let mut quote = false;
    while let Some(ch) = scan.peek() {
        match ch {
            '"' => quote = !quote,
            '[' | '(' | '{' if !quote => depth += 1_i32,
            ']' | ')' | '}' if !quote => depth -= 1_i32,
            ';' if !quote && depth <= 0_i32 => {
                out.push(scan.since(start));
                scan.bump();
                start = scan.mark();
                continue;
            }
            _ => {}
        }
        scan.bump();
    }
    out.push(scan.since(start));
    out
}

/// `id [Title]`, `id["Title"]`, `"Title"`, or `Some words`: (id, title).
fn subgraph_head(head: &str) -> (String, String) {
    if head.starts_with('"') {
        return (head.trim_matches('"').to_owned(), head.to_owned());
    }
    if let Some((before, after)) = head.split_once('[') {
        let id = before.trim();
        let title = after.trim_end().trim_end_matches(']');
        let named = if id.is_empty() { title } else { id };
        return (named.to_owned(), title.to_owned());
    }
    (head.to_owned(), head.to_owned())
}

/// The shapes by how they are written: opener, closer, shape. Longer
/// openers come first, so `((` is not read as `(`.
const FORMS: &[(&str, &str, Shape)] = &[
    ("(((", ")))", Shape::DoubleCircle),
    ("((", "))", Shape::Circle),
    ("([", "])", Shape::Stadium),
    ("[[", "]]", Shape::Subroutine),
    ("[(", ")]", Shape::Cylinder),
    ("{{", "}}", Shape::Hexagon),
    ("[/", "/]", Shape::Lean),
    ("[/", "\\]", Shape::Trapezoid),
    ("[\\", "\\]", Shape::LeanBack),
    ("[\\", "/]", Shape::TrapezoidDown),
    ("(", ")", Shape::Round),
    ("[", "]", Shape::Rect),
    ("{", "}", Shape::Diamond),
    (">", "]", Shape::Flag),
];

/// The shape opened where `scan` is, and its label.
///
/// # Errors
///
/// Returns "cannot read the shape at `<rest>`" when no form of [`FORMS`] opens
/// there and closes, for instance `A[label` with no `]`.
fn shape_label<'text>(scan: &mut Scan<'text>) -> Result<(Shape, &'text str), String> {
    for &(open, close, shape) in FORMS {
        let mut probe = scan.clone();
        if !probe.eat_str(open) {
            continue;
        }
        let label = if probe.rest().trim_start().starts_with('"') {
            quoted(&mut probe, close)
        } else {
            nested(&mut probe, open, close)
        };
        if let Some(text) = label {
            *scan = probe;
            return Ok((shape, text));
        }
    }
    Err(format!("cannot read the shape at `{}`", scan.rest()))
}

/// A quoted label, quotes kept, and anything up to and through `close`
/// after it.
fn quoted<'text>(scan: &mut Scan<'text>, close: &str) -> Option<&'text str> {
    let start = scan.mark();
    scan.take_until("\"")?;
    scan.bump();
    scan.take_until("\"")?;
    scan.bump();
    let label = scan.since(start);
    scan.take_until(close)?;
    scan.eat_str(close);
    Some(label)
}

/// A label up to `close`, read through it; a one-character opener inside
/// the label nests, and its closer then belongs to the label.
fn nested<'text>(scan: &mut Scan<'text>, open: &str, close: &str) -> Option<&'text str> {
    let opener = open.chars().last()?;
    let closer = close.chars().next()?;
    let start = scan.mark();
    let mut depth = 0_usize;
    loop {
        if depth == 0 && scan.starts_with(close) {
            let label = scan.since(start);
            scan.eat_str(close);
            return Some(label);
        }
        let ch = scan.bump()?;
        if ch == opener && open.len() == 1 {
            depth += 1;
        } else if ch == closer && depth > 0 && close.len() == 1 {
            depth -= 1;
        }
    }
}
