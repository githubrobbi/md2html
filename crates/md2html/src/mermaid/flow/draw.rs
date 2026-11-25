// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A laid-out flowchart as SVG: subgraph frames, edges and their labels,
//! nodes with their outlines and words.

use core::iter::once;

use super::layout::{Layout, layout};
use super::model::{EDGE_FONT, Graph, Head, NodeId, Shape, Stroke, TITLE_FONT};
use crate::mermaid::geom::px;
use crate::mermaid::plane::{Bounds, Point, Size};
use crate::mermaid::table::Table;
use crate::mermaid::title::Name;
use crate::mermaid::{Drawing, svg_close, svg_open};
use crate::text::{esc, put};

/// How many edges the description reads out at most.
const DESCRIBED_EDGES: usize = 60;

/// A flowchart as SVG, the author's title under it; `n` numbers the
/// diagram on its page. None when the layout looks up a part it does not
/// hold.
pub(in crate::mermaid) fn draw(graph: &Graph, n: usize) -> Option<Drawing> {
    let laid = layout(graph)?;
    let names = graph.nodes.map(|node| node.label.joined());
    let name = Name::of(graph.acc_title.as_deref(), graph.caption.as_ref(), || {
        let first: Vec<&str> = names.values().take(3).map(String::as_str).collect();
        format!(
            "{}{}",
            first.join(", "),
            if names.len() > 3 { ", …" } else { "" }
        )
    });
    let desc = match &graph.desc {
        Some(desc) => desc.clone(),
        None => describe(graph, &names)?,
    };
    let mut out = String::new();
    svg_open(&mut out, n, laid.size, &name, &desc);
    frames(&mut out, graph, &laid);
    edges(&mut out, graph, &laid, n);
    for (node, bounds) in graph.nodes.values().zip(&laid.nodes) {
        put!(out, "<g class=\"node\" data-id=\"{}\">", esc(&node.id));
        outline(&mut out, node.shape, *bounds);
        node.label.draw(&mut out, bounds.centre, "label");
        out.push_str("</g>");
    }
    Some(svg_close(out, graph.caption.clone()))
}

/// What the picture shows, edge by edge, for readers who cannot see it.
fn describe(graph: &Graph, names: &Table<NodeId, String>) -> Option<String> {
    let mut said = String::new();
    for edge in graph.edges.values().take(DESCRIBED_EDGES) {
        let via = edge
            .label
            .as_ref()
            .map(|label| format!(" ({})", label.joined()))
            .unwrap_or_default();
        put!(
            said,
            "{} → {}{via}; ",
            names.get(edge.from)?,
            names.get(edge.to)?
        );
    }
    let mut sentence = said.trim_end_matches("; ").to_owned();
    sentence.push('.');
    Some(sentence)
}

/// The subgraphs' frames and titles, outermost first so inner ones lie on
/// top.
fn frames(out: &mut String, graph: &Graph, laid: &Layout) {
    let mut outermost_first: Vec<_> = graph.clusters.iter().collect();
    outermost_first.sort_by_key(|(_, cluster)| cluster.within.len());
    for (id, cluster) in outermost_first {
        let Some(frame) = laid.frames.get(&id) else {
            continue;
        };
        put!(
            out,
            "<g class=\"cluster\"><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\"/>",
            frame.left,
            frame.top,
            frame.right - frame.left,
            frame.bottom - frame.top
        );
        if !cluster.title.is_blank() {
            let height = cluster.title.size(TITLE_FONT).height;
            cluster.title.draw(
                out,
                Point::new(
                    frame.left.midpoint(frame.right),
                    frame.top + px(6.0_f64) + height / 2.0_f64,
                ),
                "cluster-title",
            );
        }
        out.push_str("</g>");
    }
}

/// The marker for an end of an edge, by its letter in the marker ids.
const fn marker(head: Head) -> Option<char> {
    match head {
        Head::Arrow => Some('a'),
        Head::Circle => Some('o'),
        Head::Cross => Some('x'),
        Head::None => None,
    }
}

