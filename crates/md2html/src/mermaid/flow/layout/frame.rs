// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! From places in the flow to boxes on the page: every vertex boxed,
//! subgraphs framed around their members, the whole moved inside the
//! canvas and mirrored for the flows that run backward.

use alloc::collections::BTreeMap;
use core::cmp::Reverse;

use super::{Kind, Layered, Layout, VertexId};
use crate::mermaid::flow::model::{ClusterId, Dir, TITLE_FONT};
use crate::mermaid::geom::{Len, px};
use crate::mermaid::plane::{Bounds, Point, Rect, Size};
use crate::mermaid::table::Table;

/// The margin around the drawing.
const PAD: Len = px(20.0_f64);

impl Layered<'_> {
    /// The finished layout.
    pub(super) fn finish(&self) -> Option<Layout> {
        let mut boxes = self.boxes();
        let mut frames = self.frames(&boxes);
        let mut size = fit(&mut boxes, &mut frames);
        match self.graph.dir {
            Dir::BottomUp => {
                for bounds in boxes.values_mut() {
                    bounds.centre.y = size.height - bounds.centre.y;
                }
                for frame in frames.values_mut() {
                    *frame = Rect {
                        top: size.height - frame.bottom,
                        bottom: size.height - frame.top,
                        ..*frame
                    };
                }
            }
            Dir::RightLeft => {
                for bounds in boxes.values_mut() {
                    bounds.centre.x = size.width - bounds.centre.x;
                }
                for frame in frames.values_mut() {
                    *frame = Rect {
                        left: size.width - frame.right,
                        right: size.width - frame.left,
                        ..*frame
                    };
                }
            }
            Dir::TopDown | Dir::LeftRight => {}
        }
        let edges = self.routes(&boxes, &mut size)?;
        size.height = size.height.max(px(1.0_f64));
        Some(Layout {
            nodes: boxes
                .values()
                .take(self.graph.nodes.len())
                .copied()
                .collect(),
            edges,
            frames,
            size,
        })
    }

    /// Every vertex's box, the leftmost touching the margin.
    fn boxes(&self) -> Table<VertexId, Bounds> {
        let least = self
            .vertices
            .values()
            .map(|vertex| vertex.across - self.across(vertex.size) / 2.0_f64)
            .fold(Len::MAX, Len::min);
        self.vertices.map(|vertex| {
            let across = vertex.across - least + PAD;
            let along = vertex.along + PAD;
            Bounds {
                centre: if self.horizontal {
                    Point::new(along, across)
                } else {
                    Point::new(across, along)
                },
                size: vertex.size,
            }
        })
    }

    /// Each subgraph's frame around its nodes and the frames inside it,
    /// inner ones first, with room for the title on top.
    fn frames(&self, boxes: &Table<VertexId, Bounds>) -> BTreeMap<ClusterId, Rect> {
        let clusters = &self.graph.clusters;
        let mut deepest_first: Vec<_> = clusters.iter().collect();
        deepest_first.sort_by_key(|(_, cluster)| Reverse(cluster.within.len()));
        let mut frames = BTreeMap::new();
        for (id, cluster) in deepest_first {
            let members = self
                .vertices
                .values()
                .zip(boxes.values())
                .filter(|(vertex, _)| {
                    vertex.kind == Kind::Node && vertex.within.last() == Some(&id)
                })
                .map(|(_, bounds)| bounds.rect());
            let inner = clusters
                .iter()
                .filter(|(_, inner)| inner.parent() == Some(id))
                .filter_map(|(inner, _)| frames.get(&inner).copied());
            let Some(around) = members.chain(inner).reduce(Rect::union) else {
                continue;
            };
            let Size {
                width: title_width,
                height: title_height,
            } = cluster.title.size(TITLE_FONT);
            let title_room = if cluster.title.is_blank() {
                px(0.0_f64)
            } else {
                title_height + px(6.0_f64)
            };
            let left = around.left - px(14.0_f64);
            let frame = Rect {
                left,
                top: around.top - px(12.0_f64) - title_room,
                right: (around.right + px(14.0_f64)).max(left + title_width + px(28.0_f64)),
                bottom: around.bottom + px(12.0_f64),
            };
            frames.insert(id, frame);
        }
        frames
    }
}

/// Moves everything inside the canvas, the margin around it, and gives the
/// canvas's size.
fn fit(boxes: &mut Table<VertexId, Bounds>, frames: &mut BTreeMap<ClusterId, Rect>) -> Size {
    let around = boxes
        .values()
        .map(|bounds| bounds.rect())
        .chain(frames.values().copied())
        .fold(Rect::NOTHING, Rect::union);
    let shift = Point::new(PAD - around.left, PAD - around.top);
    for bounds in boxes.values_mut() {
        bounds.centre = Point::new(bounds.centre.x + shift.x, bounds.centre.y + shift.y);
    }
    for frame in frames.values_mut() {
        *frame = frame.moved(shift);
    }
    Size::new(
        PAD.mul_add(2.0_f64, around.right - around.left),
        PAD.mul_add(2.0_f64, around.bottom - around.top),
    )
}
