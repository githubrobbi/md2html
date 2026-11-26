#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The translation review site: one page per language for native speakers.

`review_site.py OUT [--md2html BIN] [--sitekey KEY] [--before REF]`
writes into OUT:

- `index.html`, the languages to choose from;
- `<code>/index.html`, a language's review, written in that language, and
  `<code>/en.html`, the same in English (review_page.py writes both): who
  the reviewer is, asked
  once at the top; then every question as a block of its own, with its
  state (not answered, not sent, sent) and its own Send, so a reviewer
  with a minute answers two and leaves: the seven coined terms as
  multiple choice (the wording now, the one before the last review, the
  alternatives of `scripts/i18n/terms.json` with our guess marked and how
  sure we are, or the reviewer's own), every interface string
  (`scripts/i18n/words.py`) under its English with what it is for, to keep
  or to change, and what reads wrong on the capabilities page; only the
  terms and strings we are unsure of are asked, the rest folded away;
- `assets/review.css` and `assets/review.js`, which run those pages;
- `pages/capabilities.html` and `pages/capabilities.<code>.html`, that
  page rendered by md2html (BIN, default `target/release/md2html`) in
  every language, side by side so its language switch links them.

Each answer is sent to the Worker that serves the site: `/api/session`
once a visit, with a Cloudflare Turnstile token (KEY is the widget's site
key, public), then `/api/answer` per question. The wording before is read
from git at REF (default the newest tag; a clone without tags names it),
so a reviewer can choose it back. `review_answers.py` reads the answers.
Exit 1 when a language's words or page cannot be read, or no tag is found
and none is named.
"""

import argparse
import json
import re
import subprocess
from pathlib import Path

# The script's own folder is first on the module path when it runs, so
# its sibling module is found.

import review_page
import words

ROOT = words.ROOT

# Each language's own name and writing direction.
LANGUAGES = {
    "de": ("Deutsch", "German", "ltr"),
    "es": ("Español", "Spanish", "ltr"),
    "fr": ("Français", "French", "ltr"),
    "he": ("עברית", "Hebrew", "rtl"),
    "id": ("Bahasa Indonesia", "Indonesian", "ltr"),
    "it": ("Italiano", "Italian", "ltr"),
    "ja": ("日本語", "Japanese", "ltr"),
    "ja-Hira": ("にほんご（ひらがな）", "Japanese, hiragana only", "ltr"),
    "ja-Latn": ("Nihongo (rōmaji)", "Japanese, rōmaji", "ltr"),
    "jv": ("Basa Jawa", "Javanese", "ltr"),
    "nds": ("Plattdüütsch", "Low German", "ltr"),
    "nl": ("Nederlands", "Dutch", "ltr"),
    "yi": ("ייִדיש", "Yiddish", "rtl"),
    "yue": ("粵語", "Cantonese", "ltr"),
    "zh-Hans": ("简体中文", "Chinese, simplified", "ltr"),
    "zh-Hant": ("繁體中文", "Chinese, traditional", "ltr"),
}

# What each string is for, in plain words, by path (a path's first
# matching prefix).
CONTEXT = [
    ("skip", "A link for keyboard and screen-reader users that jumps past the top bar to the text."),
    ("search_page", "The placeholder inside the search box."),
    ("search", "The search button in the top bar."),
    ("keys.step", "A hint under the search results: the arrow keys move through them."),
    ("keys.open", "A hint under the search results: Enter opens one."),
    ("keys.close", "A hint under the search results: Escape closes the search."),
    ("none", "Shown when a search finds nothing."),
    ("results", "The count of search results (more than one)."),
    ("result", "The count when the search finds exactly one result."),
    ("glossary_intro", "The line under the glossary chapter's title, explaining how its words work."),
    ("glossary_shared", "Added to that line when some terms come from a shared glossary; {address} is its link."),
    ("glossary_heading", "The heading an author writes for a glossary chapter; md2html recognises it."),
    ("glossary", "The title of the glossary chapter md2html adds at the end of a page."),
    ("back", "The link in a glossary entry that returns to the line the reader was reading."),
    ("all_terms", "A link that opens the full shared glossary."),
    ("keep_outline", "A link in the outline that keeps it open beside the text."),
    ("outline_tips.hide", "The tooltip of the outline button while the outline is shown."),
    ("outline_tips.show", "The tooltip of the outline button while the outline is hidden."),
    ("outline_tips.keys", "Added to those tooltips: the keyboard shortcut."),
    ("outline", "The name of the outline: the list of chapters at the side of the page."),
    ("notes", "The heading of the footnotes at the end of the page."),
    ("note_n", "The label of a footnote reference, for screen readers."),
    ("link_section", "The link beside a heading that points at that section."),
    ("copy.label", "The button on a code block that copies it."),
    ("copy.copied", "What that button says after copying."),
    ("callouts.note", "The label of a boxed note in the text."),
    ("callouts.tip", "The label of a boxed tip."),
    ("callouts.important", "The label of a boxed 'important' aside."),
    ("callouts.warning", "The label of a boxed warning."),
    ("callouts.caution", "The label of a boxed caution (stronger than a warning)."),
    ("footer", "The line at the foot of the page; {file} is the Markdown file's name."),
    ("months", "A month's name, as written in a date."),
    ("top", "The name of the page's opening in the search results."),
    ("figure.label", "The name of the enlarged view of a picture or diagram."),
    ("figure.close_key", "The tooltip of that view's close button."),
    ("figure.close", "The close button of the enlarged view."),
    ("figure.position", "Where the enlarged picture is among them: picture {this} of {count}."),
    ("figure.enlarge", "The button that enlarges a diagram."),
    ("size.group", "The name of the text size controls."),
    ("size.smaller.words", "The button that makes the text smaller (its tooltip adds the new size)."),
    ("size.larger.words", "The button that makes the text larger."),
    ("size.as_written", "The tooltip when the text is at its normal size."),
    ("size.scaled", "The tooltip when the text is larger or smaller; {percent} is the size."),
    ("size.keys", "Added to those tooltips: the keyboard shortcuts."),
    ("size.smallest", "Said when the text cannot get any smaller."),
    ("size.largest", "Said when the text cannot get any larger."),
    ("theme.button", "The button that switches between light, dark and the system's theme."),
    ("theme.tip", "Its tooltip: the theme {now}, then what the next clicks give."),
    ("theme.system_now", "The theme 'System' with what the system shows right now ({mode} is 'dark' or 'light')."),
    ("theme.system", "The theme that follows the computer's setting."),
    ("theme.light", "The light theme."),
    ("theme.dark", "The dark theme."),
    ("theme.dark_mode", "'dark', inside the sentence above."),
    ("theme.light_mode", "'light', inside the sentence above."),
    ("table", "The name of a wide table's scrolling area, for screen readers."),
    ("pages.previous", "The link to the previous page of a set of pages."),
    ("pages.next", "The link to the next page."),
    ("pages.label", "The name of those links together."),
    ("task.done", "A finished item of a task list, for screen readers."),
    ("task.open", "An item of a task list still to do."),
    ("guide_heading", "The heading an author writes for a 'how to read this' chapter; md2html recognises it."),
    ("contents_heading", "The heading an author writes for a table of contents; md2html recognises it."),
    ("lang.label", "The language menu's name."),
    ("lang.original", "Marks the original among the languages."),
    ("lang.notice", "The note on a translated page; {original} is the original's language."),
    ("lang.on_device", "A heading in the menu: translations the browser makes itself."),
    ("lang.machine", "Explains such a machine translation."),
    ("lang.show_original", "Turns the machine translation off again."),
    ("lang.translating", "Shown while the browser translates."),
    ("lang.cannot", "When the browser cannot translate; {language} is that language's own name (e.g. 'Français')."),
    ("lang.more", "Opens the list of further languages."),
    ("money.label", "The name of the currency switch in the bar: the button that shows a page's amounts in another currency (e.g. US dollars instead of yen)."),
    ("money.rate", "Under the converted amounts; {date} is the day of the exchange rates (e.g. '26 November 2026')."),
    ("money.more", "Opens the list of the other currencies a page's amounts can be shown in."),
    ("export.label", "The menu for saving or printing the page."),
    ("export.html", "Saves the page as a file to read offline."),
    ("export.print", "Prints the page, or saves it as PDF through the print dialog."),
    ("export.pdf", "Downloads the PDF copy."),
    ("export.epub", "Downloads the EPUB copy for e-readers."),
    ("prefs", "The name of the panel with text size, theme and saving."),
    ("credit", "The credit at the very foot; {md2html} is a link to md2html."),
    ("share.copied", "Said after a link was copied."),
    ("share.passage", "Copies a link to the text the reader selected."),
    ("share.row", "Copies a link to a table's row."),
    ("share.thing", "Copies a link to a figure, a diagram or a code block."),
    ("live", "Said when an open page reloaded with newer content."),
]


# Every English text of a review page, by key: the page is written in the
# language it reviews from its translation of this table (`page` in
# `scripts/i18n/terms.json`), and in English from this one. `{name}` is
# filled in; `{braces}` is shown as it is. A key marked by HTML_KEYS holds
# markup of its own.
UI = {
    "title": "md2html in {language}: review",
    "all_languages": "All languages",
    "in_english": "Read this page in English",
    "in_own": "Read this page in {language}",
    "heading": "md2html in {language}: does it read right?",
    "intro": ("md2html turns a Markdown file into a web page, with its controls in seventeen languages. "
              "Its {language} was written with care but not by a native speaker. We ask only what we are "
              "unsure of: <strong>{count} questions</strong>, one at a time. Answer one, press its Send, and "
              "it counts; two answers help, and you never need to finish."),
    "translated": ("This page was translated for you, also not by a native speaker: forgive its mistakes, "
                   "or tell us about them at the end."),
    "page_link": ('<a href="{page}" target="_blank" rel="noopener">The capabilities page in {language}</a> '
                  'shows the words below in use (<a href="{english_page}" target="_blank" rel="noopener">'
                  "the English original</a>)."),
    "me_heading": "About you, first",
    "me_intro": "Asked once, kept in this browser, sent with each answer. All three are optional.",
    "me_name": "Your name, to thank you in the release notes",
    "me_contact": "How to reach you if there is a question (never published)",
    "me_level": "Your {language} is",
    "level_choose": "choose…",
    "level_native": "my first language",
    "level_fluent": "fluent, not my first",
    "level_learning": "still learning",
    "terms_heading": "1. The words md2html had to choose ({count})",
    "terms_intro": ("Which word would you expect in software in your language? Each shows our guess and why "
                    "we are unsure. Nothing is chosen for you."),
    "strings_heading": "2. The wordings we are unsure of ({count})",
    "strings_intro": ("Each under its English, with why we ask. If one should change, choose “Should change” "
                      "and write it as it should read."),
    "sure_summary": "The {count} we are sure of: open only if you like",
    "sure_intro": ("Standard wordings we did not want to spend your time on. If one is wrong after all, it can "
                   "be answered here like the others."),
    "more_heading": "3. If you have more time",
    "more_intro": ('Read <a href="{page}" target="_blank" rel="noopener">the capabilities page</a> in your '
                   "language; if something reads wrong, tell us here."),
    "page_question": "Something on the capabilities page reads wrong?",
    "page_hint": "Quote it, and write it as it should read.",
    "else_question": "Anything else",
    "send": "Send this answer",
    "state_open": "not answered",
    "state_ready": "not sent",
    "state_changed": "changed, not sent",
    "state_sent": "sent",
    "reset": "reset",
    "reset_title": "Reset this question: clears it and withdraws the answer sent",
    "our_guess": "our guess",
    "our_guess_line": "Our guess:",
    "why_we_ask": "Why we ask:",
    "used_on_page": "Used on the page:",
    "now_word": "the word used now",
    "before_word": "the word used before the last review",
    "another_word": "Another word (write it below)",
    "your_word": "Your word",
    "why_optional": "Why? (optional)",
    "reads_right": "Reads right",
    "should_change": "Should change",
    "wording_before": "The wording before:",
    "keep_slots": "Keep anything in {braces} and any <tags> as they are: md2html fills them in.",
    "better_wording": "Better wording",
    "sure_unsure": "we are unsure",
    "sure_likely": "probably right",
    "sure_sure": "fairly sure",
    "sent_count": "{sent} of {count} sent",
    "only_open": "Show only what I have not sent",
    "sending": "Sending…",
    "arrived": "Thank you, it arrived.",
    "failed": "It did not arrive ({error}). It is kept here: try again in a moment.",
    "withdrawing": "Withdrawing…",
    "withdrawn": "Reset: the answer sent is withdrawn.",
    "not_withdrawn": "It could not be withdrawn ({error}).",
    "missing_term": "Choose a word first (or write your own).",
    "missing_string": "Choose “Reads right” or “Should change” first; to change it, edit the wording.",
    "missing_text": "Write something first.",
}

# The keys of UI whose text holds markup (links, emphasis), written as is.
HTML_KEYS = {"intro", "page_link", "more_intro"}


def context(path):
    """What the string at `path` is for."""
    for prefix, text in CONTEXT:
        if path == prefix or path.startswith(prefix + ".") or path.startswith(prefix + "["):
            return text
    raise KeyError(f"no context for {path}")


def git_show(ref, rel):
    """A file's text at a git ref, or None when it was not there."""
    done = subprocess.run(["git", "-C", str(ROOT), "show", f"{ref}:{rel}"],
                          capture_output=True, text=True, check=False)
    return done.stdout if done.returncode == 0 else None


def strings_before(code, ref):
    """`code`'s strings at `ref`, read from a copy of its file then."""
    rel = words.path_of(code).relative_to(ROOT).as_posix()
    text = git_show(ref, rel)
    return {} if text is None else words.read(code, text)


# What ends a sentence, in the scripts of the languages here.
STOPS = ".!?。！？"


def prose(markdown):
    """A page's lines of text: its front matter, code and tables left out,
    and the commonest markup taken off (emphasis, code, links)."""
    out = []
    fenced = False
    body = markdown.split("\n---\n", 1)[1] if markdown.startswith("---\n") else markdown
    for line in body.splitlines():
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced or line.lstrip().startswith(("|", "<", "![")):
            continue
        line = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", line)
        line = re.sub(r"[*`]+", "", line).lstrip("#> -").strip()
        if line:
            out.append(line)
    return out


def example(markdown, word):
    """The first sentence of a page's text that uses `word` (any case), or
    None."""
    lowered = word.lower()
    for line in prose(markdown):
        at = line.lower().find(lowered)
        if at < 0:
            continue
        start = max((line.rfind(stop, 0, at) for stop in STOPS), default=-1) + 1
        ends = [i for i in (line.find(stop, at + len(word)) for stop in STOPS) if i >= 0]
        end = min(ends) + 1 if ends else len(line)
        sentence = line[start:end].strip()
        return sentence if len(sentence) <= 240 else sentence[:239] + "…"
    return None


def chapters(markdown):
    """The `##` headings of a page outside code, in order."""
    out = []
    fenced = False
    for line in markdown.splitlines():
        if line.startswith("```"):
            fenced = not fenced
        elif not fenced and line.startswith("## "):
            out.append(line[3:].strip())
    return out


def language_data(code, terms, ref):
    """Everything a language's review page shows, as JSON-ready data."""
    own, english_name, direction = LANGUAGES[code]
    english = words.read("en")
    now = words.read(code)
    before = strings_before(code, ref)
    items = []
    for path, text in now.items():
        old = before.get(path)
        items.append({
            "path": path,
            "en": english[path],
            "now": text,
            "before": old if old not in (None, text) else None,
            "what": context(path),
            "doubt": terms["doubts"][code].get(path),
        })
    page = (ROOT / f"docs/capabilities.{code}.md").read_text(encoding="utf-8")
    original = (ROOT / "docs/capabilities.md").read_text(encoding="utf-8")
    heads = chapters(page)
    heads_en = chapters(original)
    if len(heads) != len(heads_en):
        raise ValueError(f"{code}: {len(heads)} chapters, English has {len(heads_en)}")
    return {
        "code": code,
        "own": own,
        "english": english_name,
        "dir": direction,
        "terms": [dict(term, **terms["languages"][code][term["id"]],
                       example=example(page, terms["languages"][code][term["id"]]["current"]))
                  for term in terms["terms"]],
        "strings": items,
        "chapters": [{"id": f"ch{n}", "title": t, "en": e} for n, (t, e) in enumerate(zip(heads, heads_en), 1)],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out", type=Path)
    parser.add_argument("--md2html", type=Path, default=ROOT / "target/release/md2html")
    parser.add_argument("--sitekey", default="1x00000000000000000000AA")
    parser.add_argument("--before", default=None)
    args = parser.parse_args()
    ref = args.before
    if not ref:
        described = subprocess.run(["git", "-C", str(ROOT), "describe", "--tags", "--abbrev=0"],
                                   capture_output=True, text=True, check=False)
        if described.returncode != 0:
            raise SystemExit("review_site: no tag to read the wording before from "
                             "(a shallow clone?): name one with --before")
        ref = described.stdout.strip()
    terms = json.loads((ROOT / "scripts/i18n/terms.json").read_text(encoding="utf-8"))
    args.out.mkdir(parents=True, exist_ok=True)
    languages = [(code, own, english, direction) for code, (own, english, direction) in LANGUAGES.items()]
    (args.out / "index.html").write_text(review_page.index_page(languages), encoding="utf-8")
    assets = args.out / "assets"
    assets.mkdir(exist_ok=True)
    for name in ("review.css", "review.js"):
        (assets / name).write_bytes((Path(__file__).resolve().parent / name).read_bytes())
    sources = [ROOT / "docs/capabilities.md"] + [ROOT / f"docs/capabilities.{code}.md" for code in LANGUAGES]
    subprocess.run([str(args.md2html), *map(str, sources), "-o", str(args.out / "pages"), "-q"], check=True)
    english_contexts = dict(CONTEXT)
    for code, (own, _, _) in LANGUAGES.items():
        folder = args.out / code
        folder.mkdir(exist_ok=True)
        data = language_data(code, terms, ref)
        own_text = terms["page"][code]
        local = review_page.Text(own_text["ui"], own_text["contexts"], own_text["terms"], own_text["doubts"], HTML_KEYS)
        english = review_page.Text(UI, english_contexts, {
            term["id"]: {"what": term["what"], "why": term["why"], "notes": [c.get("note", "") for c in term["choices"]]}
            for term in data["terms"]
        }, {path: doubt["why"] for path, doubt in terms["doubts"][code].items()}, HTML_KEYS)
        back = own_text["ui"]["in_own"].replace("{language}", own)
        (folder / "index.html").write_text(review_page.language_page(data, args.sitekey, local), encoding="utf-8")
        (folder / "en.html").write_text(review_page.language_page(data, args.sitekey, english, back), encoding="utf-8")
    print(f"review site written to {args.out} ({len(LANGUAGES)} languages, before = {ref})")


if __name__ == "__main__":
    main()
