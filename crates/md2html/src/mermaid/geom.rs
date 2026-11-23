// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Lengths on a drawing, and the few other numbers the layout reckons with.
//!
//! Every sum, product and quotient of floating-point numbers the
//! diagrams need is written in this file, once, behind these types; the
//! layout and the drawing add, scale and compare lengths without touching
//! the numbers inside.
//!
//! This file is the one exception to the lint table in md2html, decided by
//! the owner on 2026-11-23: a drawing's geometry is floating-point by
//! nature (positions, widths, midpoints, the reach of a circle), and this
//! is the only place its arithmetic is written. Nothing else in md2html
//! does floating-point arithmetic, and nothing else carries an exception.

#![expect(
    clippy::float_arithmetic,
    reason = "the geometry of a drawing is floating-point; this file is where its arithmetic \
              lives, as the one exception the owner allowed (see the module documentation)"
)]

use core::fmt;
use core::iter::Sum;
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A length or a coordinate, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub(super) struct Len(f64);

/// `pixels` as a length.
pub(super) const fn px(pixels: f64) -> Len {
    Len(pixels)
}

/// A count to scale a length by: lines, nested frames, places in a rank.
/// Every count here is far below `u32::MAX`; a larger one is taken as that.
pub(super) fn count(items: usize) -> f64 {
    u32::try_from(items).map_or_else(|_| f64::from(u32::MAX), f64::from)
}

impl Len {
    /// Further right or down than anything: where a search for the least
    /// starts.
    pub(super) const MAX: Self = Self(f64::INFINITY);
    /// Further left or up than anything: where a search for the most starts.
    pub(super) const MIN: Self = Self(f64::NEG_INFINITY);

    /// The smaller of the two.
    pub(super) const fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// The larger of the two.
    pub(super) const fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    /// The length without its sign.
    pub(super) const fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Halfway between the two.
    pub(super) const fn midpoint(self, other: Self) -> Self {
        Self(self.0.midpoint(other.0))
    }

    /// `self` times `factor`, plus `add`, rounded once.
    pub(super) fn mul_add(self, factor: f64, add: Self) -> Self {
        Self(self.0.mul_add(factor, add.0))
    }

    /// The length of the diagonal with `self` and `other` as its sides.
    pub(super) fn hypot(self, other: Self) -> Self {
        Self(self.0.hypot(other.0))
    }

    /// How many times `whole` goes into `self`.
    pub(super) fn ratio(self, whole: Self) -> f64 {
        self.0 / whole.0
    }

    /// Kept between `low` and `high` (`low` is never above `high` here).
    pub(super) const fn clamp(self, low: Self, high: Self) -> Self {
        self.max(low).min(high)
    }
}

impl Add for Len {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Len {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign for Len {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Len {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Neg for Len {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Mul<f64> for Len {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<f64> for Len {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self(self.0 / rhs)
    }
}

impl Sum for Len {
    fn sum<Lengths: Iterator<Item = Self>>(iter: Lengths) -> Self {
        Self(iter.map(|length| length.0).sum())
    }
}

impl<'len> Sum<&'len Self> for Len {
    fn sum<Lengths: Iterator<Item = &'len Self>>(iter: Lengths) -> Self {
        iter.copied().sum()
    }
}

/// Written as its number of pixels, with the precision the format asks for.
impl fmt::Display for Len {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// An average, gathered one value at a time.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Mean {
    /// The values so far, added.
    sum: f64,
    /// How many there were.
    items: usize,
}

impl Mean {
    /// Counts one more value.
    pub(super) fn add(&mut self, value: f64) {
        self.sum += value;
        self.items = self.items.saturating_add(1);
    }

    /// The average of the values counted.
    pub(super) fn value(self) -> f64 {
        self.sum / count(self.items)
    }
}

/// How far from a diamond's centre toward a point its outline lies.
///
/// The answer is a fraction of the way along the line to the point.
/// `across` and `down` are the point's offsets as fractions of the
/// diamond's half-width and half-height.
pub(super) fn diamond_reach(across: f64, down: f64) -> f64 {
    1.0_f64 / (across + down)
}
