// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A sequence diagram as read: its participants and what happens between
//! them, in order.

use crate::mermaid::label::Label;
use crate::mermaid::table::{Filled, Id};

/// The number of a participant: its place among them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(super) struct ActorId(usize);

impl Id for ActorId {
    fn at(index: usize) -> Self {
        Self(index)
    }
    fn index(self) -> usize {
        self.0
    }
}

/// A participant.
#[derive(Clone, Debug)]
pub(super) struct Actor {
    /// Its label.
    pub(super) label: Label,
    /// How it is drawn.
    pub(super) figure: Figure,
}

/// How a participant is drawn, by the word that declares it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Figure {
    /// `participant`, or a name a message uses first: a box.
    Participant,
    /// `actor`: a person.
    Actor,
}

/// The head of a message's arrow.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Tip {
    /// `>>`: filled.
    Filled,
    /// `)`: open.
    Open,
    /// `x`: a cross.
    Cross,
    /// `>`: none.
    None,
}

/// A message's line and head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct Arrow {
    /// Drawn dotted.
    pub(super) dotted: bool,
    /// The head.
    pub(super) tip: Tip,
}

/// What a message does to the activation of its participants.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Toggle {
    /// Nothing.
    Keep,
    /// `+`: activates the receiver.
    Activate,
    /// `-`: deactivates the sender.
    Deactivate,
}

/// A message between two participants, or to itself.
#[derive(Debug)]
pub(super) struct Message {
    /// The sender.
    pub(super) from: ActorId,
    /// The receiver.
    pub(super) to: ActorId,
    /// The arrow drawn.
    pub(super) arrow: Arrow,
    /// The words on it.
    pub(super) text: Label,
    /// What it does to activations.
    pub(super) toggle: Toggle,
}

/// Where a note stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Side {
    /// Left of a participant.
    Left,
    /// Over one participant or across several.
    Over,
    /// Right of a participant.
    Right,
}

/// A note beside or over participants.
#[derive(Debug)]
pub(super) struct Note {
    /// The first participant it spans.
    pub(super) first: ActorId,
    /// The last participant it spans.
    pub(super) last: ActorId,
    /// Where it stands.
    pub(super) side: Side,
    /// The words.
    pub(super) text: Label,
}

/// One step of the diagram, in order.
#[derive(Debug)]
pub(super) enum Step {
    /// A message.
    Message(Message),
    /// A note.
    Note(Note),
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
        /// The participant.
        who: ActorId,
        /// On, or off.
        on: bool,
    },
}

/// A sequence diagram as read.
#[derive(Debug)]
pub(in crate::mermaid) struct Diagram {
    /// The participants, in the order they were first named: at least one.
    pub(super) actors: Filled<ActorId, Actor>,
    /// What happens, in order.
    pub(super) steps: Vec<Step>,
    /// Messages are numbered (`autonumber`).
    pub(super) numbered: bool,
    /// The author's title, if given.
    pub(super) title: Option<String>,
}

impl Diagram {
    /// How many participants it has.
    pub(in crate::mermaid) fn actor_count(&self) -> usize {
        self.actors.count()
    }
}