/// The edges, then their labels over them.
fn edges(out: &mut String, graph: &Graph, laid: &Layout, n: usize) {
    let horizontal = graph.dir.horizontal();
    for (edge, route) in graph.edges.values().zip(&laid.edges) {
        let class = match edge.stroke {
            Stroke::Solid => "edge",
            Stroke::Dotted => "edge dotted",
            Stroke::Thick => "edge thick",
            Stroke::Invisible => continue,
        };
        let end = marker(edge.head)
            .map(|mark| format!(" marker-end=\"url(#dg{n}-{mark})\""))
            .unwrap_or_default();
        let start = marker(edge.tail)
            .map(|mark| format!(" marker-start=\"url(#dg{n}-{mark})\""))
            .unwrap_or_default();
        put!(
            out,
            "<path class=\"{class}\" d=\"{}\"{start}{end}/>",
            path_d(&route.path, horizontal)
        );
    }
    for (edge, route) in graph.edges.values().zip(&laid.edges) {
        if let (Some(label), Some(at)) = (&edge.label, route.label) {
            let Size { width, height } = label.size(EDGE_FONT);
            put!(
                out,
                "<rect class=\"edge-label-bg\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\"/>",
                at.x - width / 2.0_f64 - px(5.0_f64),
                at.y - height / 2.0_f64 - px(2.0_f64),
                width + px(10.0_f64),
                height + px(4.0_f64)
            );
            label.draw(out, at, "edge-label");
        }
    }
}

/// A path through points, smoothed: straight from end to end when two, a
/// curve through the waypoints otherwise.
fn path_d(points: &[Point], horizontal: bool) -> String {
    let mut path = String::new();
    match points {
        [] => {}
        [start, end] => {
            put!(path, "M{:.1} {:.1}", start.x, start.y);
            let straight =
                (start.x - end.x).abs() < px(1.0_f64) || (start.y - end.y).abs() < px(1.0_f64);
            if straight {
                put!(path, "L{:.1} {:.1}", end.x, end.y);
            } else if horizontal {
                let middle = start.x.midpoint(end.x);
                put!(
                    path,
                    "C{middle:.1} {:.1} {middle:.1} {:.1} {:.1} {:.1}",
                    start.y,
                    end.y,
                    end.x,
                    end.y
                );
            } else {
                let middle = start.y.midpoint(end.y);
                put!(
                    path,
                    "C{:.1} {middle:.1} {:.1} {middle:.1} {:.1} {:.1}",
                    start.x,
                    end.x,
                    end.x,
                    end.y
                );
            }
        }
        [only] => put!(path, "M{:.1} {:.1}", only.x, only.y),
        [first, .., last] => {
            put!(path, "M{:.1} {:.1}", first.x, first.y);
            curve(&mut path, points, *first, *last);
        }
    }
    path
}

/// Catmull-Rom through the points, as cubic curves: each piece leaves a
/// point along the line from the one before it to the one after.
fn curve(path: &mut String, points: &[Point], first: Point, last: Point) {
    let before = once(&first).chain(points);
    let from = points.iter();
    let to = points.iter().skip(1);
    let after = points.iter().skip(2).chain(once(&last));
    for (((p0, p1), p2), p3) in before.zip(from).zip(to).zip(after) {
        let lead = Point::new(
            p1.x + (p2.x - p0.x) / 6.0_f64,
            p1.y + (p2.y - p0.y) / 6.0_f64,
        );
        let trail = Point::new(
            p2.x - (p3.x - p1.x) / 6.0_f64,
            p2.y - (p3.y - p1.y) / 6.0_f64,
        );
        put!(
            path,
            "C{:.1} {:.1} {:.1} {:.1} {:.1} {:.1}",
            lead.x,
            lead.y,
            trail.x,
            trail.y,
            p2.x,
            p2.y
        );
    }
}

/// A polygon through `corners`.
fn polygon(out: &mut String, corners: &[Point]) {
    out.push_str("<polygon class=\"shape\" points=\"");
    for (place, corner) in corners.iter().enumerate() {
        let gap = if place > 0 { " " } else { "" };
        put!(out, "{gap}{:.1},{:.1}", corner.x, corner.y);
    }
    out.push_str("\"/>");
}

