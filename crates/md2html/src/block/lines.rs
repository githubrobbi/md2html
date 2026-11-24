// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! The lines the block parser reads, taken from the source one at a time
//! and passed through the containers open around them.
//!
//! The source is read a line at a time, when the parser asks for one. A
//! container (a quote, a list item, a footnote) takes its lines from the
//! lines around it as they come: it decides from the line and from what it
//! has taken so far whether the line is its own, and gives it with its
//! marks taken off. No one looks further ahead than the two lines a
//! paragraph looks at, so what is held is a few lines a container,
//! whatever the length of the text. The blank lines a list item has taken
//! are its own only when more of the item follows them; until that is
//! known they are held as a count, and a count is what is given.

use alloc::collections::VecDeque;

use super::line::{Line, indent, strip_cols};
use super::opening::{Fence, Note, Opening, quoted};

/// What a container is, for the lines it takes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Container {
    /// A block quote: the `>` lines, and the lazy lines of its paragraph.
    Quote,
    /// A list item whose content starts at column `col`: the lines
    /// indented that far, the blank lines among them, lazy lines.
    Item {
        /// The column its content starts at.
        col: usize,
    },
    /// A footnote definition: lines indented four, blank lines, and the
    /// lines that continue its paragraph.
    Note,
    /// A list item that opens empty above a blank line: its first line is
    /// all it holds.
    Nothing,
}

/// What reading inside a container gave, and what it left.
pub(super) struct Inside<Read> {
    /// What the reader made of the container's lines.
    pub(super) read: Read,
    /// The blank lines that closed the container: taken, but not its own.
    pub(super) trailing: usize,
}

/// Blank lines in a row, held as a count.
#[derive(Clone, Copy, Debug)]
struct Run {
    /// The number of the first.
    first: usize,
    /// How many; never zero.
    count: usize,
}

impl Run {
    /// The run of one blank line, numbered `no`.
    const fn one(no: usize) -> Self {
        Self {
            first: no,
            count: 1,
        }
    }

    /// The run with one more line.
    const fn longer(self) -> Self {
        Self {
            first: self.first,
            count: self.count.saturating_add(1),
        }
    }

    /// The run without its first line, if more are left.
    fn after_first(self) -> Option<Self> {
        let count = self.count.checked_sub(1).filter(|left| *left > 0)?;
        Some(Self {
            first: self.first.saturating_add(1),
            count,
        })
    }
}

/// A line ready to be given, or a run of blank lines.
#[derive(Debug)]
enum Entry<'src> {
    /// One line.
    Line(Line<'src>),
    /// Blank lines, each made when it is given.
    Blanks(Run),
}

impl Entry<'_> {
    /// How many lines it gives.
    const fn len(&self) -> usize {
        match self {
            Self::Line(_) => 1,
            Self::Blanks(run) => run.count,
        }
    }
}

/// What a quote or a list item has taken so far, as far as its next line
/// is concerned.
#[derive(Clone, Debug)]
struct Taken<'src> {
    /// The code fence its lines are inside, if any: no lazy line there.
    fence: Option<Fence>,
    /// Its last line, for whether it leaves a paragraph open.
    last: Last<'src>,
}

/// A container's last line, as far as a lazy line after it is concerned.
#[derive(Clone, Debug)]
enum Last<'src> {
    /// It leaves a paragraph open that a lazy line may continue, or not.
    Known(bool),
    /// This line, read only when a lazy line asks: most lines are a
    /// container's own by their marks, and nothing asks.
    Line(Line<'src>),
}

impl Last<'_> {
    /// Whether it leaves a paragraph open.
    fn leaves_open(&self, math: bool) -> bool {
        match self {
            Self::Known(open) => *open,
            Self::Line(line) => leaves_open(line, math),
        }
    }
}

/// Whether `line`, standing last in a container, leaves a paragraph open.
fn leaves_open(line: &Line<'_>, math: bool) -> bool {
    line.lazy || Opening::of(line, math).continues_paragraph()
}

impl<'src> Taken<'src> {
    /// What a container whose first line is `first` has taken.
    fn first(first: &Line<'src>, math: bool) -> Self {
        Self {
            fence: Fence::track(None, &first.text),
            last: Last::Known(leaves_open(first, math)),
        }
    }

    /// `line` taken, the container's marks off.
    fn mark(&mut self, line: &Line<'src>) {
        self.fence = Fence::track(self.fence, &line.text);
        self.last = Last::Line(line.clone());
    }

