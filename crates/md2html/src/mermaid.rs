// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

#![expect(
    clippy::cognitive_complexity,
    clippy::float_arithmetic,
    clippy::indexing_slicing,
    clippy::let_underscore_untyped,
    clippy::min_ident_chars,
    clippy::missing_asserts_for_indexing,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::shadow_unrelated,
    clippy::string_add,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    reason = "adoption debt: the forge's lint table, burned down as GOTO.md section 3 lists"
)]

//! Mermaid diagrams drawn here, without a script: flowcharts and sequence
//! diagrams are read, laid out and written as inline SVG that takes the
//! page's colours (both themes), prints, and keeps its words as text. A
//! flowchart is laid out the layered way: cycles broken, nodes ranked,
//! long edges split by invisible waypoints, each rank ordered to cut
//! crossings, then placed toward the middle of its neighbours. Other
//! diagram kinds are left to `mermaid.js` when the author supplies it, and
//! shown as code otherwise.
//!
//! Exception: parsing, layered layout and drawing of two diagram kinds; to be
//! split by kind (`scripts/ci/file_size_exceptions.txt`).

#![allow(
    clippy::cast_precision_loss,
    reason = "layout turns counts of nodes, ranks and lines, all far below 2^52, into coordinates"
)]

use alloc::collections::BTreeMap;

use crate::text::{esc, put};

/// What came of a diagram block.
pub(crate) enum Outcome {
    /// The SVG, drawn here.
    Drawn(String),
    /// A kind this does not draw (`pie`, `gantt`, ...): its name.
    Unsupported(String),
    /// A flowchart or sequence diagram that could not be read: why.
    Unreadable(String),
}

/// The diagram in a block of Mermaid source. `n` numbers the diagrams of a
/// page, so their marker ids stay apart.
pub(crate) fn render(source: &str, n: usize) -> Outcome {
    let lines: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("%%"))
        .collect();
    let Some(head) = lines.first() else {
        return Outcome::Unreadable(String::from("the block is empty"));
    };
    let mut words = head.split_whitespace();
    let kind = words.next().unwrap_or_default();
    match kind {
        "flowchart" | "graph" | "flowchart-elk" => {
            let dir = Dir::parse(words.next().unwrap_or("TD"));
            match flow::parse(&lines[1..], dir) {
                Ok(graph) => {
                    if graph.nodes.is_empty() {
                        return Outcome::Unreadable(String::from("the flowchart has no nodes"));
                    }
                    if graph.nodes.len() > 400 {
                        return Outcome::Unreadable(String::from(
                            "the flowchart has more than 400 nodes",
                        ));
                    }
                    Outcome::Drawn(flow::draw(&graph, n))
                }
                Err(e) => Outcome::Unreadable(e),
            }
        }
        "sequenceDiagram" => match seq::parse(&lines[1..]) {
            Ok(diagram) => Outcome::Drawn(seq::draw(&diagram, n)),
            Err(e) => Outcome::Unreadable(e),
        },
        other => Outcome::Unsupported(other.trim_end_matches(':').to_owned()),
    }
}

/// The width of text in the page's sans font at `size` pixels, estimated
/// from the shapes of the letters (no font is measured; the boxes are made
/// a little roomy for it).
fn text_width(text: &str, size: f64) -> f64 {
    let em: f64 = text
        .chars()
        .map(|ch| match ch {
            'i' | 'l' | 'j' | '!' | '|' | '.' | ',' | ':' | ';' | '\'' | 'I' | '`' => 0.28_f64,
            ' ' => 0.29_f64,
            'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '/' | '-' => 0.37_f64,
            'm' | 'w' => 0.84_f64,
            'M' | 'W' | '@' | '%' => 0.92_f64,
            'A'..='Z' => 0.67_f64,
            '0'..='9' => 0.6_f64,
            'a'..='z' => 0.55_f64,
            c if is_wide(c) => 1.0_f64,
            _ => 0.62_f64,
        })
        .sum();
    em * size
}

/// A character that takes a full em: CJK, emoji.
fn is_wide(ch: char) -> bool {
    matches!(u32::from(ch), 0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6 | 0x1F300..=0x1FAFF)
}

/// A label's lines: `<br>` and `\n` break it, `#quot;`-style and HTML
/// entities are decoded, Markdown marks and quotes dropped.
fn label_lines(raw: &str) -> Vec<String> {
    let mut text = raw.trim().to_owned();
    if text.len() >= 2
        && ((text.starts_with('"') && text.ends_with('"'))
            || (text.starts_with('`') && text.ends_with('`')))
    {
        text = text[1..text.len() - 1].to_owned();
    }
    for br in ["<br/>", "<br />", "<br>", "<BR>", "<BR/>", "\\n"] {
        text = text.replace(br, "\n");
    }
    // Mermaid's own escapes: #quot; #amp; #35; ...
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(at) = rest.find('#') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        if let Some(end) = tail.find(';').filter(|e| *e <= 8) {
            let name = &tail[..end];
            let ch = match name {
                "quot" => Some('"'),
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "nbsp" => Some('\u{a0}'),
                _ => name.parse::<u32>().ok().and_then(char::from_u32),
            };
            if let Some(c) = ch {
                out.push(c);
            } else {
                out.push('#');
                out.push_str(name);
                out.push(';');
            }
            rest = &tail[end + 1..];
        } else {
            out.push('#');
            rest = tail;
        }
    }
    out.push_str(rest);
    let out = crate::inline::decode_entities(&out)
        .replace("**", "")
        .replace("__", "");
    out.split('\n').map(|l| l.trim().to_owned()).collect()
}

/// Lines broken at word boundaries to fit `max` pixels at `size`.
fn wrap(lines: Vec<String>, max: f64, size: f64) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        if text_width(&line, size) <= max {
            out.push(line);
            continue;
        }
        let mut cur = String::new();
        for word in line.split(' ') {
            let candidate = if cur.is_empty() {
                word.to_owned()
            } else {
                format!("{cur} {word}")
            };
            if !cur.is_empty() && text_width(&candidate, size) > max {
                out.push(core::mem::take(&mut cur));
                word.clone_into(&mut cur);
            } else {
                cur = candidate;
            }
        }
        out.push(cur);
    }
    out
}

/// The lines of a label as SVG text, centred on (x, y).
fn text_block(out: &mut String, lines: &[String], x: f64, y: f64, class: &str) {
    let lh = 18.0_f64;
    let top = y - lh * (lines.len() as f64 - 1.0_f64) / 2.0_f64;
    put!(
        out,
        "<text class=\"{class}\" x=\"{x:.1}\" y=\"{top:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\">"
    );
    for (k, line) in lines.iter().enumerate() {
        if k == 0 {
            put!(out, "<tspan x=\"{x:.1}\">{}</tspan>", esc(line));
        } else {
            put!(out, "<tspan x=\"{x:.1}\" dy=\"{lh}\">{}</tspan>", esc(line));
        }
    }
    out.push_str("</text>");
}

/// The size of a label's lines: (width, height).
fn label_size(lines: &[String], size: f64) -> (f64, f64) {
    let w = lines
        .iter()
        .map(|l| text_width(l, size))
        .fold(0.0_f64, f64::max);
    (w, 18.0 * lines.len().max(1) as f64)
}

/// A flowchart's direction.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Dir {
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
    fn parse(word: &str) -> Self {
        match word.trim_end_matches(';').to_ascii_uppercase().as_str() {
            "BT" => Self::BottomUp,
            "LR" => Self::LeftRight,
            "RL" => Self::RightLeft,
            _ => Self::TopDown,
        }
    }

    /// Whether ranks run across the page (left to right or back).
    const fn horizontal(self) -> bool {
        matches!(self, Self::LeftRight | Self::RightLeft)
    }
}

/// The SVG around a drawing, with its title and description for readers
/// who cannot see it, and the arrowheads.
fn svg_open(out: &mut String, n: usize, width: f64, height: f64, title: &str, desc: &str) {
    put!(
        out,
        "<figure class=\"diagram{}\"><svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width:.0} {height:.0}\" width=\"{width:.0}\" height=\"{height:.0}\" role=\"img\" aria-labelledby=\"dg{n}-t dg{n}-d\">\
<title id=\"dg{n}-t\">{}</title><desc id=\"dg{n}-d\">{}</desc><defs>\
<marker id=\"dg{n}-a\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path class=\"head\" d=\"M0 0L10 5L0 10z\"/></marker>\
<marker id=\"dg{n}-o\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto-start-reverse\"><circle class=\"head-o\" cx=\"5\" cy=\"5\" r=\"3.6\"/></marker>\
<marker id=\"dg{n}-x\" viewBox=\"0 0 10 10\" refX=\"7\" refY=\"5\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto-start-reverse\"><path class=\"head-x\" d=\"M2 2L8 8M8 2L2 8\"/></marker>\
<marker id=\"dg{n}-v\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path class=\"head-x\" d=\"M1 1L9 5L1 9\"/></marker>\
</defs>",
        if width > 1_300.0_f64 { " wide" } else { "" },
        esc(title),
        esc(desc)
    );
}

/// Flowcharts.
mod flow {
    use super::{BTreeMap, Dir, esc, label_lines, label_size, svg_open, text_block, wrap};
    use crate::text::put;

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
    pub(super) struct Node {
        /// Its id in the source.
        pub(super) id: String,
        /// Its label's lines.
        pub(super) label: Vec<String>,
        /// Its outline.
        pub(super) shape: Shape,
        /// The innermost subgraph holding it.
        pub(super) cluster: Option<usize>,
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
        pub(super) from: usize,
        /// To this node.
        pub(super) to: usize,
        /// Its label's lines.
        pub(super) label: Option<Vec<String>>,
        /// Its line.
        pub(super) stroke: Stroke,
        /// At the start.
        pub(super) tail: Head,
        /// At the end.
        pub(super) head: Head,
        /// Ranks it spans at least (`--->` is two).
        pub(super) minlen: usize,
    }

    /// A subgraph.
    #[derive(Clone, Debug)]
    pub(super) struct Cluster {
        /// Its title.
        pub(super) title: Vec<String>,
        /// The subgraph it sits in.
        pub(super) parent: Option<usize>,
    }

    /// A flowchart as read.
    #[derive(Debug)]
    pub(super) struct Graph {
        /// Which way the graph flows.
        pub(super) dir: Dir,
        /// The nodes, in the order they were first named.
        pub(super) nodes: Vec<Node>,
        /// The edges between them.
        pub(super) edges: Vec<Edge>,
        /// The subgraphs.
        pub(super) clusters: Vec<Cluster>,
        /// The author's title, if given.
        pub(super) title: Option<String>,
        /// The author's description, if given.
        pub(super) desc: Option<String>,
    }

