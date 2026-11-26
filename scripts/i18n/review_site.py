#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The translation review site: one page per language for native speakers.

`review_site.py OUT [--md2html BIN] [--sitekey KEY] [--before REF]`
writes into OUT:

- `index.html`, the languages to choose from;
- `<code>/index.html`, a language's review: who the reviewer is, asked
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
import html
import json
import re
import subprocess
from pathlib import Path

# The script's own folder is first on the module path when it runs, so
# its sibling module is found.

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


def esc(text):
    """Text for HTML."""
    return html.escape(text, quote=True)


# How sure the guess for a term is, unsure first: the order the terms are
# shown in, so a reviewer with little time spends it where it counts.
SURENESS = {
    "unsure": "we are unsure",
    "likely": "probably right",
    "sure": "fairly sure",
}


def question(kind, item, title, body, now=None, asked=True):
    """One question as a block with its state, its own Send and a note
    (review.js keeps the state); one we are sure of is not `asked`, and is
    left out of the count."""
    extra = f' data-now="{esc(now)}"' if now is not None else ""
    folded = "" if asked else " sure-one"
    return (f'<section class="q st-open{folded}" data-kind="{kind}" data-item="{esc(item)}"{extra}>'
            f'<div class="q-head"><h3 class="q-title">{title}</h3>'
            '<button type="button" class="q-state">not answered</button></div>'
            f'{body}<div class="q-foot"><button type="button" class="send">Send this answer</button>'
            '<span class="q-note" role="status"></span></div></section>')


def term_card(term):
    """A term's multiple choice, our guess marked but not chosen."""
    item = term["id"]
    name = f"term|{item}|choice"
    options = [("current", term["current"], "the word used now")]
    if term.get("previous"):
        options.append(("previous", term["previous"], "the word used before the last review"))
    options += [(f"alt{n}", alt["text"], alt.get("note", "")) for n, alt in enumerate(term["choices"])]
    pick = term["pick"]
    rows = "".join(
        f'<label class="opt"><input type="radio" name="{name}" value="{esc(value)}">'
        f'<span><strong>{esc(text)}</strong>'
        + (' <span class="guess">our guess</span>' if value == pick else "")
        + f' <span class="note">{esc(note)}</span></span></label>'
        for value, text, note in options
    )
    rows += (f'<label class="opt"><input type="radio" name="{name}" value="other">'
             '<span>Another word (write it below)</span></label>')
    guessed = dict((value, text) for value, text, _ in options)[pick]
    example = (f'<p class="what">Used on the page: “{esc(term["example"])}”</p>' if term.get("example") else "")
    title = (f'{esc(term["en"])} <span class="sure c-{term["confidence"]}">'
             f'{SURENESS[term["confidence"]]}</span>')
    body = (f'<p class="what">{esc(term["what"])}</p>{example}'
            f'<p class="hint">Our guess: <strong>{esc(guessed)}</strong>. {esc(term["why"])}</p>{rows}'
            f'<div class="hidden" data-when="choice=other"><input type="text" name="term|{item}|other" '
            f'aria-label="Your word for {esc(term["en"])}"></div>'
            f'<textarea name="term|{item}|note" placeholder="Why? (optional)" '
            f'aria-label="Why, for {esc(term["en"])}"></textarea>')
    return question("term", item, title, body, asked=term["confidence"] != "sure")


def string_card(item, direction):
    """An interface string to keep or to change."""
    path = item["path"]
    name = f"string|{esc(path)}|verdict"
    before = ""
    if item["before"]:
        before = (f'<label><input type="radio" name="{name}" value="before">'
                  f'<span>The wording before: <span dir="{direction}">{esc(item["before"])}</span></span></label>')
    doubt = item["doubt"]
    why = (f'<p class="hint">Why we ask: {esc(doubt["why"])}</p>' if doubt else "")
    body = (f'<p class="now" dir="{direction}">{esc(item["now"])}</p>'
            f'<p class="what">{esc(item["what"])}</p>{why}'
            f'<div class="row" role="radiogroup" aria-label="{esc(item["en"])}">'
            f'<label><input type="radio" name="{name}" value="ok"><span>Reads right</span></label>'
            f'<label><input type="radio" name="{name}" value="change"><span>Should change</span></label>'
            f'{before}</div>'
            f'<div class="hidden" data-when="verdict=change"><textarea dir="{direction}" name="string|{esc(path)}|text" '
            f'aria-label="Better wording">{esc(item["now"])}</textarea>'
            '<p class="note">Keep anything in {braces} and any &lt;tags&gt; as they are: md2html fills them in.</p></div>')
    title = f'<span class="en">{esc(item["en"])}</span>'
    if doubt:
        title += f' <span class="sure c-{doubt["confidence"]}">{SURENESS[doubt["confidence"]]}</span>'
    return question("string", path, title, body, now=item["now"], asked=bool(doubt))


