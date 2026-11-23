// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Points, sizes and boxes on a drawing, made of lengths.

use super::geom::{Len, diamond_reach, px};

/// A point on the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Point {
    /// Across, from the left.
    pub(super) x: Len,
    /// Down, from the top.
    pub(super) y: Len,
}

impl Point {
    /// The point at `x` across and `y` down.
    pub(super) const fn new(x: Len, y: Len) -> Self {
        Self { x, y }
    }

    /// Halfway between the two.
    pub(super) const fn midpoint(self, other: Self) -> Self {
        Self::new(self.x.midpoint(other.x), self.y.midpoint(other.y))
    }
}

/// How wide and how high something is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Size {
    /// Across.
    pub(super) width: Len,
    /// Down.
    pub(super) height: Len,
}

impl Size {
    /// `width` across and `height` down.
    pub(super) const fn new(width: Len, height: Len) -> Self {
        Self { width, height }
    }
}

/// A box by its four sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    /// The left side.
    pub(super) left: Len,
    /// The top side.
    pub(super) top: Len,
    /// The right side.
    pub(super) right: Len,
    /// The bottom side.
    pub(super) bottom: Len,
}

impl Rect {
    /// Inside out: the union of nothing, which any box replaces.
    pub(super) const NOTHING: Self = Self {
        left: Len::MAX,
        top: Len::MAX,
        right: Len::MIN,
        bottom: Len::MIN,
    };

    /// The smallest box holding both.
    pub(super) const fn union(self, other: Self) -> Self {
        Self {
            left: self.left.min(other.left),
            top: self.top.min(other.top),
            right: self.right.max(other.right),
            bottom: self.bottom.max(other.bottom),
        }
    }

    /// The box moved by `by`.
    pub(super) fn moved(self, by: Point) -> Self {
        Self {
            left: self.left + by.x,
            top: self.top + by.y,
            right: self.right + by.x,
            bottom: self.bottom + by.y,
        }
    }
}

/// A box by its centre and size: a node as placed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bounds {
    /// The middle.
    pub(super) centre: Point,
    /// The size.
    pub(super) size: Size,
}

/// The outline a line from the centre of a box leaves it through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outline {
    /// The box itself.
    Box,
    /// The diamond with its tips on the middles of the box's sides.
    Diamond,
    /// The circle as wide as the box.
    Circle,
}

impl Bounds {
    /// The box's four sides.
    pub(super) fn rect(self) -> Rect {
        let Self { centre, size } = self;
        Rect {
            left: centre.x - size.width / 2.0_f64,
            top: centre.y - size.height / 2.0_f64,
            right: centre.x + size.width / 2.0_f64,
            bottom: centre.y + size.height / 2.0_f64,
        }
    }

    /// Where the line from the centre toward `to` leaves `outline`; the
    /// centre itself when `to` is there too.
    pub(super) fn exit(self, outline: Outline, to: Point) -> Point {
        /// Closer than this is the same place.
        const NEAR: Len = px(1e-9_f64);
        let Self { centre, size } = self;
        let (dx, dy) = (to.x - centre.x, to.y - centre.y);
        if dx.abs() < NEAR && dy.abs() < NEAR {
            return centre;
        }
        let (half_width, half_height) = (size.width / 2.0_f64, size.height / 2.0_f64);
        let reach = |half: Len, offset: Len| {
            if offset.abs() > NEAR {
                half.ratio(offset.abs())
            } else {
                f64::INFINITY
            }
        };
        let along = match outline {
            Outline::Diamond => {
                diamond_reach(dx.abs().ratio(half_width), dy.abs().ratio(half_height))
            }
            Outline::Circle => half_width.ratio(dx.hypot(dy)),
            Outline::Box => reach(half_width, dx).min(reach(half_height, dy)),
        };
        Point::new(dx.mul_add(along, centre.x), dy.mul_add(along, centre.y))
    }
}
