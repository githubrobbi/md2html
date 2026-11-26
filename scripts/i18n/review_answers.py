#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The answers native speakers sent from the review site, as one report.

`review_answers.py ANSWERS [--names]` reads ANSWERS, the export of the
review site's database (`wrangler d1 execute md2html-review --remote
--json --command "SELECT * FROM answer"`, a JSON list whose `results` hold
one row per answer sent: `id`, `received`, `client`, `lang`, `kind`,
`item`, `answer`, `reviewer`). A reviewer is a browser (`client`, a random
id it made); the newest row of a reviewer for a question is their answer,
and one whose answer is null was withdrawn. It writes Markdown to standard
output, language by language: how many reviewers and how well they know
the language; for each coined term, which word each chose and why; each
interface string some reviewer would change, with the wording now and each
proposal; the comments on the chapters and in general. `--names` adds the
names reviewers gave, to thank them; how to reach them is never printed.

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


def answers(export):
    """Each reviewer's answers, per language: {lang: {client: {"reviewer":
    dict, "answers": {(kind, item): answer}}}}, the newest row of each
    question kept and a withdrawn one left out."""
    rows = sorted((row for block in export for row in block.get("results", [])), key=lambda row: row["id"])
    out = defaultdict(dict)
    for row in rows:
        who = out[row["lang"]].setdefault(row["client"], {"reviewer": {}, "answers": {}})
        who["reviewer"] = json.loads(row["reviewer"] or "{}") or who["reviewer"]
        key = (row["kind"], row["item"])
        if row["answer"] is None:
            who["answers"].pop(key, None)
        else:
            who["answers"][key] = json.loads(row["answer"])
    return out


def quoted(text):
    """`text` on one line, quoted."""
    return "“" + " ".join(str(text).split()) + "”"


def report(found, names):
    """The Markdown report of the answers `found`."""
    terms = json.loads((words.ROOT / "scripts/i18n/terms.json").read_text(encoding="utf-8"))
    english = words.read("en")
    total = sum(len(people) for people in found.values())
    langs = sum(1 for people in found.values() if people)
    lines = [f"# Translation reviews: {total} {'reviewer' if total == 1 else 'reviewers'} in "
             f"{langs} {'language' if langs == 1 else 'languages'}", ""]
    for code in review_site.LANGUAGES:
        people = list(found.get(code, {}).values())
        if not people:
            continue
        tags = {id(person): f"r{n}" for n, person in enumerate(people, 1)}
        own, english_name, _ = review_site.LANGUAGES[code]
        levels = Counter(LEVELS.get(p["reviewer"].get("level", ""), "not said") for p in people)
        lines += [f"## {own} ({english_name}, `{code}`): {len(people)} "
                  f"{'reviewer' if len(people) == 1 else 'reviewers'}", "",
                  "Reviewers: " + ", ".join(f"{n} {level}" for level, n in levels.items()), ""]
        if names:
            given = sorted({p["reviewer"].get("name", "").strip() for p in people} - {""})
            lines += ["Names: " + (", ".join(given) or "none given"), ""]
        lines += ["### Terms", ""]
        now = terms["languages"][code]
        for term in terms["terms"]:
            entry = now[term["id"]]
            alternatives = {f"alt{n}": alt["text"] for n, alt in enumerate(entry["choices"])}
            picks = []
            for person in people:
                answer = person["answers"].get(("term", term["id"]))
                if not answer or not answer.get("choice"):
                    continue
                choice = answer["choice"]
                word = {"current": entry["current"], "previous": entry.get("previous"),
                        "other": answer.get("other", "")}.get(choice, alternatives.get(choice, choice))
                note = f" ({quoted(answer['note'])})" if answer.get("note", "").strip() else ""
                picks.append(f"{quoted(word)}{note} [{tags[id(person)]}]")
            if picks:
                lines.append(f"- **{term['en']}** (now {quoted(entry['current'])}): " + "; ".join(picks))
        lines += ["", "### Strings someone would change", ""]
        current = words.read(code)
        kept = Counter()
        changes = defaultdict(list)
        for person in people:
            for (kind, path), answer in person["answers"].items():
                if kind != "string":
                    continue
                verdict = answer.get("verdict")
                if verdict == "ok":
                    kept[path] += 1
                elif verdict in ("change", "before"):
                    text = answer.get("text", "") if verdict == "change" else "(the wording before)"
                    changes[path].append(f"{quoted(text)} [{tags[id(person)]}]")
        for path in current:
            if path in changes:
                lines.append(f"- `{path}`: English {quoted(english[path])}; now {quoted(current[path])}; "
                             f"kept by {kept[path]}; proposed: " + "; ".join(changes[path]))
        for path in sorted(set(changes) - set(current)):
            lines.append(f"- `{path}` (not a string of this language now): " + "; ".join(changes[path]))
        lines += ["", f"Answers keeping a string as it is: {sum(kept.values())}.", "", "### Chapters and the rest", ""]
        page = (words.ROOT / f"docs/capabilities.{code}.md").read_text(encoding="utf-8")
        titles = {f"ch{n}": title for n, title in enumerate(review_site.chapters(page), 1)}
        for person in people:
            for (kind, item), answer in person["answers"].items():
                if kind == "chapter" and answer.get("text", "").strip():
                    lines.append(f"- {titles.get(item, item)} [{tags[id(person)]}]: {quoted(answer['text'])}")
                elif kind == "general" and answer.get("text", "").strip():
                    where = "on the page" if item == "page" else "in general"
                    lines.append(f"- {where} [{tags[id(person)]}]: {quoted(answer['text'])}")
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
        found = answers(export)
    except (OSError, ValueError, KeyError) as err:
        sys.exit(f"review_answers: cannot read {files[0]}: {err}")
    sys.stdout.write(report(found, names))


if __name__ == "__main__":
    main()