    /// The flowchart in a block's lines after its header.
    pub(super) fn parse(lines: &[&str], dir: Dir) -> Result<Graph, String> {
        let mut g = Graph {
            dir,
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
            title: None,
            desc: None,
        };
        let mut ids: BTreeMap<String, usize> = BTreeMap::new();
        let mut stack: Vec<usize> = Vec::new();
        // A subgraph's own id, which an edge may name as if it were a node.
        let mut cluster_ids: BTreeMap<String, usize> = BTreeMap::new();
        for raw in lines {
            for stmt in split_statements(raw) {
                let stmt = stmt.trim();
                if stmt.is_empty() || stmt.starts_with("%%") {
                    continue;
                }
                let word = stmt.split_whitespace().next().unwrap_or_default();
                match word {
                    "subgraph" => {
                        let rest = stmt["subgraph".len()..].trim();
                        let (id, title) = subgraph_head(rest);
                        g.clusters.push(Cluster {
                            title: label_lines(&title),
                            parent: stack.last().copied(),
                        });
                        let c = g.clusters.len() - 1;
                        cluster_ids.insert(id, c);
                        stack.push(c);
                        continue;
                    }
                    "end" if stmt == "end" => {
                        stack.pop();
                        continue;
                    }
                    "direction" | "classDef" | "class" | "style" | "linkStyle" | "click"
                    | "callback" => continue,
                    _ => {}
                }
                if let Some(t) = stmt.strip_prefix("accTitle:") {
                    g.title = Some(t.trim().to_owned());
                    continue;
                }
                if let Some(d) = stmt.strip_prefix("accDescr:") {
                    g.desc = Some(d.trim().to_owned());
                    continue;
                }
                chain(stmt, &mut g, &mut ids, stack.last().copied(), &cluster_ids)?;
            }
        }
        Ok(g)
    }

    /// A line split at `;` outside quotes and brackets.
    fn split_statements(line: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut depth = 0_i32;
        let mut quote = false;
        let mut start = 0;
        for (at, ch) in line.char_indices() {
            match ch {
                '"' => quote = !quote,
                '[' | '(' | '{' if !quote => depth += 1_i32,
                ']' | ')' | '}' if !quote => depth -= 1_i32,
                ';' if !quote && depth <= 0_i32 => {
                    out.push(&line[start..at]);
                    start = at + 1;
                }
                _ => {}
            }
        }
        out.push(&line[start..]);
        out
    }

    /// `id [Title]`, `id["Title"]`, `"Title"`, or `Some words`: (id, title).
    fn subgraph_head(rest: &str) -> (String, String) {
        if rest.starts_with('"') {
            return (rest.trim_matches('"').to_owned(), rest.to_owned());
        }
        if let Some(open) = rest.find('[') {
            let id = rest[..open].trim().to_owned();
            let title = rest[open + 1..].trim_end().trim_end_matches(']').to_owned();
            return (if id.is_empty() { title.clone() } else { id }, title);
        }
        (rest.to_owned(), rest.to_owned())
    }

    /// One statement: nodes joined by edges, `A --> B & C -- yes --> D`.
    fn chain(
        stmt: &str,
        g: &mut Graph,
        ids: &mut BTreeMap<String, usize>,
        cluster: Option<usize>,
        cluster_ids: &BTreeMap<String, usize>,
    ) -> Result<(), String> {
        let chars: Vec<char> = stmt.chars().collect();
        let mut i = 0_usize;
        let mut prev: Vec<usize> = group(&chars, &mut i, g, ids, cluster, cluster_ids)?;
        loop {
            skip_ws(&chars, &mut i);
            if i >= chars.len() {
                break;
            }
            let Some(op) = edge_op(&chars, &mut i) else {
                return Err(format!(
                    "cannot read `{}`",
                    chars[i..].iter().collect::<String>()
                ));
            };
            skip_ws(&chars, &mut i);
            let next = group(&chars, &mut i, g, ids, cluster, cluster_ids)?;
            if next.is_empty() {
                return Err(String::from("an edge leads nowhere"));
            }
            for &a in &prev {
                for &b in &next {
                    g.edges.push(Edge {
                        from: a,
                        to: b,
                        label: op.label.clone().map(|l| wrap(l, 170.0, 13.0)),
                        stroke: op.stroke,
                        tail: op.tail,
                        head: op.head,
                        minlen: op.minlen,
                    });
                }
            }
            prev = next;
        }
        Ok(())
    }

    /// Passes over white space.
    fn skip_ws(chars: &[char], i: &mut usize) {
        while *i < chars.len() && chars[*i].is_whitespace() {
            *i += 1;
        }
    }

    /// Nodes joined by `&`.
    fn group(
        chars: &[char],
        i: &mut usize,
        g: &mut Graph,
        ids: &mut BTreeMap<String, usize>,
        cluster: Option<usize>,
        cluster_ids: &BTreeMap<String, usize>,
    ) -> Result<Vec<usize>, String> {
        let mut out = Vec::new();
        loop {
            skip_ws(chars, i);
            let n = node(chars, i, g, ids, cluster, cluster_ids)?;
            out.push(n);
            skip_ws(chars, i);
            if *i < chars.len() && chars[*i] == '&' {
                *i += 1;
                continue;
            }
            break;
        }
        Ok(out)
    }

    /// Whether an edge starts at `i`.
    fn at_edge(chars: &[char], i: usize) -> bool {
        let s: String = chars[i..chars.len().min(i + 3)].iter().collect();
        s.starts_with("--")
            || s.starts_with("-.")
            || s.starts_with("==")
            || s.starts_with("~~")
            || s.starts_with("<-")
            || s.starts_with("<=")
            || s.starts_with("o--")
            || s.starts_with("x--")
    }

    /// One node: its id, and a shape with its label the first time.
    fn node(
        chars: &[char],
        i: &mut usize,
        g: &mut Graph,
        ids: &mut BTreeMap<String, usize>,
        cluster: Option<usize>,
        cluster_ids: &BTreeMap<String, usize>,
    ) -> Result<usize, String> {
        let start = *i;
        while *i < chars.len() {
            let c = chars[*i];
            if c.is_whitespace() || "[({>&;|".contains(c) || (*i > start && at_edge(chars, *i)) {
                break;
            }
            *i += 1;
        }
        let id: String = chars[start..*i].iter().collect();
        if id.is_empty() {
            return Err(String::from("a node without an id"));
        }
        let (shape, label) = if *i < chars.len() && "[({>".contains(chars[*i]) {
            let (s, l) = shape_label(chars, i)?;
            (Some(s), Some(l))
        } else {
            (None, None)
        };
        // `node:::className`
        if chars.get(*i) == Some(&':')
            && chars.get(*i + 1) == Some(&':')
            && chars.get(*i + 2) == Some(&':')
        {
            *i += 3;
            while *i < chars.len()
                && !chars[*i].is_whitespace()
                && !at_edge(chars, *i)
                && chars[*i] != '&'
            {
                *i += 1;
            }
        }
        let id = id.split(":::").next().unwrap_or_default().to_owned();
        if shape.is_none() && cluster_ids.contains_key(&id) && !ids.contains_key(&id) {
            // An edge to a subgraph lands on its first node, or an anchor in
            // it.
            let c = cluster_ids[&id];
            if let Some(first) = g.nodes.iter().position(|n| n.cluster == Some(c)) {
                return Ok(first);
            }
        }
        if let Some(&n) = ids.get(&id) {
            if let (Some(s), Some(l)) = (shape, label) {
                g.nodes[n].shape = s;
                g.nodes[n].label = wrap(label_lines(&l), 230.0, 14.0);
            }
            Ok(n)
        } else {
            let text = label.unwrap_or_else(|| id.clone());
            g.nodes.push(Node {
                id: id.clone(),
                label: wrap(label_lines(&text), 230.0, 14.0),
                shape: shape.unwrap_or(Shape::Rect),
                cluster,
            });
            let n = g.nodes.len() - 1;
            ids.insert(id, n);
            Ok(n)
        }
    }

    /// The shape opened at `i`, and its label.
    fn shape_label(chars: &[char], i: &mut usize) -> Result<(Shape, String), String> {
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
        let rest: String = chars[*i..].iter().collect();
        for (open, close, shape) in FORMS {
            if !rest.starts_with(open) {
                continue;
            }
            let body = &rest[open.len()..];
            let end = if body.trim_start().starts_with('"') {
                // A quoted label: up to its closing quote, then the closer.
                let q0 = body.find('"').unwrap_or(0);
                let q1 = body[q0 + 1..].find('"').map(|q| q0 + 1 + q);
                q1.and_then(|q| body[q + 1..].find(close).map(|c| (q + 1 + c, &body[..=q])))
            } else {
                matching(body, open, close).map(|c| (c, &body[..c]))
            };
            let Some((at, label)) = end else { continue };
            *i += open.chars().count() + body[..at].chars().count() + close.chars().count();
            return Ok((*shape, label.to_owned()));
        }
        Err(format!("cannot read the shape at `{rest}`"))
    }

    /// Where `close` ends a label that `open` began, nested openers counted.
    fn matching(body: &str, open: &str, close: &str) -> Option<usize> {
        let first_open = open.chars().last()?;
        let first_close = close.chars().next()?;
        let mut depth = 0_i32;
        for (at, ch) in body.char_indices() {
            if body[at..].starts_with(close) && depth == 0_i32 {
                return Some(at);
            }
            if ch == first_open && open.len() == 1 {
                depth += 1_i32;
            } else if ch == first_close && depth > 0_i32 && close.len() == 1 {
                depth -= 1_i32;
            }
        }
        None
    }

    /// An edge operator read.
    struct Op {
        /// The label on the edge.
        label: Option<Vec<String>>,
        /// How the line is drawn.
        stroke: Stroke,
        /// The mark at its start.
        tail: Head,
        /// The mark at its end.
        head: Head,
        /// How many ranks it spans at least.
        minlen: usize,
    }

