#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC.
#
# The tests of the translation review tools in scripts/i18n, run by the
# gate `gate-tests`: words.py reads every language's words file with
# English's paths and writes a string back, a Template's slots kept or the
# sentence refused (on copies of the files); review_site.py writes a
# review page per language holding every string, the seven terms and every
# chapter, and the index linking them; review_answers.py reports what
# reviews chose and proposed, and gives names only when asked, never how
# to reach a reviewer. Exit 1 when a case fails.

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
case_ "the site has the index and a review per language, each with every string, term and chapter" \
  python3 -I -c '
import json, re, subprocess, sys
sys.path.insert(0, sys.argv[1])
from pathlib import Path
import review_site, words
out = Path(sys.argv[2]) / "site"
subprocess.run([sys.executable, sys.argv[1] + "/review_site.py", str(out), "--md2html", sys.argv[3], "--sitekey", "KEY"], check=True, capture_output=True)
index = (out / "index.html").read_text()
english = words.read("en")
for code in review_site.LANGUAGES:
    assert f"href=\"{code}/\"" in index, code
    page = (out / code / "index.html").read_text()
    data = json.loads(re.search(r"<script type=\"application/json\" id=\"data\">(.*?)</script>", page, re.S).group(1).replace("<\\/", "</"))
    assert [s["path"] for s in data["strings"]] == list(english), code
    assert len(data["terms"]) == 7 and all(t["current"] for t in data["terms"]), code
    assert page.count("name=\"chapter|") == len(data["chapters"]) > 10, code
    assert "data-sitekey=\"KEY\"" in page and f"capabilities.{code}.html" in page, code
' "$here" "$work" "$work/md2html"

cat > "$work/answers.json" <<'EOF'
[{"results": [
  {"id": 1, "received": "2026-11-26T10:00:00Z", "lang": "fr", "review": "{\"lang\":\"fr\",\"reviewer\":{\"name\":\"Camille\",\"contact\":\"camille@example.org\",\"level\":\"native\"},\"terms\":{\"outline\":{\"choice\":\"other\",\"other\":\"Sommaire\",\"note\":\"plus courant\"}},\"strings\":{\"skip\":{\"verdict\":\"ok\"},\"lang.cannot\":{\"verdict\":\"change\",\"text\":\"Traduction en {language} impossible.\"}},\"chapters\":{\"ch3\":\"une coquille\"},\"general\":\"Bravo\"}"},
  {"id": 2, "received": "2026-11-26T11:00:00Z", "lang": "fr", "review": "{\"lang\":\"fr\",\"reviewer\":{\"level\":\"fluent\"},\"terms\":{\"outline\":{\"choice\":\"current\"}},\"strings\":{\"skip\":{\"verdict\":\"ok\"}}}"}
]}]
EOF
case_ "the report counts the choices and proposals, and names reviewers only when asked" \
  python3 -I -c '
import subprocess, sys
script = sys.argv[1] + "/review_answers.py"
plain = subprocess.run([sys.executable, script, sys.argv[2]], check=True, capture_output=True, text=True).stdout
named = subprocess.run([sys.executable, script, sys.argv[2], "--names"], check=True, capture_output=True, text=True).stdout
assert "2 reviews" in plain and "1 first language" in plain and "1 fluent" in plain, plain
assert "“Sommaire” (“plus courant”) [#1]" in plain, plain
assert "\x60lang.cannot\x60" in plain and "“Traduction en {language} impossible.” [#1]" in plain, plain
assert "kept by 0" in plain and "Answers keeping a string as it is: 2." in plain, plain
assert "[#1]: “une coquille”" in plain and "ch3 [#1]" not in plain and "in general [#1]: “Bravo”" in plain, plain
assert "Camille" not in plain and "Names: Camille" in named, named
assert "example.org" not in plain + named
' "$here" "$work/answers.json"

if (( failed > 0 )); then
  printf '❌ %d of %d i18n cases failed\n' "$failed" "$count"
  exit 1
fi
printf '✅ %d i18n cases passed\n' "$count"
