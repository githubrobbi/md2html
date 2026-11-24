#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
"""md2html's version, said the same wherever it is said.

The gate `versions` (scripts/ci/gates.toml) and the release's helper
(docs/policies/release_policy.md). The version is written in three places
that must agree (AGENTS.md, section 11), and the minimum Rust in one:

1. `[workspace.package] version` in the root `Cargo.toml`, which every
   member inherits; the workspace's path dependencies require it and
   `Cargo.lock` records it for every member.
2. The capabilities page and its translations (`docs/capabilities.md`,
   `docs/capabilities.*.md`): every `version:` line and every
   `md2html X.Y.Z` they say.
   The install command there and in `README.md` (`cargo install --git
   ... --tag vX.Y.Z ...`) names the release's tag, so it installs the
   release, not `main`.
3. `CHANGELOG.md`: the newest released heading, the first `## [X.Y.Z] -
   date` below `## [Unreleased]` (the form `just ship`'s roll reads), and
   the compare link `[Unreleased]: .../compare/vX.Y.Z...HEAD` under it.

The minimum Rust is `rust-version` in `crates/md2html/Cargo.toml`; the
README, `docs/SUPPLY-CHAIN.md` and `docs/policies/release_policy.md` state
it to readers, and must state that one.

Commands:

    versions.py [check] [--tag vX.Y.Z]   the agreement; with --tag, the
                                         tag names that version too
    versions.py stamp X.Y.Z [--date D]   write X.Y.Z into the capabilities
                                         pages (and today's date, or D,
                                         into their front matter) and into
                                         the install commands, and renew
                                         the fingerprint of each translation
                                         that was current
    versions.py notes X.Y.Z              print the changelog's section for
                                         X.Y.Z (the release notes)

`--root DIR` reads another tree than this repository (the gate's test).
Python 3.8 or later and its standard library. Exit 1 on a finding.
"""

import argparse
import datetime
import hashlib
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]

SEMVER = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
# `version = "..."` as a TOML key at the start of a line.
TOML_VERSION = re.compile(r'^version\s*=\s*"([^"]*)"', re.M)
# A version line of a page's front matter (also in the page's quoted copy
# of its own front matter).
PAGE_VERSION = re.compile(r"^version:[ \t]*(.*)$", re.M)
# The version a page names in its text: `md2html 0.7.7`.
NAMED_VERSION = re.compile(r"\bmd2html (\d+\.\d+\.\d+)\b")
# An install command from the repository, and the release tag it names.
INSTALL = re.compile(r"^.*\bcargo install --git \S+.*$", re.M)
INSTALL_TAG = re.compile(r"--tag v(\d+\.\d+\.\d+)\b")
UNRELEASED = "## [Unreleased]"
RELEASED = re.compile(r"^## \[([^\]]+)\] - (\d{4}-\d{2}-\d{2})$")
COMPARE = re.compile(r"^\[Unreleased\]: (\S+)/compare/v(\S+)\.\.\.HEAD$", re.M)


def table(text, name):
    """The body of the TOML table `[name]` in `text`, to the next table."""
    found = re.search(
        r"^\[" + re.escape(name) + r"\][ \t]*\n(.*?)(?=^\[|\Z)", text, re.M | re.S
    )
    return found.group(1) if found else None


def workspace_version(root, findings):
    """The `[workspace.package]` version of the root `Cargo.toml`."""
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    package = table(text, "workspace.package")
    found = TOML_VERSION.search(package or "")
    if found is None:
        findings.append("Cargo.toml: [workspace.package] has no version")
        return None
    version = found.group(1)
    if not SEMVER.match(version):
        findings.append(f"Cargo.toml: version {version} is not MAJOR.MINOR.PATCH")
    dependencies = table(text, "workspace.dependencies") or ""
    for line in dependencies.splitlines():
        if "path =" not in line:
            continue
        required = re.search(r'version\s*=\s*"([^"]*)"', line)
        name = line.split("=", 1)[0].strip()
        if required is None or required.group(1) != version:
            findings.append(
                f"Cargo.toml: the workspace dependency {name} does not require "
                f"{version} (a path dependency is versioned in lockstep)"
            )
    return version