    /// Whether `line`, without the container's mark, continues the
    /// paragraph its last line leaves open. A line lazy in an outer
    /// container is lazy here too.
    fn takes_lazy(&self, line: &Line<'_>, math: bool) -> bool {
        self.fence.is_none()
            && self.last.leaves_open(math)
            && (line.lazy || Opening::of(line, math).is_lazy())
    }
}

/// How a stage takes its lines.
#[derive(Clone, Debug)]
enum Rule<'src> {
    /// It is the source.
    Source,
    /// A quote, and what it has taken.
    Quote(Taken<'src>),
    /// A list item.
    Item(ItemRule<'src>),
    /// A footnote, and whether its last line was blank: then only an
    /// indented line goes on.
    Note(bool),
    /// It takes no more lines; this many blank lines it took last were
    /// not its own.
    Closed(usize),
}

/// How a list item takes its lines.
#[derive(Clone, Debug)]
struct ItemRule<'src> {
    /// The column its content starts at.
    col: usize,
    /// What it has taken.
    taken: Taken<'src>,
    /// Blank lines it has taken and not yet given.
    held: Option<Run>,
}

/// What a stage does with the line below it.
enum Verdict<'src> {
    /// The line is its own: this is it, its marks off, with the blank
    /// lines held before it, which are its own too.
    Take {
        /// The blank lines held before it.
        held: Option<Run>,
        /// The line.
        line: Line<'src>,
    },
    /// A blank line it holds until it knows whether the item goes on.
    Hold,
    /// The line is not its own: it is closed.
    End,
}

impl<'src> Verdict<'src> {
    /// The verdict that takes `line`, nothing held before it.
    const fn take(line: Line<'src>) -> Self {
        Self::Take { held: None, line }
    }
}

impl<'src> Rule<'src> {
    /// What this stage does with `below`, the next line of the stage
    /// under it, if there is one.
    fn judge(&mut self, below: Option<&Line<'src>>, math: bool) -> Verdict<'src> {
        let verdict = match (below, &mut *self) {
            (None, _) | (_, Self::Source | Self::Closed(_)) => Verdict::End,
            (Some(line), Self::Quote(taken)) => quote_takes(taken, line, math),
            (Some(line), Self::Item(item)) => item.takes(line, math),
            (Some(line), Self::Note(after_blank)) => note_takes(after_blank, line, math),
        };
        if matches!(verdict, Verdict::End) {
            *self = Self::Closed(self.trailing());
        }
        verdict
    }

    /// The blank lines that would close it now: an item's held ones.
    const fn trailing(&self) -> usize {
        match self {
            Self::Item(item) => item.trailing(),
            Self::Closed(trailing) => *trailing,
            Self::Source | Self::Quote(_) | Self::Note(_) => 0,
        }
    }
}

/// A quote's verdict on `line`.
fn quote_takes<'src>(taken: &mut Taken<'src>, line: &Line<'src>, math: bool) -> Verdict<'src> {
    if let Some(text) = quoted(&line.text).filter(|_| !line.lazy) {
        let inner = line.with(text);
        taken.mark(&inner);
        return Verdict::take(inner);
    }
    if taken.takes_lazy(line, math) {
        taken.last = Last::Known(true);
        return Verdict::take(line.to_lazy());
    }
    Verdict::End
}

impl<'src> ItemRule<'src> {
    /// The blank lines it holds, which close it if it ends now.
    const fn trailing(&self) -> usize {
        match self.held {
            Some(run) => run.count,
            None => 0,
        }
    }

    /// The item's verdict on `line`; its blank lines are held.
    fn takes(&mut self, line: &Line<'src>, math: bool) -> Verdict<'src> {
        if line.is_blank() {
            self.held = Some(self.held.map_or_else(|| Run::one(line.no), Run::longer));
            self.taken.last = Last::Known(false);
            return Verdict::Hold;
        }
        let inner = if !line.lazy && indent(&line.text) >= self.col {
            let inner = line.with(strip_cols(&line.text, self.col));
            self.taken.mark(&inner);
            inner
        } else if self.taken.takes_lazy(line, math) {
            self.taken.last = Last::Known(true);
            line.to_lazy()
        } else {
            return Verdict::End;
        };
        Verdict::Take {
            held: self.held.take(),
            line: inner,
        }
    }
}

/// A footnote's verdict on `line`.
fn note_takes<'src>(after_blank: &mut bool, line: &Line<'src>, math: bool) -> Verdict<'src> {
    if line.is_blank() {
        *after_blank = true;
        return Verdict::take(line.with(""));
    }
    if indent(&line.text) >= 4 {
        *after_blank = false;
        return Verdict::take(line.with(strip_cols(&line.text, 4)));
    }
    let body = line.text.trim_start();
    if !*after_blank && !Opening::of(line, math).interrupts() && Note::read(body).is_none() {
        return Verdict::take(line.with(body));
    }
    Verdict::End
}

