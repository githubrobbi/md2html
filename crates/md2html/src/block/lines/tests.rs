// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! What the line reader holds and does, by size: lines read one at a
//! time, a few held a level of nesting, a line queued once however deep.

use super::{Container, Lines};
use crate::block::opening::{Marker, quoted};

/// The sizes the bounds are checked at: what is held must not grow
/// from the one to the other.
const SIZES: [usize; 2] = [1_000, 100_000];

/// Reads every line `lines` has left, two looked at before each is
/// read, as a paragraph does; gives how many there were and the most
/// lines held at once.
fn read_all(lines: &mut Lines<'_>) -> (usize, usize) {
    let mut read = 0_usize;
    let mut most = 0_usize;
    while lines.peek_two().0.is_some() {
        most = most.max(lines.held());
        lines.bump();
        read += 1;
    }
    (read, most)
}

/// Reads the lines of `levels` quotes, one in the other, opened on the
/// first line; gives how many the innermost held and the most lines
/// held at once.
fn read_quoted(lines: &mut Lines<'_>, levels: u8) -> (usize, usize) {
    let Some(inner_levels) = levels.checked_sub(1) else {
        return read_all(lines);
    };
    let Some(head) = lines.bump() else {
        return (0, 0);
    };
    let first = head.with(quoted(&head.text).unwrap_or_default());
    lines
        .inside(first, Container::Quote, |inner| {
            read_quoted(inner, inner_levels)
        })
        .read
}

/// The source is read a line at a time: a text of one-letter lines,
/// of blank lines or of lines of spaces holds two lines at most, at
/// any length.
///
/// # Panics
///
/// When a line is lost or made up, or more than two lines are held at
/// once, at 1,000 lines or at 100,000.
#[test]
fn the_source_is_read_a_line_at_a_time() {
    for count in SIZES {
        for line in ["a\n", "\n", "   \n"] {
            let text = line.repeat(count);
            let (read, most) = read_all(&mut Lines::new(&text, 1, true));
            assert_eq!(read, count, "{line:?}");
            assert!(most <= 2, "{count} of {line:?}: {most} held");
        }
    }
}

/// A line read through quotes nested thirty deep is put in a queue
/// once, where the innermost quote takes it.
///
/// On its way up it is passed from hand to hand, a lazy line too, so
/// the work a line costs is a judgment a level.
///
/// # Panics
///
/// When the innermost quote does not read every line, or the lines put
/// in a queue are more than the lines read and the one each level
/// starts with.
#[test]
fn a_line_rests_only_where_it_stops() {
    for count in [100, 1_000] {
        let text = format!("{quotes}a\n{quotes}b\nlazy\n", quotes = "> ".repeat(30)).repeat(count);
        let mut lines = Lines::new(&text, 1, true);
        let (read, _) = read_quoted(&mut lines, 30);
        assert_eq!(read, count * 3);
        assert!(
            lines.rested <= read + 30,
            "{count}: {} put in a queue for {read} lines",
            lines.rested
        );
    }
}

/// Quotes nested three deep read their lines as they come: what they
/// hold grows with the depth, not with the lines.
///
/// # Panics
///
/// When the innermost quote does not read every line, or more than
/// ten lines are held at once, at 1,000 lines or at 100,000.
#[test]
fn nested_quotes_hold_a_few_lines_a_level() {
    for count in SIZES {
        let text = "> > > a\n".repeat(count);
        let (read, most) = read_quoted(&mut Lines::new(&text, 1, true), 3);
        assert_eq!(read, count);
        assert!(most <= 10, "{count} lines: {most} held");
    }
}

/// A list item's blank lines are held as a count until the item goes
/// on (they are its own) or ends (they close it): either way they
/// cost one entry, however many there are.
///
/// # Panics
///
/// When the item does not read its first line, every blank line and
/// the line after them; when it reads its trailing blank lines, or
/// does not count them; or when more than four lines are held at
/// once, at 1,000 blank lines or at 100,000.
#[test]
fn an_item_holds_its_blank_lines_as_a_count() {
    for count in SIZES {
        for (after, inner, trailing) in [("  b\n", count + 2, 0), ("", 1, count)] {
            let text = format!("- a\n{}{after}", "\n".repeat(count));
            let mut lines = Lines::new(&text, 1, true);
            let Some(head) = lines.bump() else {
                panic!("the marker's line");
            };
            let Some(marker) = Marker::read(&head.text, head.col) else {
                panic!("a marker");
            };
            let holds = Container::Item { col: marker.col };
            let inside = lines.inside(marker.first_line(&head), holds, read_all);
            let (read, most) = inside.read;
            assert_eq!(read, inner, "{after:?}");
            assert_eq!(inside.trailing, trailing, "{after:?}");
            assert!(most <= 4, "{count} blank lines: {most} held");
            assert!(lines.bump().is_none(), "every line read");
        }
    }
}
