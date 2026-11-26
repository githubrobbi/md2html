#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The project's site on GitHub Pages (lane:pages), written by md2html.

What a visitor clicks to see md2html at work: the README as the front page
(`index.html`, and `README.html` for the pages that link to it by name),
the documentation as the handbook md2html makes of `docs/` (the
capabilities page in all seventeen languages among it), and the documents
at the root that those pages link to. Every page is a self-contained file
md2html wrote; a file a page links to that is no page (a screenshot the
README opens in full) is copied beside it. A link that still leads
nowhere fails the build, so the site never ships a broken link.

usage: pages_site.py MD2HTML SOURCES OUT
  MD2HTML  the md2html binary to write the pages with
  SOURCES  the checkout whose documents become the site
  OUT      the folder to write the site into; must not exist

`pages.yml` runs this script from the default branch on a checkout of
the released tag, with that tag's own md2html, so a release made before
the script existed gets its site too. Exit 1 on a failed render or a
broken link, 2 on wrong arguments.
"""

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

# The documents at the root that the site's pages link to, by name.
ROOT_PAGES = ("CHANGELOG", "SECURITY", "AGENTS", "GOTO")

# A link that leaves the site: a scheme (`https:`, `data:`, `mailto:`), a
# fragment of the same page, or a host.
EXTERNAL = re.compile(r"^(?:[a-z][a-z0-9+.-]*:|#|//)", re.IGNORECASE)
LINK = re.compile(r'\s(?:href|src)="([^"]+)"')


def render(md2html, sources, source, target):
    """`source` of `sources` (a Markdown file or `docs/`) written by
    md2html to `target`."""
    subprocess.run([str(md2html), str(source), "-o", str(target)], cwd=sources, check=True,
                   stdout=subprocess.DEVNULL)


def targets(page, out):
    """Where each relative link of `page` leads, as a path under `out`."""
    for match in LINK.finditer(page.read_text(encoding="utf-8")):
        url = match.group(1)
        if EXTERNAL.match(url):
            continue
        path = unquote(urlsplit(url).path)
        if path:
            yield Path(os.path.relpath((page.parent / path).resolve(), out.resolve()))


def missing(out):
    """The links of the site's pages that lead to no file: (target, a page
    that links to it), each target once."""
    found = {}
    for page in sorted(out.rglob("*.html")):
        for target in targets(page, out):
            if not (out / target).exists():
                found.setdefault(target, page.relative_to(out))
    return found


def build(md2html, sources, out):
    """Writes the site of `sources` into `out`; the broken links left, if
    any."""
    out.mkdir(parents=True)
    render(md2html, sources, "docs", out / "docs")
    render(md2html, sources, "README.md", out / "index.html")
    shutil.copyfile(out / "index.html", out / "README.html")
    for name in ROOT_PAGES:
        render(md2html, sources, f"{name}.md", out / f"{name}.html")
    broken = {}
    for target, page in missing(out).items():
        source = sources / target
        if target.parts[0] != ".." and source.is_file() and source.suffix != ".md":
            (out / target).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, out / target)
        else:
            broken[target] = page
    return broken


def main(argv):
    if len(argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2
    md2html, sources, out = (Path(arg).resolve() for arg in argv[1:])
    if not md2html.is_file():
        print(f"pages_site: no md2html at {md2html}", file=sys.stderr)
        return 2
    if not (sources / "README.md").is_file():
        print(f"pages_site: {sources} holds no README.md", file=sys.stderr)
        return 2
    if out.exists():
        print(f"pages_site: {out} exists; the site is written into a new folder", file=sys.stderr)
        return 2
    try:
        broken = build(md2html, sources, out)
    except subprocess.CalledProcessError as failed:
        print(f"pages_site: md2html failed on {failed.cmd[1]} (exit {failed.returncode})", file=sys.stderr)
        return 1
    for target, page in sorted(broken.items()):
        print(f"pages_site: {page} links to {target}, which is not on the site", file=sys.stderr)
    if broken:
        return 1
    pages = sum(1 for _ in out.rglob("*.html"))
    print(f"pages_site: {pages} pages in {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
