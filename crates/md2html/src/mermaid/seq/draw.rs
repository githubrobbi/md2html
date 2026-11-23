// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A sequence diagram as SVG: a column for each participant, far enough
//! apart for what passes between them, then the steps row by row down the
//! page.

use super::columns::{ACTOR_FONT, Columns, PAD, TEXT_FONT};
use super::model::{ActorId, Diagram, Message, Note, Side, Step, Tip, Toggle};
use crate::mermaid::geom::{Len, count, px};
use crate::mermaid::label::text_width;
use crate::mermaid::plane::{Point, Size};
use crate::mermaid::svg_open;
use crate::text::{esc, put};

/// The size of a frame's keyword.
const KIND_FONT: Len = px(12.0_f64);
/// How many steps the description reads out at most.
const DESCRIBED_STEPS: usize = 60;

/// An activation bar.
struct Bar {
    /// The centre of its participant's column.
    centre: Len,
    /// Its top.
    start: Len,
    /// Its bottom.
    end: Len,
    /// How many activations of the same participant it lies inside.
    depth: usize,
}

/// A line dividing a frame.
struct Divide {
    /// Where it is, down the page.
    y: Len,
    /// The words on it.
    text: String,
}

/// A frame being drawn.
struct Frame {
    /// Its keyword.
    kind: String,
    /// Its condition or title.
    text: String,
    /// Its top.
    top: Len,
    /// The lines that divide it.
    divides: Vec<Divide>,
    /// The leftmost and rightmost participant it covers, once known.
    span: Option<(ActorId, ActorId)>,
}

/// The steps being drawn, row by row.
struct Rows<'diagram> {
    /// The diagram.
    diagram: &'diagram Diagram,
    /// The diagram's number on its page.
    n: usize,
    /// The columns.
    columns: Columns,
    /// How far down the page the rows have come.
    y: Len,
    /// What has been drawn.
    body: String,
    /// The frames open, innermost last.
    frames: Vec<Frame>,
    /// The activation bars finished.
    bars: Vec<Bar>,
    /// The number of the last message.
    number: usize,
    /// The leftmost anything reaches.
    left_edge: Len,
    /// The rightmost anything reaches.
    right_edge: Len,
}

/// A sequence diagram as SVG; `n` numbers the diagram on its page. None
/// when a step names a participant the diagram does not hold.
pub(in crate::mermaid) fn draw(diagram: &Diagram, n: usize) -> Option<String> {
    let columns = Columns::new(diagram)?;
    let box_height = diagram
        .actors
        .values()
        .map(|actor| actor.label.size(ACTOR_FONT).height + px(22.0_f64))
        .fold(px(40.0_f64), Len::max);
    let last = columns.last();
    let right_edge = last.centre + last.width / 2.0_f64;
    let mut rows = Rows {
        diagram,
        n,
        columns,
        y: PAD + box_height + px(24.0_f64),
        body: String::new(),
        frames: Vec::new(),
        bars: Vec::new(),
        number: 0,
        left_edge: PAD,
        right_edge,
    };
    for step in &diagram.steps {
        rows.step(step)?;
    }
    rows.finish(box_height)
}

