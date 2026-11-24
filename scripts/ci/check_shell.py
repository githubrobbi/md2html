#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
"""Gate `shellcheck`: every shell script of the repository, read by shellcheck.

What is read:

- every file git tracks that is a shell script: a `.sh` file, or a file
  whose first line is a `#!` naming `sh` or `bash` (the git hooks);
- the body of every `just` recipe, as `just --dump --dump-format json`
  gives it: a recipe with a `#!` line is one script; the lines of any
  other recipe run one by one in the justfile's shell (`bash -euo
  pipefail -c`), and are read together as one bash script.  A `{{ ... }}`
  interpolation is read as one plain word.

Not read: `scripts/ci/md2html-gen-hooks/templates/`, which holds pieces of
the two git hooks, not scripts; the generated hooks that put them together
(`scripts/hooks/_lint_fast.sh`, `_lint_pre_push.sh`) are read whole.
GitHub workflows' `run:` blocks are gate `actionlint`'s.

Refused besides shellcheck's findings: a recipe line that starts with `-`,
which tells `just` to go on when the line fails, and a `# shellcheck
disable=` directive anywhere: a finding is fixed, not silenced.

Exit 0 clean, 1 a finding, 2 the check cannot run (a tool missing, nothing
found to read).
"""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

TEMPLATES = "scripts/ci/md2html-gen-hooks/templates/"
SHEBANG = re.compile(rb"^#!\s*(\S*/)?(env\s+)?(sh|bash)(\s|$)")
DISABLE = re.compile(r"#\s*shellcheck\s+disable\s*=")
JUST_SHELL = ["bash", "-euo", "pipefail", "-c"]


def fail(message: str) -> None:
    """Stop with exit 2: the check itself cannot run."""
    print(f"ERROR: {message}", file=sys.stderr)
    sys.exit(2)


def run(argv: list[str], stdin: str | None = None) -> subprocess.CompletedProcess[str]:
    """Run a command, its output captured as text."""
    return subprocess.run(argv, input=stdin, capture_output=True, text=True, check=False)


def shell_files(root: Path) -> list[str]:
    """The tracked shell scripts, templates excepted."""
    listed = run(["git", "-C", str(root), "ls-files", "-z"])
    if listed.returncode != 0:
        fail(f"git ls-files failed: {listed.stderr.strip()}")
    found = []
    for name in listed.stdout.split("\0"):
        if not name or name.startswith(TEMPLATES):
            continue
        path = root / name
        if not path.is_file():
            continue
        if name.endswith(".sh"):
            found.append(name)
            continue
        with path.open("rb") as handle:
            first = handle.readline()
        if SHEBANG.match(first):
            found.append(name)
    return found


def render(line: list[object]) -> str:
    """One recipe line as text, each interpolation one plain word."""
    return "".join(part if isinstance(part, str) else "__just_value__" for part in line)


def recipe_scripts(root: Path) -> tuple[list[tuple[str, str]], list[str]]:
    """(name, script) for every recipe, and the lines that ignore a failure."""
    dumped = run(["just", "--justfile", str(root / "justfile"), "--dump", "--dump-format", "json"])
    if dumped.returncode != 0:
        fail(f"just --dump failed: {dumped.stderr.strip()}")
    document = json.loads(dumped.stdout)
    shell = document["settings"]["shell"]
    if [shell["command"], *shell["arguments"]] != JUST_SHELL:
        fail(f"the justfile's shell is {shell}, this gate reads recipes as {JUST_SHELL}")
    scripts = []
    ignoring = []
    for name, recipe in sorted(document["recipes"].items()):
        lines = [render(line) for line in recipe["body"]]
        if not lines:
            continue
        if recipe["shebang"]:
            if not SHEBANG.match(lines[0].encode()):
                fail(f"recipe `{name}` runs `{lines[0]}`, not bash: teach this gate to read it")
            scripts.append((name, "\n".join(lines) + "\n"))
            continue
        body = []
        for line in lines:
            if line.startswith("-") or line.startswith("@-"):
                ignoring.append(f"recipe `{name}`: `{line}`")
            body.append(line.lstrip("@-"))
        scripts.append((name, "#!/usr/bin/env bash\nset -euo pipefail\n" + "\n".join(body) + "\n"))
    return scripts, ignoring


def main() -> int:
    """Read everything, report every finding, exit as the module says."""
    for tool in ("git", "just", "shellcheck"):
        if shutil.which(tool) is None:
            fail(f"`{tool}` not found - install it (just install-dev-tools)")
    top = run(["git", "rev-parse", "--show-toplevel"])
    if top.returncode != 0:
        fail("not inside the md2html git checkout")
    root = Path(top.stdout.strip())

    findings = 0
    files = shell_files(root)
    scripts, ignoring = recipe_scripts(root)
    if not files or not scripts:
        fail(f"nothing to read ({len(files)} scripts, {len(scripts)} recipes)")

    for name in files:
        text = (root / name).read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), start=1):
            if DISABLE.search(line):
                print(f"{name}:{number}: a shellcheck directive silences a finding; fix the code instead")
                findings += 1
    checked = run(["shellcheck", "--external-sources", *files], stdin=None)
    if checked.returncode not in (0, 1):
        fail(f"shellcheck failed: {checked.stderr.strip()}")
    if checked.returncode == 1:
        print(checked.stdout, end="")
        findings += 1

    for name, script in scripts:
        if DISABLE.search(script):
            print(f"recipe `{name}`: a shellcheck directive silences a finding; fix the code instead")
            findings += 1
        result = run(["shellcheck", "--shell=bash", "-"], stdin=script)
        if result.returncode not in (0, 1):
            fail(f"shellcheck failed on recipe `{name}`: {result.stderr.strip()}")
        if result.returncode == 1:
            print(f"── recipe `{name}` (line numbers count from its first body line)")
            print(result.stdout, end="")
            findings += 1

    for line in ignoring:
        print(f"{line}: a `-` line goes on when it fails; handle the failure instead")
        findings += 1

    if findings:
        print(f"❌ shellcheck: findings in {findings} place(s)")
        return 1
    print(f"✅ shellcheck: {len(files)} scripts and {len(scripts)} recipes, nothing to fix")
    return 0


if __name__ == "__main__":
    sys.exit(main())
