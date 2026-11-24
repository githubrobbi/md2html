// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Reading front to back without a position to get wrong.
//!
//! A parser that keeps its place as a number has to prove, at every use,
//! that the number is inside the text and on a character boundary. These
//! two readers keep the place themselves: [`Scan`] over text, [`Cursor`]
//! over a slice of anything (the block parser reads its lines with it).
//! Every answer is an `Option` or a piece of the input, so there is
//! nothing to index and nothing that can be out of range.

/// A place a [`Scan`] has been at, to come back to or to cut from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Mark(usize);

/// Text, read from the front.
#[derive(Clone, Debug)]
pub(crate) struct Scan<'text> {
    /// All of it.
    text: &'text str,
    /// What has not been read yet: always a tail of `text`.
    rest: &'text str,
}

impl<'text> Scan<'text> {
    /// A reader at the start of `text`.
    #[must_use]
    pub(crate) const fn new(text: &'text str) -> Self {
        Self { text, rest: text }
    }

    /// What has not been read yet.
    #[must_use]
    pub(crate) const fn rest(&self) -> &'text str {
        self.rest
    }

    /// Whether everything has been read.
    #[must_use]
    pub(crate) const fn is_done(&self) -> bool {
        self.rest.is_empty()
    }

    /// The place it is at.
    #[must_use]
    pub(crate) const fn mark(&self) -> Mark {
        Mark(self.text.len().saturating_sub(self.rest.len()))
    }

    /// What has been read so far.
    #[must_use]
    pub(crate) fn seen(&self) -> &'text str {
        self.text.get(..self.mark().0).unwrap_or_default()
    }

    /// What has been read since `mark`.
    #[must_use]
    pub(crate) fn since(&self, mark: Mark) -> &'text str {
        self.text.get(mark.0..self.mark().0).unwrap_or_default()
    }

    /// Goes back (or forward) to a place it has been at.
    pub(crate) fn rewind(&mut self, mark: Mark) {
        if let Some(rest) = self.text.get(mark.0..) {
            self.rest = rest;
        }
    }

    /// The next character, not read.
    #[must_use]
    pub(crate) fn peek(&self) -> Option<char> {
        self.rest.chars().next()
    }

    /// The character `ahead` places on: `peek_nth(0)` is [`Scan::peek`].
    #[must_use]
    pub(crate) fn peek_nth(&self, ahead: usize) -> Option<char> {
        self.rest.chars().nth(ahead)
    }

    /// The character just read.
    #[must_use]
    pub(crate) fn prev(&self) -> Option<char> {
        self.seen().chars().next_back()
    }

    /// Whether the unread text starts with `word`.
    #[must_use]
    pub(crate) fn starts_with(&self, word: &str) -> bool {
        self.rest.starts_with(word)
    }

    /// Reads one character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        let mut chars = self.rest.chars();
        let ch = chars.next()?;
        self.rest = chars.as_str();
        Some(ch)
    }

    /// Reads `ch` if it is next; says whether it was.
    pub(crate) fn eat(&mut self, ch: char) -> bool {
        self.eat_if(|next| next == ch).is_some()
    }

    /// Reads the next character if `test` accepts it.
    pub(crate) fn eat_if<Test>(&mut self, test: Test) -> Option<char>
    where
        Test: FnOnce(char) -> bool,
    {
        let ch = self.peek().filter(|next| test(*next))?;
        self.bump();
        Some(ch)
    }

    /// Reads `word` if the unread text starts with it; says whether it did.
    pub(crate) fn eat_str(&mut self, word: &str) -> bool {
        match self.rest.strip_prefix(word) {
            Some(rest) => {
                self.rest = rest;
                true
            }
            None => false,
        }
    }

    /// Reads the characters `test` accepts, and gives them.
    pub(crate) fn take_while<Test>(&mut self, mut test: Test) -> &'text str
    where
        Test: FnMut(char) -> bool,
    {
        let end = self.rest.find(|ch| !test(ch)).unwrap_or(self.rest.len());
        self.take_to(end)
    }

    /// Reads up to `word`, which stays unread, and gives what came before
    /// it. Without `word` ahead nothing is read.
    pub(crate) fn take_until(&mut self, word: &str) -> Option<&'text str> {
        let end = self.rest.find(word)?;
        Some(self.take_to(end))
    }

    /// Reads `count` characters, or all that are left, and gives them.
    pub(crate) fn take_chars(&mut self, count: usize) -> &'text str {
        let end = self
            .rest
            .char_indices()
            .nth(count)
            .map_or(self.rest.len(), |(at, _)| at);
        self.take_to(end)
    }

    /// Reads everything that is left, and gives it.
    pub(crate) fn take_rest(&mut self) -> &'text str {
        self.take_to(self.rest.len())
    }

    /// Reads the first `end` bytes of the unread text. `end` comes from a
    /// search in that text, so it is a character boundary inside it; were
    /// it not, nothing is read.
    fn take_to(&mut self, end: usize) -> &'text str {
        let (taken, rest) = self.rest.split_at_checked(end).unwrap_or(("", self.rest));
        self.rest = rest;
        taken
    }
}

/// A place a [`Cursor`] has been at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Spot(usize);