def language_page(data, sitekey):
    """A language's review page."""
    code, own, direction = data["code"], data["own"], data["dir"]
    order = list(SURENESS)
    terms = sorted(data["terms"], key=lambda t: order.index(t["confidence"]))
    asked_terms = [t for t in terms if t["confidence"] != "sure"]
    sure_terms = [t for t in terms if t["confidence"] == "sure"]
    doubtful = sorted((s for s in data["strings"] if s["doubt"]), key=lambda s: order.index(s["doubt"]["confidence"]))
    sure_strings = [s for s in data["strings"] if not s["doubt"]]
    asked = len(asked_terms) + len(doubtful)
    terms_html = "".join(term_card(t) for t in asked_terms)
    strings_html = "".join(string_card(s, direction) for s in doubtful)
    sure_html = "".join(term_card(t) for t in sure_terms) + "".join(string_card(s, direction) for s in sure_strings)
    sure_count = len(sure_terms) + len(sure_strings)
    on_page = question("general", "page", "Something on the capabilities page reads wrong?",
                       f'<p class="what">Quote it, and write it as it should read.</p>'
                       f'<textarea dir="{direction}" name="general|page|text" aria-label="On the capabilities page"></textarea>')
    general = question("general", "all", "Anything else",
                       f'<textarea dir="{direction}" name="general|all|text" aria-label="Anything else"></textarea>')
    payload = json.dumps(dict(data, sitekey=sitekey), ensure_ascii=False).replace("</", "<\\/")
    page = f"../pages/capabilities.{code}.html"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>md2html in {esc(own)}: review</title>
<link rel="stylesheet" href="../assets/review.css">
</head>
<body>
<main>
<p class="muted"><a href="../">All languages</a></p>
<h1>md2html in <span lang="{code}" dir="{direction}">{esc(own)}</span>: does it read right?</h1>
<p>md2html turns a Markdown file into a web page, with its controls in seventeen languages. Its {esc(data['english'])} was written with care but not by a native speaker. We ask only what we are unsure of: <strong>{asked} questions</strong>, one at a time. Answer one, press its Send, and it counts; two answers help, and you never need to finish.</p>
<p><a href="{page}" target="_blank" rel="noopener">The capabilities page in {esc(own)}</a> shows every word below in use (<a href="../pages/capabilities.html" target="_blank" rel="noopener">the English original</a>).</p>
<section class="me" id="me" lang="en">
<h2 style="border:0;margin:0;padding:0">About you, first</h2>
<p class="muted">Asked once, kept in this browser, sent with each answer. All three are optional.</p>
<label>Your name, to thank you in the release notes<input type="text" name="name" autocomplete="name"></label>
<label>How to reach you if there is a question (never published)<input type="email" name="contact" autocomplete="email"></label>
<label>Your {esc(data['english'])} is
<select name="level"><option value="">choose…</option><option value="native">my first language</option><option value="fluent">fluent, not my first</option><option value="learning">still learning</option></select></label>
</section>
<h2 lang="en">1. The words md2html had to choose ({len(asked_terms)})</h2>
<p lang="en" class="muted">Which word would you expect in software in your language? Each shows our guess and why we are unsure. Nothing is chosen for you.</p>
<div lang="{code}">{terms_html}</div>
<h2 lang="en">2. The wordings we are unsure of ({len(doubtful)})</h2>
<p lang="en" class="muted">Each under its English, with why we ask. If one should change, choose “Should change” and write it as it should read.</p>
<div lang="{code}">{strings_html}</div>
<details class="sure-ones" lang="en"><summary>The {sure_count} we are sure of: open only if you like</summary>
<p class="muted">Standard wordings we did not want to spend your time on. If one is wrong after all, it can be answered here like the others.</p>
<div lang="{code}">{sure_html}</div></details>
<h2 lang="en">3. If you have more time</h2>
<p lang="en" class="muted">Read <a href="{page}" target="_blank" rel="noopener">the capabilities page</a> in your language; if something reads wrong, tell us here.</p>
<div lang="{code}">{on_page}{general}</div>
</main>
<div class="bar" lang="en"><span id="sent"></span><label><input type="checkbox" id="only-open"> Show only what I have not sent</label><div id="check"></div></div>
<script type="application/json" id="data">{payload}</script>
<script src="../assets/review.js"></script>
</body>
</html>
"""


def index_page():
    """The languages to choose from."""
    items = "".join(
        f'<li><a href="{code}/" lang="{code}"><span dir="{direction}">{esc(own)}</span><small lang="en">{esc(english)}</small></a></li>'
        for code, (own, english, direction) in LANGUAGES.items()
    )
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>md2html translation review</title>
<link rel="stylesheet" href="assets/review.css">
</head>
<body>
<main>
<h1>Help md2html read right in your language</h1>
<p>md2html makes web pages from Markdown, with its buttons, tips and messages in seventeen languages. None of the sixteen translations was written by a native speaker. If one of these is your language, you can tell us what reads wrong, mostly by choosing, one question at a time: a minute helps, no account needed.</p>
<ul class="langs">{items}</ul>
<p class="muted">md2html is open source: <a href="https://github.com/githubrobbi/md2html">github.com/githubrobbi/md2html</a>.</p>
</main>
</body>
</html>
"""


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
    (args.out / "index.html").write_text(index_page(), encoding="utf-8")
    assets = args.out / "assets"
    assets.mkdir(exist_ok=True)
    for name in ("review.css", "review.js"):
        (assets / name).write_bytes((Path(__file__).resolve().parent / name).read_bytes())
    sources = [ROOT / "docs/capabilities.md"] + [ROOT / f"docs/capabilities.{code}.md" for code in LANGUAGES]
    subprocess.run([str(args.md2html), *map(str, sources), "-o", str(args.out / "pages"), "-q"], check=True)
    for code in LANGUAGES:
        folder = args.out / code
        folder.mkdir(exist_ok=True)
        data = language_data(code, terms, ref)
        (folder / "index.html").write_text(language_page(data, args.sitekey), encoding="utf-8")
    print(f"review site written to {args.out} ({len(LANGUAGES)} languages, before = {ref})")


if __name__ == "__main__":
    main()
