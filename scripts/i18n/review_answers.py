#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The reviews native speakers sent from the review site, as one report.

`review_answers.py ANSWERS [--names]` reads ANSWERS, the export of the
review site's database (`wrangler d1 execute md2html-review --remote
--json --command "SELECT * FROM answers"`, a JSON list whose `results`
hold rows of `id`, `received`, `lang` and `review`), and writes Markdown
to standard output, language by language: how many reviews and from whom
(first language, fluent, learning); for each coined term, which word each
review chose and why; each interface string some review would change,
with the wording now and each proposal; and the comments on the chapters
and in general. `--names` adds the names reviewers gave, to thank them;
how to reach them is never printed.

A proposal is put into the words file with `words.py --set CODE PATH
TEXT`, after reading it. Exit 1 when ANSWERS cannot be read.
"""

import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

# The script's own folder is first on the module path when it runs, so
# its sibling modules are found.
import review_site
import words

LEVELS = {"native": "first language", "fluent": "fluent", "learning": "learning", "": "not said"}


def rows(export):
    """The rows of a wrangler export, each with its review parsed."""
    out = []
    for block in export:
        for row in block.get("results", []):
            review = json.loads(row["review"])
            out.append({"id": row["id"], "received": row["received"], "lang": row["lang"], "review": review})
    return out


def quoted(text):
    """`text` on one line, quoted."""
    return "“" + " ".join(str(text).split()) + "”"


def report(found, names):
    """The Markdown report of `found` reviews."""
    terms = json.loads((words.ROOT / "scripts/i18n/terms.json").read_text(encoding="utf-8"))
    english = words.read("en")
    by_lang = defaultdict(list)
    for row in found:
        by_lang[row["lang"]].append(row)
    lines = [f"# Translation reviews: {len(found)} in {len(by_lang)} "
             + ("language" if len(by_lang) == 1 else "languages"), ""]
    for code in review_site.LANGUAGES:
        reviews = by_lang.get(code)
        if not reviews:
            continue
        own, english_name, _ = review_site.LANGUAGES[code]
        levels = Counter(LEVELS.get(r["review"].get("reviewer", {}).get("level", ""), "not said") for r in reviews)
        plural = "review" if len(reviews) == 1 else "reviews"
        lines += [f"## {own} ({english_name}, `{code}`): {len(reviews)} {plural}", "",
                  "Reviewers: " + ", ".join(f"{n} {level}" for level, n in levels.items()), ""]
        if names:
            given = sorted({r["review"].get("reviewer", {}).get("name", "").strip() for r in reviews} - {""})
            lines += ["Names: " + (", ".join(given) or "none given"), ""]
        lines += ["### Terms", ""]
        now = terms["languages"][code]
        for term in terms["terms"]:
            entry = now[term["id"]]
            alternatives = {f"alt{n}": alt["text"] for n, alt in enumerate(entry["choices"])}
            picks = []
            for r in reviews:
                answer = r["review"].get("terms", {}).get(term["id"], {})
                choice = answer.get("choice")
                if not choice:
                    continue
                word = {"current": entry["current"], "previous": entry.get("previous"),
                        "other": answer.get("other", "")}.get(choice, alternatives.get(choice, choice))
                note = f" ({quoted(answer['note'])})" if answer.get("note", "").strip() else ""
                picks.append(f"{quoted(word)}{note} [#{r['id']}]")
            if picks:
                lines.append(f"- **{term['en']}** (now {quoted(entry['current'])}): " + "; ".join(picks))
        lines += ["", "### Strings someone would change", ""]
        current = words.read(code)
        kept = Counter()
        changes = defaultdict(list)
        for r in reviews:
            for path, answer in r["review"].get("strings", {}).items():
                verdict = answer.get("verdict")
                if verdict == "ok":
                    kept[path] += 1
                elif verdict in ("change", "before"):
                    text = answer.get("text", "") if verdict == "change" else "(the wording before)"
                    changes[path].append(f"{quoted(text)} [#{r['id']}]")
        for path in current:
            if path in changes:
                lines.append(f"- `{path}`: English {quoted(english[path])}; now {quoted(current[path])}; "
                             f"kept by {kept[path]}; proposed: " + "; ".join(changes[path]))
        unknown = sorted(set(changes) - set(current))
        for path in unknown:
            lines.append(f"- `{path}` (not a string of this language now): " + "; ".join(changes[path]))
        lines += ["", f"Answers keeping a string as it is: {sum(kept.values())}.", "", "### Chapters and the rest", ""]
        page = (words.ROOT / f"docs/capabilities.{code}.md").read_text(encoding="utf-8")
        titles = {f"ch{n}": title for n, title in enumerate(review_site.chapters(page), 1)}
        for r in reviews:
            for chapter, comment in r["review"].get("chapters", {}).items():
                lines.append(f"- {titles.get(chapter, chapter)} [#{r['id']}]: {quoted(comment)}")
            if r["review"].get("general", "").strip():
                lines.append(f"- in general [#{r['id']}]: {quoted(r['review']['general'])}")
        lines.append("")
    return "\n".join(lines)


def main():
    args = sys.argv[1:]
    names = "--names" in args
    files = [a for a in args if a != "--names"]
    if len(files) != 1:
        sys.exit("usage: review_answers.py ANSWERS [--names]")
    try:
        export = json.loads(Path(files[0]).read_text(encoding="utf-8"))
        found = rows(export)
    except (OSError, ValueError, KeyError) as err:
        sys.exit(f"review_answers: cannot read {files[0]}: {err}")
    sys.stdout.write(report(found, names))


if __name__ == "__main__":
    main()
