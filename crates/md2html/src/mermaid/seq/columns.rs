// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The columns of a sequence diagram, one for each participant: each box
//! as wide as its name, the columns far enough apart for their boxes and
//! for every message and note between them.

use core::iter::once;

use super::model::{ActorId, Diagram, Message, Note, Side, Step};
use crate::mermaid::geom::{Len, count, px};
use crate::mermaid::table::Id as _;

/// The margin around the drawing.
pub(super) const PAD: Len = px(20.0_f64);
/// The size of a participant's name.
pub(super) const ACTOR_FONT: Len = px(14.0_f64);
/// The size of the words of messages and notes.
pub(super) const TEXT_FONT: Len = px(13.0_f64);

/// A participant's column.
///
/// Its activations open and close like brackets, and each one closed is
/// a bar as deep as the activations still open around it. The fields are
/// private so that only [`Self::activate`], [`Self::deactivate`] and
/// [`Self::open_bars`] change or read the activations, and the column's
/// place is set only by [`Columns::new`].
#[derive(Clone, Debug)]
pub(super) struct Column {
    /// The width of its box.
    width: Len,
    /// The room its messages to itself take to its right.
    loop_room: Len,
    /// Its centre across the page.
    centre: Len,
    /// Where its open activations began, innermost last.
    active: Vec<Len>,
}

/// An activation bar.
pub(super) struct Bar {
    /// The centre of its participant's column.
    pub(super) centre: Len,
    /// Its top.
    pub(super) start: Len,
    /// Its bottom.
    pub(super) end: Len,
    /// How many activations of the same participant it lies inside.
    pub(super) depth: usize,
}

impl Column {
    /// The width of its box.
    pub(super) const fn width(&self) -> Len {
        self.width
    }

    /// Its centre across the page.
    pub(super) const fn centre(&self) -> Len {
        self.centre
    }

    /// Whether an activation is open.
    pub(super) fn is_active(&self) -> bool {
        !self.active.is_empty()
    }

    /// Opens an activation at `at`, inside the ones already open.
    pub(super) fn activate(&mut self, at: Len) {
        self.active.push(at);
    }

    /// Closes the innermost activation at `end`: its bar, or none when no
    /// activation is open.
    #[must_use = "the bar of the activation closed is drawn only if it is kept"]
    pub(super) fn deactivate(&mut self, end: Len) -> Option<Bar> {
        let start = self.active.pop()?;
        Some(Bar {
            centre: self.centre,
            start,
            end,
            depth: self.active.len(),
        })
    }

    /// The bars of the activations still open, closed at `end`, outermost
    /// first.
    pub(super) fn open_bars(&self, end: Len) -> impl Iterator<Item = Bar> {
        self.active
            .iter()
            .enumerate()
            .map(move |(depth, &start)| Bar {
                centre: self.centre,
                start,
                end,
                depth,
            })
    }

    /// A column for a box `width` wide, not yet placed.
    const fn new(width: Len) -> Self {
        Self {
            width,
            loop_room: px(0.0_f64),
            centre: px(0.0_f64),
            active: Vec::new(),
        }
    }

    /// Where a message leaves it: beside its innermost activation.
    pub(super) fn bar_x(&self) -> Len {
        px(5.0_f64).mul_add(count(self.active.len()), self.centre)
    }
}

/// A column after the first, and how far its centre is from the previous
/// column's.
#[derive(Clone, Debug)]
struct Spaced {
    /// The distance from the previous column's centre.
    gap: Len,
    /// The column.
    column: Column,
}

/// The columns, one for each participant: never none, because a diagram
/// has at least one participant.
#[derive(Clone, Debug)]
pub(super) struct Columns {
    /// The first column, at the left margin.
    first: Column,
    /// The others, in order, each with its gap.
    rest: Vec<Spaced>,
}