    /// The edge at `i`: `-->`, `---`, `-.->`, `==>`, `--o`, `<-->`,
    /// `-- text -->`, `-->|text|`.
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    fn edge_op(chars: &[char], i: &mut usize) -> Option<Op> {
        let start = *i;
        let mut tail = Head::None;
        match chars.get(*i) {
            Some('<') => {
                tail = Head::Arrow;
                *i += 1;
            }
            Some('o') if chars.get(*i + 1) == Some(&'-') => {
                tail = Head::Circle;
                *i += 1;
            }
            Some('x') if chars.get(*i + 1) == Some(&'-') => {
                tail = Head::Cross;
                *i += 1;
            }
            _ => {}
        }
        let body_start = *i;
        let mut stroke = Stroke::Solid;
        let mut dashes = 0_usize;
        while *i < chars.len() && matches!(chars[*i], '-' | '=' | '.' | '~') {
            match chars[*i] {
                '=' => stroke = Stroke::Thick,
                '.' => stroke = Stroke::Dotted,
                '~' => stroke = Stroke::Invisible,
                _ => {}
            }
            dashes += 1;
            *i += 1;
        }
        if dashes < 2 {
            *i = start;
            return None;
        }
        let mut head = Head::None;
        let mut label: Option<Vec<String>> = None;
        match chars.get(*i) {
            Some('>') => {
                head = Head::Arrow;
                *i += 1;
            }
            Some('o')
                if chars
                    .get(*i + 1)
                    .is_none_or(|c| c.is_whitespace() || *c == '|') =>
            {
                head = Head::Circle;
                *i += 1;
            }
            Some('x')
                if chars
                    .get(*i + 1)
                    .is_none_or(|c| c.is_whitespace() || *c == '|') =>
            {
                head = Head::Cross;
                *i += 1;
            }
            Some(c)
                if (c.is_whitespace() || (stroke == Stroke::Dotted && c.is_alphanumeric()))
                    && dashes <= 2
                    && stroke != Stroke::Invisible =>
            {
                // `-- text -->`, `-. text .->`, `== text ==>`: the label sits
                // inside the edge.
                let rest: String = chars[*i..].iter().collect();
                let closers: &[&str] = match stroke {
                    Stroke::Dotted => &[".->", ".-"],
                    Stroke::Thick => &["==>", "==="],
                    _ => &["-->", "---", "--o", "--x"],
                };
                if let Some((at, closer)) = closers
                    .iter()
                    .filter_map(|c| rest.find(c).map(|at| (at, *c)))
                    .min_by_key(|(at, _)| *at)
                {
                    let text = rest[..at].trim();
                    if !text.is_empty() && !text.contains("--") {
                        label = Some(label_lines(text));
                        *i += rest[..at].chars().count() + closer.chars().count();
                        head = match closer.chars().last() {
                            Some('>') => Head::Arrow,
                            Some('o') => Head::Circle,
                            Some('x') => Head::Cross,
                            _ => Head::None,
                        };
                        // A longer closer (`--->`) stretches the edge.
                        while *i < chars.len() && matches!(chars[*i], '-' | '=' | '.') {
                            *i += 1;
                        }
                        if chars.get(*i) == Some(&'>') {
                            head = Head::Arrow;
                            *i += 1;
                        }
                    }
                }
            }
            _ => {}
        }
        let _ = body_start;
        // `-->|text|`
        let mut j = *i;
        while j < chars.len() && chars[j] == ' ' {
            j += 1;
        }
        if label.is_none() && chars.get(j) == Some(&'|') {
            if let Some(close) = chars[j + 1..].iter().position(|c| *c == '|') {
                let text: String = chars[j + 1..j + 1 + close].iter().collect();
                label = Some(label_lines(&text));
                *i = j + 1 + close + 1;
            }
        }
        let minlen = if dashes > 3 { (dashes - 2).min(4) } else { 1 };
        Some(Op {
            label,
            stroke,
            tail,
            head,
            minlen,
        })
    }

    /// A node placed: its centre and size.
    #[derive(Clone, Copy, Debug, Default)]
    pub(super) struct Placed {
        /// The centre, across.
        pub(super) x: f64,
        /// The centre, down.
        pub(super) y: f64,
        /// The width.
        pub(super) width: f64,
        /// The height.
        pub(super) height: f64,
    }

    /// The layout: every node's box (waypoints included), the path of each
    /// edge, each cluster's box, the whole size.
    pub(super) struct Layout {
        /// Each node's box.
        pub(super) boxes: Vec<Placed>,
        /// Each edge's path, point by point.
        pub(super) paths: Vec<Vec<(f64, f64)>>,
        /// Where each edge's label goes.
        pub(super) label_at: Vec<Option<(f64, f64)>>,
        /// Each subgraph's frame: x, y, width, height.
        pub(super) clusters: Vec<Option<(f64, f64, f64, f64)>>,
        /// The drawing's width.
        pub(super) width: f64,
        /// The drawing's height.
        pub(super) height: f64,
    }

    /// A node's drawn size for its label and shape.
    fn node_size(node: &Node) -> (f64, f64) {
        let (tw, th) = label_size(&node.label, 14.0);
        let (w, h) = (tw + 30.0_f64, th + 20.0_f64);
        match node.shape {
            Shape::Diamond => (
                f64::mul_add(tw, 1.25, th) + 40.0,
                th + 34.0 + (tw * 0.22).min(40.0),
            ),
            Shape::Circle | Shape::DoubleCircle => {
                let d = (tw + 24.0).max(th + 24.0);
                let d = if node.shape == Shape::DoubleCircle {
                    d + 10.0
                } else {
                    d
                };
                (d, d)
            }
            Shape::Cylinder => (w, h + 16.0),
            Shape::Lean | Shape::LeanBack | Shape::Trapezoid | Shape::TrapezoidDown => {
                (tw + 50.0, h)
            }
            Shape::Stadium | Shape::Flag => (tw + 40.0, h),
            Shape::Hexagon | Shape::Subroutine => (tw + 46.0, h),
            Shape::Rect | Shape::Round => (w.max(60.0), h),
        }
    }

