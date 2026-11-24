#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
"""What the repository's documents name exists.

The gate `doc-references` (scripts/ci/gates.toml). A document that names a
file, a folder, a `just` recipe or a gate that is not there sends its reader
nowhere; this check holds the claims that can be held by reading the tree:

1. Every repository path a tracked Markdown document names exists: a path in
   backticks or a link target that starts with a top-level entry of the
   repository (`docs/...`, `crates/...`, `.github/...`), a crate-relative
   `src/...` path, a `.rs` file named by its last parts (`serve/http.rs`),
   and every relative link. A rendered page (`README.html`) counts when its
   Markdown is tracked.
2. Every `just <recipe>` a document names is a recipe or an alias of the
   justfile.
3. AGENTS.md, section 6, has a row for every gate of `scripts/ci/gates.toml`,
   and names no gate that is not there.

What is not checked, by rule rather than by list: the documents that record
the past (CHANGELOG.md, what the repository held at each release, and the
research of `reports/` and `research_notes/`, which describes md2html 0.3.3
and other projects' code), text with a wildcard, a placeholder or a setting
in it (`*`, `{`, `<`, `$`, `=`), paths outside the repository (`~/`, `/`),
and addresses on the web.

Python 3.8 or later and its standard library; run from anywhere. Prints
each finding as `file:line: message` and exits 1 when there is one.
"""

import pathlib
import re
import subprocess
import sys
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parents[2]

# Characters that make a span an example or a pattern rather than a path.
PATTERN = re.compile(r"[*{}<>$|=\\]")
# A `just` recipe run as a command: `just go`, `just pr "..."`, `... && just go`.
RECIPE = re.compile(r"(?:^|&& |; )(?:\$ )?just ([a-z][a-z0-9_-]*)")
# A recipe's header in a justfile: `name args:` (not `name := value`).
HEADER = re.compile(r"^@?([A-Za-z_][A-Za-z0-9_-]*)(?:\s[^:=]*(?:=[^:]*)?)?:(?!=)")
ALIAS = re.compile(r"^alias\s+([A-Za-z_][A-Za-z0-9_-]*)\s*:=")
SPAN = re.compile(r"`([^`\n]+)`")
LINK = re.compile(r"\]\(([^)\s]+)\)")


def tracked():
    """Every tracked file, and every folder that holds one."""
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True
    ).stdout.decode("utf-8")
    files = {name for name in out.split("\0") if name}
    folders = set()
    for name in files:
        parts = name.split("/")[:-1]
        for end in range(1, len(parts) + 1):
            folders.add("/".join(parts[:end]))
    return files, folders


def recipes():
    """The recipe and alias names of the justfile and every file it imports."""
    names = set()
    for path in [ROOT / "justfile", *sorted((ROOT / "just").glob("*.just"))]:
        for line in path.read_text(encoding="utf-8").splitlines():
            found = HEADER.match(line) or ALIAS.match(line)
            if found and not line.startswith(("import ", "set ", "export ")):
                names.add(found.group(1))
    return names


def gates():
    """The gate ids of the manifest."""
    text = (ROOT / "scripts/ci/gates.toml").read_text(encoding="utf-8")
    return set(re.findall(r'^\[\[gate\]\]\s*\nid = "([^"]+)"', text, re.M))


def strip_fragment(target):
    """A path without its `#anchor`, `?query` or `:line`."""
    target = target.split("#", 1)[0].split("?", 1)[0]
    return re.sub(r":\d+(-\d+)?$", "", target)


