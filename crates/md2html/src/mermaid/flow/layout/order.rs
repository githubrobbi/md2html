// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Ordering each rank to cut crossings.
//!
//! First seen comes first; then sweeps down and up sort each rank by the
//! mean place of its neighbours, members of a subgraph kept together. The
//! order with the fewest crossings wins.

use alloc::collections::BTreeMap;
use core::cmp::Ordering;

use super::{Layered, Vertex, VertexId};
use crate::mermaid::flow::model::ClusterId;
use crate::mermaid::geom::{Mean, compare, count};
use crate::mermaid::table::{Id as _, Table};

/// How many sweeps, down and up in turn.
const SWEEPS: usize = 16;

/// One step of a vertex's sort key.
///
/// A step is a mean place, and the number of what it is the mean of (a
/// subgraph, or the vertex itself), which orders equal means. A vertex's
/// whole key is a list of steps, compared step by step, the shorter first
/// when one starts the other: the order of slices.
#[derive(Clone, Copy, Debug)]
struct Key {
    /// The mean place.
    value: f64,
    /// The subgraph's number, or the vertex's.
    tie: usize,
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        compare(self.value, other.value).then(self.tie.cmp(&other.tie))
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Key {}

impl Layered<'_> {
    /// Orders every rank.
    pub(super) fn order(&mut self) -> Option<()> {
        self.number_slots()?;
        let mut best = self.layers.clone();
        let mut fewest = self.crossings()?;
        for sweep in 0..SWEEPS {
            let down = sweep % 2 == 0;
            if down {
                for layer in self.layers.iter_mut().skip(1) {
                    reorder(&mut self.vertices, layer, down)?;
                }
            } else {
                for layer in self.layers.iter_mut().rev().skip(1) {
                    reorder(&mut self.vertices, layer, down)?;
                }
            }
            let crossings = self.crossings()?;
            if crossings < fewest {
                fewest = crossings;
                best.clone_from(&self.layers);
            }
        }
        self.layers = best;
        Some(())
    }

    /// Gives every vertex its place in its rank.
    fn number_slots(&mut self) -> Option<()> {
        for layer in &self.layers {
            for (slot, &id) in layer.iter().enumerate() {
                self.vertices.get_mut(id)?.slot = slot;
            }
        }
        Some(())
    }

    /// How many pairs of edges cross between neighbouring ranks.
    fn crossings(&self) -> Option<usize> {
        let mut total = 0_usize;
        for layer in self.layers.iter().take(self.layers.len().saturating_sub(1)) {
            let mut ends: Vec<(usize, usize)> = Vec::new();
            for &id in layer {
                let vertex = self.vertices.get(id)?;
                for &next in &vertex.succs {
                    ends.push((vertex.slot, self.vertices.get(next)?.slot));
                }
            }
            for (skip, &(first_from, first_to)) in ends.iter().enumerate() {
                for &(second_from, second_to) in ends.iter().skip(skip + 1) {
                    if (first_from < second_from && first_to > second_to)
                        || (first_from > second_from && first_to < second_to)
                    {
                        total += 1;
                    }
                }
            }
        }
        Some(total)
    }
}

/// The mean place of a vertex's neighbours on the side it is swept from:
/// above going down, below going up; its own place when it has none there.
fn barycentre(
    vertices: &Table<VertexId, Vertex<'_>>,
    vertex: &Vertex<'_>,
    down: bool,
) -> Option<f64> {
    let near = if down { &vertex.preds } else { &vertex.succs };
    if near.is_empty() {
        return Some(count(vertex.slot));
    }
    let mut mean = Mean::default();
    for &other in near {
        mean.add(count(vertices.get(other)?.slot));
    }
    Some(mean.value())
}

/// Sorts one rank by its vertices' barycentres. Members of one subgraph
/// sort by the subgraph's mean first, so they stay together.
fn reorder(
    vertices: &mut Table<VertexId, Vertex<'_>>,
    layer: &mut Vec<VertexId>,
    down: bool,
) -> Option<()> {
    let mut bary: Vec<(f64, VertexId, &[ClusterId])> = Vec::with_capacity(layer.len());
    for &id in layer.iter() {
        let vertex = vertices.get(id)?;
        bary.push((barycentre(vertices, vertex, down)?, id, vertex.within));
    }
    let mut groups: BTreeMap<ClusterId, Mean> = BTreeMap::new();
    for &(value, _, within) in &bary {
        for &cluster in within {
            groups.entry(cluster).or_default().add(value);
        }
    }
    let mut keyed: Vec<(Vec<Key>, VertexId)> = bary
        .iter()
        .map(|&(value, id, within)| {
            let mut key: Vec<Key> = within
                .iter()
                .map(|cluster| {
                    groups.get(cluster).map(|mean| Key {
                        value: mean.value(),
                        tie: cluster.index(),
                    })
                })
                .collect::<Option<_>>()?;
            key.push(Key {
                value,
                tie: id.index(),
            });
            Some((key, id))
        })
        .collect::<Option<_>>()?;
    keyed.sort_by(|(first, _), (second, _)| first.cmp(second));
    *layer = keyed.into_iter().map(|(_, id)| id).collect();
    for (slot, &id) in layer.iter().enumerate() {
        vertices.get_mut(id)?.slot = slot;
    }
    Some(())
}
