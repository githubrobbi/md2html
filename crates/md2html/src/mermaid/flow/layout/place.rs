// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Placing the vertices.
//!
//! Each rank is packed, then each vertex moved toward the median of its
//! neighbours in passes down and up, never closer to the vertex beside it
//! than the gap; then the ranks are spaced along the flow.

use core::cmp::Ordering;

use super::{Layered, VertexId};
use crate::mermaid::geom::{Len, px};

/// How many passes, down and up in turn.
const PASSES: usize = 40;

/// The median of sorted lengths, halfway between the middle two when there
/// is an even number; none of none.
const fn median(sorted: &[Len]) -> Option<Len> {
    let (low, high) = sorted.split_at(sorted.len() / 2);
    match (low.last(), high.first()) {
        (Some(&below), Some(&above)) if sorted.len() % 2 == 0 => Some(below.midpoint(above)),
        (_, middle) => middle.copied(),
    }
}

impl Layered<'_> {
    /// Places every vertex across and along the flow.
    pub(super) fn place(&mut self) -> Option<()> {
        let layers = core::mem::take(&mut self.layers);
        for layer in &layers {
            self.pack(layer)?;
        }
        for pass in 0..PASSES {
            let down = pass % 2 == 0;
            if down {
                for layer in &layers {
                    self.settle(layer, down)?;
                }
            } else {
                for layer in layers.iter().rev() {
                    self.settle(layer, down)?;
                }
            }
        }
        self.space_ranks(&layers)?;
        self.layers = layers;
        Some(())
    }

    /// The least distance between the centres of two vertices side by side.
    fn spacing(&self, first: VertexId, second: VertexId) -> Option<Len> {
        let (one, other) = (self.vertices.get(first)?, self.vertices.get(second)?);
        let roomy = usize::from(one.kind.takes_room()) + usize::from(other.kind.takes_room());
        // More room beside nodes and labels than beside bare waypoints, and
        // more between different subgraphs.
        let base = match roomy {
            0 => px(10.0_f64),
            1 => px(18.0_f64),
            _ => px(28.0_f64),
        };
        let apart = if one.within.first() == other.within.first() {
            px(0.0_f64)
        } else {
            px(26.0_f64)
        };
        let gap = base + apart;
        Some(self.across(one.size) / 2.0_f64 + gap + self.across(other.size) / 2.0_f64)
    }

    /// Puts a rank's vertices side by side, as close as they may be.
    fn pack(&mut self, layer: &[VertexId]) -> Option<()> {
        let mut position = px(0.0_f64);
        let mut prev: Option<VertexId> = None;
        for &id in layer {
            if let Some(before) = prev {
                position += self.spacing(before, id)?;
            }
            self.vertices.get_mut(id)?.across = position;
            prev = Some(id);
        }
        Some(())
    }

    /// Moves a rank's vertices toward where their neighbours want them,
    /// keeping their order and their gaps: the furthest each may go left
    /// and right is found, and each takes the middle.
    fn settle(&mut self, layer: &[VertexId], down: bool) -> Option<()> {
        let wanted = layer
            .iter()
            .map(|&id| self.wanted(id, down))
            .collect::<Option<Vec<Len>>>()?;
        let gaps = layer
            .iter()
            .zip(layer.iter().skip(1))
            .map(|(&first, &second)| self.spacing(first, second))
            .collect::<Option<Vec<Len>>>()?;
        let mut lefts = Vec::with_capacity(wanted.len());
        let mut want = wanted.iter().copied();
        if let Some(first) = want.next() {
            let mut last = first;
            lefts.push(last);
            for (here, gap) in want.zip(&gaps) {
                last = here.max(last + *gap);
                lefts.push(last);
            }
        }
        let mut rights = Vec::with_capacity(wanted.len());
        let mut want_back = wanted.iter().rev().copied();
        if let Some(first) = want_back.next() {
            let mut next = first;
            rights.push(next);
            for (here, gap) in want_back.zip(gaps.iter().rev()) {
                next = here.min(next - *gap);
                rights.push(next);
            }
        }
        rights.reverse();
        for ((&id, left), right) in layer.iter().zip(lefts).zip(rights) {
            self.vertices.get_mut(id)?.across = left.midpoint(right);
        }
        Some(())
    }

    /// Where a vertex's neighbours want it: the median of the ones it is
    /// swept from, and of the others too for a waypoint or a vertex with
    /// none on that side; where it is when it has no neighbours at all.
    fn wanted(&self, id: VertexId, down: bool) -> Option<Len> {
        let vertex = self.vertices.get(id)?;
        let (first, second) = if down {
            (&vertex.preds, &vertex.succs)
        } else {
            (&vertex.succs, &vertex.preds)
        };
        let placed = |near: &Vec<VertexId>| {
            near.iter()
                .map(|&other| self.vertices.get(other).map(|neighbour| neighbour.across))
                .collect::<Option<Vec<Len>>>()
        };
        let mut near = placed(first)?;
        if near.is_empty() || vertex.kind.is_waypoint() {
            near.extend(placed(second)?);
        }
        near.sort_by(|one, other| one.partial_cmp(other).unwrap_or(Ordering::Equal));
        Some(median(&near).unwrap_or(vertex.across))
    }

    /// Spaces the ranks along the flow: each as thick as its thickest
    /// vertex, with room between for the edges.
    fn space_ranks(&mut self, layers: &[Vec<VertexId>]) -> Option<()> {
        let rank_gap = if self.horizontal {
            px(56.0_f64)
        } else {
            px(46.0_f64)
        } + if self.graph.clusters.is_empty() {
            px(0.0_f64)
        } else {
            px(16.0_f64)
        };
        let mut position = px(0.0_f64);
        for layer in layers {
            let mut deepest = px(0.0_f64);
            let mut bare = true;
            for &id in layer {
                let vertex = self.vertices.get(id)?;
                deepest = deepest.max(self.along(vertex.size));
                bare &= vertex.kind.is_waypoint();
            }
            let thick = deepest.max(if bare { px(6.0_f64) } else { px(0.0_f64) });
            let centre = position + thick / 2.0_f64;
            for &id in layer {
                self.vertices.get_mut(id)?.along = centre;
            }
            position += thick + rank_gap;
        }
        Some(())
    }
}
