#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The notes of a GitHub release: what a reader comes for first.

The downloads open the page, one link per system, then the package
managers and the three lines that start md2html; what changed follows as
the changelog's summary, with every change folded under it; checksums,
provenance, the SBOM and the build come last, folded too. release.yml
writes the release's notes with this, and so does a maintainer who
rewrites an existing release's page (`gh release edit vX.Y.Z
--notes-file`), so both read the same.

Read from the environment:
  VERSION            the version, `vX.Y.Z` or `X.Y.Z`
  GITHUB_REPOSITORY  `owner/name` of the source
  RELEASE_REPO       `owner/name` the release and its files are in, when
                     not the source's (optional)
  COMMIT             the commit the release was built from
  TRIGGERED_BY       what started the build (optional)
  CODESIGN, BREW, WINGET
                     `true` when lane:codesign, lane:brew, lane:winget are on
The changelog's section for the version comes from
`scripts/ci/versions.py notes`; a version without one is an error, so a
release is never published without its notes. Writes the notes to
standard output; exit 1 on a missing value or section.
"""

import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The archives a release ships, in the order a reader looks for them.
DOWNLOADS = (
    ("Windows x64", "md2html-windows-x64.zip"),
    ("macOS, Apple silicon", "md2html-macos-arm64.zip"),
    ("macOS, Intel", "md2html-macos-x64.zip"),
    ("Linux x64 (static: any distribution, WSL, Alpine)", "md2html-linux-x64.zip"),
)

# The same binaries bare, for scripts that fetch one file.
RAW = (
    "md2html-windows-x64.exe",
    "md2html-macos-arm64",
    "md2html-macos-x64",
    "md2html-linux-x64",
)

SIGNED_MACOS = (
    "The macOS binaries and archives are signed with the Developer ID of "
    "Sky, LLC and notarized by Apple after the build, on the maintainer's "
    "Mac, once their build provenance had been verified: "
    "`codesign --verify --strict md2html` checks the signature, and "
    "`spctl --assess --type open --context context:primary-signature md2html` "
    "answers `source=Notarized Developer ID`. Signing changed their bytes, "
    "so the attestation of the build does not match them, nor the "
    "`CHECKSUMS.txt` and `SHA256SUMS` written again to list them; every "
    "other asset verifies with the command above."
)

ADHOC_MACOS = (
    "The macOS binaries are ad-hoc signed, not notarized: their attestation "
    "matches them like every other asset."
)


def need(name):
    """The environment's value of `name`; a missing one ends the run."""
    value = os.environ.get(name, "")
    if not value:
        sys.exit(f"release_notes: {name} is not set")
    return value


def on(name):
    """Whether the lane `name` names is on."""
    return os.environ.get(name, "") == "true"


def changes(version):
    """The changelog's section for `version` (`X.Y.Z`), split into its
    summary (before the first `###`) and the rest."""
    done = subprocess.run(
        [sys.executable, str(ROOT / "scripts/ci/versions.py"), "notes", version],
        capture_output=True,
        text=True,
        check=False,
    )
    if done.returncode != 0:
        sys.exit(f"release_notes: no changelog section for {version}: {done.stderr.strip()}")
    text = done.stdout.strip()
    head, mark, rest = text.partition("\n### ")
    return head.strip(), (mark.lstrip("\n") + rest).strip()


def notes():
    """The notes, as Markdown."""
    version = need("VERSION").removeprefix("v")
    tag = f"v{version}"
    repo = need("GITHUB_REPOSITORY")
    release_repo = os.environ.get("RELEASE_REPO") or repo
    commit = need("COMMIT")
    started = os.environ.get("TRIGGERED_BY", "the signed tag")
    owner = repo.split("/", 1)[0]
    base = f"https://github.com/{release_repo}/releases/download/{tag}"
    summary, details = changes(version)

    out = [
        "md2html turns Markdown into one complete web page: outline, search, "
        "themes, glossary, diagrams and maths inside one HTML file, and EPUB "
        "and PDF besides.",
        "",
        "## ⬇️ Download",
        "",
        "| Your system | Download |",
        "| --- | --- |",
    ]
    out += [f"| {system} | **[{name}]({base}/{name})** |" for system, name in DOWNLOADS]
    out += [
        "",
        "Unzip it and run `md2html notes.md`: it writes `notes.html` beside "
        "the Markdown (`md2html --help` lists the rest). To run it from "
        "anywhere, put `md2html` (`md2html.exe` on Windows) in a folder on "
        "your `PATH`.",
    ]
    if not on("CODESIGN"):
        out += [
            "",
            "On macOS this build is not notarized: the first launch of a "
            "downloaded copy asks once (right-click it and choose Open, or "
            "run `xattr -d com.apple.quarantine md2html`). Homebrew installs "
            "it without asking.",
        ]
    out += ["", "### Or install it with one command", ""]
    if on("BREW"):
        out.append(f"- **Homebrew** (macOS, Linux): `brew install {owner}/md2html/md2html`")
    if on("WINGET"):
        out.append("- **winget** (Windows): `winget install SkyLLC.md2html`")
    out += [
        f"- **From the source** (with Rust): "
        f"`cargo install --git https://github.com/{repo} --tag {tag} md2html --locked`",
        "",
        f"## What's new in {tag}",
        "",
        summary,
        "",
        f"<details><summary>Every change in {tag}</summary>",
        "",
        details,
        "",
        "</details>",
        "",
        f"The same list is in [CHANGELOG.md](https://github.com/{repo}/blob/{tag}/CHANGELOG.md) "
        "and in every download.",
        "",
        "## Verify a download",
        "",
        "<details><summary>Checksums, build provenance, SBOM and the build</summary>",
        "",
        "Every file's SHA-256 is in `CHECKSUMS.txt` (and `SHA256SUMS`, the "
        "same list under its usual name):",
        "",
        "```bash",
        "sha256sum -c CHECKSUMS.txt --ignore-missing",
        "```",
        "",
        "Every file carries a build-provenance attestation, signed with the "
        "release workflow's identity; it proves the bytes were built by this "
        "repository's release workflow from the commit below:",
        "",
        "```bash",
        f"gh attestation verify <file> --owner {owner}",
        "```",
        "",
        SIGNED_MACOS if on("CODESIGN") else ADHOC_MACOS,
        "",
        "Each crate's CycloneDX SBOM is attached as `sbom-<crate>.cdx.json`, "
        "under the same attestation.",
        "",
        "For scripts that fetch one file, the binaries are attached bare too: "
        + ", ".join(f"`{name}`" for name in RAW)
        + ".",
        "",
        f"Built from {commit}, started by {started}, with the toolchain "
        "`rust-toolchain.toml` pins and `[profile.release]` (LTO, one "
        "codegen unit, symbols stripped).",
        "",
        "</details>",
    ]
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    sys.stdout.write(notes())