    /// The layered layout of a flowchart.
    #[expect(
        clippy::too_many_lines,
        reason = "the layered layout's phases, in order"
    )]
    #[expect(
        clippy::many_single_char_names,
        reason = "geometry and scanning: x, y, w, h, i, j are the domain's names"
    )]
    pub(super) fn layout(g: &Graph) -> Layout {
        let horizontal = g.dir.horizontal();
        let real = g.nodes.len();
        // Sizes along the rank (across) and between ranks (along).
        let mut size: Vec<(f64, f64)> = g.nodes.iter().map(node_size).collect();
        let mut cluster_of: Vec<Option<usize>> = g.nodes.iter().map(|n| n.cluster).collect();
        // 1. Break cycles: an edge back to a node on the current path is laid
        //    out reversed.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); real];
        for (k, e) in g.edges.iter().enumerate() {
            adj[e.from].push(k);
        }
        let mut reversed = vec![false; g.edges.len()];
        let mut state = vec![0_u8; real];
        for root in 0..real {
            if state[root] != 0 {
                continue;
            }
            let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
            state[root] = 1;
            while let Some((v, at)) = stack.pop() {
                if at < adj[v].len() {
                    stack.push((v, at + 1));
                    let k = adj[v][at];
                    let w = g.edges[k].to;
                    if w == v {
                        continue;
                    }
                    match state[w] {
                        0 => {
                            state[w] = 1;
                            stack.push((w, 0));
                        }
                        1 => reversed[k] = true,
                        _ => {}
                    }
                } else {
                    state[v] = 2;
                }
            }
        }
        // Directed edges of the layout: (u, v, minlen, edge index); self-loops
        // left out.
        let dag: Vec<(usize, usize, usize, usize)> = g
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.from != e.to)
            .map(|(k, e)| {
                let minlen = e.minlen.max(if e.label.is_some() { 2 } else { 1 });
                if reversed[k] {
                    (e.to, e.from, minlen, k)
                } else {
                    (e.from, e.to, minlen, k)
                }
            })
            .collect();
        // 2. Rank: the longest path from the sources, then sources pulled down
        //    to their successors.
        let mut rank = vec![0_usize; real];
        let mut indeg = vec![0_usize; real];
        for &(_, v, _, _) in &dag {
            indeg[v] += 1;
        }
        let mut queue: Vec<usize> = (0..real).filter(|v| indeg[*v] == 0).collect();
        let mut order = Vec::with_capacity(real);
        while let Some(v) = queue.pop() {
            order.push(v);
            for &(u, w, minlen, _) in &dag {
                if u == v {
                    rank[w] = rank[w].max(rank[v] + minlen);
                    indeg[w] -= 1;
                    if indeg[w] == 0 {
                        queue.push(w);
                    }
                }
            }
        }
        for v in order.iter().rev() {
            let has_in = dag.iter().any(|&(_, w, _, _)| w == *v);
            let outs: Vec<usize> = dag
                .iter()
                .filter(|&&(u, _, _, _)| u == *v)
                .map(|&(_, w, m, _)| rank[w].saturating_sub(m))
                .collect();
            if !has_in && !outs.is_empty() {
                rank[*v] = outs.into_iter().min().unwrap_or(rank[*v]);
            }
        }
        // 3. Long edges become chains of waypoints, one per rank crossed; a
        //    labelled edge's middle waypoint carries its label's size.
        let mut chain_of: Vec<Vec<usize>> = vec![Vec::new(); g.edges.len()];
        let mut links: Vec<(usize, usize)> = Vec::new();
        let mut label_node: Vec<Option<usize>> = vec![None; g.edges.len()];
        for &(u, v, _, k) in &dag {
            let span = rank[v] - rank[u];
            let mut prev = u;
            let mut path = vec![u];
            let common = common_cluster(g, cluster_of[u], cluster_of[v]);
            for step in 1..span {
                let d = rank.len();
                rank.push(rank[u] + step);
                let label_here = g.edges[k].label.is_some() && step == span / 2;
                size.push(if label_here {
                    let (lw, lh) =
                        label_size(g.edges[k].label.as_deref().unwrap_or_default(), 13.0);
                    if horizontal {
                        (lh + 8.0_f64, lw + 12.0_f64)
                    } else {
                        (lw + 12.0_f64, lh + 8.0_f64)
                    }
                } else {
                    (1.0_f64, 1.0_f64)
                });
                cluster_of.push(common);
                if label_here {
                    label_node[k] = Some(d);
                }
                links.push((prev, d));
                path.push(d);
                prev = d;
            }
            links.push((prev, v));
            path.push(v);
            if reversed[k] {
                path.reverse();
            }
            chain_of[k] = path;
        }
        let total = rank.len();
        let ranks = rank.iter().copied().max().unwrap_or(0) + 1;
        // 4. Order each rank: first seen first, then barycentre sweeps,
        //    subgraphs kept together.
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); ranks];
        for v in 0..total {
            layers[rank[v]].push(v);
        }
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); total];
        let mut succs: Vec<Vec<usize>> = vec![Vec::new(); total];
        for &(a, b) in &links {
            succs[a].push(b);
            preds[b].push(a);
        }
        let mut pos = vec![0_f64; total];
        let reindex = |layers: &Vec<Vec<usize>>, pos: &mut Vec<f64>| {
            for layer in layers {
                for (k, &v) in layer.iter().enumerate() {
                    pos[v] = k as f64;
                }
            }
        };
        reindex(&layers, &mut pos);
        let crossings = |layers: &Vec<Vec<usize>>, pos: &Vec<f64>| -> usize {
            let mut count = 0;
            for layer in layers.iter().take(layers.len().saturating_sub(1)) {
                let es: Vec<(f64, f64)> = layer
                    .iter()
                    .flat_map(|&a| succs[a].iter().map(move |&b| (a, b)))
                    .map(|(a, b)| (pos[a], pos[b]))
                    .collect();
                for x in 0..es.len() {
                    for y in x + 1..es.len() {
                        if (es[x].0 - es[y].0) * (es[x].1 - es[y].1) < 0.0_f64 {
                            count += 1;
                        }
                    }
                }
            }
            count
        };
        let mut best = layers.clone();
        let mut best_cross = crossings(&layers, &pos);
        for sweep in 0_i32..16_i32 {
            let down = sweep % 2_i32 == 0_i32;
            let range: Vec<usize> = if down {
                (1..ranks).collect()
            } else {
                (0..ranks.saturating_sub(1)).rev().collect()
            };
            for r in range {
                let mut keyed: Vec<(Vec<(f64, usize)>, usize)> = layers[r]
                    .iter()
                    .map(|&v| {
                        let near = if down { &preds[v] } else { &succs[v] };
                        let bary = if near.is_empty() {
                            pos[v]
                        } else {
                            near.iter().map(|&u| pos[u]).sum::<f64>() / near.len() as f64
                        };
                        (vec![(bary, v)], v)
                    })
                    .collect();
                // Members of one subgraph sort by the subgraph's mean, so they
                // stay together.
                let mut group_mean: BTreeMap<usize, (f64, usize)> = BTreeMap::new();
                for (key, v) in &keyed {
                    let mut c = cluster_of[*v];
                    while let Some(ci) = c {
                        let e = group_mean.entry(ci).or_insert((0.0_f64, 0));
                        e.0 += key[0].0;
                        e.1 += 1;
                        c = g.clusters[ci].parent;
                    }
                }
                for (key, v) in &mut keyed {
                    let mut path = Vec::new();
                    let mut c = cluster_of[*v];
                    while let Some(ci) = c {
                        let (sum, n) = group_mean[&ci];
                        path.push((sum / n as f64, ci));
                        c = g.clusters[ci].parent;
                    }
                    path.reverse();
                    path.push(key[0]);
                    *key = path;
                }
                keyed.sort_by(|a, b| {
                    for (x, y) in a.0.iter().zip(&b.0) {
                        match x
                            .0
                            .partial_cmp(&y.0)
                            .unwrap_or(core::cmp::Ordering::Equal)
                            .then(x.1.cmp(&y.1))
                        {
                            core::cmp::Ordering::Equal => {}
                            other => return other,
                        }
                    }
                    a.0.len().cmp(&b.0.len())
                });
                layers[r] = keyed.into_iter().map(|(_, v)| v).collect();
                for (k, &v) in layers[r].iter().enumerate() {
                    pos[v] = k as f64;
                }
            }
            let c = crossings(&layers, &pos);
            if c < best_cross {
                best_cross = c;
                best.clone_from(&layers);
            }
        }
        layers = best;
        // 5. Place: pack each rank, then move each node toward its neighbours'
        //    median in passes down and up, never closer to its neighbour than
        //    the gap.
        let across = |v: usize| if horizontal { size[v].1 } else { size[v].0 };
        let along = |v: usize| if horizontal { size[v].0 } else { size[v].1 };
        let gap = |a: usize, b: usize| -> f64 {
            let wa = if a < real || size[a].0 > 2.0_f64 {
                1.0_f64
            } else {
                0.0_f64
            };
            let wb = if b < real || size[b].0 > 2.0_f64 {
                1.0_f64
            } else {
                0.0_f64
            };
            let base = if wa + wb >= 2.0_f64 {
                28.0_f64
            } else if wa + wb >= 1.0_f64 {
                18.0_f64
            } else {
                10.0_f64
            };
            let edge = if top_cluster(g, cluster_of[a]) == top_cluster(g, cluster_of[b]) {
                0.0_f64
            } else {
                26.0_f64
            };
            base + edge
        };
        let mut x = vec![0_f64; total];
        for layer in &layers {
            let mut cur = 0.0_f64;
            for (k, &v) in layer.iter().enumerate() {
                if k > 0 {
                    let u = layer[k - 1];
                    cur += across(u) / 2.0_f64 + gap(u, v) + across(v) / 2.0_f64;
                }
                x[v] = cur;
            }
        }
        for pass in 0_i32..40_i32 {
            let down = pass % 2_i32 == 0_i32;
            let range: Vec<usize> = if down {
                (0..ranks).collect()
            } else {
                (0..ranks).rev().collect()
            };
            for r in range {
                let layer = &layers[r];
                if layer.is_empty() {
                    continue;
                }
                let want: Vec<f64> = layer
                    .iter()
                    .map(|&v| {
                        let mut near: Vec<f64> = if down {
                            preds[v].iter().map(|&u| x[u]).collect()
                        } else {
                            succs[v].iter().map(|&u| x[u]).collect()
                        };
                        if near.is_empty() || v >= real {
                            near.extend(if down {
                                succs[v].iter().map(|&u| x[u]).collect::<Vec<_>>()
                            } else {
                                preds[v].iter().map(|&u| x[u]).collect()
                            });
                        }
                        if near.is_empty() {
                            return x[v];
                        }
                        near.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
                        let m = near.len();
                        if m % 2 == 1 {
                            near[m / 2]
                        } else {
                            f64::midpoint(near[m / 2 - 1], near[m / 2])
                        }
                    })
                    .collect();
                let n = layer.len();
                let sep = |k: usize| {
                    across(layer[k - 1]) / 2.0_f64
                        + gap(layer[k - 1], layer[k])
                        + across(layer[k]) / 2.0_f64
                };
                let mut left = want.clone();
                for k in 1..n {
                    left[k] = left[k].max(left[k - 1] + sep(k));
                }
                let mut right = want;
                for k in (0..n.saturating_sub(1)).rev() {
                    right[k] = right[k].min(right[k + 1] - sep(k + 1));
                }
                for (k, &v) in layer.iter().enumerate() {
                    x[v] = f64::midpoint(left[k], right[k]);
                }
            }
        }
        // Ranks: each as thick as its thickest node, with room between for
        // edges.
        let rank_gap = if horizontal { 56.0_f64 } else { 46.0_f64 }
            + if g.clusters.is_empty() {
                0.0_f64
            } else {
                16.0_f64
            };
        let mut rank_pos = vec![0_f64; ranks];
        let mut thick = vec![0_f64; ranks];
        for (r, layer) in layers.iter().enumerate() {
            thick[r] = layer.iter().map(|&v| along(v)).fold(0.0_f64, f64::max).max(
                if layer.iter().all(|&v| v >= real) {
                    6.0
                } else {
                    0.0
                },
            );
        }
        let mut cur = 0.0_f64;
        for r in 0..ranks {
            rank_pos[r] = cur + thick[r] / 2.0_f64;
            cur += thick[r] + rank_gap;
        }
        // Normalise and map to the page's axes.
        let pad = 20.0_f64;
        let min_x = (0..total)
            .map(|v| x[v] - across(v) / 2.0_f64)
            .fold(f64::INFINITY, f64::min);
        let mut boxes: Vec<Placed> = (0..total)
            .map(|v| {
                let u = x[v] - min_x + pad;
                let w = rank_pos[rank[v]] + pad;
                let (bw, bh) = size[v];
                if horizontal {
                    Placed {
                        x: w,
                        y: u,
                        width: bw,
                        height: bh,
                    }
                } else {
                    Placed {
                        x: u,
                        y: w,
                        width: bw,
                        height: bh,
                    }
                }
            })
            .collect();
        // Subgraph boxes around their members, inner ones first, titles on top.
        let mut clusters: Vec<Option<(f64, f64, f64, f64)>> = vec![None; g.clusters.len()];
        let depth = |c: usize| {
            let mut d = 0_i32;
            let mut p = g.clusters[c].parent;
            while let Some(q) = p {
                d += 1_i32;
                p = g.clusters[q].parent;
            }
            d
        };
        let mut by_depth: Vec<usize> = (0..g.clusters.len()).collect();
        by_depth.sort_by_key(|c| core::cmp::Reverse(depth(*c)));
        for &c in &by_depth {
            let mut b: Option<(f64, f64, f64, f64)> = None;
            let mut grow = |l: f64, t: f64, r: f64, bt: f64| {
                b = Some(match b {
                    None => (l, t, r, bt),
                    Some((l0, t0, r0, b0)) => (l0.min(l), t0.min(t), r0.max(r), b0.max(bt)),
                });
            };
            for v in 0..real {
                if cluster_of[v] == Some(c) {
                    let p = boxes[v];
                    grow(
                        p.x - p.width / 2.0_f64,
                        p.y - p.height / 2.0_f64,
                        p.x + p.width / 2.0_f64,
                        p.y + p.height / 2.0_f64,
                    );
                }
            }
            for (k, inner) in g.clusters.iter().enumerate() {
                if inner.parent == Some(c) {
                    if let Some((l, t, r, bt)) = clusters[k] {
                        grow(l, t, r, bt);
                    }
                }
            }
            if let Some((l, t, r, bt)) = b {
                let (tw, th) = label_size(&g.clusters[c].title, 13.0);
                let title_h = if g.clusters[c].title.iter().all(String::is_empty) {
                    0.0_f64
                } else {
                    th + 6.0_f64
                };
                let (l, t, r, bt) = (
                    l - 14.0_f64,
                    t - 12.0_f64 - title_h,
                    r + 14.0_f64,
                    bt + 12.0_f64,
                );
                let r = r.max(l + tw + 28.0);
                clusters[c] = Some((l, t, r, bt));
            }
        }
        // Everything inside the canvas.
        let mut left = f64::INFINITY;
        let mut top = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        let mut bottom = f64::NEG_INFINITY;
        for p in &boxes {
            left = left.min(p.x - p.width / 2.0);
            top = top.min(p.y - p.height / 2.0);
            right = right.max(p.x + p.width / 2.0);
            bottom = bottom.max(p.y + p.height / 2.0);
        }
        for (l, t, r, b) in clusters.iter().flatten() {
            left = left.min(*l);
            top = top.min(*t);
            right = right.max(*r);
            bottom = bottom.max(*b);
        }
        let (dx, dy) = (pad - left, pad - top);
        for p in &mut boxes {
            p.x += dx;
            p.y += dy;
        }
        for c in clusters.iter_mut().flatten() {
            *c = (c.0 + dx, c.1 + dy, c.2 + dx, c.3 + dy);
        }
        let mut width = 2.0_f64.mul_add(pad, right - left);
        let mut height = 2.0_f64.mul_add(pad, bottom - top);
        // Bottom-up and right-to-left are the mirror images.
        if g.dir == Dir::BottomUp {
            for p in &mut boxes {
                p.y = height - p.y;
            }
            for c in clusters.iter_mut().flatten() {
                *c = (c.0, height - c.3, c.2, height - c.1);
            }
        }
        if g.dir == Dir::RightLeft {
            for p in &mut boxes {
                p.x = width - p.x;
            }
            for c in clusters.iter_mut().flatten() {
                *c = (width - c.2, c.1, width - c.0, c.3);
            }
        }
        // Edge paths: through the waypoints, ends on the outlines.
        let mut paths = Vec::with_capacity(g.edges.len());
        let mut label_at = vec![None; g.edges.len()];
        for (k, e) in g.edges.iter().enumerate() {
            if e.from == e.to {
                // A loop on one node: out of its side and back.
                let p = boxes[e.from];
                let (sx, sy) = (p.x + p.width / 2.0_f64, p.y);
                paths.push(vec![
                    (sx, sy - 8.0_f64),
                    (sx + 30.0_f64, sy - 22.0_f64),
                    (sx + 30.0_f64, sy + 22.0_f64),
                    (sx, sy + 8.0_f64),
                ]);
                label_at[k] = Some((sx + 34.0_f64, sy));
                width = width.max(sx + 60.0);
                continue;
            }
            let pts: Vec<(f64, f64)> = chain_of[k]
                .iter()
                .map(|&v| (boxes[v].x, boxes[v].y))
                .collect();
            let mut pts = pts;
            let n = pts.len();
            if n >= 2 {
                pts[0] = port(&g.nodes[e.from], boxes[e.from], pts[1], horizontal);
                pts[n - 1] = port(&g.nodes[e.to], boxes[e.to], pts[n - 2], horizontal);
            }
            if let Some(d) = label_node[k] {
                label_at[k] = Some((boxes[d].x, boxes[d].y));
            } else if e.label.is_some() && n >= 2 {
                label_at[k] = Some((
                    f64::midpoint(pts[0].0, pts[n - 1].0),
                    f64::midpoint(pts[0].1, pts[n - 1].1),
                ));
            }
            paths.push(pts);
        }
        height = height.max(1.0);
        Layout {
            boxes,
            paths,
            label_at,
            clusters,
            width,
            height,
        }
    }

    /// The subgraph holding both, innermost.
    fn common_cluster(g: &Graph, a: Option<usize>, b: Option<usize>) -> Option<usize> {
        let mut chain_a = Vec::new();
        let mut c = a;
        while let Some(x) = c {
            chain_a.push(x);
            c = g.clusters[x].parent;
        }
        let mut c = b;
        while let Some(x) = c {
            if chain_a.contains(&x) {
                return Some(x);
            }
            c = g.clusters[x].parent;
        }
        None
    }

    /// The outermost subgraph holding a node.
    fn top_cluster(g: &Graph, c: Option<usize>) -> Option<usize> {
        let mut c = c?;
        while let Some(p) = g.clusters[c].parent {
            c = p;
        }
        Some(c)
    }

    /// Where an edge meets a node: on the face toward the next rank (top or
    /// bottom, or a side when ranks run across), near its middle, leaning
    /// toward the other end; a pointed shape at its tip.
    fn port(node: &Node, p: Placed, to: (f64, f64), horizontal: bool) -> (f64, f64) {
        let pointed = matches!(
            node.shape,
            Shape::Diamond | Shape::Circle | Shape::DoubleCircle | Shape::Hexagon
        );
        if horizontal {
            let side = if to.0 >= p.x { 1.0 } else { -1.0 };
            let lean = if pointed {
                0.0
            } else {
                ((to.1 - p.y) * 0.25).clamp(-p.height * 0.3, p.height * 0.3)
            };
            if (to.0 - p.x).abs() < p.width / 2.0 {
                return clip(node, p, to);
            }
            (p.x + side * p.width / 2.0, p.y + lean)
        } else {
            let side = if to.1 >= p.y { 1.0 } else { -1.0 };
            let lean = if pointed {
                0.0
            } else {
                ((to.0 - p.x) * 0.25).clamp(-p.width * 0.3, p.width * 0.3)
            };
            if (to.1 - p.y).abs() < p.height / 2.0 {
                return clip(node, p, to);
            }
            (p.x + lean, p.y + side * p.height / 2.0)
        }
    }

    /// Where the line from a node's centre toward `to` leaves its outline.
    fn clip(node: &Node, p: Placed, to: (f64, f64)) -> (f64, f64) {
        let (dx, dy) = (to.0 - p.x, to.1 - p.y);
        if dx.abs() < 1e-9_f64 && dy.abs() < 1e-9_f64 {
            return (p.x, p.y);
        }
        let (hw, hh) = (p.width / 2.0_f64, p.height / 2.0_f64);
        let t = match node.shape {
            Shape::Diamond => 1.0_f64 / (dx.abs() / hw + dy.abs() / hh),
            Shape::Circle | Shape::DoubleCircle => hw / dx.hypot(dy),
            _ => {
                let tx = if dx.abs() > 1e-9_f64 {
                    hw / dx.abs()
                } else {
                    f64::INFINITY
                };
                let ty = if dy.abs() > 1e-9_f64 {
                    hh / dy.abs()
                } else {
                    f64::INFINITY
                };
                tx.min(ty)
            }
        };
        (f64::mul_add(dx, t, p.x), f64::mul_add(dy, t, p.y))
    }

    /// A path through points, smoothed: straight from end to end when two,
    /// a curve through the waypoints otherwise.
    fn path_d(pts: &[(f64, f64)], horizontal: bool) -> String {
        let mut d = String::new();
        let Some(first) = pts.first() else { return d };
        put!(d, "M{:.1} {:.1}", first.0, first.1);
        if pts.len() == 2 {
            let (a, b) = (pts[0], pts[1]);
            let straight = (a.0 - b.0).abs() < 1.0_f64 || (a.1 - b.1).abs() < 1.0_f64;
            if straight {
                put!(d, "L{:.1} {:.1}", b.0, b.1);
            } else if horizontal {
                let mx = f64::midpoint(a.0, b.0);
                put!(
                    d,
                    "C{mx:.1} {:.1} {mx:.1} {:.1} {:.1} {:.1}",
                    a.1,
                    b.1,
                    b.0,
                    b.1
                );
            } else {
                let my = f64::midpoint(a.1, b.1);
                put!(
                    d,
                    "C{:.1} {my:.1} {:.1} {my:.1} {:.1} {:.1}",
                    a.0,
                    b.0,
                    b.0,
                    b.1
                );
            }
            return d;
        }
        // Catmull-Rom through the points, as cubic curves.
        for k in 0..pts.len() - 1 {
            let p0 = if k == 0 { pts[0] } else { pts[k - 1] };
            let (p1, p2) = (pts[k], pts[k + 1]);
            let p3 = if k + 2 < pts.len() { pts[k + 2] } else { p2 };
            let c1 = (
                p1.0 + (p2.0 - p0.0) / 6.0_f64,
                p1.1 + (p2.1 - p0.1) / 6.0_f64,
            );
            let c2 = (
                p2.0 - (p3.0 - p1.0) / 6.0_f64,
                p2.1 - (p3.1 - p1.1) / 6.0_f64,
            );
            put!(
                d,
                "C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
                c1.0,
                c1.1,
                c2.0,
                c2.1,
                p2.0,
                p2.1
            );
        }
        d
    }

    /// A node's outline as SVG.
    #[expect(
        clippy::many_single_char_names,
        reason = "geometry and scanning: x, y, w, h, i, j are the domain's names"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    fn outline(out: &mut String, node: &Node, p: Placed) {
        let (x, y, w, h) = (
            p.x - p.width / 2.0_f64,
            p.y - p.height / 2.0_f64,
            p.width,
            p.height,
        );
        let (cx, cy) = (p.x, p.y);
        let poly = |out: &mut String, pts: &[(f64, f64)]| {
            out.push_str("<polygon class=\"shape\" points=\"");
            for (k, (px, py)) in pts.iter().enumerate() {
                put!(out, "{}{px:.1},{py:.1}", if k > 0 { " " } else { "" });
            }
            out.push_str("\"/>");
        };
        match node.shape {
            Shape::Rect => {
                put!(
                    out,
                    "<rect class=\"shape\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"4\"/>"
                );
            }
            Shape::Round => {
                put!(
                    out,
                    "<rect class=\"shape\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"12\"/>"
                );
            }
            Shape::Stadium => {
                put!(
                    out,
                    "<rect class=\"shape\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" rx=\"{:.1}\"/>",
                    h / 2.0_f64
                );
            }
            Shape::Subroutine => {
                put!(
                    out,
                    "<rect class=\"shape\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\"/><path class=\"shape-line\" d=\"M{:.1} {y:.1}V{:.1}M{:.1} {y:.1}V{:.1}\"/>",
                    x + 8.0_f64,
                    y + h,
                    x + w - 8.0_f64,
                    y + h
                );
            }
            Shape::Cylinder => {
                let ry = 8.0_f64;
                put!(
                    out,
                    "<path class=\"shape\" d=\"M{x:.1} {t:.1}A{rx:.1} {ry} 0 0 1 {r:.1} {t:.1}V{b:.1}A{rx:.1} {ry} 0 0 1 {x:.1} {b:.1}Z\"/><path class=\"shape-line\" d=\"M{x:.1} {t:.1}A{rx:.1} {ry} 0 0 0 {r:.1} {t:.1}\"/>",
                    t = y + ry,
                    b = y + h - ry,
                    r = x + w,
                    rx = w / 2.0_f64
                );
            }
            Shape::Circle => {
                put!(
                    out,
                    "<circle class=\"shape\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\"/>",
                    w / 2.0_f64
                );
            }
            Shape::DoubleCircle => {
                put!(
                    out,
                    "<circle class=\"shape\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\"/><circle class=\"shape-line\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\"/>",
                    w / 2.0_f64,
                    w / 2.0_f64 - 5.0_f64
                );
            }
            Shape::Diamond => poly(out, &[(cx, y), (x + w, cy), (cx, y + h), (x, cy)]),
            Shape::Hexagon => poly(out, &[
                (x + 14.0_f64, y),
                (x + w - 14.0_f64, y),
                (x + w, cy),
                (x + w - 14.0_f64, y + h),
                (x + 14.0_f64, y + h),
                (x, cy),
            ]),
            Shape::Flag => poly(out, &[
                (x, y),
                (x + w, y),
                (x + w, y + h),
                (x, y + h),
                (x + 14.0_f64, cy),
            ]),
            Shape::Lean => poly(out, &[
                (x + 14.0_f64, y),
                (x + w, y),
                (x + w - 14.0_f64, y + h),
                (x, y + h),
            ]),
            Shape::LeanBack => poly(out, &[
                (x, y),
                (x + w - 14.0_f64, y),
                (x + w, y + h),
                (x + 14.0_f64, y + h),
            ]),
            Shape::Trapezoid => poly(out, &[
                (x + 14.0_f64, y),
                (x + w - 14.0_f64, y),
                (x + w, y + h),
                (x, y + h),
            ]),
            Shape::TrapezoidDown => poly(out, &[
                (x, y),
                (x + w, y),
                (x + w - 14.0_f64, y + h),
                (x + 14.0_f64, y + h),
            ]),
        }
    }

    /// A flowchart as SVG.
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    pub(super) fn draw(g: &Graph, n: usize) -> String {
        let lay = layout(g);
        let horizontal = g.dir.horizontal();
        let names: Vec<String> = g.nodes.iter().map(|node| node.label.join(" ")).collect();
        let title = g.title.clone().unwrap_or_else(|| {
            let first: Vec<&str> = names.iter().take(3).map(String::as_str).collect();
            format!(
                "{}{}",
                first.join(", "),
                if names.len() > 3 { ", …" } else { "" }
            )
        });
        // The description says what the picture shows, edge by edge.
        let desc = g.desc.clone().unwrap_or_else(|| {
            let mut s = String::new();
            for e in g.edges.iter().take(60) {
                let via = e
                    .label
                    .as_ref()
                    .map(|l| format!(" ({})", l.join(" ")))
                    .unwrap_or_default();
                put!(s, "{} → {}{via}; ", names[e.from], names[e.to]);
            }
            s.trim_end_matches("; ").to_owned() + "."
        });
        let mut out = String::new();
        svg_open(&mut out, n, lay.width, lay.height, &title, &desc);
        // Subgraphs, outermost first so inner ones lie on top.
        let mut order: Vec<usize> = (0..g.clusters.len()).collect();
        order.sort_by_key(|&c| {
            let mut d = 0_i32;
            let mut p = g.clusters[c].parent;
            while let Some(q) = p {
                d += 1_i32;
                p = g.clusters[q].parent;
            }
            d
        });
        for c in order {
            if let Some((l, t, r, b)) = lay.clusters[c] {
                put!(
                    out,
                    "<g class=\"cluster\"><rect x=\"{l:.1}\" y=\"{t:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\"/>",
                    r - l,
                    b - t
                );
                let title = &g.clusters[c].title;
                if !title.iter().all(String::is_empty) {
                    let (_, th) = label_size(title, 13.0);
                    text_block(
                        &mut out,
                        title,
                        f64::midpoint(l, r),
                        t + 6.0 + th / 2.0,
                        "cluster-title",
                    );
                }
                out.push_str("</g>");
            }
        }
        // Edges, then their labels over them.
        for (k, e) in g.edges.iter().enumerate() {
            if e.stroke == Stroke::Invisible {
                continue;
            }
            let class = match e.stroke {
                Stroke::Dotted => "edge dotted",
                Stroke::Thick => "edge thick",
                _ => "edge",
            };
            let mark = |h: Head| match h {
                Head::Arrow => Some('a'),
                Head::Circle => Some('o'),
                Head::Cross => Some('x'),
                Head::None => None,
            };
            let end = mark(e.head)
                .map(|m| format!(" marker-end=\"url(#dg{n}-{m})\""))
                .unwrap_or_default();
            let start = mark(e.tail)
                .map(|m| format!(" marker-start=\"url(#dg{n}-{m})\""))
                .unwrap_or_default();
            put!(
                out,
                "<path class=\"{class}\" d=\"{}\"{start}{end}/>",
                path_d(&lay.paths[k], horizontal)
            );
        }
        for (k, e) in g.edges.iter().enumerate() {
            if let (Some(label), Some((x, y))) = (&e.label, lay.label_at[k]) {
                let (w, h) = label_size(label, 13.0);
                put!(
                    out,
                    "<rect class=\"edge-label-bg\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\"/>",
                    x - w / 2.0_f64 - 5.0_f64,
                    y - h / 2.0_f64 - 2.0_f64,
                    w + 10.0_f64,
                    h + 4.0_f64
                );
                text_block(&mut out, label, x, y, "edge-label");
            }
        }
        for (v, node) in g.nodes.iter().enumerate() {
            let p = lay.boxes[v];
            put!(out, "<g class=\"node\" data-id=\"{}\">", esc(&node.id));
            outline(&mut out, node, p);
            text_block(&mut out, &node.label, p.x, p.y, "label");
            out.push_str("</g>");
        }
        out.push_str("</svg></figure>");
        out
    }
}