/// The source, or one container open in it: the lines it has ready.
#[derive(Debug)]
struct Stage<'src> {
    /// How it takes lines.
    rule: Rule<'src>,
    /// What it has taken and not given yet, in order.
    ready: VecDeque<Entry<'src>>,
    /// The last line it gave was blank.
    after_blank: bool,
}

impl<'src> Stage<'src> {
    /// A stage whose first line is `first`, taking more by `rule`.
    fn new(rule: Rule<'src>, first: Option<Line<'src>>) -> Self {
        Self {
            rule,
            ready: first.map(Entry::Line).into_iter().collect(),
            after_blank: false,
        }
    }

    /// How many lines it has ready, counted up to `enough`.
    fn ready_up_to(&self, enough: usize) -> usize {
        let mut count = 0_usize;
        for entry in &self.ready {
            if count >= enough {
                break;
            }
            count = count.saturating_add(entry.len());
        }
        count
    }

    /// Makes the entry `nth` from the front a line of its own, when it
    /// is a run of blank lines.
    fn split_run(&mut self, nth: usize) {
        let Some(entry) = self.ready.get_mut(nth) else {
            return;
        };
        let Entry::Blanks(run) = *entry else {
            return;
        };
        *entry = Entry::Line(Line::blank(run.first));
        if let Some(rest) = run.after_first() {
            self.ready
                .insert(nth.saturating_add(1), Entry::Blanks(rest));
        }
    }

    /// The line `nth` from the front, when it is ready.
    fn line(&self, nth: usize) -> Option<&Line<'src>> {
        match self.ready.get(nth)? {
            Entry::Line(line) => Some(line),
            Entry::Blanks(_) => None,
        }
    }

    /// Gives its next line.
    fn give(&mut self) -> Option<Line<'src>> {
        let line = match self.ready.pop_front()? {
            Entry::Line(line) => line,
            Entry::Blanks(run) => {
                if let Some(rest) = run.after_first() {
                    self.ready.push_front(Entry::Blanks(rest));
                }
                Line::blank(run.first)
            }
        };
        self.after_blank = line.is_blank();
        Some(line)
    }
}

/// The source's lines not read yet.
#[derive(Debug)]
struct Source<'src> {
    /// The text after the lines read.
    rest: &'src str,
    /// The number of the next line.
    no: usize,
}

impl<'src> Source<'src> {
    /// Reads the next line. A line ends at its line ending: the ending of
    /// the last one starts no other line.
    fn next_line(&mut self) -> Option<Line<'src>> {
        if self.rest.is_empty() {
            return None;
        }
        let (raw, rest) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = rest;
        let line = Line::source(raw, self.no);
        self.no = self.no.saturating_add(1);
        Some(line)
    }
}

/// The lines of a text, read as the parser asks for them, through the
/// containers it has opened.
#[derive(Debug)]
pub(super) struct Lines<'src> {
    /// The source's lines not read yet.
    source: Source<'src>,
    /// The source's stage first, then one for each container open, the
    /// innermost last: the parser reads from the last.
    stages: Vec<Stage<'src>>,
    /// `$$` opens maths, which a lazy line does not continue.
    math: bool,
}

impl<'src> Lines<'src> {
    /// The lines of `text`, the first numbered `first`.
    pub(super) fn new(text: &'src str, first: usize, math: bool) -> Self {
        Self {
            source: Source {
                rest: text,
                no: first,
            },
            stages: vec![Stage::new(Rule::Source, None)],
            math,
        }
    }