class Tree:
    """The repository as the documents may name it."""

    def __init__(self):
        self.files, self.folders = tracked()
        self.top = {name.split("/", 1)[0] for name in self.files}
        self.entries = self.files | self.folders

    def has(self, path):
        """Whether `path` (relative to the root) is a file or folder here,
        or a page rendered from a tracked Markdown file."""
        path = path.rstrip("/")
        if path in self.entries:
            return True
        if path.endswith(".html"):
            return path[: -len(".html")] + ".md" in self.files
        return False

    def ends(self, tail):
        """Whether some tracked file or folder ends with `/tail`."""
        tail = "/" + tail.rstrip("/")
        return any(entry.endswith(tail) for entry in self.entries)

    def named(self, span, doc):
        """Whether a path a document names in backticks exists; `None` when
        the span is not a repository path at all."""
        span = strip_fragment(span.strip())
        if not span or " " in span or ":" in span or PATTERN.search(span):
            return None
        if span.startswith(("~", "/", "http:", "https:", "-")) or "://" in span:
            return None
        first = span.split("/", 1)[0]
        if "/" in span and first in self.top:
            return self.has(span)
        if span.startswith("src/") or span.endswith(".rs"):
            return self.has(span) or self.ends(span)
        return None

    def linked(self, target, doc):
        """Whether a relative link of a document leads to something here;
        `None` for a link to the web, an anchor or an e-mail address."""
        if re.match(r"^[a-z][a-z0-9+.-]*:", target) or target.startswith("#"):
            return None
        target = urllib.parse.unquote(strip_fragment(target))
        if not target or PATTERN.search(target):
            return None
        joined = (pathlib.PurePosixPath(doc).parent / target).as_posix()
        parts = []
        for part in joined.split("/"):
            if part == "..":
                if not parts:
                    return False
                parts.pop()
            elif part not in ("", "."):
                parts.append(part)
        return self.has("/".join(parts))


def documents(tree):
    """The documents read: every tracked Markdown file but those that
    record the past."""
    return sorted(
        name
        for name in tree.files
        if name.endswith(".md")
        and name != "CHANGELOG.md"
        and not name.startswith(("reports/", "research_notes/"))
    )


def check_document(tree, names, doc, findings):
    """Every path, link and recipe `doc` names."""
    fenced = False
    text = (ROOT / doc).read_text(encoding="utf-8")
    for number, line in enumerate(text.splitlines(), 1):
        if line.lstrip().startswith(("```", "~~~")):
            fenced = not fenced
            continue
        where = f"{doc}:{number}"
        spans = SPAN.findall(line) if not fenced else []
        for span in spans:
            if tree.named(span, doc) is False:
                findings.append(f"{where}: `{span}` is not in the repository")
        if not fenced:
            for target in LINK.findall(SPAN.sub("", line)):
                if tree.linked(target, doc) is False:
                    findings.append(f"{where}: the link {target} leads nowhere")
        candidates = spans if not fenced else [line]
        for candidate in candidates:
            for recipe in RECIPE.findall(candidate.strip()):
                if recipe not in names:
                    findings.append(f"{where}: `just {recipe}` is not a recipe")


def check_gate_table(findings):
    """AGENTS.md section 6 names every gate, and only gates."""
    text = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
    section = re.search(r"^## 6\..*?(?=^## )", text, re.M | re.S)
    if section is None:
        findings.append("AGENTS.md: no section 6 (when a gate fails)")
        return
    named = set()
    for row in section.group(0).splitlines():
        if row.startswith("| `"):
            named.update(re.findall(r"`([a-z0-9-]+)`", row.split("|")[1]))
    known = gates()
    for gate in sorted(known - named):
        findings.append(f"AGENTS.md: section 6 has no row for the gate `{gate}`")
    for name in sorted(named - known):
        findings.append(f"AGENTS.md: section 6 names `{name}`, which is no gate")


def main():
    """Runs every check; prints the findings; 1 when there is one."""
    tree = Tree()
    names = recipes()
    findings = []
    for doc in documents(tree):
        check_document(tree, names, doc, findings)
    check_gate_table(findings)
    for finding in findings:
        print(finding)
    if findings:
        print(f"doc-references: {len(findings)} finding(s)", file=sys.stderr)
        return 1
    print("doc-references: every path, recipe and gate the documents name exists")
    return 0


if __name__ == "__main__":
    sys.exit(main())
