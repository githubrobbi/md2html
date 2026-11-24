# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""Writes the corpus the differential test renders: 8,298 Markdown files.

The repository's own Markdown, the 652 examples of the CommonMark
specification, and seeded mutations of both: windows of them cut, copied,
reversed, a byte changed or a token from the grammar's edge put in (marks,
brackets, entities, raw HTML, maths, diagram syntax, front matter, broken
UTF-8), and blocks that are languages of their own (diagrams, code,
maths, tables) mutated on their own. One file in forty keeps its broken
UTF-8; the rest are made valid, so they reach the renderer.

The seed is fixed, so the corpus is a function of the repository: a run
compares two programs on the same files, wherever it runs.

usage: python3 scripts/differential/corpus.py OUT

It needs Python 3.8 or later and nothing outside the standard library.
"""

import json
import os
import random
import re
import shutil
import subprocess
import sys

SEED = 20261123

# Diagrams that use more of the syntax than the repository's own.
EXTRA_MERMAID = [
    b"```mermaid\nflowchart TD\n  A[Start] --> B{Choice}\n  B -->|yes| C(Round)\n  B -->|no| D((Circle))\n  C --> E[[Sub]]\n  D --> E\n  E --> F[(Db)]\n  F -.-> G>Flag]\n  G ==> H{{Hex}}\n  H --o I[/Lean/]\n  I --x J[\\Back\\]\n  J <--> A\n```\n",
    b"```mermaid\nflowchart LR\n  subgraph one [First group]\n    a --> b\n    b --> c\n  end\n  subgraph two\n    direction TB\n    d --> e\n  end\n  c --> d\n  a -- text --> e\n  e ~~~ a\n```\n",
    b"```mermaid\ngraph BT\n  x1([Stadium]) --> x2[/Trap\\]\n  x2 --> x3[\\Down/]\n  x3 --> x4(((Double)))\n  x4 --> x1\n  x1 & x2 --> x5\n```\n",
    b"```mermaid\nsequenceDiagram\n  participant A as Alice\n  actor B as Bob\n  A->>B: Hello\n  activate B\n  B-->>A: Hi\n  deactivate B\n  Note over A,B: A note\n  loop Every minute\n    A->>B: ping\n    alt ok\n      B-->>A: pong\n    else late\n      B--xA: timeout\n    end\n  end\n  opt maybe\n    A-)B: async\n  end\n  par one\n    A->>B: x\n  and two\n    B->>A: y\n  end\n  Note right of B: side\n  Note left of A: other\n  A->>A: self\n```\n",
    b"```mermaid\nflowchart RL\n  A[\"A label<br/>with a break\"] --> B[\"\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e \xc3\xa9\xc3\xa8\"]\n  B --> C\n  C --> A\n  style A fill:#f9f\n  classDef k fill:#333\n  class B k\n  click A href \"https://example.com\"\n```\n",
    b"```mermaid\npie title Pets\n \"Dogs\" : 3\n \"Cats\" : 4\n```\n",
]

# What a mutation puts in: the edges of the grammar.
TOKENS = [
    b"*", b"**", b"_", b"__", b"`", b"```", b"~~", b"[", b"]", b"](", b")", b"![", b"<", b">",
    b"&", b"&amp;", b"&#x41;", b"&#0;", b"%", b"%a", b"%C3%A9", b"\\", b"$", b"$$", b"|", b"\n",
    b"\n\n", b"\t", b"    ", b"> ", b"- ", b"1. ", b"# ", b"###### ", b"---\n", b"===\n",
    b"\xc3\xa9", b"\xe6\x97\xa5", b"\xf0\x9f\x98\x80", b"\xe2\x80\x8b", b"\r\n", b"\r", b"\x00",
    b"\xff", b"\xc3", b"<script>", b"</script>", b"<img src=\"%a\xc3\xa9.png\">",
    b"<a href=\"javascript:x\">", b"<!--", b"-->", b"<![CDATA[", b"<?x", b"<div style=\"x:y\">",
    b"</div>", b"[^1]", b"[^1]: note\n", b"[x]: /url \"t\"\n", b"-->", b"-.->", b"==>", b"--o",
    b"--x", b"<-->", b"subgraph ", b"end\n", b"{{", b"}}", b"((", b"))", b"[[", b"]]", b"[(",
    b")]", b"|a|", b":::k", b"->>", b"-->>", b"Note over ", b"loop ", b"alt ", b"else ",
    b"participant ", b"activate ", b"\\frac{", b"}{", b"^", b"_{", b"\\sqrt[", b"\\begin{",
    b"\\end{", b"> [!NOTE]\n", b"> [!WARNING]\n", b":---:", b"| a | b |\n|---|---|\n", b"- [ ] ",
    b"- [x] ", b"*[HTML]: x\n", b"{#id}", b"==", b"++", b"^^", b"https://a.b/c?d=e&f",
    b"<https://x.y>", b"www.a.b", b"a@b.c", b"---\ntitle: T\nlang: de\n---\n",
    b"---\ntoc: no\nlarge: yes\n---\n",
    b"```rust\nfn f<'a>(x: &'a str) -> u8 { b'x' }\n```\n",
    b"```py\ndef f(x):\n    return f\"{x!r}\" # c\n```\n",
    b"```js\nconst r = /a\\/b/g; `t${x}`\n```\n", b"```sh\necho \"$X\" <<EOF\nEOF\n```\n",
    b"```html\n<a b=\"c\">&amp;</a><!-- x -->\n```\n", b"```json\n{\"a\": [1, 2.5e3, null]}\n```\n",
    b"```diff\n- a\n+ b\n@@ x @@\n```\n", b"```toml\n[a]\nb = \"c\" # d\n```\n",
    b"```yaml\na: b\n- c: 'd'\n```\n", b"```css\na > b { c: #fff; }\n```\n",
    b"```sql\nSELECT 'a' -- c\n```\n",
]


class Corpus:
    """The files written so far, and the generator that chooses."""

    def __init__(self, out):
        self.out = out
        self.rng = random.Random(SEED)
        self.count = 0

    def mutate(self, data, steps):
        """`data` changed `steps` times."""
        data = bytearray(data)
        for _ in range(steps):
            op = self.rng.randrange(7)
            at = self.rng.randrange(len(data) + 1)
            if op == 0 and data:
                end = min(len(data), at + self.rng.randrange(1, 40))
                del data[at:end]
            elif op == 1 and data:
                end = min(len(data), at + self.rng.randrange(1, 60))
                data[at:at] = data[at:end]
            elif op == 2 and data:
                data[min(at, len(data) - 1)] = self.rng.randrange(256)
            elif op == 3 and len(data) > 4:
                a, b = sorted(self.rng.randrange(len(data)) for _ in range(2))
                data[a:b] = data[a:b][::-1] if b - a < 30 else data[a:b]
            else:
                data[at:at] = self.rng.choice(TOKENS) * self.rng.choice([1, 1, 1, 2, 3, 50])
        return bytes(data)

    def window(self, data, size):
        """A stretch of `data` at most `size` bytes long."""
        if len(data) <= size:
            return data
        at = self.rng.randrange(len(data) - size)
        return data[at:at + size]

    def emit(self, prefix, data):
        """Writes one generated file; most are made valid UTF-8 first."""
        if self.rng.randrange(40):
            data = data.decode("utf-8", "replace").encode()
        self.write("%s%05d.md" % (prefix, self.count), data)
        self.count += 1

    def write(self, name, data):
        """Writes `data` as `name`."""
        with open(os.path.join(self.out, name), "wb") as file:
            file.write(data)


def blocks_of(texts):
    """The diagrams, code, maths and tables of `texts`, to mutate alone."""
    blocks = {"mermaid": [], "code": [], "math": [], "table": []}
    for data in texts:
        text = data.decode("utf-8", "replace")
        for found in re.finditer(r"^```(\w+)[^\n]*\n(.*?)^```", text, re.S | re.M):
            kind = "mermaid" if found.group(1) == "mermaid" else "code"
            blocks[kind].append(found.group(0).encode())
        for found in re.finditer(r"\$\$.*?\$\$|\$[^$\n]{2,80}\$", text, re.S):
            blocks["math"].append(found.group(0).encode())
        for found in re.finditer(r"(?:^\|[^\n]*\n){3,}", text, re.M):
            blocks["table"].append(found.group(0).encode())
    blocks["mermaid"] += EXTRA_MERMAID
    return blocks


def main():
    """Writes the corpus into the folder named on the command line."""
    if len(sys.argv) != 2:
        sys.exit("usage: corpus.py OUT")
    repo = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, check=True, text=True
    ).stdout.strip()
    out = sys.argv[1]
    if os.path.isdir(out):
        shutil.rmtree(out)
    os.makedirs(out)
    corpus = Corpus(out)
    listed = subprocess.run(
        ["git", "ls-files", "-z", "*.md"], cwd=repo, capture_output=True, check=True
    ).stdout.split(b"\0")
    # The repository's Markdown as of this tree, the planning notes under
    # docs/dev (ignored by git) left out by asking git.
    own = []
    for at, name in enumerate(n for n in listed if n):
        with open(os.path.join(repo.encode(), name), "rb") as file:
            data = file.read()
        corpus.write("r%03d.md" % at, data)
        own.append(data)
    spec_path = os.path.join(repo, "crates/md2html/tests/commonmark/spec-0.31.2.json")
    with open(spec_path, encoding="utf-8") as file:
        spec = json.load(file)
    seeds = list(own)
    for example in spec:
        data = example["markdown"].encode()
        corpus.write("s%03d.md" % example["example"], data)
        seeds.append(data)
    for _ in range(3000):
        seed = corpus.rng.choice(seeds)
        stretch = corpus.window(seed, corpus.rng.choice([200, 800, 3000, 6000]))
        corpus.emit("f", corpus.mutate(stretch, corpus.rng.choice([1, 2, 4, 8, 20])))
    blocks = blocks_of(own)
    for kind, count in (("mermaid", 2500), ("code", 800), ("math", 600), ("table", 400)):
        pool = blocks[kind]
        for _ in range(count if pool else 0):
            block = corpus.mutate(corpus.rng.choice(pool), corpus.rng.choice([1, 1, 2, 3, 6, 12]))
            corpus.emit(kind[0], b"# T\n\ntext\n\n" + block + b"\n\nafter\n")
    for _ in range(300):
        first, second = corpus.rng.choice(seeds), corpus.rng.choice(seeds)
        joined = corpus.window(first, 1500) + corpus.rng.choice(TOKENS) + corpus.window(second, 1500)
        corpus.emit("x", joined)
    print("corpus: %d files in %s" % (len(os.listdir(out)), out))


main()