/// A rectangle with corners rounded by `radius`.
fn rounded(out: &mut String, corner: Point, size: Size, radius: &str) {
    put!(
        out,
        "<rect class=\"shape\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{radius}\"/>",
        corner.x,
        corner.y,
        size.width,
        size.height
    );
}

/// A node's outline as SVG.
fn outline(out: &mut String, shape: Shape, bounds: Bounds) {
    let Bounds { centre, size } = bounds;
    let Size { width, height } = size;
    let (x, y) = (centre.x - width / 2.0_f64, centre.y - height / 2.0_f64);
    let (right, bottom) = (x + width, y + height);
    let inset = px(14.0_f64);
    match shape {
        Shape::Rect => rounded(out, Point::new(x, y), size, "4"),
        Shape::Round => rounded(out, Point::new(x, y), size, "12"),
        Shape::Stadium => rounded(
            out,
            Point::new(x, y),
            size,
            &format!("{:.1}", height / 2.0_f64),
        ),
        Shape::Subroutine => {
            put!(
                out,
                "<rect class=\"shape\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{width:.1}\" height=\"{height:.1}\"/><path class=\"shape-line\" d=\"M{:.1} {y:.1}V{bottom:.1}M{:.1} {y:.1}V{bottom:.1}\"/>",
                x + px(8.0_f64),
                right - px(8.0_f64),
            );
        }
        Shape::Cylinder => cylinder(out, Point::new(x, y), size),
        Shape::Circle => {
            put!(
                out,
                "<circle class=\"shape\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\"/>",
                centre.x,
                centre.y,
                width / 2.0_f64
            );
        }
        Shape::DoubleCircle => {
            let Point { x: cx, y: cy } = centre;
            put!(
                out,
                "<circle class=\"shape\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\"/><circle class=\"shape-line\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" r=\"{:.1}\"/>",
                width / 2.0_f64,
                width / 2.0_f64 - px(5.0_f64)
            );
        }
        Shape::Diamond => polygon(out, &[
            Point::new(centre.x, y),
            Point::new(right, centre.y),
            Point::new(centre.x, bottom),
            Point::new(x, centre.y),
        ]),
        Shape::Hexagon => polygon(out, &[
            Point::new(x + inset, y),
            Point::new(right - inset, y),
            Point::new(right, centre.y),
            Point::new(right - inset, bottom),
            Point::new(x + inset, bottom),
            Point::new(x, centre.y),
        ]),
        Shape::Flag => polygon(out, &[
            Point::new(x, y),
            Point::new(right, y),
            Point::new(right, bottom),
            Point::new(x, bottom),
            Point::new(x + inset, centre.y),
        ]),
        Shape::Lean => polygon(out, &[
            Point::new(x + inset, y),
            Point::new(right, y),
            Point::new(right - inset, bottom),
            Point::new(x, bottom),
        ]),
        Shape::LeanBack => polygon(out, &[
            Point::new(x, y),
            Point::new(right - inset, y),
            Point::new(right, bottom),
            Point::new(x + inset, bottom),
        ]),
        Shape::Trapezoid => polygon(out, &[
            Point::new(x + inset, y),
            Point::new(right - inset, y),
            Point::new(right, bottom),
            Point::new(x, bottom),
        ]),
        Shape::TrapezoidDown => polygon(out, &[
            Point::new(x, y),
            Point::new(right, y),
            Point::new(right - inset, bottom),
            Point::new(x + inset, bottom),
        ]),
    }
}

/// A cylinder standing in the box at `corner` of `size`: a tube with an
/// elliptic top and bottom.
fn cylinder(out: &mut String, corner: Point, size: Size) {
    let Point { x, y } = corner;
    let lid = px(8.0_f64);
    let top = y + lid;
    let bottom = y + size.height - lid;
    let right = x + size.width;
    let radius = size.width / 2.0_f64;
    put!(
        out,
        "<path class=\"shape\" d=\"M{x:.1} {top:.1}A{radius:.1} {lid} 0 0 1 {right:.1} {top:.1}V{bottom:.1}A{radius:.1} {lid} 0 0 1 {x:.1} {bottom:.1}Z\"/><path class=\"shape-line\" d=\"M{x:.1} {top:.1}A{radius:.1} {lid} 0 0 0 {right:.1} {top:.1}\"/>"
    );
}
