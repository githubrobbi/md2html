#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The translation review site: one page per language for native speakers.

`review_site.py OUT [--md2html BIN] [--sitekey KEY] [--before REF]`
writes into OUT:

- `index.html`, the languages to choose from;
- `<code>/index.html`, a language's review: the seven coined terms as
  multiple choice (the wording now, the one before the last review, the
  alternatives of `scripts/i18n/terms.json`, or the reviewer's own), every
  interface string (`scripts/i18n/words.py`) beside its English with what
  it is for, to keep or to change, and a comment per chapter of the
  capabilities page;
- `pages/capabilities.html` and `pages/capabilities.<code>.html`, that
  page rendered by md2html (BIN, default `target/release/md2html`) in
  every language, side by side so its language switch links them.

A review is sent as JSON to `/api/answers` with a Cloudflare Turnstile
token (KEY is the widget's site key, public); the Worker that serves the
site stores it. The wording before is read from git at REF (default the
newest tag; a clone without tags names it), so a reviewer can choose it
back. `review_answers.py` reads the answers. Exit 1 when a language's
words or page cannot be read, or no tag is found and none is named.
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


STYLE = """
:root{--bg:#fbfaf7;--fg:#1d1d1b;--muted:#66645e;--line:#dedbd2;--card:#fff;--accent:#2f5d8a;--accent-fg:#fff;--ok:#2e7d4f;--warn:#a65c00;--focus:#2f5d8a}
@media (prefers-color-scheme:dark){:root{--bg:#161615;--fg:#ecebe6;--muted:#a3a199;--line:#34332f;--card:#1f1f1d;--accent:#8db7e0;--accent-fg:#111;--ok:#7cc79a;--warn:#e0a458;--focus:#8db7e0}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:17px/1.55 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans","Noto Sans CJK JP","Noto Sans Hebrew",sans-serif}
main{max-width:52rem;margin:0 auto;padding:1.5rem 1rem 6rem}
h1{font-size:1.7rem;line-height:1.25;margin:.5rem 0 .25rem}
h2{font-size:1.25rem;margin:2.5rem 0 .5rem;padding-top:.5rem;border-top:1px solid var(--line)}
p,li{max-width:44rem}
.muted{color:var(--muted)}
.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:1rem 1.1rem;margin:.9rem 0}
.card h3{margin:0 0 .2rem;font-size:1.05rem}
.en{color:var(--muted);font-size:.92rem;margin:.1rem 0 .4rem}
.now{font-size:1.1rem;margin:.2rem 0}
.what{color:var(--muted);font-size:.85rem;margin:.15rem 0 .5rem}
label.opt{display:flex;gap:.55rem;align-items:flex-start;padding:.4rem .5rem;border-radius:8px;cursor:pointer}
label.opt:hover{background:color-mix(in srgb,var(--accent) 8%,transparent)}
label.opt input{margin-top:.35rem}
.note{color:var(--muted);font-size:.85rem}
textarea,input[type=text],input[type=email],select{width:100%;font:inherit;color:inherit;background:var(--bg);border:1px solid var(--line);border-radius:8px;padding:.5rem .6rem}
textarea{min-height:2.6rem;resize:vertical}
:focus-visible{outline:3px solid var(--focus);outline-offset:2px}
.row{display:flex;gap:.6rem;flex-wrap:wrap;margin:.3rem 0}
.row label{display:flex;gap:.35rem;align-items:center;padding:.25rem .6rem;border:1px solid var(--line);border-radius:999px;cursor:pointer}
.row input:checked+span{font-weight:600}
.hidden{display:none}
button{font:inherit;padding:.65rem 1.2rem;border-radius:8px;border:0;background:var(--accent);color:var(--accent-fg);cursor:pointer}
button[disabled]{opacity:.5;cursor:default}
.bar{position:fixed;inset:auto 0 0 0;background:var(--card);border-top:1px solid var(--line);padding:.6rem 1rem;display:flex;gap:1rem;align-items:center;justify-content:center;flex-wrap:wrap}
progress{width:12rem}
.langs{display:grid;grid-template-columns:repeat(auto-fill,minmax(14rem,1fr));gap:.7rem;padding:0;list-style:none}
.langs a{display:block;padding:.8rem 1rem;border:1px solid var(--line);border-radius:10px;background:var(--card);color:inherit;text-decoration:none}
.langs a:hover{border-color:var(--accent)}
.langs small{display:block;color:var(--muted)}
.thanks{font-size:1.1rem;color:var(--ok)}
.error{color:var(--warn)}
code{font-size:.9em}
a{color:var(--accent)}
"""

SCRIPT = r"""
const D = JSON.parse(document.getElementById('data').textContent);
const KEY = 'md2html-review-' + D.code;
const form = document.getElementById('review');
const store = {
  load() { try { return JSON.parse(localStorage.getItem(KEY) || '{}'); } catch { return {}; } },
  save(v) { try { localStorage.setItem(KEY, JSON.stringify(v)); } catch {} },
};
function collect() {
  const out = { lang: D.code, reviewer: {}, terms: {}, strings: {}, chapters: {}, general: '' };
  for (const el of form.elements) {
    // Only this page's fields ("kind|key|field"); Turnstile adds its own.
    if (!el.name.includes('|') || (el.type === 'radio' && !el.checked)) continue;
    const [kind, key, field] = el.name.split('|');
    if (kind === 'reviewer') out.reviewer[key] = el.value;
    else if (kind === 'general') out.general = el.value;
    else if (kind === 'chapter') { if (el.value.trim()) out.chapters[key] = el.value; }
    else { (out[kind][key] ||= {})[field] = el.value; }
  }
  return out;
}
function restore(v) {
  for (const el of form.elements) {
    if (!el.name.includes('|')) continue;
    const [kind, key, field] = el.name.split('|');
    let val;
    if (kind === 'reviewer') val = v.reviewer?.[key];
    else if (kind === 'general') val = v.general;
    else if (kind === 'chapter') val = v.chapters?.[key];
    else val = v[kind]?.[key]?.[field];
    if (val === undefined) continue;
    if (el.type === 'radio') el.checked = el.value === val; else el.value = val;
  }
}
function update() {
  const v = collect();
  for (const card of form.querySelectorAll('[data-other]')) {
    const picked = card.querySelector('input[type=radio]:checked');
    card.querySelector('.other').classList.toggle('hidden', !picked || picked.value !== card.dataset.other);
  }
  const total = D.terms.length + D.strings.length;
  const done = Object.values(v.terms).filter(t => t.choice).length + Object.values(v.strings).filter(s => s.verdict).length;
  const bar = document.getElementById('progress');
  bar.max = total; bar.value = done;
  document.getElementById('count').textContent = done + ' / ' + total;
  store.save(v);
}
form.addEventListener('input', update);
form.addEventListener('change', update);
restore(store.load());
update();
form.addEventListener('submit', async (e) => {
  e.preventDefault();
  const status = document.getElementById('status');
  const button = document.getElementById('send');
  const token = form.querySelector('[name="cf-turnstile-response"]')?.value || '';
  if (!token) { status.textContent = 'Please wait for the check above the button to finish, then send again.'; status.className = 'error'; return; }
  button.disabled = true; status.textContent = 'Sending…'; status.className = '';
  try {
    const res = await fetch('/api/answers', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ token, review: collect() }) });
    if (!res.ok) throw new Error(await res.text());
    status.textContent = 'Thank you! Your review has arrived. You can close this page, or change something and send again.';
    status.className = 'thanks';
  } catch (err) {
    status.textContent = 'It did not arrive (' + err.message + '). Your answers are kept in this browser: please try again in a moment.';
    status.className = 'error';
  } finally {
    button.disabled = false;
    window.turnstile?.reset();
  }
});
"""


def esc(text):
    """Text for HTML."""
    return html.escape(text, quote=True)


def term_card(term):
    """A term's multiple choice."""
    name = f"terms|{term['id']}|choice"
    options = [("current", term["current"], "the word used now")]
    if term.get("previous"):
        options.append(("previous", term["previous"], "the word used before the last review"))
    options += [(f"alt{n}", alt["text"], alt.get("note", "")) for n, alt in enumerate(term["choices"])]
    rows = "".join(
        f'<label class="opt"><input type="radio" name="{name}" value="{esc(value)}">'
        f'<span><strong>{esc(text)}</strong> <span class="note">{esc(note)}</span></span></label>'
        for value, text, note in options
    )
    rows += (f'<label class="opt"><input type="radio" name="{name}" value="other">'
             '<span>Another word (write it below)</span></label>')
    example = (f'<p class="what">Used on the page: “{esc(term["example"])}”</p>' if term.get("example") else "")
    return (f'<div class="card" data-other="other"><h3>{esc(term["en"])}</h3>'
            f'<p class="what">{esc(term["what"])}</p>{example}{rows}'
            f'<div class="other hidden"><input type="text" name="terms|{term["id"]}|other" '
            f'aria-label="Your word for {esc(term["en"])}"></div>'
            f'<textarea name="terms|{term["id"]}|note" placeholder="Why? (optional)" '
            f'aria-label="Why, for {esc(term["en"])}"></textarea></div>')


def string_card(item, direction):
    """An interface string to keep or to change."""
    key = item["path"]
    before = ""
    if item["before"]:
        before = (f'<label><input type="radio" name="strings|{esc(key)}|verdict" value="before">'
                  f'<span>The wording before: <span dir="{direction}">{esc(item["before"])}</span></span></label>')
    return (f'<div class="card" data-other="change"><p class="en">English: {esc(item["en"])}</p>'
            f'<p class="now" dir="{direction}">{esc(item["now"])}</p>'
            f'<p class="what">{esc(item["what"])}</p>'
            f'<div class="row" role="radiogroup" aria-label="{esc(item["en"])}">'
            f'<label><input type="radio" name="strings|{esc(key)}|verdict" value="ok"><span>Reads right</span></label>'
            f'<label><input type="radio" name="strings|{esc(key)}|verdict" value="change"><span>Should change</span></label>'
            f'{before}</div>'
            f'<div class="other hidden"><textarea dir="{direction}" name="strings|{esc(key)}|text" '
            f'aria-label="Better wording">{esc(item["now"])}</textarea>'
            f'<p class="note">Keep anything in {{braces}} and any &lt;tags&gt; as they are: md2html fills them in.</p></div></div>')


def language_page(data, sitekey):
    """A language's review page."""
    code, own, direction = data["code"], data["own"], data["dir"]
    terms = "".join(term_card(t) for t in data["terms"])
    strings = "".join(string_card(s, direction) for s in data["strings"])
    chapters = "".join(
        f'<div class="card"><h3 dir="{direction}">{esc(c["title"])}</h3><p class="en">{esc(c["en"])}</p>'
        f'<textarea dir="{direction}" name="chapter|{c["id"]}" aria-label="{esc(c["en"])}" '
        'placeholder="What reads wrong in this chapter? Quote it and give the better wording."></textarea></div>'
        for c in data["chapters"]
    )
    payload = json.dumps(data, ensure_ascii=False).replace("</", "<\\/")
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>md2html in {esc(own)}: review</title>
<style>{STYLE}</style>
<script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>
</head>
<body>
<main>
<p class="muted"><a href="../">All languages</a></p>
<h1>md2html in <span lang="{code}" dir="{direction}">{esc(own)}</span>: does it read right?</h1>
<p>md2html turns a Markdown file into a web page, with its controls in seventeen languages. Its {esc(data['english'])} was written with care but not by a native speaker. You can make it right: about fifteen minutes is plenty, and every answer helps, even a few.</p>
<p>Choose what reads naturally to you. Nothing is changed until your answers are read. Your answers stay in this browser until you send them, so you can stop and come back.</p>
<p><a href="../pages/capabilities.{code}.html" target="_blank" rel="noopener">The capabilities page in {esc(own)}</a> shows every word below in use (<a href="../pages/capabilities.html" target="_blank" rel="noopener">the English original</a>).</p>
<form id="review" lang="{code}">
<h2 lang="en">1. Seven words md2html had to choose</h2>
<p lang="en" class="muted">Which word would you expect in software in your language?</p>
{terms}
<h2 lang="en">2. The words on every page</h2>
<p lang="en" class="muted">The buttons, tips and messages, beside the English. If one should change, choose “Should change” and write it as it should read.</p>
{strings}
<h2 lang="en">3. The capabilities page, chapter by chapter</h2>
<p lang="en" class="muted">Read a chapter of <a href="../pages/capabilities.{code}.html" target="_blank" rel="noopener">the page</a>; if something reads wrong, quote it here with the better wording.</p>
{chapters}
<h2 lang="en">4. Anything else, and who you are</h2>
<div class="card" lang="en">
<label>Anything else<textarea dir="{direction}" name="general|text"></textarea></label>
<label>Your name (optional, to thank you in the release notes)<input type="text" name="reviewer|name" autocomplete="name"></label>
<label>How to reach you, if there is a question (optional, never published)<input type="email" name="reviewer|contact" autocomplete="email"></label>
<label>Your {esc(data['english'])} is
<select name="reviewer|level"><option value="">choose…</option><option value="native">my first language</option><option value="fluent">fluent, not my first</option><option value="learning">still learning</option></select></label>
</div>
<div class="cf-turnstile" data-sitekey="{esc(sitekey)}"></div>
<div class="bar" lang="en"><span>Answered <span id="count"></span></span><progress id="progress" value="0" max="1"></progress><button id="send" type="submit">Send my review</button><span id="status" role="status"></span></div>
</form>
</main>
<script type="application/json" id="data">{payload}</script>
<script>{SCRIPT}</script>
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
<style>{STYLE}</style>
</head>
<body>
<main>
<h1>Help md2html read right in your language</h1>
<p>md2html makes web pages from Markdown, with its buttons, tips and messages in seventeen languages. None of the sixteen translations was written by a native speaker. If one of these is your language, you can tell us what reads wrong, mostly by choosing: about fifteen minutes, no account needed.</p>
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
