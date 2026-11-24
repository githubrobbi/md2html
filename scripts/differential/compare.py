# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""Renders a corpus with md2html in six ways and judges what came out.

With `--base`, every file is rendered by both programs in each mode and
the results are compared byte for byte: the page or fragment on standard
output, the book and the page of `--epub`, standard error (the file's
path written as `<in>`) and the exit code. A change not meant to change
output must leave every one the same; one meant to names its differences
here, to be read one by one.

Without it, the program is only judged on its own: no file may make it
panic, end by a signal, or run past the time limit. That is the weekly
check of generated documents.

usage:
  python3 scripts/differential/compare.py --new BIN [--base BIN] CORPUS

The environment is fixed (a date, no shared glossary, no home folder, no
Chrome), so two runs of one program give the same bytes. It needs Python
3.8 or later and nothing outside the standard library.
"""

import argparse
import hashlib
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

# The six ways a file is rendered: the arguments after its path.
MODES = {
    "page": ["-o", "-"],
    "html": ["--html", "--embed", "--csp", "-o", "-"],
    "fragment": ["--fragment", "--no-math", "--mermaid", "code", "--no-glossary", "-o", "-"],
    "options": ["--lang", "de", "--large", "--ttl", "5", "--title", "T <x>", "--brand", "B&B",
                "--footer", "*f* [l](u)", "--index", "--no-toc", "--mermaid", "js", "-o", "-"],
    "check": ["--check"],
    "epub": None,
}

# How long one render may take.
LIMIT_SECONDS = 180

# What a run ending in each of these ways is called.
CRASHES = ("panic", "signal", "timeout")


def environment(home):
    """The fixed environment every render gets."""
    return {
        "SOURCE_DATE_EPOCH": "1700000000",
        "MD2HTML_GLOSSARY": os.path.join(home, "no-glossary.md"),
        "HOME": home,
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "MD2HTML_CHROME": os.path.join(home, "no-chrome"),
    }


def render(binary, path, mode, home):
    """[how it ended, digest of what it wrote, last line of a crash]."""
    env = environment(home)
    folder = os.path.dirname(path)
    try:
        if mode == "epub":
            with tempfile.TemporaryDirectory() as out:
                page = os.path.join(out, "p.html")
                ran = subprocess.run([binary, "--html", "--epub", "-q", path, "-o", page],
                                     capture_output=True, env=env, timeout=LIMIT_SECONDS, cwd=folder)
                written = b""
                for name in ("p.epub", "p.html"):
                    try:
                        with open(os.path.join(out, name), "rb") as file:
                            written += file.read()
                    except FileNotFoundError:
                        written += b"<no " + name.encode() + b">"
        else:
            ran = subprocess.run([binary, path] + MODES[mode], capture_output=True, env=env,
                                 timeout=LIMIT_SECONDS, cwd=folder)
            written = b""
    except subprocess.TimeoutExpired:
        return ["timeout", "", ""]
    err = ran.stderr.replace(path.encode(), b"<in>")
    digest = hashlib.sha256(ran.stdout + b"\0" + written + b"\0" + err).hexdigest()[:24]
    if ran.returncode == 101 or b"panicked at" in ran.stderr:
        how = "panic"
    elif ran.returncode < 0:
        how = "signal"
    else:
        how = str(ran.returncode)
    lines = err.decode("utf-8", "replace").strip().splitlines()
    last = lines[-1][:200] if how in CRASHES and lines else ""
    return [how, digest, last]


def sweep(binary, corpus, names, jobs, home):
    """Every file of `names` in every mode, by `binary`."""
    work = [(name, mode) for name in names for mode in MODES]
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        results = pool.map(lambda job: render(binary, os.path.join(corpus, job[0]), job[1], home),
                           work)
        return {"%s %s" % job: result for job, result in zip(work, results)}


def main():
    """Renders, compares, prints the verdict; exits 1 on any finding."""
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--new", required=True, help="the program judged")
    parser.add_argument("--base", help="the program it must match, byte for byte")
    parser.add_argument("--jobs", type=int, default=os.cpu_count() or 4)
    parser.add_argument("--only", action="append", default=[], help="a file name prefix to render; repeatable")
    parser.add_argument("corpus")
    args = parser.parse_args()
    corpus = os.path.abspath(args.corpus)
    names = sorted(name for name in os.listdir(corpus) if name.endswith(".md"))
    if args.only:
        names = [name for name in names if name.startswith(tuple(args.only))]
    with tempfile.TemporaryDirectory() as home:
        new = sweep(os.path.abspath(args.new), corpus, names, args.jobs, home)
        base = sweep(os.path.abspath(args.base), corpus, names, args.jobs, home) if args.base else {}
    crashed = sorted((key, got) for key, got in new.items() if got[0] in CRASHES)
    differ = sorted(key for key, got in new.items() if key in base and base[key][:2] != got[:2])
    fixed = sorted(key for key, was in base.items() if was[0] in CRASHES and new[key][0] not in CRASHES)
    print("renders: %d (%d files, %d modes)" % (len(new), len(names), len(MODES)))
    print("crashed or ran out of time: %d" % len(crashed))
    for key, got in crashed[:40]:
        print("   %-8s %s  %s" % (got[0], key, got[2]))
    if args.base:
        print("output differs from the base: %d" % len(differ))
        for key in differ[:60]:
            print("   differs  %s  exit %s -> %s" % (key, base[key][0], new[key][0]))
        print("crashed with the base, not now: %d" % len(fixed))
    sys.exit(1 if crashed or differ else 0)


main()