    /// The next line, not read.
    pub(super) fn peek(&mut self) -> Option<&Line<'src>> {
        let top = self.top();
        self.fill(top, 1);
        let stage = self.stages.get_mut(top)?;
        stage.split_run(0);
        stage.line(0)
    }

    /// The next two lines, not read.
    pub(super) fn peek_two(&mut self) -> (Option<&Line<'src>>, Option<&Line<'src>>) {
        let top = self.top();
        self.fill(top, 2);
        let Some(stage) = self.stages.get_mut(top) else {
            return (None, None);
        };
        stage.split_run(0);
        stage.split_run(1);
        (stage.line(0), stage.line(1))
    }

    /// Reads the next line.
    pub(super) fn bump(&mut self) -> Option<Line<'src>> {
        self.give(self.top())
    }

    /// Reads the next line if `test` accepts it.
    pub(super) fn bump_if<Test>(&mut self, test: Test) -> Option<Line<'src>>
    where
        Test: FnOnce(&Line<'src>) -> bool,
    {
        if self.peek().is_some_and(test) {
            self.bump()
        } else {
            None
        }
    }

    /// Reads the blank lines next; says how many there were.
    pub(super) fn skip_blank(&mut self) -> usize {
        let mut count = 0_usize;
        while self.bump_if(Line::is_blank).is_some() {
            count = count.saturating_add(1);
        }
        count
    }

    /// Whether the line read last was blank.
    pub(super) fn after_blank(&self) -> bool {
        self.stages.last().is_some_and(|stage| stage.after_blank)
    }

    /// What `read` makes of the lines of a container whose first line,
    /// its marks off, is `first`; the lines it holds are read from these
    /// as `read` asks for them. Whatever `read` leaves of them is read
    /// too, so the lines after the container come next.
    pub(super) fn inside<Read>(
        &mut self,
        first: Line<'src>,
        container: Container,
        read: impl FnOnce(&mut Self) -> Read,
    ) -> Inside<Read> {
        let rule = match container {
            Container::Quote => Rule::Quote(Taken::first(&first, self.math)),
            Container::Item { col } => Rule::Item(ItemRule {
                col,
                taken: Taken::first(&first, self.math),
                held: None,
            }),
            Container::Note => Rule::Note(false),
            Container::Nothing => Rule::Closed(0),
        };
        self.stages.push(Stage::new(rule, Some(first)));
        let made = read(self);
        while self.bump().is_some() {}
        Inside {
            read: made,
            trailing: self.stages.pop().map_or(0, |stage| stage.rule.trailing()),
        }
    }

    /// The innermost stage, the one the parser reads.
    fn top(&self) -> usize {
        self.stages.len().saturating_sub(1)
    }

    /// Gives the next line of stage `at`.
    fn give(&mut self, at: usize) -> Option<Line<'src>> {
        self.fill(at, 1);
        self.stages.get_mut(at)?.give()
    }

    /// Makes `want` lines ready at stage `at`, or all it has left.
    fn fill(&mut self, at: usize, want: usize) {
        while self
            .stages
            .get(at)
            .is_some_and(|stage| stage.ready_up_to(want) < want)
        {
            if !self.take(at) {
                break;
            }
        }
    }

    /// Takes one line into stage `at` from the one under it (from the
    /// text, for the source's); says whether more may come.
    fn take(&mut self, at: usize) -> bool {
        let Some(under) = at.checked_sub(1) else {
            let line = self.source.next_line();
            let more = line.is_some();
            if let Some(stage) = self.stages.get_mut(at) {
                stage.ready.extend(line.map(Entry::Line));
            }
            return more;
        };
        if self
            .stages
            .get(at)
            .is_none_or(|stage| matches!(stage.rule, Rule::Closed(_)))
        {
            return false;
        }
        self.fill(under, 1);
        let math = self.math;
        let (below, above) = self.stages.split_at_mut(at);
        let (Some(lower), Some(stage)) = (below.last_mut(), above.first_mut()) else {
            return false;
        };
        lower.split_run(0);
        match stage.rule.judge(lower.line(0), math) {
            Verdict::Take { held, line } => {
                lower.give();
                stage.ready.extend(held.map(Entry::Blanks));
                stage.ready.push_back(Entry::Line(line));
                true
            }
            Verdict::Hold => {
                lower.give();
                true
            }
            Verdict::End => false,
        }
    }

    /// How many lines all the stages have ready, a run of blank lines
    /// counted as one.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.stages.iter().map(|stage| stage.ready.len()).sum()
    }
}

#[cfg(test)]
mod tests {
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
}