def lock_versions(root, version, findings):
    """Every workspace member in `Cargo.lock` (a package without a source)
    is recorded at `version`."""
    text = (root / "Cargo.lock").read_text(encoding="utf-8")
    for block in text.split("[[package]]")[1:]:
        if re.search(r"^source\s*=", block, re.M):
            continue
        name = re.search(r'^name\s*=\s*"([^"]*)"', block, re.M)
        recorded = TOML_VERSION.search(block)
        if name and recorded and recorded.group(1) != version:
            findings.append(
                f"Cargo.lock: {name.group(1)} is recorded at {recorded.group(1)}, "
                f"not {version} (run `cargo update --workspace`)"
            )


def pages(root):
    """The capabilities page first, then its translations."""
    docs = root / "docs"
    return [docs / "capabilities.md"] + sorted(docs.glob("capabilities.*.md"))


def front_matter(text):
    """The span of the front matter block at the top of `text`, or None."""
    found = re.match(r"---\n(.*?\n)(?:---|\.\.\.)\n", text, re.S)
    return found.span(1) if found else None


def page_versions(root, version, findings):
    """Every version a capabilities page states is `version`."""
    for page in pages(root):
        name = page.relative_to(root).as_posix()
        text = page.read_text(encoding="utf-8")
        span = front_matter(text)
        if span is None or not PAGE_VERSION.search(text[span[0] : span[1]]):
            findings.append(f"{name}: the front matter has no version")
        for found in PAGE_VERSION.finditer(text):
            if found.group(1).strip() != version:
                line = text.count("\n", 0, found.start()) + 1
                findings.append(
                    f"{name}:{line}: version {found.group(1).strip()}, not {version}"
                )
        for found in NAMED_VERSION.finditer(text):
            if found.group(1) != version:
                line = text.count("\n", 0, found.start()) + 1
                findings.append(
                    f"{name}:{line}: names md2html {found.group(1)}, not {version}"
                )


def install_commands(root, version, findings):
    """Every `cargo install --git` command of the README and the
    capabilities pages names the tag `v<version>`."""
    for page in [root / "README.md"] + pages(root):
        name = page.relative_to(root).as_posix()
        text = page.read_text(encoding="utf-8")
        commands = list(INSTALL.finditer(text))
        if page.name == "README.md" and not commands:
            findings.append(f"{name}: no `cargo install --git` command")
        for found in commands:
            line = text.count("\n", 0, found.start()) + 1
            tag = INSTALL_TAG.search(found.group(0))
            if tag is None:
                findings.append(f"{name}:{line}: the install command names no `--tag v{version}`")
            elif tag.group(1) != version:
                findings.append(f"{name}:{line}: the install command names v{tag.group(1)}, not v{version}")


def newest_release(lines):
    """The index of `## [Unreleased]` and the first released heading's
    match below it (or None)."""
    if lines.count(UNRELEASED) != 1:
        return None, None
    start = lines.index(UNRELEASED)
    for line in lines[start + 1 :]:
        if line.startswith("## "):
            return start, RELEASED.match(line)
    return start, None


def changelog(root, version, findings):
    """The changelog is in the form the roll reads, and its newest release
    is `version`."""
    text = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    lines = text.splitlines()
    start, newest = newest_release(lines)
    if start is None:
        findings.append(f"CHANGELOG.md: no single `{UNRELEASED}` heading")
        return
    if newest is None:
        findings.append(
            "CHANGELOG.md: the heading after Unreleased is not `## [X.Y.Z] - YYYY-MM-DD`"
        )
    elif newest.group(1) != version:
        findings.append(
            f"CHANGELOG.md: the newest release is {newest.group(1)}, not {version}"
        )
    link = COMPARE.search(text)
    if link is None:
        findings.append("CHANGELOG.md: no `[Unreleased]: .../compare/vX.Y.Z...HEAD` link")
    elif link.group(2) != version:
        findings.append(
            f"CHANGELOG.md: the Unreleased link compares from v{link.group(2)}, not v{version}"
        )
    if not re.search(r"^\[" + re.escape(version) + r"\]: ", text, re.M):
        findings.append(f"CHANGELOG.md: no link for [{version}]")


def minimum_rust(root, findings):
    """The documents that state md2html's minimum Rust state its
    `rust-version`."""
    manifest = (root / "crates/md2html/Cargo.toml").read_text(encoding="utf-8")
    found = re.search(r'^rust-version\s*=\s*"([^"]*)"', manifest, re.M)
    if found is None:
        findings.append("crates/md2html/Cargo.toml: no rust-version")
        return
    rust = found.group(1)
    statements = {
        "README.md": f"You need Rust {rust} or newer.",
        "docs/SUPPLY-CHAIN.md": f"| Oldest Rust promised | {rust} |",
        "docs/policies/release_policy.md": f"Today that is {rust},",
    }
    for name, statement in statements.items():
        if statement not in (root / name).read_text(encoding="utf-8"):
            findings.append(f"{name}: does not say `{statement}` (rust-version {rust})")


