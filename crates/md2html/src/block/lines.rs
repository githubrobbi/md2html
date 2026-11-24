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
    /// The line is not its own, and is given back: it is closed.
    End(Line<'src>),
}

impl<'src> Verdict<'src> {
    /// The verdict that takes `line`, nothing held before it.
    const fn take(line: Line<'src>) -> Self {
        Self::Take { held: None, line }
    }
}

impl<'src> Rule<'src> {
    /// What this stage does with `below`, the next line of the stage
    /// under it.
    fn judge(&mut self, below: Line<'src>, math: bool) -> Verdict<'src> {
        let verdict = match self {
            Self::Source | Self::Closed(_) => Verdict::End(below),
            Self::Quote(taken) => quote_takes(taken, below, math),
            Self::Item(item) => item.takes(below, math),
            Self::Note(after_blank) => note_takes(after_blank, below, math),
        };
        if matches!(verdict, Verdict::End(_)) {
            self.close();
        }
        verdict
    }

    /// Closes it: no more lines come to it.
    fn close(&mut self) {
        *self = Self::Closed(self.trailing());
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
fn quote_takes<'src>(taken: &mut Taken<'src>, line: Line<'src>, math: bool) -> Verdict<'src> {
    if let Some(text) = (!line.lazy).then(|| quoted(&line.text)).flatten() {
        let inner = line.with(text);
        taken.mark(&inner);
        return Verdict::take(inner);
    }
    if taken.takes_lazy(&line, math) {
        taken.last = Last::Known(true);
        return Verdict::take(line.into_lazy());
    }
    Verdict::End(line)
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
    fn takes(&mut self, line: Line<'src>, math: bool) -> Verdict<'src> {
        if line.is_blank() {
            self.held = Some(self.held.map_or_else(|| Run::one(line.no), Run::longer));
            self.taken.last = Last::Known(false);
            return Verdict::Hold;
        }
        let inner = if !line.lazy && indent(&line.text) >= self.col {
            let inner = line.with(strip_cols(&line.text, self.col));
            self.taken.mark(&inner);
            inner
        } else if self.taken.takes_lazy(&line, math) {
            self.taken.last = Last::Known(true);
            line.into_lazy()
        } else {
            return Verdict::End(line);
        };
        Verdict::Take {
            held: self.held.take(),
            line: inner,
        }
    }
}

/// A footnote's verdict on `line`.
fn note_takes<'src>(after_blank: &mut bool, line: Line<'src>, math: bool) -> Verdict<'src> {
    if line.is_blank() {
        *after_blank = true;
        return Verdict::take(line.with(""));
    }
    if indent(&line.text) >= 4 {
        *after_blank = false;
        return Verdict::take(line.with(strip_cols(&line.text, 4)));
    }
    let body = line.text.trim_start();
    if !*after_blank && !Opening::of(&line, math).interrupts() && Note::read(body).is_none() {
        return Verdict::take(line.with(body));
    }
    Verdict::End(line)
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

    /// Takes its next line out, not yet given: the stage above may leave
    /// it ([`Stage::put_back`]).
    fn next(&mut self) -> Option<Line<'src>> {
        Some(match self.ready.pop_front()? {
            Entry::Line(line) => line,
            Entry::Blanks(run) => {
                if let Some(rest) = run.after_first() {
                    self.ready.push_front(Entry::Blanks(rest));
                }
                Line::blank(run.first)
            }
        })
    }

    /// Puts back `line`, taken out by [`Stage::next`] and left by the
    /// stage above: it is its next line again.
    fn put_back(&mut self, line: Line<'src>) {
        self.ready.push_front(Entry::Line(line));
    }

    /// Gives its next line.
    fn give(&mut self) -> Option<Line<'src>> {
        let line = self.next()?;
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
    /// How many lines were put in a stage's queue: a line rests where it
    /// stops, not at every stage it passes.
    #[cfg(test)]
    rested: usize,
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
            #[cfg(test)]
            rested: 0,
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

    /// Takes one line into stage `at` (from the text, for the source's);
    /// says whether more may come.
    ///
    /// The line is the next one of the nearest stage under `at` that has
    /// one ready, or of the text, and it is passed up through the stages
    /// between, each judging it by its rule and giving the next what it
    /// takes. On its way up it is held in hand, not put in each stage's
    /// queue: it rests in a queue only where it stops, so a line costs a
    /// judgment a level and nothing more.
    fn take(&mut self, at: usize) -> bool {
        let Some(under) = at.checked_sub(1) else {
            let Some(line) = self.source.next_line() else {
                return false;
            };
            if let Some(stage) = self.stages.get_mut(at) {
                stage.ready.push_back(Entry::Line(line));
                #[cfg(test)]
                self.rest();
            }
            return true;
        };
        if self
            .stages
            .get(at)
            .is_none_or(|stage| matches!(stage.rule, Rule::Closed(_)))
        {
            return false;
        }
        match self.next_under(under) {
            Some((from, line)) => self.pass_up(from, line, at),
            None => false,
        }
    }

    /// The next line of the nearest stage at or under `at` that has one
    /// ready, or of the text, and that stage. None when there is none:
    /// the text has ended, or a stage closed with nothing ready, and the
    /// stages above it to `at` and one more are closed for want of a line.
    fn next_under(&mut self, at: usize) -> Option<(usize, Line<'src>)> {
        let mut from = at;
        loop {
            let stage = self.stages.get_mut(from)?;
            if let Some(line) = stage.next() {
                return Some((from, line));
            }
            if matches!(stage.rule, Rule::Closed(_)) {
                break;
            }
            match from.checked_sub(1) {
                Some(lower) => from = lower,
                None => match self.source.next_line() {
                    Some(line) => return Some((from, line)),
                    None => break,
                },
            }
        }
        self.close_above(from, at.saturating_add(1));
        None
    }

    /// Closes the stages above `from` up to `to`: no line comes to them.
    fn close_above(&mut self, from: usize, to: usize) {
        for stage in self
            .stages
            .iter_mut()
            .take(to.saturating_add(1))
            .skip(from.saturating_add(1))
        {
            stage.rule.close();
        }
    }

    /// Passes `line`, the next line of stage `from`, up to stage `at`:
    /// each stage above takes it or leaves it. Says whether more may come.
    fn pass_up(&mut self, from: usize, mut line: Line<'src>, at: usize) -> bool {
        let math = self.math;
        let mut under = from;
        while under < at {
            let above = under.saturating_add(1);
            let (below, rest) = self.stages.split_at_mut(above);
            let (Some(lower), Some(stage)) = (below.last_mut(), rest.first_mut()) else {
                return false;
            };
            let blank = line.is_blank();
            match stage.rule.judge(line, math) {
                Verdict::Take {
                    held: None,
                    line: inner,
                } if above < at => {
                    lower.after_blank = blank;
                    line = inner;
                }
                Verdict::Take { held, line: inner } => {
                    lower.after_blank = blank;
                    if let Some(run) = held {
                        stage.ready.push_back(Entry::Blanks(run));
                    }
                    stage.ready.push_back(Entry::Line(inner));
                    #[cfg(test)]
                    self.rest();
                    return true;
                }
                Verdict::Hold => {
                    lower.after_blank = blank;
                    return true;
                }
                Verdict::End(left) => {
                    lower.put_back(left);
                    self.close_above(above, at);
                    return false;
                }
            }
            under = above;
        }
        false
    }

    /// Counts a line put in a stage's queue.
    #[cfg(test)]
    const fn rest(&mut self) {
        self.rested = self.rested.saturating_add(1);
    }

    /// How many lines all the stages have ready, a run of blank lines
    /// counted as one.
    #[cfg(test)]
    pub(super) fn held(&self) -> usize {
        self.stages.iter().map(|stage| stage.ready.len()).sum()
    }
}

#[cfg(test)]
mod tests;
