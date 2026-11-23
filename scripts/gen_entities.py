# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""Writes md2html's table of HTML's named character references.

The table is the WHATWG list that CommonMark refers to, as Python's
standard library carries it (`html.entities.html5`). CommonMark knows a
reference only in its form ending in `;`, so the names without one are
left out. The names are written sorted, so a lookup is a binary search,
and in parts short enough for the repository's file-size limit.

    python3 scripts/gen_entities.py          # write the table
    python3 scripts/gen_entities.py --check  # fail when it is out of date

`just entities` and `just entities-check` run the same.
"""

import html.entities
import pathlib
import sys

# Where the table goes, from the repository's root.
TABLE = pathlib.Path("crates/md2html/src/inline/entity")

# How many names one part holds, so no file passes 800 lines.
PER_PART = 720

HEADER = """\
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
"""

WARNING = """\
// Written by `scripts/gen_entities.py` (`just entities`) from the WHATWG
// list in Python's `html.entities.html5`: do not edit by hand.
"""


def literal(text):
    """A Rust string literal of `text`, every character past printable ASCII escaped."""
    out = []
    for ch in text:
        if ch in '"\\':
            out.append("\\" + ch)
        elif " " <= ch <= "~":
            out.append(ch)
        else:
            out.append("\\u{%x}" % ord(ch))
    return '"' + "".join(out) + '"'


def names():
    """The names ending in `;`, without it, sorted, with their characters."""
    table = {
        name[:-1]: chars
        for name, chars in html.entities.html5.items()
        if name.endswith(";")
    }
    return sorted(table.items())


def files():
    """Every file of the table: its path from the repository's root, and its text."""
    entries = names()
    parts = [entries[at : at + PER_PART] for at in range(0, len(entries), PER_PART)]
    out = {}
    index = [
        HEADER,
        "\n",
        "//! HTML's named character references, as `CommonMark` reads them: the\n",
        f"//! {len(entries)} names that end in `;`, sorted, in parts.\n",
        "\n",
        WARNING,
        "\n",
    ]
    for number, _ in enumerate(parts, start=1):
        index.append(f"mod part{number};\n")
    index.append(
        "\n"
        "/// The parts of the table, each sorted and each wholly before the\n"
        "/// next: a name and the characters it stands for (one, or two for a\n"
        "/// few names).\n"
    )
    # As rustfmt writes it: on one line while the line fits.
    members = ", ".join(f"part{number}::NAMES" for number in range(1, len(parts) + 1))
    index.append(f"pub(super) const PARTS: [&[(&str, &str)]; {len(parts)}] = [{members}];\n")
    out[TABLE / "names.rs"] = "".join(index)
    for number, part in enumerate(parts, start=1):
        text = [
            HEADER,
            "\n",
            f"//! Part {number} of the named character references, `{part[0][0]}` to\n",
            f"//! `{part[-1][0]}`.\n",
            "\n",
            WARNING,
            "\n",
            "/// The names of this part, sorted, and the characters each stands for.\n",
            "pub(super) const NAMES: &[(&str, &str)] = &[\n",
        ]
        for name, chars in part:
            text.append(f"    ({literal(name)}, {literal(chars)}),\n")
        text.append("];\n")
        out[TABLE / "names" / f"part{number}.rs"] = "".join(text)
    return out


def main():
    """Writes the table, or with `--check` says whether it is up to date."""
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        sys.exit("usage: gen_entities.py [--check]")
    stale = []
    for path, text in files().items():
        if check:
            if not path.is_file() or path.read_text(encoding="utf-8") != text:
                stale.append(str(path))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
    if stale:
        sys.exit("out of date (run `just entities`): " + ", ".join(stale))


if __name__ == "__main__":
    main()
