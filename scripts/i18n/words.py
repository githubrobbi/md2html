#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The interface words of each language, read from its words file.

`crates/md2html/src/config/words/<code>.rs` holds one `Ui` table of
string literals. `read(code)` gives its strings in the file's order, each
under a path (`keys.step`, `months[3]`, `lang.cannot`), with a Template
(`lead` and `tails` around values the page fills in) joined into one
sentence whose slots are named (`This browser cannot translate into
{language}.`), as a reader sees it. `split(path, text)` takes such a
sentence apart again into its lead and tails, refusing one whose slots
were lost or reordered. The review site (`scripts/i18n/review_site.py`)
shows these; the answers' import puts them back.

Left out: how a date is ordered (`date_fmt`), the headings a glossary
table's column of meanings may have (`meaning_heads`), and the joint and
unit of a text-size step: shapes, not sentences.

Run alone, `words.py CODE` prints a language's strings as JSON; `words.py
--set CODE PATH TEXT` writes one string (a Template's sentence, its slots
kept) into the words file; `words.py --check` checks that every language
has exactly English's paths and that every string goes back into the file
as it came out (exit 1 otherwise).
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORDS = ROOT / "crates/md2html/src/config/words"

# Every language md2html speaks besides English, in the order the review
# site lists them.
CODES = (
    "de", "es", "fr", "he", "id", "it", "ja", "ja-Hira", "ja-Latn",
    "jv", "nds", "nl", "yi", "yue", "zh-Hans", "zh-Hant",
)

# Fields that are shapes, not sentences.
SKIPPED = re.compile(r"^(date_fmt|meaning_heads)(\b|\.|\[)|\.(joint|unit)$")

# The names of a Template's slots, by path; a path not listed numbers them.
SLOTS = {
    "results": ["number"],
    "glossary_shared": ["address"],
    "note_n": ["number"],
    "footer": ["file"],
    "figure.position": ["this", "count"],
    "size.scaled": ["percent"],
    "theme.tip": ["now", "next", "after"],
    "theme.system_now": ["mode"],
    "lang.notice": ["original"],
    "lang.cannot": ["language"],
    "credit": ["md2html"],
}

TOKEN = re.compile(
    r'(?P<str>"(?:[^"\\]|\\.)*")|(?P<ident>[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)'
    r"|(?P<punct>[{}\[\]():,&])|(?P<num>\d+)|(?P<space>\s+|//[^\n]*)",
    re.S,
)


def path_of(code):
    """The words file of `code`."""
    return WORDS / (code.replace("-", "_").lower() + ".rs")


def unescape(literal):
    """A Rust string literal's text."""
    body = literal[1:-1]
    out = []
    i = 0
    while i < len(body):
        ch = body[i]
        if ch != "\\":
            out.append(ch)
            i += 1
            continue
        nxt = body[i + 1]
        if nxt == "u":
            end = body.index("}", i)
            out.append(chr(int(body[i + 3:end], 16)))
            i = end + 1
            continue
        out.append({"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'", "0": "\0"}[nxt])
        i += 2
    return "".join(out)


def escape(text):
    """`text` as a Rust string literal, written as the words files write
    it: quotes and backslashes escaped, a no-break space as `\\u{a0}`."""
    body = text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    body = body.replace("\u00a0", "\\u{a0}").replace("\u202f", "\\u{202f}")
    return '"' + body + '"'


def tokens(source):
    """The tokens of the `WORDS` table, as (kind, text, start, end)."""
    start = source.index("pub(super) static WORDS: Ui =")
    start = source.index("=", start) + 1
    pos = start
    while pos < len(source):
        match = TOKEN.match(source, pos)
        if not match:
            if source[pos] == ";":
                return
            raise ValueError(f"unreadable at {source[pos:pos + 30]!r}")
        kind = match.lastgroup
        if kind != "space":
            yield kind, match.group(), match.start(), match.end()
        pos = match.end()


class Reader:
    """Reads the `WORDS` table into (path, literal span, text) leaves."""

    def __init__(self, source):
        self.toks = list(tokens(source))
        self.i = 0
        self.leaves = []

    def peek(self):
        return self.toks[self.i]

    def take(self, text=None):
        tok = self.toks[self.i]
        if text is not None and tok[1] != text:
            raise ValueError(f"expected {text!r}, found {tok[1]!r}")
        self.i += 1
        return tok

    def value(self, path):
        kind, text, _, _ = self.peek()
        if kind == "str":
            _, lit, start, end = self.take()
            self.leaves.append((path, (start, end), unescape(lit)))
        elif text == "&":
            self.take()
            self.value(path)
        elif text == "[":
            self.take()
            index = 0
            while self.peek()[1] != "]":
                self.value(f"{path}[{index}]")
                index += 1
                if self.peek()[1] == ",":
                    self.take()
            self.take("]")
        elif kind == "ident":
            self.take()
            if self.peek()[1] == "(":
                self.take()
                self.value(path)
                self.take(")")
            elif self.peek()[1] == "{":
                self.take()
                while self.peek()[1] != "}":
                    _, field, _, _ = self.take()
                    self.take(":")
                    self.value(f"{path}.{field}" if path else field)
                    if self.peek()[1] == ",":
                        self.take()
                self.take("}")
        else:
            raise ValueError(f"unexpected {text!r} at {path}")


def leaves(code, source=None):
    """Every string of `code`'s table: (path, (start, end) of its literal
    in the file, text). `source` is the file's text when not the file on
    disk (an earlier version of it)."""
    if source is None:
        source = path_of(code).read_text(encoding="utf-8")
    reader = Reader(source)
    reader.value("")
    return reader.leaves


def slot_names(path, count):
    """The names of a Template's `count` slots."""
    names = SLOTS.get(path, [])
    return names if len(names) == count else [str(n + 1) for n in range(count)]


def read(code, source=None):
    """`code`'s strings: an ordered dict of path to sentence, a Template
    joined with its named slots (from `source`, as `leaves` reads it)."""
    out = {}
    templates = {}
    for path, _, text in leaves(code, source):
        if SKIPPED.search(path):
            continue
        if path.endswith(".lead") or re.search(r"\.tails\[\d+\]$", path):
            base = path.rsplit(".", 1)[0]
            templates.setdefault(base, []).append(text)
            out.setdefault(base, None)
            continue
        out[path] = text
    for base, parts in templates.items():
        names = slot_names(base, len(parts) - 1)
        sentence = parts[0]
        for name, tail in zip(names, parts[1:]):
            sentence += "{" + name + "}" + tail
        out[base] = sentence
    return out


def split(path, sentence, count):
    """A Template's sentence taken apart into its lead and `count` tails;
    ValueError when its slots are not all there, once each, in order."""
    names = slot_names(path, count)
    parts = []
    rest = sentence
    for name in names:
        mark = "{" + name + "}"
        if rest.count(mark) != 1:
            raise ValueError(f"{path}: {mark} must appear once")
        head, rest = rest.split(mark)
        parts.append(head)
    parts.append(rest)
    return parts


def write(code, path, sentence):
    """Writes `sentence` as the string at `path` of `code`'s words file: a
    plain string's literal, or a Template's lead and tails taken apart
    from it. ValueError for a path the file does not have, or a sentence
    whose slots were lost."""
    file = path_of(code)
    source = file.read_text(encoding="utf-8")
    found = leaves(code, source)
    plain = [span for leaf, span, _ in found if leaf == path]
    if plain:
        replacements = [(plain[0], sentence)]
    else:
        parts = [span for leaf, span, _ in found
                 if leaf == f"{path}.lead" or re.fullmatch(re.escape(path) + r"\.tails\[\d+\]", leaf)]
        if not parts:
            raise ValueError(f"{code}: no string at {path}")
        texts = split(path, sentence, len(parts) - 1)
        replacements = list(zip(parts, texts))
    for (start, end), text in sorted(replacements, reverse=True):
        source = source[:start] + escape(text) + source[end:]
    file.write_text(source, encoding="utf-8")


def check():
    """Every language has English's paths, and every table survives a
    round trip; the problems found, one per line."""
    problems = []
    english = read("en")
    for code in ("en",) + CODES:
        strings = read(code)
        if list(strings) != list(english):
            missing = set(english) - set(strings)
            extra = set(strings) - set(english)
            problems.append(f"{code}: paths differ (missing {sorted(missing)}, extra {sorted(extra)})")
        source = path_of(code).read_text(encoding="utf-8")
        for path, (start, end), text in leaves(code):
            if unescape(escape(text)) != text:
                problems.append(f"{code}: {path} does not survive escaping")
            if unescape(source[start:end]) != text:
                problems.append(f"{code}: {path} misread")
    return problems


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--set":
        write(sys.argv[2], sys.argv[3], sys.argv[4])
        sys.exit(0)
    if sys.argv[1:] == ["--check"]:
        found = check()
        for line in found:
            print(line)
        sys.exit(1 if found else 0)
    if len(sys.argv) != 2:
        sys.exit("usage: words.py CODE | --set CODE PATH TEXT | --check")
    print(json.dumps(read(sys.argv[1]), ensure_ascii=False, indent=2))
