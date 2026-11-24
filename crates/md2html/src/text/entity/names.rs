// SPDX-License-Identifier: (MIT OR Apache-2.0) AND BSD-3-Clause
// Copyright (c) 2026 Sky, LLC
// Copyright (c) WHATWG (Apple, Google, Mozilla, Microsoft)

//! HTML's named character references, as `CommonMark` reads them: the
//! 2125 names that end in `;`, sorted, in parts.

// Written by `scripts/gen_entities.py` (`just entities`) from the WHATWG's
// list in `scripts/entities/entities.json`: do not edit by hand.

mod part1;
mod part2;
mod part3;

/// The parts of the table, each sorted and each wholly before the
/// next: a name and the characters it stands for (one, or two for a
/// few names).
pub(super) const PARTS: [&[(&str, &str)]; 3] = [part1::NAMES, part2::NAMES, part3::NAMES];