/// Sequence diagrams.
mod seq {
    use super::{BTreeMap, esc, label_lines, label_size, svg_open, text_block, text_width};
    use crate::text::put;

    /// A message's line and head.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(super) struct Arrow {
        /// Drawn dotted.
        dotted: bool,
        /// The head: `>` filled, `)` open, `x` a cross, ` ` none.
        head: char,
    }

    /// One step of the diagram, in order.
    #[derive(Debug)]
    pub(super) enum Step {
        /// A message between two participants (or to itself).
        Message {
            /// The sender, by its place among the actors.
            from: usize,
            /// The receiver.
            to: usize,
            /// The arrow drawn.
            arrow: Arrow,
            /// The words on it, line by line.
            text: Vec<String>,
            /// Whether it activates (`+`) or deactivates (`-`) the receiver.
            activate: Option<bool>,
        },
        /// A note beside or over participants.
        Note {
            /// The actors it spans, first and last.
            at: (usize, usize),
            /// The side: -1 left of, 0 over, 1 right of.
            side: i8,
            /// The words, line by line.
            text: Vec<String>,
        },
        /// `loop`, `alt`, `opt`, `par`, `critical`, `break`, `rect`: a frame
        /// opens.
        Open {
            /// The frame's keyword.
            kind: String,
            /// Its condition or title.
            text: String,
        },
        /// `else`, `and`, `option`: the frame divides.
        Divide {
            /// The words on the dividing line.
            text: String,
        },
        /// `end`
        Close,
        /// `activate` / `deactivate`
        Activation {
            /// The actor.
            who: usize,
            /// On, or off.
            on: bool,
        },
    }

    /// A sequence diagram as read.
    #[derive(Debug)]
    pub(super) struct Diagram {
        /// The actors: id, label lines, and whether drawn as a person.
        pub(super) actors: Vec<(String, Vec<String>, bool)>,
        /// What happens, in order.
        pub(super) steps: Vec<Step>,
        /// Messages are numbered (`autonumber`).
        pub(super) numbered: bool,
        /// The author's title, if given.
        pub(super) title: Option<String>,
    }

    /// The arrows of a sequence diagram: how each is written, whether dotted,
    /// its head.
    const ARROWS: &[(&str, bool, char)] = &[
        ("-->>", true, '>'),
        ("->>", false, '>'),
        ("--x", true, 'x'),
        ("-x", false, 'x'),
        ("--)", true, ')'),
        ("-)", false, ')'),
        ("-->", true, ' '),
        ("->", false, ' '),
    ];

    /// The diagram in a block's lines after its header.
    #[expect(
        clippy::too_many_lines,
        reason = "one dispatch over the kinds it handles, read top to bottom"
    )]
    pub(super) fn parse(lines: &[&str]) -> Result<Diagram, String> {
        let mut d = Diagram {
            actors: Vec::new(),
            steps: Vec::new(),
            numbered: false,
            title: None,
        };
        let mut ids: BTreeMap<String, usize> = BTreeMap::new();
        let mut open = 0_usize;
        let mut boxes = 0_usize;
        let mut ensure = |d: &mut Diagram, id: &str, label: Option<&str>, actor: bool| -> usize {
            let id = id.trim().to_owned();
            if let Some(&k) = ids.get(&id) {
                if let Some(l) = label {
                    d.actors[k].1 = label_lines(l);
                }
                return k;
            }
            d.actors
                .push((id.clone(), label_lines(label.unwrap_or(&id)), actor));
            ids.insert(id, d.actors.len() - 1);
            d.actors.len() - 1
        };
        for line in lines {
            let word = line.split_whitespace().next().unwrap_or_default();
            let rest = line[word.len()..].trim();
            match word {
                "participant" | "actor" => {
                    let (id, label) = match rest.split_once(" as ") {
                        Some((id, label)) => (id, Some(label)),
                        None => (rest, None),
                    };
                    ensure(&mut d, id, label, word == "actor");
                }
                "autonumber" => d.numbered = true,
                "title" => d.title = Some(rest.trim_start_matches(':').trim().to_owned()),
                "activate" | "deactivate" => {
                    let who = ensure(&mut d, rest, None, false);
                    d.steps.push(Step::Activation {
                        who,
                        on: word == "activate",
                    });
                }
                "box" => boxes += 1,
                "loop" | "alt" | "opt" | "par" | "critical" | "break" | "rect" => {
                    open += 1;
                    d.steps.push(Step::Open {
                        kind: word.to_owned(),
                        text: if word == "rect" {
                            String::new()
                        } else {
                            rest.to_owned()
                        },
                    });
                }
                "else" | "and" | "option" => d.steps.push(Step::Divide {
                    text: rest.to_owned(),
                }),
                "end" => {
                    if open > 0 {
                        open -= 1;
                        d.steps.push(Step::Close);
                    } else {
                        boxes = boxes.saturating_sub(1);
                    }
                }
                _ if word.eq_ignore_ascii_case("note") => {
                    let (place, text) = rest
                        .split_once(':')
                        .ok_or_else(|| format!("a note without text: `{line}`"))?;
                    let place = place.trim();
                    let (side, who) = if let Some(w) = place.strip_prefix("right of") {
                        (1, w)
                    } else if let Some(w) = place.strip_prefix("left of") {
                        (-1, w)
                    } else if let Some(w) = place.strip_prefix("over") {
                        (0, w)
                    } else {
                        return Err(format!("cannot place the note `{line}`"));
                    };
                    let mut names = who.split(',');
                    let a = ensure(&mut d, names.next().unwrap_or_default(), None, false);
                    let b = names.next().map_or(a, |n| ensure(&mut d, n, None, false));
                    d.steps.push(Step::Note {
                        at: (a.min(b), a.max(b)),
                        side,
                        text: label_lines(text),
                    });
                }
                "accTitle:" | "accDescr:" | "links" | "link" | "destroy" | "create" => {}
                _ => {
                    let Some((head, text)) = line.split_once(':').or(Some((line, ""))) else {
                        continue;
                    };
                    let Some((at, arrow)) = ARROWS
                        .iter()
                        .filter_map(|(a, dotted, h)| head.find(a).map(|at| (at, (a, *dotted, *h))))
                        .min_by_key(|(at, (a, _, _))| (*at, core::cmp::Reverse(a.len())))
                    else {
                        return Err(format!("cannot read `{line}`"));
                    };
                    let from = head[..at].trim();
                    let mut to = head[at + arrow.0.len()..].trim();
                    let mut activate = None;
                    if let Some(t) = to.strip_prefix('+') {
                        activate = Some(true);
                        to = t.trim();
                    } else if let Some(t) = to.strip_prefix('-') {
                        activate = Some(false);
                        to = t.trim();
                    }
                    if from.is_empty() || to.is_empty() {
                        return Err(format!("cannot read `{line}`"));
                    }
                    let a = ensure(&mut d, from, None, false);
                    let b = ensure(&mut d, to, None, false);
                    d.steps.push(Step::Message {
                        from: a,
                        to: b,
                        arrow: Arrow {
                            dotted: arrow.1,
                            head: arrow.2,
                        },
                        text: label_lines(text),
                        activate,
                    });
                }
            }
        }
        let _ = boxes;
        if d.actors.is_empty() {
            return Err(String::from("the sequence diagram has no participants"));
        }
        Ok(d)
    }

    /// A frame being drawn: its kind, its text, its top, its divisions, and the
    /// leftmost and rightmost participant it covers.
    type Frame = (String, String, f64, Vec<(f64, String)>, usize, usize);

    /// A sequence diagram as SVG.
    #[expect(clippy::too_many_lines, reason = "one pass over the steps, row by row")]
    #[expect(
        clippy::many_single_char_names,
        reason = "geometry and scanning: x, y, w, h, i, j are the domain's names"
    )]
    pub(super) fn draw(d: &Diagram, n: usize) -> String {
        let count = d.actors.len();
        let box_w: Vec<f64> = d
            .actors
            .iter()
            .map(|(_, l, _)| (label_size(l, 14.0).0 + 28.0).max(96.0))
            .collect();
        let box_h = d
            .actors
            .iter()
            .map(|(_, l, _)| label_size(l, 14.0).1 + 22.0_f64)
            .fold(40.0_f64, f64::max);
        // Columns: far enough apart for their boxes and for every message
        // between them.
        let mut gaps: Vec<f64> = (1..count)
            .map(|k| f64::midpoint(box_w[k - 1], box_w[k]) + 36.0_f64)
            .collect();
        let mut self_room = vec![0_f64; count];
        for step in &d.steps {
            match step {
                Step::Message {
                    from,
                    to,
                    text,
                    arrow: _,
                    activate: _,
                } if from != to => {
                    let (a, b) = (*from.min(to), *from.max(to));
                    let need = label_size(text, 13.0).0
                        + 40.0_f64
                        + if d.numbered { 24.0_f64 } else { 0.0_f64 };
                    let have: f64 = gaps[a..b].iter().sum();
                    if have < need {
                        gaps[b - 1] += need - have;
                    }
                }
                Step::Message {
                    from,
                    text,
                    to: _,
                    arrow: _,
                    activate: _,
                } => {
                    self_room[*from] = self_room[*from].max(label_size(text, 13.0).0 + 50.0);
                }
                Step::Note { at, side, text } if *side != 0 || at.0 == at.1 => {
                    let w = label_size(text, 13.0).0 + 24.0_f64;
                    let k = at.0;
                    if *side > 0 && k + 1 < count {
                        gaps[k] = gaps[k].max(w + 30.0);
                    } else if *side < 0 && k > 0 {
                        gaps[k - 1] = gaps[k - 1].max(w + 30.0);
                    }
                }
                _ => {}
            }
        }
        for k in 0..count.saturating_sub(1) {
            gaps[k] = gaps[k].max(self_room[k] + box_w[k + 1] / 2.0);
        }
        let pad = 20.0_f64;
        let mut xs = vec![pad + box_w[0] / 2.0_f64];
        for k in 1..count {
            xs.push(xs[k - 1] + gaps[k - 1]);
        }
        let mut body = String::new();
        let mut y = pad + box_h + 24.0_f64;
        let mut frames: Vec<Frame> = Vec::new();
        let mut active: Vec<Vec<f64>> = vec![Vec::new(); count];
        let mut bars: Vec<(usize, f64, f64, usize)> = Vec::new();
        let mut number = 0_usize;
        let mut right_edge = xs[count - 1] + box_w[count - 1] / 2.0_f64;
        let mut left_edge = pad;
        let touch = |frames: &mut Vec<Frame>, a: usize, b: usize| {
            for f in frames.iter_mut() {
                f.4 = f.4.min(a);
                f.5 = f.5.max(b);
            }
        };
        let bar_x =
            |who: usize, active: &Vec<Vec<f64>>| 5.0_f64.mul_add(active[who].len() as f64, xs[who]);
        for step in &d.steps {
            match step {
                Step::Message {
                    from,
                    to,
                    arrow,
                    text,
                    activate,
                } => {
                    let (_, th) = label_size(text, 13.0);
                    let has_text = !(text.len() == 1 && text[0].is_empty());
                    y += if has_text { th + 6.0_f64 } else { 6.0_f64 };
                    number += 1;
                    let class = if arrow.dotted { "msg dotted" } else { "msg" };
                    let marker = match arrow.head {
                        '>' => format!(" marker-end=\"url(#dg{n}-a)\""),
                        'x' => format!(" marker-end=\"url(#dg{n}-x)\""),
                        ')' => format!(" marker-end=\"url(#dg{n}-v)\""),
                        _ => String::new(),
                    };
                    touch(&mut frames, *from.min(to), *from.max(to));
                    if from == to {
                        let x = bar_x(*from, &active);
                        put!(
                            body,
                            "<path class=\"{class}\" d=\"M{x:.1} {y:.1}H{:.1}V{:.1}H{:.1}\"{marker}/>",
                            x + 36.0_f64,
                            y + 22.0_f64,
                            x + 2.0_f64
                        );
                        if has_text {
                            let w = label_size(text, 13.0).0;
                            text_block(
                                &mut body,
                                text,
                                x + 44.0 + w / 2.0,
                                y - th / 2.0 + 2.0,
                                "msg-text",
                            );
                            right_edge = right_edge.max(x + 52.0 + w);
                        }
                        if d.numbered {
                            put!(
                                body,
                                "<circle class=\"num\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"9\"/><text class=\"num-text\" x=\"{x:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\">{number}</text>"
                            );
                        }
                        y += 34.0_f64;
                    } else {
                        let x1 = if from < to {
                            bar_x(*from, &active)
                        } else {
                            10.0_f64.mul_add(
                                -f64::from(u8::from(!active[*from].is_empty())),
                                bar_x(*from, &active),
                            )
                        };
                        let x2 = if from < to {
                            xs[*to] - 2.0_f64
                        } else {
                            xs[*to] + 2.0_f64
                        };
                        put!(
                            body,
                            "<path class=\"{class}\" d=\"M{x1:.1} {y:.1}H{x2:.1}\"{marker}/>"
                        );
                        if has_text {
                            text_block(
                                &mut body,
                                text,
                                f64::midpoint(xs[*from], xs[*to]),
                                y - th / 2.0 - 2.0,
                                "msg-text",
                            );
                        }
                        if d.numbered {
                            put!(
                                body,
                                "<circle class=\"num\" cx=\"{x1:.1}\" cy=\"{y:.1}\" r=\"9\"/><text class=\"num-text\" x=\"{x1:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\">{number}</text>"
                            );
                        }
                        y += 22.0_f64;
                    }
                    match activate {
                        Some(true) => active[*to].push(y - 16.0_f64),
                        Some(false) => {
                            if let Some(start) = active[*from].pop() {
                                bars.push((*from, start, y - 16.0_f64, active[*from].len()));
                            }
                        }
                        None => {}
                    }
                }
                Step::Activation { who, on } => {
                    if *on {
                        active[*who].push(y);
                    } else if let Some(start) = active[*who].pop() {
                        bars.push((*who, start, y, active[*who].len()));
                    }
                }
                Step::Note { at, side, text } => {
                    let (tw, th) = label_size(text, 13.0);
                    let w = tw + 20.0_f64;
                    let h = th + 12.0_f64;
                    let (l, r) = match side {
                        1 => (xs[at.0] + 10.0_f64, xs[at.0] + 10.0_f64 + w),
                        -1 => (xs[at.0] - 10.0_f64 - w, xs[at.0] - 10.0_f64),
                        _ => {
                            let (a, b) = (xs[at.0], xs[at.1]);
                            let mid = f64::midpoint(a, b);
                            let half = (w / 2.0).max((b - a) / 2.0 + 30.0);
                            (mid - half, mid + half)
                        }
                    };
                    touch(&mut frames, at.0, at.1);
                    y += 8.0_f64;
                    put!(
                        body,
                        "<rect class=\"note\" x=\"{l:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{h:.1}\" rx=\"3\"/>",
                        r - l
                    );
                    text_block(
                        &mut body,
                        text,
                        f64::midpoint(l, r),
                        y + h / 2.0,
                        "note-text",
                    );
                    right_edge = right_edge.max(r);
                    left_edge = left_edge.min(l);
                    y += h + 10.0_f64;
                }
                Step::Open { kind, text } => {
                    y += 10.0_f64;
                    frames.push((kind.clone(), text.clone(), y, Vec::new(), usize::MAX, 0));
                    y += 26.0_f64;
                }
                Step::Divide { text } => {
                    y += 8.0_f64;
                    if let Some(f) = frames.last_mut() {
                        f.3.push((y, text.clone()));
                    }
                    y += 22.0_f64;
                }
                Step::Close => {
                    y += 10.0_f64;
                    if let Some((kind, text, top, divides, a, b)) = frames.pop() {
                        let (a, b) = if a == usize::MAX {
                            (0, count - 1)
                        } else {
                            (a, b)
                        };
                        let l =
                            8.0_f64.mul_add(-(frames.len() as f64), xs[a] - box_w[a] / 2.0 + 4.0);
                        let r = 8.0_f64.mul_add(frames.len() as f64, xs[b] + box_w[b] / 2.0 - 4.0);
                        let mut frame = String::new();
                        put!(
                            frame,
                            "<g class=\"frame\"><rect x=\"{l:.1}\" y=\"{top:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>",
                            r - l,
                            y - top
                        );
                        if kind != "rect" {
                            let kw = text_width(&kind, 12.0) + 16.0_f64;
                            put!(
                                frame,
                                "<path class=\"frame-tab\" d=\"M{l:.1} {top:.1}h{kw:.1}v14l-6 6H{l:.1}z\"/><text class=\"frame-kind\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">{}</text>",
                                l + 6.0_f64,
                                top + 10.0_f64,
                                esc(&kind)
                            );
                            if !text.is_empty() {
                                put!(
                                    frame,
                                    "<text class=\"frame-text\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">[{}]</text>",
                                    l + kw + 8.0_f64,
                                    top + 10.0_f64,
                                    esc(&text)
                                );
                            }
                        }
                        for (dy, dtext) in divides {
                            put!(
                                frame,
                                "<path class=\"frame-divide\" d=\"M{l:.1} {dy:.1}H{r:.1}\"/>"
                            );
                            if !dtext.is_empty() {
                                put!(
                                    frame,
                                    "<text class=\"frame-text\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">[{}]</text>",
                                    l + 8.0_f64,
                                    dy + 11.0_f64,
                                    esc(&dtext)
                                );
                            }
                        }
                        frame.push_str("</g>");
                        // A frame lies under what it frames.
                        body.insert_str(0, &frame);
                        left_edge = left_edge.min(l);
                        right_edge = right_edge.max(r);
                    }
                    y += 6.0_f64;
                }
            }
        }
        for (who, stack) in active.iter().enumerate() {
            for (k, start) in stack.iter().enumerate() {
                bars.push((who, *start, y, k));
            }
        }
        y += 18.0_f64;
        let bottom = y;
        let height = bottom + box_h + pad;
        let shift = if left_edge < pad {
            pad - left_edge
        } else {
            0.0_f64
        };
        let width = right_edge + pad + shift;
        let mut out = String::new();
        let names: Vec<String> = d.actors.iter().map(|(_, l, _)| l.join(" ")).collect();
        let title = d.title.clone().unwrap_or_else(|| names.join(", "));
        let mut desc = String::new();
        for step in d.steps.iter().take(60) {
            if let Step::Message {
                from,
                to,
                text,
                arrow: _,
                activate: _,
            } = step
            {
                put!(
                    desc,
                    "{} → {}: {}; ",
                    names[*from],
                    names[*to],
                    text.join(" ")
                );
            }
        }
        let desc = desc.trim_end_matches("; ").to_owned() + ".";
        svg_open(&mut out, n, width, height, &title, &desc);
        put!(out, "<g transform=\"translate({shift:.1} 0)\">");
        for (k, x) in xs.iter().enumerate() {
            put!(
                out,
                "<path class=\"lifeline\" d=\"M{x:.1} {:.1}V{bottom:.1}\"/>",
                pad + box_h
            );
            let _ = k;
        }
        for (who, start, end, depth) in &bars {
            let x = 5.0_f64.mul_add(*depth as f64, xs[*who] - 5.0);
            put!(
                out,
                "<rect class=\"activation\" x=\"{x:.1}\" y=\"{start:.1}\" width=\"10\" height=\"{:.1}\"/>",
                (end - start).max(8.0)
            );
        }
        out.push_str(&body);
        for (k, (_, label, actor)) in d.actors.iter().enumerate() {
            for top in [pad, bottom] {
                let (l, w) = (xs[k] - box_w[k] / 2.0_f64, box_w[k]);
                let class = if *actor { "actor person" } else { "actor" };
                put!(
                    out,
                    "<g class=\"{class}\"><rect x=\"{l:.1}\" y=\"{top:.1}\" width=\"{w:.1}\" height=\"{box_h:.1}\" rx=\"5\"/>"
                );
                text_block(&mut out, label, xs[k], top + box_h / 2.0, "label");
                out.push_str("</g>");
            }
        }
        out.push_str("</g></svg></figure>");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{Outcome, flow, render};
    use crate::text::put;

    fn svg(src: &str) -> String {
        match render(src, 1) {
            Outcome::Drawn(s) => s,
            Outcome::Unsupported(k) => panic!("unsupported {k}"),
            Outcome::Unreadable(e) => panic!("unreadable: {e}"),
        }
    }

    /// No two real nodes of a laid-out flowchart overlap.
    fn assert_no_overlap(src: &str) {
        let lines: Vec<&str> = src
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let dir = super::Dir::parse(lines[0].split_whitespace().nth(1).unwrap_or("TD"));
        let g = flow::parse(&lines[1..], dir).expect("parses");
        let lay = flow::layout(&g);
        for a in 0..g.nodes.len() {
            for b in a + 1..g.nodes.len() {
                let (p, q) = (lay.boxes[a], lay.boxes[b]);
                let apart = (p.x - q.x).abs() >= f64::midpoint(p.width, q.width) - 0.5_f64
                    || (p.y - q.y).abs() >= f64::midpoint(p.height, q.height) - 0.5_f64;
                assert!(
                    apart,
                    "{} and {} overlap in {src:?}",
                    g.nodes[a].id, g.nodes[b].id
                );
            }
            let p = lay.boxes[a];
            assert!(
                p.x - p.width / 2.0_f64 >= 0.0_f64
                    && p.y - p.height / 2.0_f64 >= 0.0_f64
                    && p.x + p.width / 2.0_f64 <= lay.width + 0.5_f64
                    && p.y + p.height / 2.0_f64 <= lay.height + 0.5_f64,
                "{} outside the canvas",
                g.nodes[a].id
            );
        }
    }

    #[test]
    fn flowcharts_of_every_shape_and_edge() {
        let src = "flowchart TD\n  A([Start]) --> B{Decide?}\n  B -->|yes| C[Do it]\n  B -- no --> D((Stop))\n  C -.-> E[(Store)]\n  C ==> F{{Hex}}\n  E --o G>Flag]\n  F --x H[/Lean/]\n  A <--> H\n  H --> A\n";
        let s = svg(src);
        assert!(s.starts_with("<figure class=\"diagram\"><svg"), "{s}");
        assert!(s.contains("role=\"img\"") && s.contains("<title id=\"dg1-t\">"));
        assert_eq!(s.matches("<g class=\"node\"").count(), 8);
        assert!(s.contains(">yes</tspan>") && s.contains(">no</tspan>"));
        assert!(s.contains("class=\"edge dotted\"") && s.contains("class=\"edge thick\""));
        assert_no_overlap(src);
    }

    #[test]
    fn subgraphs_left_right_and_labels_with_breaks() {
        assert!(
            svg("flowchart LR\n  SP(Persist) -.optional.-> SC(Coordinate)\n")
                .contains(">optional</tspan>")
        );
        let src = "flowchart LR\n  subgraph one [The first]\n    a1[\"Line one<br/>line two\"] --> a2\n  end\n  subgraph two\n    b1 --> b2\n  end\n  a2 --> b1\n  c & d --> a1\n";
        let s = svg(src);
        assert_eq!(s.matches("<g class=\"cluster\"").count(), 2);
        assert!(
            s.contains(">Line one</tspan><tspan") && s.contains(">line two</tspan>"),
            "{s}"
        );
        assert!(s.contains(">The first</tspan>"));
        assert_no_overlap(src);
    }

    #[test]
    fn diagrams_from_this_machine_lay_out() {
        for src in [
            "graph TD\n    A[Push to main] --> B[Optimized CI Starts]\n    B --> C{All Phases Pass?}\n    C -->|Yes| D[Auto-increment Version]\n    C -->|No| E[Fix Issues]\n    E --> A\n    D --> F[Build]\n    F --> G[Release]\n",
            "flowchart TD\n    User([User]) --> Entry[Entry Point]\n    Entry --> Core[Core Engine]\n    Core --> StorageA[(Database)]\n    Core --> StorageB[(Cache)]\n    Core -.-> External[External API]\n",
            "flowchart TB\n    subgraph Gateway[\"Hermes Gateway\"]\n        subgraph Adapters[\"Platform adapters\"]\n            tg[Telegram]\n            dc[Discord]\n        end\n        router[Router]\n    end\n    tg --> router\n    dc --> router\n    router --> agent[AIAgent]\n",
            "graph TD\n  A-->B\n  A-->C\n  B-->D\n  C-->D\n  D-->E\n  A-->E\n  B-->E\n  C-->F\n  F-->E\n",
        ] {
            let _ = svg(src);
            assert_no_overlap(src);
        }
    }

    #[test]
    fn sequence_diagrams() {
        let s = svg(
            "sequenceDiagram\n    autonumber\n    participant C as Client\n    participant K as Ktor pipeline<br/>(Netty + plugins)\n    actor U as User\n    C->>+K: GET /assess\n    K-->>-C: 200 OK\n    loop every minute\n      C-)K: ping\n    end\n    alt failure\n      K--xC: error\n    else success\n      Note over C,K: all good\n    end\n    U->>U: think\n    Note right of U: done\n",
        );
        assert_eq!(
            s.matches("<g class=\"actor").count(),
            6,
            "each participant top and bottom"
        );
        assert!(
            s.contains("class=\"frame\"")
                && s.contains(">loop</text>")
                && s.contains(">[failure]</text>")
        );
        assert!(s.contains("class=\"num\""), "autonumber");
        assert!(s.contains("class=\"activation\""));
        assert!(s.contains(">GET /assess</tspan>"));
    }

    #[test]
    fn other_kinds_are_named_and_bad_input_is_refused() {
        assert!(
            matches!(render("pie title Pets\n \"Dogs\" : 3\n", 1), Outcome::Unsupported(k) if k == "pie")
        );
        assert!(matches!(
            render("gantt\n title x\n", 1),
            Outcome::Unsupported(_)
        ));
        assert!(matches!(
            render("flowchart TD\n A --> \n", 1),
            Outcome::Unreadable(_)
        ));
        assert!(matches!(render("", 1), Outcome::Unreadable(_)));
    }

    #[test]
    fn hostile_flowcharts_stay_bounded() {
        let mut src = String::from("flowchart TD\n");
        for k in 0_i32..150_i32 {
            put!(
                src,
                "  n{k} --> n{}\n  n{k} --> n{}\n",
                (k * 7_i32 + 3_i32) % 150_i32,
                (k * 13_i32 + 1_i32) % 150_i32
            );
        }
        let started = std::time::Instant::now();
        let _ = svg(&src);
        assert!(started.elapsed().as_secs() < 10, "{:?}", started.elapsed());
        assert!(matches!(
            render(
                &format!(
                    "flowchart TD\n{}",
                    (0_i32..500_i32).fold(String::new(), |mut acc, k| {
                        put!(acc, "a{k}\n");
                        acc
                    })
                ),
                1
            ),
            Outcome::Unreadable(_)
        ));
    }
}