impl Rows<'_> {
    /// One step.
    fn step(&mut self, step: &Step) -> Option<()> {
        match step {
            Step::Message(message) => self.message(message)?,
            Step::Activation { who, on } => {
                let column = self.columns.get_mut(*who)?;
                if *on {
                    column.active.push(self.y);
                } else if let Some(start) = column.active.pop() {
                    self.bars.push(Bar {
                        centre: column.centre,
                        start,
                        end: self.y,
                        depth: column.active.len(),
                    });
                }
            }
            Step::Note(note) => self.note(note)?,
            Step::Open { kind, text } => {
                self.y += px(10.0_f64);
                self.frames.push(Frame {
                    kind: kind.clone(),
                    text: text.clone(),
                    top: self.y,
                    divides: Vec::new(),
                    span: None,
                });
                self.y += px(26.0_f64);
            }
            Step::Divide { text } => {
                self.y += px(8.0_f64);
                if let Some(frame) = self.frames.last_mut() {
                    frame.divides.push(Divide {
                        y: self.y,
                        text: text.clone(),
                    });
                }
                self.y += px(22.0_f64);
            }
            Step::Close => {
                self.y += px(10.0_f64);
                if let Some(frame) = self.frames.pop() {
                    self.close(&frame)?;
                }
                self.y += px(6.0_f64);
            }
        }
        Some(())
    }

    /// Every open frame now covers the participants from `low` to `high`.
    fn touch(&mut self, low: ActorId, high: ActorId) {
        for frame in &mut self.frames {
            frame.span = Some(match frame.span {
                None => (low, high),
                Some((first, last)) => (first.min(low), last.max(high)),
            });
        }
    }

    /// A message: its arrow, its words, its number.
    fn message(&mut self, message: &Message) -> Option<()> {
        let Message {
            from,
            to,
            arrow,
            text,
            toggle,
        } = message;
        let height = text.size(TEXT_FONT).height;
        let has_text = !text.is_empty();
        self.y += if has_text {
            height + px(6.0_f64)
        } else {
            px(6.0_f64)
        };
        self.number += 1;
        let class = if arrow.dotted { "msg dotted" } else { "msg" };
        let n = self.n;
        let marker = match arrow.tip {
            Tip::Filled => format!(" marker-end=\"url(#dg{n}-a)\""),
            Tip::Cross => format!(" marker-end=\"url(#dg{n}-x)\""),
            Tip::Open => format!(" marker-end=\"url(#dg{n}-v)\""),
            Tip::None => String::new(),
        };
        self.touch((*from).min(*to), (*from).max(*to));
        let y = self.y;
        let sender = self.columns.get(*from)?;
        let start = sender.bar_x();
        let leaves = if from == to {
            put!(
                self.body,
                "<path class=\"{class}\" d=\"M{start:.1} {y:.1}H{:.1}V{:.1}H{:.1}\"{marker}/>",
                start + px(36.0_f64),
                y + px(22.0_f64),
                start + px(2.0_f64)
            );
            if has_text {
                let width = text.size(TEXT_FONT).width;
                text.draw(
                    &mut self.body,
                    Point::new(
                        start + px(44.0_f64) + width / 2.0_f64,
                        y - height / 2.0_f64 + px(2.0_f64),
                    ),
                    "msg-text",
                );
                self.right_edge = self.right_edge.max(start + px(52.0_f64) + width);
            }
            start
        } else {
            let (from_centre, active) = (sender.centre, !sender.active.is_empty());
            let to_centre = self.columns.get(*to)?.centre;
            let (leaves, arrives) = if from < to {
                (start, to_centre - px(2.0_f64))
            } else if active {
                (start - px(10.0_f64), to_centre + px(2.0_f64))
            } else {
                (start, to_centre + px(2.0_f64))
            };
            put!(
                self.body,
                "<path class=\"{class}\" d=\"M{leaves:.1} {y:.1}H{arrives:.1}\"{marker}/>"
            );
            if has_text {
                text.draw(
                    &mut self.body,
                    Point::new(
                        from_centre.midpoint(to_centre),
                        y - height / 2.0_f64 - px(2.0_f64),
                    ),
                    "msg-text",
                );
            }
            leaves
        };
        if self.diagram.numbered {
            let number = self.number;
            put!(
                self.body,
                "<circle class=\"num\" cx=\"{leaves:.1}\" cy=\"{y:.1}\" r=\"9\"/><text class=\"num-text\" x=\"{leaves:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" dominant-baseline=\"central\">{number}</text>"
            );
        }
        self.y += if from == to {
            px(34.0_f64)
        } else {
            px(22.0_f64)
        };
        self.toggle(*toggle, *from, *to)
    }

    /// What a message does to activations, once it is drawn.
    fn toggle(&mut self, toggle: Toggle, from: ActorId, to: ActorId) -> Option<()> {
        let at = self.y - px(16.0_f64);
        match toggle {
            Toggle::Keep => {}
            Toggle::Activate => self.columns.get_mut(to)?.active.push(at),
            Toggle::Deactivate => {
                let column = self.columns.get_mut(from)?;
                if let Some(start) = column.active.pop() {
                    self.bars.push(Bar {
                        centre: column.centre,
                        start,
                        end: at,
                        depth: column.active.len(),
                    });
                }
            }
        }
        Some(())
    }

    /// A note beside or over participants.
    fn note(&mut self, note: &Note) -> Option<()> {
        let Size {
            width: text_width,
            height: text_height,
        } = note.text.size(TEXT_FONT);
        let (width, height) = (text_width + px(20.0_f64), text_height + px(12.0_f64));
        let first = self.columns.get(note.first)?.centre;
        let (left, right) = match note.side {
            Side::Right => (first + px(10.0_f64), first + px(10.0_f64) + width),
            Side::Left => (first - px(10.0_f64) - width, first - px(10.0_f64)),
            Side::Over => {
                let last = self.columns.get(note.last)?.centre;
                let middle = first.midpoint(last);
                let half = (width / 2.0_f64).max((last - first) / 2.0_f64 + px(30.0_f64));
                (middle - half, middle + half)
            }
        };
        self.touch(note.first, note.last);
        self.y += px(8.0_f64);
        let y = self.y;
        put!(
            self.body,
            "<rect class=\"note\" x=\"{left:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{height:.1}\" rx=\"3\"/>",
            right - left
        );
        note.text.draw(
            &mut self.body,
            Point::new(left.midpoint(right), y + height / 2.0_f64),
            "note-text",
        );
        self.right_edge = self.right_edge.max(right);
        self.left_edge = self.left_edge.min(left);
        self.y += height + px(10.0_f64);
        Some(())
    }

    /// A frame closed: drawn under what it frames, as wide as the
    /// participants it covers (all of them when it covers none), a little
    /// wider for each frame around it.
    fn close(&mut self, frame: &Frame) -> Option<()> {
        let (first, last) = frame.span.unwrap_or_else(|| self.columns.span());
        let depth = count(self.frames.len());
        let (low, high) = (self.columns.get(first)?, self.columns.get(last)?);
        let left = px(-8.0_f64).mul_add(depth, low.centre - low.width / 2.0_f64 + px(4.0_f64));
        let right = px(8.0_f64).mul_add(depth, high.centre + high.width / 2.0_f64 - px(4.0_f64));
        let top = frame.top;
        let mut drawn = String::new();
        put!(
            drawn,
            "<g class=\"frame\"><rect x=\"{left:.1}\" y=\"{top:.1}\" width=\"{:.1}\" height=\"{:.1}\"/>",
            right - left,
            self.y - top
        );
        if frame.kind != "rect" {
            let tab = text_width(&frame.kind, KIND_FONT) + px(16.0_f64);
            put!(
                drawn,
                "<path class=\"frame-tab\" d=\"M{left:.1} {top:.1}h{tab:.1}v14l-6 6H{left:.1}z\"/><text class=\"frame-kind\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">{}</text>",
                left + px(6.0_f64),
                top + px(10.0_f64),
                esc(&frame.kind)
            );
            if !frame.text.is_empty() {
                put!(
                    drawn,
                    "<text class=\"frame-text\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">[{}]</text>",
                    left + tab + px(8.0_f64),
                    top + px(10.0_f64),
                    esc(&frame.text)
                );
            }
        }
        for divide in &frame.divides {
            let at = divide.y;
            put!(
                drawn,
                "<path class=\"frame-divide\" d=\"M{left:.1} {at:.1}H{right:.1}\"/>"
            );
            if !divide.text.is_empty() {
                put!(
                    drawn,
                    "<text class=\"frame-text\" x=\"{:.1}\" y=\"{:.1}\" dominant-baseline=\"central\">[{}]</text>",
                    left + px(8.0_f64),
                    at + px(11.0_f64),
                    esc(&divide.text)
                );
            }
        }
        drawn.push_str("</g>");
        self.body.insert_str(0, &drawn);
        self.left_edge = self.left_edge.min(left);
        self.right_edge = self.right_edge.max(right);
        Some(())
    }

    /// The whole diagram: lifelines, activation bars, the rows drawn, and
    /// each participant's box at the top and the bottom.
    fn finish(mut self, box_height: Len) -> Option<String> {
        for column in self.columns.values() {
            for (depth, &start) in column.active.iter().enumerate() {
                self.bars.push(Bar {
                    centre: column.centre,
                    start,
                    end: self.y,
                    depth,
                });
            }
        }
        let bottom = self.y + px(18.0_f64);
        let height = bottom + box_height + PAD;
        let shift = if self.left_edge < PAD {
            PAD - self.left_edge
        } else {
            px(0.0_f64)
        };
        let width = self.right_edge + PAD + shift;
        let diagram = self.diagram;
        let names = diagram.actors.map(|actor| actor.label.joined());
        let title = diagram.title.clone().unwrap_or_else(|| {
            names
                .values()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        });
        let mut said = String::new();
        for step in diagram.steps.iter().take(DESCRIBED_STEPS) {
            if let Step::Message(message) = step {
                put!(
                    said,
                    "{} → {}: {}; ",
                    names.get(message.from)?,
                    names.get(message.to)?,
                    message.text.joined()
                );
            }
        }
        let mut desc = said.trim_end_matches("; ").to_owned();
        desc.push('.');
        let mut out = String::new();
        svg_open(&mut out, self.n, Size::new(width, height), &title, &desc);
        put!(out, "<g transform=\"translate({shift:.1} 0)\">");
        for column in self.columns.values() {
            put!(
                out,
                "<path class=\"lifeline\" d=\"M{:.1} {:.1}V{bottom:.1}\"/>",
                column.centre,
                PAD + box_height
            );
        }
        for bar in &self.bars {
            let x = px(5.0_f64).mul_add(count(bar.depth), bar.centre - px(5.0_f64));
            put!(
                out,
                "<rect class=\"activation\" x=\"{x:.1}\" y=\"{:.1}\" width=\"10\" height=\"{:.1}\"/>",
                bar.start,
                (bar.end - bar.start).max(px(8.0_f64))
            );
        }
        out.push_str(&self.body);
        for (actor, column) in diagram.actors.values().zip(self.columns.values()) {
            let class = if actor.person {
                "actor person"
            } else {
                "actor"
            };
            let left = column.centre - column.width / 2.0_f64;
            for top in [PAD, bottom] {
                put!(
                    out,
                    "<g class=\"{class}\"><rect x=\"{left:.1}\" y=\"{top:.1}\" width=\"{:.1}\" height=\"{box_height:.1}\" rx=\"5\"/>",
                    column.width
                );
                actor.label.draw(
                    &mut out,
                    Point::new(column.centre, top + box_height / 2.0_f64),
                    "label",
                );
                out.push_str("</g>");
            }
        }
        out.push_str("</g></svg></figure>");
        Some(out)
    }
}
