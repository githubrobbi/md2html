#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of the translation review tools in scripts/i18n, run by the
# gate `gate-tests`: words.py reads every language's words file with
# English's paths and writes a string back, a Template's slots kept or the
# sentence refused (on copies of the files); review_site.py writes a
# review page per language, who the reviewer is asked first, then every
# string (those the last review changed first), the seven terms (each with
# our guess, the least sure first), every chapter and the rest each a
# question of its own with its state and its Send, and the index linking
# them; review_answers.py takes each reviewer's newest answer, leaves a
# withdrawn one out, and gives names only when asked, never how to reach a
# reviewer. Exit 1 when a case fails.

set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
failed=0
count=0

# case NAME COMMAND...: one case, passed when COMMAND exits 0.
case_() {
  local name="$1" out
  shift
  count=$((count + 1))
  if out="$("$@" 2>&1)"; then
    printf '  ✓ %s\n' "$name"
  else
    printf '  ✗ %s\n' "$name"
    printf '%s\n' "$out" | sed 's/^/      /'
    failed=$((failed + 1))
  fi
}

case_ "every language's words file has English's strings, each read back as written" \
  python3 "$here/words.py" --check

cp -R "$here/../../crates/md2html/src/config/words" "$work/words"
case_ "a string and a Template's sentence are written back, slots kept; a lost slot is refused" \
  python3 -I -c '
import sys
sys.path.insert(0, sys.argv[1])
from pathlib import Path
import words
words.WORDS = Path(sys.argv[2])
words.write("fr", "skip", "Aller au \"texte\"")
words.write("fr", "lang.cannot", "Traduire en {language} : impossible ici.")
now = words.read("fr")
assert now["skip"] == "Aller au \"texte\"", now["skip"]
assert now["lang.cannot"] == "Traduire en {language} : impossible ici.", now["lang.cannot"]
assert "lead: \"Traduire en \"" in (words.WORDS / "fr.rs").read_text()
try:
    words.write("fr", "lang.cannot", "no slot")
    raise SystemExit("a sentence without its slot was written")
except ValueError:
    pass
try:
    words.write("fr", "no.such", "x")
    raise SystemExit("a path the file lacks was written")
except ValueError:
    pass
assert words.unescape(words.escape("a b \"c\" \\d")) == "a b \"c\" \\d"
' "$here" "$work/words"

