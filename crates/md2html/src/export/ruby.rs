// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! An author's ruby as the XHTML of a book has it.
//!
//! HTML's parser takes a ruby that holds no ruby text, and ruby text
//! inside another element of the ruby
//! (`<ruby>漢<span><rt>kan</rt></span></ruby>`); XHTML's content model takes
//! neither, and epubcheck refuses the book. The web page keeps the markup as
//! the author wrote it; the book says the same in XHTML's terms. The balancing
//! gives a ruby md2html closes its ruby text already ([`crate::sanitize`]);
//! this is the ruby the author closed.

use super::conform::{next_tag, tag_name};
use super::legacy::TagKind;
use crate::scan::Scan;

/// The elements ruby text stands in: a ruby, and a run of ruby text in one.
const HOLDERS: [&str; 2] = ["ruby", "rtc"];

/// Ruby text, and the parentheses a reader without ruby shows around it.
const RUBY_TEXT: [&str; 2] = ["rt", "rp"];

/// How far a ruby's ruby text has come, for what XHTML still wants of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Annotation {
    /// None yet: the ruby wants an `rt`.
    Missing,
    /// A parenthesis (`rp`) opened before ruby text not given yet: an `rp`
    /// stands next to an `rt`, so one is wanted before the next `rp` or
    /// the ruby's end.
    Opened,
    /// Ruby text given last: an `rt`, or a run of it in an `rtc`.
    Given,
    /// Ruby text given and its parenthesis closed after it.
    Closed,
}

/// What an element open inside a ruby is to the rewrite.
#[derive(Debug, PartialEq, Eq)]
enum State {
    /// A ruby, and how far its ruby text has come.
    Ruby(Annotation),
    /// Open in the book as in the page.
    Open,
    /// Closed in the book before ruby text that stood in it, which XHTML
    /// does not let it hold: its end tag is left out.
    Ended,
}

/// An element open inside a ruby, the ruby among them.
#[derive(Debug)]
struct Held {
    /// Its name, in lower case.
    name: String,
    /// What it is to the rewrite.
    state: State,
}

/// The chapter with its rubies as XHTML's content model has them.
///
/// Ruby text that stands inside another element of its ruby is moved out
/// of it: what stood between is closed before the ruby text, and its end
/// tag is left out, so what follows it stands in the ruby. A ruby whose
/// base (its text, or an element in it) has no ruby text after it, and a
/// parenthesis without the ruby text it stands next to, get an empty `rt`.
/// The chapter is well formed, so each end tag closes the element open
/// last.
pub(super) fn ruby(xhtml: &str) -> String {
    let mut open: Vec<Held> = Vec::new();
    let mut out = String::with_capacity(xhtml.len() + xhtml.len() / 32);
    let mut scan = Scan::new(xhtml);
    while let Some(text) = scan.take_until("<") {
        if !text.trim().is_empty() {
            base(&mut open);
        }
        out.push_str(text);
        tag(&mut open, next_tag(&mut scan), &mut out);
    }
    let rest = scan.take_rest();
    if !rest.trim().is_empty() {
        base(&mut open);
    }
    out.push_str(rest);
    out
}

/// Writes one tag, given what stands between its `<` and `>`.
fn tag(open: &mut Vec<Held>, inside: &str, out: &mut String) {
    let name = tag_name(inside);
    let is_ruby = name.eq_ignore_ascii_case("ruby");
    if !open.is_empty() || is_ruby {
        let kind = TagKind::of(inside);
        let lower = name.to_ascii_lowercase();
        let ruby_text = RUBY_TEXT.contains(&lower.as_str()) || lower == "rtc";
        if ruby_text && kind != TagKind::Closing {
            stand_in_ruby(open, out);
            annotate(open, &lower, out);
        } else if kind != TagKind::Closing {
            base(open);
        }
        match kind {
            TagKind::Empty => {}
            TagKind::Opening => {
                let state = if is_ruby {
                    State::Ruby(Annotation::Missing)
                } else {
                    State::Open
                };
                open.push(Held { name: lower, state });
            }
            TagKind::Closing => match open.pop().map(|held| held.state) {
                Some(State::Ended) => return,
                Some(State::Ruby(Annotation::Missing | Annotation::Opened)) => {
                    out.push_str(EMPTY_RT);
                }
                Some(State::Ruby(Annotation::Given | Annotation::Closed) | State::Open) | None => {}
            },
        }
    }
    out.push('<');
    out.push_str(inside);
    out.push('>');
}

