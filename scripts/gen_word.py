# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""Writes md2html's table of the characters a heading's anchor keeps besides letters.

GitHub gives a heading the anchor its text makes once lowercased, every
character removed that is not a letter, a mark, a decimal digit, a
connector punctuation (`_`), a joiner (U+200C, U+200D), a space or a
hyphen, and each space made a hyphen. The standard library knows the
letters (`char::is_alphabetic`, Unicode's `Alphabetic` property); the
rest is this table: the code points of the general categories
Nonspacing_Mark, Spacing_Mark, Enclosing_Mark, Decimal_Number and
Connector_Punctuation, and the two joiners, as sorted ranges.

The table is made from one file, `scripts/unicode/DerivedGeneralCategory.txt`:
the Unicode Character Database's general categories, vendored unchanged
from https://www.unicode.org/Public/16.0.0/ucd/extracted/DerivedGeneralCategory.txt
(fetched 2026-11-25), the version of Unicode the standard library of
md2html's minimum Rust is built on. The generator refuses any other bytes
in that file (`SOURCE_SHA256`), so the table is a function of the
repository alone.

It needs Python 3.8 or later and nothing outside the standard library;
what it does (read lines, sort numbers, format) is the same in every such
version, so the interpreter is no input of the table. It writes `\\n`
line ends on every system and `--check` compares bytes.

    python3 scripts/gen_word.py          # write the table
    python3 scripts/gen_word.py --check  # fail when it is out of date

`just word-table` and `just word-table-check` run the same; the gate
`word-drift` runs the check on every push and pull request, and
md2html's tests check the table against the file code point by code
point (`crates/md2html/src/tests/anchors.rs`).
"""

import hashlib
import pathlib
import sys

# The oldest Python this script is written for.
OLDEST_PYTHON = (3, 8)

# The Unicode Character Database's general categories, from the repository's root.
SOURCE = pathlib.Path("scripts/unicode/DerivedGeneralCategory.txt")

# The SHA-256 of the file as fetched; any other bytes are refused.
SOURCE_SHA256 = "7676ab755a41ef82108460238569e60ad65c191ddafe61b36c6765ec1353f293"

# Where the table goes, from the repository's root.
TABLE = pathlib.Path("crates/md2html/src/text/word.rs")

# The general categories kept, by the names the file's headings give
# them: the marks, the decimal digits, the connector punctuation.
KEPT = (
    "Nonspacing_Mark",
    "Spacing_Mark",
    "Enclosing_Mark",
    "Decimal_Number",
    "Connector_Punctuation",
)

# A heading of the file, before the lines of one general category.
CATEGORY = "# General_Category="

# The joiners, which a word keeps too (Unicode's `Join_Control`).
JOINERS = (0x200C, 0x200D)

# The table is the Unicode Character Database's data incorporated into
# source code, under the Unicode License v3: the file says so beside
# md2html's own licence, and NOTICE carries Unicode's notice into every
# release.
HEADER = """\
// SPDX-License-Identifier: (MIT OR Apache-2.0) AND Unicode-3.0
// Copyright (c) 2026 Sky, LLC
// Copyright (c) 1991-2024 Unicode, Inc.
"""

WARNING = """\
// Written by `scripts/gen_word.py` (`just word-table`) from the Unicode
// Character Database's `DerivedGeneralCategory.txt` in `scripts/unicode/`:
// do not edit by hand.
"""


def source():
    """The file's lines, once its bytes are known to be the recorded ones."""
    raw = SOURCE.read_bytes()
    found = hashlib.sha256(raw).hexdigest()
    if found != SOURCE_SHA256:
        sys.exit(
            f"{SOURCE} is not the recorded file (SHA-256 {found}, expected"
            f" {SOURCE_SHA256}); restore it, or record a new version here only"
            " with md2html's minimum Rust"
        )
    return raw.decode("utf-8").splitlines()


def version(lines):
    """The Unicode version the file's first line names."""
    first = lines[0]
    prefix = "# DerivedGeneralCategory-"
    if not first.startswith(prefix) or not first.endswith(".txt"):
        sys.exit(f"{SOURCE} does not start with its name and version")
    return first[len(prefix) : -len(".txt")]


def kept(lines):
    """Every code point of the kept categories, and the joiners."""
    points = set(JOINERS)
    category = None
    for line in lines:
        if line.startswith(CATEGORY):
            category = line[len(CATEGORY) :].strip()
            continue
        data = line.split("#", 1)[0].strip()
        if not data or category not in KEPT:
            continue
        span = data.split(";")[0].strip()
        first, _, last = span.partition("..")
        points.update(range(int(first, 16), int(last or first, 16) + 1))
    return sorted(points)


def ranges(points):
    """Sorted code points as ranges, first and last, none adjacent to the next."""
    out = []
    for point in points:
        if out and out[-1][1] + 1 == point:
            out[-1][1] = point
        else:
            out.append([point, point])
    return out


def hex_literal(point):
    """A code point as a Rust literal: hexadecimal, four digits at least, in
    groups of four from the right as clippy reads long ones (`0x1_E01F`)."""
    digits = f"{point:04X}"
    groups = []
    while digits:
        groups.insert(0, digits[-4:])
        digits = digits[:-4]
    return "0x" + "_".join(groups)


def table():
    """The table's file, as text."""
    lines = source()
    spans = ranges(kept(lines))
    text = [
        HEADER,
        "\n",
        "//! The characters besides letters that a heading's anchor keeps, as GitHub\n",
        "//! writes anchors: the marks, the decimal digits, the connector\n",
        f"//! punctuation and the joiners (U+200C, U+200D) of Unicode {version(lines)}.\n",
        "\n",
        WARNING,
        "\n",
        "/// The characters, as ranges of the first and the last code point:\n",
        "/// sorted, apart, none touching the next.\n",
        f"pub(super) const KEPT: [(u32, u32); {len(spans)}] = [\n",
    ]
    for first, last in spans:
        text.append(f"    ({hex_literal(first)}, {hex_literal(last)}),\n")
    text.append("];\n")
    return "".join(text)


def main():
    """Writes the table, or with `--check` says whether it is up to date."""
    if sys.version_info < OLDEST_PYTHON:
        sys.exit("gen_word.py needs Python %d.%d or later" % OLDEST_PYTHON)
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        sys.exit("usage: gen_word.py [--check]")
    wanted = table().encode("utf-8")
    if check:
        if not TABLE.is_file() or TABLE.read_bytes() != wanted:
            sys.exit(f"out of date (run `just word-table`): {TABLE}")
    else:
        TABLE.write_bytes(wanted)


if __name__ == "__main__":
    main()