# A stand-in for md2html: makes the folder -o names, as rendering would.
cat > "$work/md2html" <<'EOF'
#!/bin/sh
while [ $# -gt 0 ]; do
  [ "$1" = "-o" ] && mkdir -p "$2"
  shift
done
EOF
chmod +x "$work/md2html"
case_ "the site has the index and a review per language: every string, term and chapter a question of its own, with its state and Send" \
  python3 -I -c '
import json, re, subprocess, sys
sys.path.insert(0, sys.argv[1])
from pathlib import Path
import review_site, words
out = Path(sys.argv[2]) / "site"
subprocess.run([sys.executable, sys.argv[1] + "/review_site.py", str(out), "--md2html", sys.argv[3], "--sitekey", "KEY", "--before", "HEAD"], check=True, capture_output=True)
index = (out / "index.html").read_text()
assert (out / "assets/review.js").is_file() and (out / "assets/review.css").is_file()
english = words.read("en")
for code in review_site.LANGUAGES:
    assert f"href=\"{code}/\"" in index, code
    page = (out / code / "index.html").read_text()
    data = json.loads(re.search(r"<script type=\"application/json\" id=\"data\">(.*?)</script>", page, re.S).group(1).replace("<\\/", "</"))
    assert data["sitekey"] == "KEY", code
    assert [s["path"] for s in data["strings"]] == list(english), code
    blocks = re.findall(r"<section class=\"q st-open\" data-kind=\"(\w+)\" data-item=\"([^\"]+)\"", page)
    kinds = [kind for kind, _ in blocks]
    assert kinds.count("term") == 7 and kinds.count("string") == len(english), code
    assert kinds.count("chapter") == len(data["chapters"]) > 10 and kinds.count("general") == 1, code
    assert page.count("class=\"send\"") == len(blocks) == page.count("class=\"q-state\""), code
    assert len(set(blocks)) == len(blocks), code
    assert page.count("class=\"guess\"") == 7 and page.count("Our guess:") == 7, code
    shown = re.findall(r"class=\"sure c-(\w+)\"", page)
    assert shown == sorted(shown, key=["unsure", "likely", "sure"].index), (code, shown)
    changed = sum(1 for s in data["strings"] if s["before"])
    assert (f"check these first ({changed})" in page) == (changed > 0), code
    assert page.index("id=\"me\"") < page.index("class=\"q "), code
    assert "../assets/review.js" in page and f"capabilities.{code}.html" in page, code
' "$here" "$work" "$work/md2html"

# Two reviewers of French: the first changes their mind on a term (the
# newer answer counts) and withdraws one string; the second keeps one.
cat > "$work/answers.json" <<'EOF'
[{"results": [
  {"id": 1, "received": "2026-11-26T10:00:00Z", "client": "a", "lang": "fr", "kind": "term", "item": "outline", "answer": "{\"choice\":\"current\"}", "reviewer": "{\"name\":\"Camille\",\"contact\":\"camille@example.org\",\"level\":\"native\"}"},
  {"id": 2, "received": "2026-11-26T10:01:00Z", "client": "a", "lang": "fr", "kind": "term", "item": "outline", "answer": "{\"choice\":\"other\",\"other\":\"Sommaire\",\"note\":\"plus courant\"}", "reviewer": "{\"name\":\"Camille\",\"contact\":\"camille@example.org\",\"level\":\"native\"}"},
  {"id": 3, "received": "2026-11-26T10:02:00Z", "client": "a", "lang": "fr", "kind": "string", "item": "lang.cannot", "answer": "{\"verdict\":\"change\",\"text\":\"Traduction en {language} impossible.\"}", "reviewer": "{\"name\":\"Camille\",\"level\":\"native\"}"},
  {"id": 4, "received": "2026-11-26T10:03:00Z", "client": "a", "lang": "fr", "kind": "string", "item": "skip", "answer": "{\"verdict\":\"change\",\"text\":\"Aller au texte\"}", "reviewer": "{\"name\":\"Camille\",\"level\":\"native\"}"},
  {"id": 5, "received": "2026-11-26T10:04:00Z", "client": "a", "lang": "fr", "kind": "string", "item": "skip", "answer": null, "reviewer": "{\"name\":\"Camille\",\"level\":\"native\"}"},
  {"id": 6, "received": "2026-11-26T10:05:00Z", "client": "a", "lang": "fr", "kind": "chapter", "item": "ch3", "answer": "{\"text\":\"une coquille\"}", "reviewer": "{\"name\":\"Camille\",\"level\":\"native\"}"},
  {"id": 7, "received": "2026-11-26T10:06:00Z", "client": "a", "lang": "fr", "kind": "general", "item": "all", "answer": "{\"text\":\"Bravo\"}", "reviewer": "{\"name\":\"Camille\",\"level\":\"native\"}"},
  {"id": 8, "received": "2026-11-26T11:00:00Z", "client": "b", "lang": "fr", "kind": "string", "item": "lang.cannot", "answer": "{\"verdict\":\"ok\"}", "reviewer": "{\"level\":\"fluent\"}"}
]}]
EOF
case_ "the report takes each reviewer's newest answer, leaves a withdrawn one out, and names reviewers only when asked" \
  python3 -I -c '
import subprocess, sys
script = sys.argv[1] + "/review_answers.py"
plain = subprocess.run([sys.executable, script, sys.argv[2]], check=True, capture_output=True, text=True).stdout
named = subprocess.run([sys.executable, script, sys.argv[2], "--names"], check=True, capture_output=True, text=True).stdout
assert "2 reviewers" in plain and "1 first language" in plain and "1 fluent" in plain, plain
assert "“Sommaire” (“plus courant”) [r1]" in plain and "“plan” [r1]" not in plain, plain
assert "\x60lang.cannot\x60" in plain and "kept by 1" in plain and "“Traduction en {language} impossible.” [r1]" in plain, plain
assert "Aller au texte" not in plain and "\x60skip\x60" not in plain, plain
assert "[r1]: “une coquille”" in plain and "ch3 [r1]" not in plain and "in general [r1]: “Bravo”" in plain, plain
assert "Camille" not in plain and "Names: Camille" in named, named
assert "example.org" not in plain + named
' "$here" "$work/answers.json"

if (( failed > 0 )); then
  printf '❌ %d of %d i18n cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ %d i18n cases passed\n' "$count"
