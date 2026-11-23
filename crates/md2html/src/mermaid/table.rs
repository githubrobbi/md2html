// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Things kept by number.

use core::iter::once;
use core::marker::PhantomData;

/// The number of one entry of a [`Table`], typed by what it numbers so that
/// a node's number cannot look up an edge.
pub(super) trait Id: Copy {
    /// The id of the entry at `index`.
    fn at(index: usize) -> Self;
    /// Where its entry is.
    fn index(self) -> usize;
}

/// Entries by id.
///
/// Ids come from [`Table::push`] and from walking a table, and a table made
/// by [`Table::map`] has the entries of its source under the same ids; a
/// lookup still answers `None` for an id it does not hold.
#[derive(Clone, Debug)]
pub(super) struct Table<Key, Item> {
    /// The entries, in the order they came.
    items: Vec<Item>,
    /// The kind of id.
    key: PhantomData<Key>,
}

/// No entries.
impl<Key, Item> Default for Table<Key, Item> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            key: PhantomData,
        }
    }
}

/// The entries in the order they come, numbered from the first.
impl<Key, Item> FromIterator<Item> for Table<Key, Item> {
    fn from_iter<Items: IntoIterator<Item = Item>>(iter: Items) -> Self {
        Self {
            items: iter.into_iter().collect(),
            key: PhantomData,
        }
    }
}

impl<Key: Id, Item> Table<Key, Item> {
    /// Adds an entry and gives its id.
    pub(super) fn push(&mut self, item: Item) -> Key {
        let id = Key::at(self.items.len());
        self.items.push(item);
        id
    }

    /// Adds the entry `make` makes knowing its id, and gives the id.
    pub(super) fn push_with<Make>(&mut self, make: Make) -> Key
    where
        Make: FnOnce(Key) -> Item,
    {
        let id = Key::at(self.items.len());
        self.items.push(make(id));
        id
    }

    /// A table of what `make` makes of each entry, under the same ids.
    pub(super) fn map<Made, Make>(&self, make: Make) -> Table<Key, Made>
    where
        Make: FnMut(&Item) -> Made,
    {
        Table {
            items: self.items.iter().map(make).collect(),
            key: PhantomData,
        }
    }

    /// A table of what `make` makes of each id, under the same ids.
    pub(super) fn map_ids<Made, Make>(&self, make: Make) -> Table<Key, Made>
    where
        Make: FnMut(Key) -> Made,
    {
        self.ids().map(make).collect()
    }

    /// The entry of `id`.
    pub(super) fn get(&self, id: Key) -> Option<&Item> {
        self.items.get(id.index())
    }

    /// The entry of `id`, to change.
    pub(super) fn get_mut(&mut self, id: Key) -> Option<&mut Item> {
        self.items.get_mut(id.index())
    }

    /// How many entries there are.
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are none.
    pub(super) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ids, in order.
    pub(super) fn ids(&self) -> impl DoubleEndedIterator<Item = Key> + use<Key, Item> {
        (0..self.items.len()).map(Key::at)
    }

    /// The entries with their ids, in order.
    pub(super) fn iter(&self) -> impl DoubleEndedIterator<Item = (Key, &Item)> {
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| (Key::at(index), item))
    }

    /// The entries, in order.
    pub(super) fn values(&self) -> core::slice::Iter<'_, Item> {
        self.items.iter()
    }

    /// The entries, in order, to change.
    pub(super) fn values_mut(&mut self) -> core::slice::IterMut<'_, Item> {
        self.items.iter_mut()
    }

    /// The same entries in a table that cannot be empty; none when there
    /// are none.
    pub(super) fn filled(self) -> Option<Filled<Key, Item>> {
        let mut items = self.items.into_iter();
        let first = items.next()?;
        Some(Filled {
            first,
            rest: items.collect(),
            key: PhantomData,
        })
    }
}

/// Entries by id, at least one.
#[derive(Clone, Debug)]
pub(super) struct Filled<Key, Item> {
    /// The first entry.
    first: Item,
    /// The others, in order.
    rest: Vec<Item>,
    /// The kind of id.
    key: PhantomData<Key>,
}

impl<Key: Id, Item> Filled<Key, Item> {
    /// The first entry.
    pub(super) const fn first(&self) -> &Item {
        &self.first
    }

    /// The entries after the first, in order.
    pub(super) fn rest(&self) -> &[Item] {
        &self.rest
    }

    /// The entry of `id`.
    pub(super) fn get(&self, id: Key) -> Option<&Item> {
        id.index()
            .checked_sub(1)
            .map_or(Some(&self.first), |place| self.rest.get(place))
    }

    /// The entries, in order.
    pub(super) fn values(&self) -> impl Iterator<Item = &Item> {
        once(&self.first).chain(&self.rest)
    }

    /// A table of what `make` makes of each entry, under the same ids.
    pub(super) fn map<Made, Make>(&self, mut make: Make) -> Filled<Key, Made>
    where
        Make: FnMut(&Item) -> Made,
    {
        Filled {
            first: make(&self.first),
            rest: self.rest.iter().map(make).collect(),
            key: PhantomData,
        }
    }
}
