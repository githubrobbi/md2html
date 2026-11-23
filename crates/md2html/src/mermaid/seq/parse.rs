// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading a sequence diagram, line by line.

use alloc::collections::BTreeMap;
use core::cmp::Reverse;

use super::model::{Actor, ActorId, Arrow, Diagram, Message, Note, Side, Step, Tip, Toggle};
use crate::mermaid::label::Label;
use crate::mermaid::missing;
use crate::mermaid::table::Table;

/// The arrows of a sequence diagram, by how each is written.
const ARROWS: &[(&str, Arrow)] = &[
    ("-->>", Arrow {
        dotted: true,
        tip: Tip::Filled,
    }),
    ("->>", Arrow {
        dotted: false,
        tip: Tip::Filled,
    }),
    ("--x", Arrow {
        dotted: true,
        tip: Tip::Cross,
    }),
    ("-x", Arrow {
        dotted: false,
        tip: Tip::Cross,
    }),
    ("--)", Arrow {
        dotted: true,
        tip: Tip::Open,
    }),
    ("-)", Arrow {
        dotted: false,
        tip: Tip::Open,
    }),
    ("-->", Arrow {
        dotted: true,
        tip: Tip::None,
    }),
    ("->", Arrow {
        dotted: false,
        tip: Tip::None,
    }),
];

/// The diagram in a block's lines after its header.
pub(in crate::mermaid) fn parse(lines: &[&str]) -> Result<Diagram, String> {
    let mut reader = Reader {
        actors: Table::default(),
        steps: Vec::new(),
        numbered: false,
        title: None,
        ids: BTreeMap::new(),
        open: 0,
    };
    for line in lines {
        reader.line(line)?;
    }
    let actors = reader
        .actors
        .filled()
        .ok_or_else(|| String::from("the sequence diagram has no participants"))?;
    Ok(Diagram {
        actors,
        steps: reader.steps,
        numbered: reader.numbered,
        title: reader.title,
    })
}

/// A sequence diagram being read.
struct Reader {
    /// The participants so far, in the order they were first named.
    actors: Table<ActorId, Actor>,
    /// What happens, so far.
    steps: Vec<Step>,
    /// Messages are numbered (`autonumber`).
    numbered: bool,
    /// The author's title, if given.
    title: Option<String>,
    /// The participants by their ids in the source.
    ids: BTreeMap<String, ActorId>,
    /// How many frames are open.
    open: usize,
}

impl Reader {
    /// The participant `id`, added the first time it is named; a `label`
    /// given later replaces the one it had.
    fn actor(&mut self, id: &str, label: Option<&str>, person: bool) -> Result<ActorId, String> {
        let name = id.trim();
        if let Some(&known) = self.ids.get(name) {
            if let Some(text) = label {
                self.actors.get_mut(known).ok_or_else(missing)?.label = Label::parse(text);
            }
            return Ok(known);
        }
        let actor = self.actors.push(Actor {
            label: Label::parse(label.unwrap_or(name)),
            person,
        });
        self.ids.insert(name.to_owned(), actor);
        Ok(actor)
    }

    /// One line.
    fn line(&mut self, line: &str) -> Result<(), String> {
        let (word, rest) = line
            .split_once(char::is_whitespace)
            .map_or((line, ""), |(word, rest)| (word, rest.trim()));
        match word {
            "participant" | "actor" => {
                let (id, label) = match rest.split_once(" as ") {
                    Some((id, label)) => (id, Some(label)),
                    None => (rest, None),
                };
                self.actor(id, label, word == "actor")?;
            }
            "autonumber" => self.numbered = true,
            "title" => self.title = Some(rest.trim_start_matches(':').trim().to_owned()),
            "activate" | "deactivate" => {
                let who = self.actor(rest, None, false)?;
                self.steps.push(Step::Activation {
                    who,
                    on: word == "activate",
                });
            }
            "loop" | "alt" | "opt" | "par" | "critical" | "break" | "rect" => {
                self.open += 1;
                self.steps.push(Step::Open {
                    kind: word.to_owned(),
                    text: if word == "rect" { "" } else { rest }.to_owned(),
                });
            }
            "else" | "and" | "option" => self.steps.push(Step::Divide {
                text: rest.to_owned(),
            }),
            // Without a frame open, `end` closes a `box`, which is not drawn.
            "end" => {
                if self.open > 0 {
                    self.open -= 1;
                    self.steps.push(Step::Close);
                }
            }
            "box" | "accTitle:" | "accDescr:" | "links" | "link" | "destroy" | "create" => {}
            _ if word.eq_ignore_ascii_case("note") => self.note(line, rest)?,
            _ => self.message(line)?,
        }
        Ok(())
    }

    /// `Note right of A: text`, `Note left of A: ...`, `Note over A,B: ...`.
    fn note(&mut self, line: &str, rest: &str) -> Result<(), String> {
        let (place, text) = rest
            .split_once(':')
            .ok_or_else(|| format!("a note without text: `{line}`"))?;
        let spot = place.trim();
        let (side, who) = if let Some(names) = spot.strip_prefix("right of") {
            (Side::Right, names)
        } else if let Some(names) = spot.strip_prefix("left of") {
            (Side::Left, names)
        } else if let Some(names) = spot.strip_prefix("over") {
            (Side::Over, names)
        } else {
            return Err(format!("cannot place the note `{line}`"));
        };
        let (first_name, more) = match who.split_once(',') {
            Some((one, more)) => (one, Some(more)),
            None => (who, None),
        };
        let one = self.actor(first_name, None, false)?;
        let other = match more {
            Some(names) => {
                let second_name = names.split_once(',').map_or(names, |(two, _)| two);
                self.actor(second_name, None, false)?
            }
            None => one,
        };
        self.steps.push(Step::Note(Note {
            first: one.min(other),
            last: one.max(other),
            side,
            text: Label::parse(text),
        }));
        Ok(())
    }

    /// `A->>B: text`, with `+` or `-` before the receiver to activate it or
    /// deactivate the sender.
    fn message(&mut self, line: &str) -> Result<(), String> {
        let (head, text) = line.split_once(':').unwrap_or((line, ""));
        let Some((sender, receiver, arrow)) = ARROWS
            .iter()
            .filter_map(|(written, arrow)| {
                head.split_once(written)
                    .map(|(before, after)| (before, after, *arrow, written.len()))
            })
            .min_by_key(|(before, _, _, length)| (before.len(), Reverse(*length)))
            .map(|(before, after, arrow, _)| (before.trim(), after.trim(), arrow))
        else {
            return Err(format!("cannot read `{line}`"));
        };
        let (toggle, target) = [('+', Toggle::Activate), ('-', Toggle::Deactivate)]
            .into_iter()
            .find_map(|(sign, toggle)| {
                receiver
                    .strip_prefix(sign)
                    .map(|after| (toggle, after.trim()))
            })
            .unwrap_or((Toggle::Keep, receiver));
        if sender.is_empty() || target.is_empty() {
            return Err(format!("cannot read `{line}`"));
        }
        let from = self.actor(sender, None, false)?;
        let to = self.actor(target, None, false)?;
        self.steps.push(Step::Message(Message {
            from,
            to,
            arrow,
            text: Label::parse(text),
            toggle,
        }));
        Ok(())
    }
}