impl Columns {
    /// The columns of a diagram, spaced for what passes between them; none
    /// when a step names a participant it does not hold.
    pub(super) fn new(diagram: &Diagram) -> Option<Self> {
        let widths = diagram
            .actors
            .map(|actor| (actor.label.size(ACTOR_FONT).width + px(28.0_f64)).max(px(96.0_f64)));
        let mut before = *widths.first();
        let mut columns = Self {
            first: Column::new(before),
            rest: Vec::with_capacity(widths.rest().len()),
        };
        for &width in widths.rest() {
            columns.rest.push(Spaced {
                gap: before.midpoint(width) + px(36.0_f64),
                column: Column::new(width),
            });
            before = width;
        }
        for step in &diagram.steps {
            match step {
                Step::Message(message) if message.from != message.to => {
                    columns.make_room(message, diagram.numbered)?;
                }
                Step::Message(message) => {
                    let column = columns.get_mut(message.from)?;
                    column.loop_room = column
                        .loop_room
                        .max(message.text.size(TEXT_FONT).width + px(50.0_f64));
                }
                Step::Note(note) if note.side != Side::Over || note.first == note.last => {
                    columns.note_room(note);
                }
                Step::Note(_)
                | Step::Open { kind: _, text: _ }
                | Step::Divide { text: _ }
                | Step::Close
                | Step::Activation { who: _, on: _ } => {}
            }
        }
        columns.place();
        Some(columns)
    }

    /// Widens each gap for the loops of the column before it, then places
    /// the columns from the left margin.
    fn place(&mut self) {
        let mut room_before = self.first.loop_room;
        for spaced in &mut self.rest {
            spaced.gap = spaced.gap.max(room_before + spaced.column.width / 2.0_f64);
            room_before = spaced.column.loop_room;
        }
        self.first.centre = PAD + self.first.width / 2.0_f64;
        let mut centre_before = self.first.centre;
        for spaced in &mut self.rest {
            spaced.column.centre = centre_before + spaced.gap;
            centre_before = spaced.column.centre;
        }
    }

    /// The columns after the first with their participants and gaps.
    fn spaced_mut(&mut self) -> impl Iterator<Item = (ActorId, &mut Spaced)> {
        self.rest
            .iter_mut()
            .enumerate()
            .map(|(place, spaced)| (ActorId::at(place + 1), spaced))
    }

    /// Widens the gaps a message crosses, at its far end, until its words
    /// fit. A message between two participants crosses at least the gap
    /// before the later one; none when that gap is not there.
    fn make_room(&mut self, message: &Message, numbered: bool) -> Option<()> {
        let (low, high) = (message.from.min(message.to), message.from.max(message.to));
        let need = message.text.size(TEXT_FONT).width
            + px(40.0_f64)
            + if numbered { px(24.0_f64) } else { px(0.0_f64) };
        let mut crossed: Vec<&mut Len> = self
            .spaced_mut()
            .filter(|(id, _)| low < *id && *id <= high)
            .map(|(_, spaced)| &mut spaced.gap)
            .collect();
        let have: Len = crossed.iter().map(|gap| **gap).sum();
        let far = crossed.pop()?;
        if have < need {
            *far += need - have;
        }
        Some(())
    }

    /// Widens the gap beside a note on one side of one participant: the
    /// gap after it for a note to its right, before it for one to its left.
    /// The last column has no gap after it, the first none before it.
    fn note_room(&mut self, note: &Note) {
        let wide = note.text.size(TEXT_FONT).width + px(24.0_f64) + px(30.0_f64);
        let beside = match note.side {
            Side::Right => self.spaced_mut().find(|(id, _)| *id > note.first),
            Side::Left => self.spaced_mut().find(|(id, _)| *id == note.first),
            Side::Over => None,
        };
        if let Some((_, spaced)) = beside {
            spaced.gap = spaced.gap.max(wide);
        }
    }

    /// The column of `id`.
    pub(super) fn get(&self, id: ActorId) -> Option<&Column> {
        id.index()
            .checked_sub(1)
            .map_or(Some(&self.first), |place| {
                self.rest.get(place).map(|spaced| &spaced.column)
            })
    }

    /// The column of `id`, to change.
    pub(super) fn get_mut(&mut self, id: ActorId) -> Option<&mut Column> {
        match id.index().checked_sub(1) {
            None => Some(&mut self.first),
            Some(place) => self.rest.get_mut(place).map(|spaced| &mut spaced.column),
        }
    }

    /// The columns, in order.
    pub(super) fn values(&self) -> impl Iterator<Item = &Column> {
        once(&self.first).chain(self.rest.iter().map(|spaced| &spaced.column))
    }

    /// The last column: the first when there is only one.
    pub(super) fn last(&self) -> &Column {
        self.rest
            .last()
            .map_or(&self.first, |spaced| &spaced.column)
    }

    /// The participants of the first and the last column.
    pub(super) fn span(&self) -> (ActorId, ActorId) {
        (ActorId::at(0), ActorId::at(self.rest.len()))
    }
}
