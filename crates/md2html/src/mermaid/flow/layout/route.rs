// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Edges routed: through their waypoints, their ends on the outlines of the
//! nodes they join, and a place for their labels.

use core::iter::once;

use super::{Layered, VertexId, Way};
use crate::mermaid::flow::model::Shape;
use crate::mermaid::geom::{Len, px};
use crate::mermaid::plane::{Bounds, Outline, Point, Size};
use crate::mermaid::table::Table;

/// An edge's way on the page.
#[derive(Clone, Debug)]
pub(in crate::mermaid::flow) struct Route {
    /// The points it passes, from end to end.
    pub(in crate::mermaid::flow) path: Vec<Point>,
    /// Where its label goes.
    pub(in crate::mermaid::flow) label: Option<Point>,
}

impl Layered<'_> {
    /// Each edge's route, in the graph's order; a loop widens the canvas by
    /// what sticks out.
    pub(super) fn routes(
        &self,
        boxes: &Table<VertexId, Bounds>,
        size: &mut Size,
    ) -> Option<Vec<Route>> {
        let mut routes = Vec::with_capacity(self.ways.len());
        for (edge, way) in self.graph.edges.values().zip(&self.ways) {
            let start_box = *boxes.get(edge.from.into())?;
            let chain = match way {
                Way::Loop => {
                    let (route, reach) = loop_route(start_box);
                    size.width = size.width.max(reach);
                    routes.push(route);
                    continue;
                }
                Way::Chain(chain) => chain,
            };
            let end_box = *boxes.get(edge.to.into())?;
            let inner = chain
                .waypoints
                .iter()
                .map(|&waypoint| boxes.get(waypoint).map(|bounds| bounds.centre))
                .collect::<Option<Vec<Point>>>()?;
            let start_shape = self.graph.nodes.get(edge.from)?.shape;
            let end_shape = self.graph.nodes.get(edge.to)?.shape;
            // Without waypoints each end aims at the other.
            let start = port(
                start_shape,
                start_box,
                inner.first().copied().unwrap_or(end_box.centre),
                self.horizontal,
            );
            let end = port(
                end_shape,
                end_box,
                inner.last().copied().unwrap_or(start),
                self.horizontal,
            );
            let label = match chain.label {
                Some(waypoint) => Some(boxes.get(waypoint)?.centre),
                None => edge.label.as_ref().map(|_| start.midpoint(end)),
            };
            routes.push(Route {
                path: once(start).chain(inner).chain(once(end)).collect(),
                label,
            });
        }
        Some(routes)
    }
}

/// A loop on the node in `bounds`: out of its side and back, its label
/// beside it; and how far right it reaches.
fn loop_route(bounds: Bounds) -> (Route, Len) {
    let beside = bounds.centre.x + bounds.size.width / 2.0_f64;
    let middle = bounds.centre.y;
    let route = Route {
        path: vec![
            Point::new(beside, middle - px(8.0_f64)),
            Point::new(beside + px(30.0_f64), middle - px(22.0_f64)),
            Point::new(beside + px(30.0_f64), middle + px(22.0_f64)),
            Point::new(beside, middle + px(8.0_f64)),
        ],
        label: Some(Point::new(beside + px(34.0_f64), middle)),
    };
    (route, beside + px(60.0_f64))
}

/// The outline an edge meets on a node of `shape`.
const fn outline(shape: Shape) -> Outline {
    match shape {
        Shape::Diamond => Outline::Diamond,
        Shape::Circle | Shape::DoubleCircle => Outline::Circle,
        Shape::Rect
        | Shape::Round
        | Shape::Stadium
        | Shape::Subroutine
        | Shape::Cylinder
        | Shape::Hexagon
        | Shape::Flag
        | Shape::Lean
        | Shape::LeanBack
        | Shape::Trapezoid
        | Shape::TrapezoidDown => Outline::Box,
    }
}

/// Where an edge meets a node.
///
/// It meets the face toward the next rank (top or bottom, or a side when
/// ranks run across), near its middle, leaning toward the other end; a
/// pointed shape at its tip. When the other end is beside the node rather
/// than past that face, it meets the outline where the line toward the
/// other end leaves it.
fn port(shape: Shape, bounds: Bounds, to: Point, horizontal: bool) -> Point {
    let pointed = matches!(
        shape,
        Shape::Diamond | Shape::Circle | Shape::DoubleCircle | Shape::Hexagon
    );
    let Bounds { centre, size } = bounds;
    if horizontal {
        let lean = if pointed {
            px(0.0_f64)
        } else {
            ((to.y - centre.y) * 0.25_f64).clamp(-size.height * 0.3_f64, size.height * 0.3_f64)
        };
        let half = size.width / 2.0_f64;
        if (to.x - centre.x).abs() < half {
            return bounds.exit(outline(shape), to);
        }
        let face = if to.x >= centre.x {
            centre.x + half
        } else {
            centre.x - half
        };
        Point::new(face, centre.y + lean)
    } else {
        let lean = if pointed {
            px(0.0_f64)
        } else {
            ((to.x - centre.x) * 0.25_f64).clamp(-size.width * 0.3_f64, size.width * 0.3_f64)
        };
        let half = size.height / 2.0_f64;
        if (to.y - centre.y).abs() < half {
            return bounds.exit(outline(shape), to);
        }
        let face = if to.y >= centre.y {
            centre.y + half
        } else {
            centre.y - half
        };
        Point::new(centre.x + lean, face)
    }
}