/// A slice, read from the front.
#[derive(Debug)]
pub(crate) struct Cursor<'items, Item> {
    /// All of them.
    items: &'items [Item],
    /// How many have been read: never more than there are.
    read: usize,
}

impl<'items, Item> Cursor<'items, Item> {
    /// A reader at the start of `items`.
    #[must_use]
    pub(crate) const fn new(items: &'items [Item]) -> Self {
        Self { items, read: 0 }
    }

    /// The place it is at.
    #[must_use]
    pub(crate) const fn spot(&self) -> Spot {
        Spot(self.read)
    }

    /// What has been read since `spot`.
    #[must_use]
    pub(crate) fn since(&self, spot: Spot) -> &'items [Item] {
        self.items.get(spot.0..self.read).unwrap_or_default()
    }

    /// The next item, not read.
    #[must_use]
    pub(crate) fn peek(&self) -> Option<&'items Item> {
        self.items.get(self.read)
    }

    /// The item `ahead` places on: `peek_nth(0)` is [`Cursor::peek`].
    #[must_use]
    pub(crate) fn peek_nth(&self, ahead: usize) -> Option<&'items Item> {
        self.items.get(self.read.checked_add(ahead)?)
    }

    /// Reads one item.
    pub(crate) fn bump(&mut self) -> Option<&'items Item> {
        let item = self.peek()?;
        self.read = self.read.saturating_add(1);
        Some(item)
    }

    /// Reads the next item if `test` accepts it.
    fn eat_if<Test>(&mut self, test: Test) -> Option<&'items Item>
    where
        Test: FnOnce(&Item) -> bool,
    {
        let item = self.peek().filter(|next| test(next))?;
        self.bump();
        Some(item)
    }

    /// Reads the items `test` accepts, and gives them.
    pub(crate) fn take_while<Test>(&mut self, mut test: Test) -> &'items [Item]
    where
        Test: FnMut(&Item) -> bool,
    {
        let spot = self.spot();
        while self.eat_if(&mut test).is_some() {}
        self.since(spot)
    }
}

#[cfg(test)]
mod tests {
    use super::{Cursor, Scan};

    /// A scan reads text by characters, runs and words, and can go back to a
    /// mark.
    ///
    /// # Panics
    ///
    /// Panics when any reader gives other text than the one before it left: a
    /// run taken too far or too short, `eat` taking a character that is not
    /// there, `take_until` not stopping before its word or not answering None
    /// when it never comes, `peek_nth` looking at the wrong character, or
    /// `rewind` and `take_chars` landing inside the two bytes of `ä` instead of
    /// counting it as one character; a slice through a character fails it too.
    #[test]
    fn a_scan_reads_text_piece_by_piece() {
        let mut scan = Scan::new("key = \"värde\" rest");
        assert_eq!(scan.take_while(char::is_alphanumeric), "key");
        assert_eq!(scan.take_while(char::is_whitespace), " ");
        assert!(scan.eat('='));
        assert!(!scan.eat('='));
        scan.take_while(char::is_whitespace);
        assert_eq!(scan.eat_if(|ch| ch == '"' || ch == '\''), Some('"'));
        let value = scan.mark();
        assert_eq!(scan.take_until("\""), Some("värde"));
        assert_eq!(scan.since(value), "värde");
        assert_eq!(scan.prev(), Some('e'));
        assert_eq!(scan.bump(), Some('"'));
        assert_eq!(scan.take_until("nowhere"), None);
        assert!(scan.eat_str(" re"));
        assert!(scan.starts_with("st"));
        assert_eq!(scan.peek_nth(1), Some('t'));
        assert_eq!(scan.take_rest(), "st");
        assert!(scan.is_done());
        assert_eq!(scan.bump(), None);
        scan.rewind(value);
        assert_eq!(scan.take_chars(2), "vä");
        assert_eq!(scan.seen(), "key = \"vä");
        assert_eq!(scan.take_chars(99), "rde\" rest");
    }

    /// A cursor reads a slice by items and runs, and gives what it read since a
    /// spot.
    ///
    /// # Panics
    ///
    /// Panics when a run under 10 is not `1, 2, 3`, when `peek` or `peek_nth`
    /// looks at the wrong item or a far place does not answer None (an overflow
    /// in the place fails it too), when `eat_if` takes an item that does not
    /// match, when `take_while` past the end does not give the rest, or when
    /// `since` does not give everything read after the spot.
    #[test]
    fn a_cursor_reads_a_slice_piece_by_piece() {
        let items = [1_u8, 2, 3, 10, 11, 4];
        let mut cursor = Cursor::new(&items);
        assert_eq!(cursor.take_while(|item| *item < 10), &[1, 2, 3]);
        let spot = cursor.spot();
        assert_eq!(cursor.peek(), Some(&10));
        assert_eq!(cursor.peek_nth(2), Some(&4));
        assert_eq!(cursor.peek_nth(usize::MAX), None);
        assert_eq!(cursor.eat_if(|item| *item == 11), None);
        assert_eq!(cursor.bump(), Some(&10));
        assert_eq!(cursor.take_while(|_| true), &[11, 4]);
        assert_eq!(cursor.bump(), None);
        assert_eq!(cursor.since(spot), &[10, 11, 4]);
    }
}
