# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""Writes md2html's table of HTML's named character references.

The table is made from one file, `scripts/entities/entities.json`: the
WHATWG's list that CommonMark refers to, vendored unchanged from
https://html.spec.whatwg.org/entities.json (fetched 2026-11-24; the
server dated it 2025-11-12). The HTML Standard says of it: "This list is
static and will not be expanded or changed in the future." The
generator refuses any other bytes in that file (`SOURCE_SHA256`), so the
table is a function of the repository alone.

CommonMark knows a reference only in its form ending in `;`, so the
names without one are left out. The names are written sorted, so a
lookup is a binary search, and in parts short enough for the
repository's file-size limit.

It needs Python 3.8 or later and nothing outside the standard library;
what it does (read JSON, sort by code point, format) is the same in every
such version, so the interpreter is no input of the table. It writes
`\\n` line ends on every system and `--check` compares bytes.

    python3 scripts/gen_entities.py          # write the table
    python3 scripts/gen_entities.py --check  # fail when it is out of date

`just entities` and `just entities-check` run the same; the gate
`entities-drift` runs the check on every push and pull request.
"""

import hashlib
import json
import pathlib
import sys

# The oldest Python this script is written for.
OLDEST_PYTHON = (3, 8)

# The WHATWG's list, from the repository's root.
SOURCE = pathlib.Path("scripts/entities/entities.json")

# The SHA-256 of the list as fetched; any other bytes are refused.
SOURCE_SHA256 = "d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6"

# Where the table goes, from the repository's root.
TABLE = pathlib.Path("crates/md2html/src/text/entity")

# How many names one part holds, so no file passes 800 lines.
PER_PART = 720

HEADER = """\
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
"""

WARNING = """\
// Written by `scripts/gen_entities.py` (`just entities`) from the WHATWG's
// list in `scripts/entities/entities.json`: do not edit by hand.
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


def source():
    """The WHATWG's list as a dictionary, once its bytes are known to be the recorded ones."""
    raw = SOURCE.read_bytes()
    found = hashlib.sha256(raw).hexdigest()
    if found != SOURCE_SHA256:
        sys.exit(
            f"{SOURCE} is not the recorded list (SHA-256 {found}, expected"
            f" {SOURCE_SHA256}); the WHATWG's list does not change, so restore it"
        )
    return json.loads(raw.decode("utf-8"))


def names():
    """The names ending in `;`, without `&` and `;`, sorted, with their characters."""
    table = {
        name[1:-1]: entry["characters"]
        for name, entry in source().items()
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
    if sys.version_info < OLDEST_PYTHON:
        sys.exit("gen_entities.py needs Python %d.%d or later" % OLDEST_PYTHON)
    check = sys.argv[1:] == ["--check"]
    if sys.argv[1:] not in ([], ["--check"]):
        sys.exit("usage: gen_entities.py [--check]")
    stale = []
    for path, text in files().items():
        wanted = text.encode("utf-8")
        if check:
            if not path.is_file() or path.read_bytes() != wanted:
                stale.append(str(path))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(wanted)
    if stale:
        sys.exit("out of date (run `just entities`): " + ", ".join(stale))


if __name__ == "__main__":
    main()
