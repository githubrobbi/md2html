<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2025-2026 Sky, LLC.
-->

# Types, traits and generics

## The rule

Data gets a type that says what it is, and a state that cannot occur gets no way to be written down. Traits and generics are used where they do work, not for decoration.

## Why

Most lint findings are symptoms of a missing type. A function with nine arguments is a struct nobody named; four booleans are an enum with a few real cases and many impossible ones; a `usize` used as a place in a text has to be checked at every use, where a type that can only hold valid places needs no check at all. Fixing the type removes the whole family of findings at once; fixing each site leaves the cause in place for the next change. This is the main way findings are fixed in this workspace ([lint-posture.md](lint-posture.md), "Fixing a finding").

## Enforced by

`Cargo.toml`, at deny: `too_many_arguments`, `type_complexity`, `struct_excessive_bools`, `fn_params_excessive_bools`, `option_option`, `large_enum_variant`, `wildcard_enum_match_arm`, `rest_pattern_accessible_field`, `wrong_self_convention`, `use_self`, `trait_duplication_in_bounds`, `impl_trait_in_params`, `partial_pub_fields`, `pub_underscore_fields`, `missing_docs_in_private_items`; at warn (an error under `-D warnings`): `multiple_bound_locations`, rustc's `unreachable_pub`. The thresholds are Clippy's defaults: seven arguments, three booleans. Gates `lint-prod`, `lint-tests`, `lint-ci`.

## Types

- **Name what travels together.** A tuple such as `(f64, f64, f64, f64)` or a long parameter list becomes a struct with named fields: a box on a drawing is `Rect { left, top, right, bottom }`, a point `Point { x, y }` (`mermaid/plane.rs`).
- **A quantity has a type.** A length on a drawing is `mermaid::geom::Len`, not a bare `f64`; its arithmetic is written once, in `geom.rs` ([lint-posture.md](lint-posture.md), "The one exception").
- **One enum for one choice.** Booleans of which only some combinations make sense are a choice between cases: name the cases. The command line's are `Action` (write or check), `Mode` (once, watch, or serve on a port) and `Shape` (page or fragment) in `cli/args.rs`.
- **Three states are three variants.** `Option<Option<T>>` becomes an enum: the sanitizer's `Verdict` on an attribute is `Value(String)`, `Bare` or `Drop`.
- **Records, not columns.** Vectors kept in parallel and indexed by the same `usize` become one vector of a record type.
- **A place is not a number.** `scan::Mark` and `scan::Spot` are newtypes over a position: only a reader hands one out, and the reader checks it when it is given back, so no caller does arithmetic on positions or checks them. In the diagrams a node, an edge or an actor is named by its own id type (`EdgeId`, `ActorId`), so the number of a node cannot look up an edge.
- **Keep the invariant inside.** `Scan` holds the text and the unread rest, and the rest is always a tail of the text; nothing outside the module can break that, so nothing outside has to check it.
- **Match every case.** No wildcard arm and no `..` over fields that could be named: when a variant or a field is added, the compiler lists every place that has to decide about it.
- **Visibility says scope.** md2html is a binary: its items are `pub(crate)`, `pub(super)` or private, never `pub`. A struct's fields are all public or all private (`partial_pub_fields`), and private when the value has an invariant.
- **Names follow the conventions.** `as_` borrows cheaply, `to_` converts at a cost, `into_` consumes (`wrong_self_convention`).
- **Every item says what it is.** Private items included (`missing_docs_in_private_items`).

## Traits

A trait earns its place with two or more real implementations, or as the seam a test substitutes. Otherwise a function or an inherent method does the job with less to read. A closed set of kinds is an enum, not `dyn Trait`: the blocks and the inline content of a document are the enums `Block` and `Node` in the page model (`model.rs`). Every kind of block and every compound kind of inline content (a link, an image, a formula) is a struct of its own that its variant carries, so a reader binds the one value it matches, and a match over them names every kind.

md2html declares two traits:

- `text::Put`, implemented for `String`. It exists so that the `put!` macro can append formatted text through one method call whether it is given a `String` or a `&mut String`.
- `mermaid::table::Id`, implemented by each id type of the diagrams. A `Table<Key, Item>` keeps its entries by an id of one type, so an id of another kind cannot be used to look one up.

## Generics

A generic parameter is right where callers really pass different types (`Cursor<'items, Item>` reads characters, lines or tokens; `Table<Key, Item>` holds nodes, edges or actors) or where a closure is inlined (`Scan::take_while` takes `Test: FnMut(char) -> bool`). Parameters have names that say what they are, and their bounds are written in one place, not split between the parameter list and the `where` clause (`multiple_bound_locations`). `impl Trait` in the parameters of a public function is refused (`impl_trait_in_params`), because a caller could not name the type; md2html has no public functions, and `impl Read` in `input::read_limited` takes a file, standard input or a test's bytes alike.
