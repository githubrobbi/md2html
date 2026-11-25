// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.

//! Mermaid diagrams drawn here, without a script.
//!
//! Flowcharts and sequence diagrams are read, laid out and written as
//! inline SVG that takes the page's colours (both themes), prints, and
//! keeps its words as text. A flowchart is laid out the layered way:
//! cycles broken, nodes ranked, long edges split by invisible waypoints,
//! each rank ordered to cut crossings, then placed toward the middle of its
//! neighbours. Other diagram kinds are left to `mermaid.js` when the author
//! supplies it, and shown as code otherwise.

mod flow;
mod geom;
mod label;
mod plane;
mod seq;
mod table;
#[cfg(test)]
mod tests;
mod title;

use geom::px;
use plane::Size;
use title::{Caption, Name};

use crate::text::{esc, put};

/// What came of a diagram block.
pub(crate) enum Outcome {
    /// The figure, drawn here.
    Drawn(Drawing),
    /// A kind this does not draw (`pie`, `gantt`, ...): its name.
    Unsupported(String),
    /// A flowchart or sequence diagram that could not be read: why.
    Unreadable(String),
}

/// A diagram drawn here: the figure, and the author's title under it.
pub(crate) struct Drawing {
    /// The figure: the SVG, and the caption under it when there is one.
    pub(crate) html: String,
    /// The author's title, shown under the drawing as its caption.
    caption: Option<Caption>,
}

impl Drawing {
    /// Whether the author's title stands under the drawing.
    pub(crate) const fn has_caption(&self) -> bool {
        self.caption.is_some()
    }
}

/// Why a diagram is not drawn when it refers to a part it does not hold:
/// a lookup by number that found nothing.
fn missing() -> String {
    String::from("the diagram refers to a part it does not hold")
}

/// The most nodes a flowchart may have and still be drawn.
const MOST_NODES: usize = 400;

/// The most edges a flowchart may have and still be drawn.
const MOST_EDGES: usize = 1000;

/// The most waypoints a flowchart's layout may make ([`flow::bends`]): what
/// a layout costs grows with them, and a few bytes of text can ask for
/// hundreds.
const MOST_BENDS: usize = 4000;

/// The most participants a sequence diagram may have and still be drawn:
/// each message is measured against every one of them.
const MOST_ACTORS: usize = 100;

/// The diagram in a block of Mermaid source, its front matter's title the
/// caption under it. `n` numbers the diagrams of a page, so their marker
/// ids stay apart.
pub(crate) fn render(source: &str, n: usize) -> Outcome {
    let front = match title::front(source) {
        Ok(front) => front,
        Err(why) => return Outcome::Unreadable(why),
    };
    let lines: Vec<&str> = front
        .body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("%%"))
        .collect();
    let Some((head, body)) = lines.split_first() else {
        return Outcome::Unreadable(String::from("the block is empty"));
    };
    let mut words = head.split_whitespace();
    match words.next() {
        Some("flowchart" | "graph" | "flowchart-elk") => {
            let dir = flow::Dir::parse(words.next().unwrap_or("TD"));
            match flow::parse(body, dir, front.caption) {
                Ok(graph) if graph.nodes.is_empty() => {
                    Outcome::Unreadable(String::from("the flowchart has no nodes"))
                }
                Ok(graph) if graph.nodes.len() > MOST_NODES => {
                    Outcome::Unreadable(format!("the flowchart has more than {MOST_NODES} nodes"))
                }
                Ok(graph) if graph.edge_count() > MOST_EDGES => {
                    Outcome::Unreadable(format!("the flowchart has more than {MOST_EDGES} edges"))
                }
                Ok(graph) if flow::bends(&graph).is_some_and(|bends| bends > MOST_BENDS) => {
                    Outcome::Unreadable(format!(
                        "the flowchart's edges cross more than {MOST_BENDS} ranks in all"
                    ))
                }
                Ok(graph) => drawn(flow::draw(&graph, n)),
                Err(why) => Outcome::Unreadable(why),
            }
        }
        Some("sequenceDiagram") => match seq::parse(body, front.caption) {
            Ok(diagram) if diagram.actor_count() > MOST_ACTORS => Outcome::Unreadable(format!(
                "the sequence diagram has more than {MOST_ACTORS} participants"
            )),
            Ok(diagram) => drawn(seq::draw(&diagram, n)),
            Err(why) => Outcome::Unreadable(why),
        },
        Some(other) => Outcome::Unsupported(other.trim_end_matches(':').to_owned()),
        None => Outcome::Unsupported(String::new()),
    }
}

/// A drawing, or why there is none.
fn drawn(drawing: Option<Drawing>) -> Outcome {
    drawing.map_or_else(|| Outcome::Unreadable(missing()), Outcome::Drawn)
}

/// The SVG around a drawing of `size`, with its name and description for
/// readers who cannot see it, and the arrowheads.
fn svg_open(out: &mut String, n: usize, size: Size, name: &Name, desc: &str) {
    let Size { width, height } = size;
    put!(
        out,
        "<figure class=\"diagram{}\"><svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width:.0} {height:.0}\" width=\"{width:.0}\" height=\"{height:.0}\" role=\"img\" aria-labelledby=\"dg{n}-t dg{n}-d\">\
<title id=\"dg{n}-t\">{}</title><desc id=\"dg{n}-d\">{}</desc><defs>\
<marker id=\"dg{n}-a\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path class=\"head\" d=\"M0 0L10 5L0 10z\"/></marker>\
<marker id=\"dg{n}-o\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto-start-reverse\"><circle class=\"head-o\" cx=\"5\" cy=\"5\" r=\"3.6\"/></marker>\
<marker id=\"dg{n}-x\" viewBox=\"0 0 10 10\" refX=\"7\" refY=\"5\" markerWidth=\"9\" markerHeight=\"9\" orient=\"auto-start-reverse\"><path class=\"head-x\" d=\"M2 2L8 8M8 2L2 8\"/></marker>\
<marker id=\"dg{n}-v\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><path class=\"head-x\" d=\"M1 1L9 5L1 9\"/></marker>\
</defs>",
        if width > px(1_300.0_f64) { " wide" } else { "" },
        esc(name.as_str()),
        esc(desc)
    );
}

/// The drawing `out` finished: the SVG closed, the author's title under
/// it as the figure's caption (its words in their own direction, whatever
/// the drawing's), the figure closed.
fn svg_close(mut out: String, caption: Option<Caption>) -> Drawing {
    out.push_str("</svg>");
    if let Some(words) = &caption {
        put!(
            out,
            "<figcaption dir=\"auto\">{}</figcaption>",
            esc(words.as_str())
        );
    }
    out.push_str("</figure>");
    Drawing { html: out, caption }
}