def check(root, tag):
    """Every finding of the agreement; with `tag`, the tag's too."""
    findings = []
    version = workspace_version(root, findings)
    if version is not None:
        lock_versions(root, version, findings)
        page_versions(root, version, findings)
        install_commands(root, version, findings)
        changelog(root, version, findings)
        if tag is not None and tag != f"v{version}":
            findings.append(f"the tag {tag} does not name the version v{version}")
    minimum_rust(root, findings)
    return version, findings


def stamp(root, version, date):
    """Write `version` and `date` into the capabilities pages; renew the
    fingerprint of each translation that matched the original before."""
    original, *translations = pages(root)
    before = hashlib.sha256(original.read_bytes()).hexdigest()
    current = {}
    for page in translations:
        text = page.read_text(encoding="utf-8")
        span = front_matter(text) or (0, 0)
        current[page] = f"source_sha256: {before}" in text[span[0] : span[1]]
    for page in [original] + translations:
        text = page.read_text(encoding="utf-8")
        span = front_matter(text)
        head, rest = (text[: span[1]], text[span[1] :]) if span else ("", text)
        head = re.sub(r"^date:.*$", f"date: {date}", head, count=1, flags=re.M)
        text = PAGE_VERSION.sub(f"version: {version}", head + rest)
        text = NAMED_VERSION.sub(f"md2html {version}", text)
        text = INSTALL_TAG.sub(f"--tag v{version}", text)
        page.write_text(text, encoding="utf-8", newline="\n")
    readme = root / "README.md"
    text = readme.read_text(encoding="utf-8")
    readme.write_text(INSTALL_TAG.sub(f"--tag v{version}", text), encoding="utf-8", newline="\n")
    after = hashlib.sha256(original.read_bytes()).hexdigest()
    stale = []
    for page in translations:
        if not current[page]:
            stale.append(page.relative_to(root).as_posix())
            continue
        text = page.read_text(encoding="utf-8")
        text = text.replace(f"source_sha256: {before}", f"source_sha256: {after}", 1)
        page.write_text(text, encoding="utf-8", newline="\n")
    return stale


def notes(root, version):
    """The body of the changelog's section for `version`, or None."""
    lines = (root / "CHANGELOG.md").read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        found = RELEASED.match(line)
        if found and found.group(1) == version:
            body = []
            for following in lines[index + 1 :]:
                if following.startswith("## ") or re.match(r"^\[[^\]]+\]: ", following):
                    break
                body.append(following)
            return "\n".join(body).strip("\n")
    return None


def arguments():
    """The command line."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--root", type=pathlib.Path, default=ROOT)
    commands = parser.add_subparsers(dest="command")
    checking = commands.add_parser("check")
    checking.add_argument("--tag")
    stamping = commands.add_parser("stamp")
    stamping.add_argument("version")
    stamping.add_argument("--date")
    noting = commands.add_parser("notes")
    noting.add_argument("version")
    return parser.parse_args()


def main():
    """Runs the command; 1 on a finding."""
    args = arguments()
    root = args.root.resolve()
    if args.command == "stamp":
        if not SEMVER.match(args.version):
            print(f"versions: {args.version} is not MAJOR.MINOR.PATCH", file=sys.stderr)
            return 1
        date = args.date or datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d")
        stale = stamp(root, args.version, date)
        print(f"versions: the capabilities pages say {args.version} ({date})")
        for name in stale:
            print(f"{name}: was stale before, so its fingerprint is not renewed")
        return 0
    if args.command == "notes":
        body = notes(root, args.version)
        if body is None:
            print(f"CHANGELOG.md: no section ## [{args.version}]", file=sys.stderr)
            return 1
        print(body)
        return 0
    version, findings = check(root, getattr(args, "tag", None))
    for finding in findings:
        print(finding)
    if findings:
        print(f"versions: {len(findings)} finding(s)", file=sys.stderr)
        return 1
    print(f"versions: {version} in every place it is said")
    return 0


if __name__ == "__main__":
    sys.exit(main())