/// Records base content (text, or an element that is not ruby text).
///
/// It stands in the base of the nearest ruby, unless it is inside ruby
/// text. Ruby text given before it annotates what came before, so the
/// ruby wants more for this.
fn base(open: &mut [Held]) {
    let in_ruby_text = open
        .iter()
        .rev()
        .find(|held| held.state != State::Ended)
        .is_some_and(|held| RUBY_TEXT.contains(&held.name.as_str()) || held.name == "rtc");
    if in_ruby_text {
        return;
    }
    if let Some(State::Ruby(annotation)) = open
        .iter_mut()
        .rev()
        .map(|held| &mut held.state)
        .find(|state| matches!(state, State::Ruby(_)))
    {
        if matches!(*annotation, Annotation::Given | Annotation::Closed) {
            *annotation = Annotation::Missing;
        }
    }
}

/// Closes, into `out`, what stands between the nearest ruby (or run of
/// ruby text) and the ruby text about to open, so that it opens there.
fn stand_in_ruby(open: &mut [Held], out: &mut String) {
    for held in open.iter_mut().rev() {
        if held.state == State::Ended {
            continue;
        }
        if HOLDERS.contains(&held.name.as_str()) {
            return;
        }
        out.push_str("</");
        out.push_str(&held.name);
        out.push('>');
        held.state = State::Ended;
    }
}

/// Ruby text with nothing in it, for a ruby that wants some.
const EMPTY_RT: &str = "<rt></rt>";

/// Records that `name` (an `rt`, an `rtc`, or a parenthesis, `rp`) opens
/// in the nearest ruby. A parenthesis that would close one with no ruby
/// text before it gets an empty `rt` first, into `out`.
fn annotate(open: &mut [Held], name: &str, out: &mut String) {
    let Some(State::Ruby(annotation)) = open
        .iter_mut()
        .rev()
        .map(|held| &mut held.state)
        .find(|state| matches!(state, State::Ruby(_)))
    else {
        return;
    };
    *annotation = match (name, *annotation) {
        ("rp", Annotation::Missing | Annotation::Closed) => Annotation::Opened,
        ("rp", Annotation::Opened) => {
            out.push_str(EMPTY_RT);
            Annotation::Closed
        }
        ("rp", Annotation::Given) => Annotation::Closed,
        _ => Annotation::Given,
    };
}

#[cfg(test)]
mod tests {
    use super::ruby;

    /// A ruby in the book holds its ruby text itself, and holds some.
    ///
    /// # Panics
    ///
    /// When ruby text inside a `span` of its ruby stays there, or the
    /// `span`'s end tag is written twice; when a ruby closed without ruby
    /// text, with base text or an element after its last ruby text, or a
    /// parenthesis without the ruby text it stands next to, gets none, or
    /// one with ruby text (an `rt` or an `rtc`) gets another; or when a
    /// ruby as XHTML has it, or markup outside a ruby, changes.
    #[test]
    fn a_ruby_holds_its_ruby_text_itself() {
        assert_eq!(
            ruby("<p><ruby>漢<span class=\"x\"><rt>kan</rt> after</span></ruby></p>"),
            "<p><ruby>漢<span class=\"x\"></span><rt>kan</rt> after<rt></rt></ruby></p>"
        );
        assert_eq!(
            ruby("<ruby>漢<em><b><rp>(</rp><rt>kan</rt><rp>)</rp></b></em></ruby>"),
            "<ruby>漢<em><b></b></em><rp>(</rp><rt>kan</rt><rp>)</rp></ruby>"
        );
        assert_eq!(
            ruby("a <ruby>漢字</ruby> b"),
            "a <ruby>漢字<rt></rt></ruby> b"
        );
        assert_eq!(
            ruby("<ruby>漢<rp>(</rp><rp>)</rp></ruby><ruby>字<rp>(</rp></ruby>"),
            "<ruby>漢<rp>(</rp><rt></rt><rp>)</rp></ruby><ruby>字<rp>(</rp><rt></rt></ruby>"
        );
        assert_eq!(
            ruby("<ruby>漢<rt>kan</rt>字</ruby><ruby>漢<rt>kan</rt><b>字</b> </ruby>"),
            "<ruby>漢<rt>kan</rt>字<rt></rt></ruby><ruby>漢<rt>kan</rt><b>字</b> <rt></rt></ruby>"
        );
        assert_eq!(
            ruby("<ruby>漢<rtc><rt>kan</rt></rtc></ruby>"),
            "<ruby>漢<rtc><rt>kan</rt></rtc></ruby>"
        );
        assert_eq!(
            ruby("<ruby><ruby>漢<rt>kan</rt></ruby><rt>x</rt></ruby>"),
            "<ruby><ruby>漢<rt>kan</rt></ruby><rt>x</rt></ruby>"
        );
        let written =
            "<p class=\"a\"><span>a<br/></span><ruby>漢<rp>(</rp><rt>kan</rt><rp>)</rp></ruby></p>";
        assert_eq!(ruby(written), written);
    }
}
