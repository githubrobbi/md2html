# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2026 Sky, LLC
"""The HTML of the translation review site (`review_site.py` builds it).

A language's review page is written in that language from its `Text`: the
page's own words (review_site.UI, translated), what each string is for,
and the notes, guesses and reasons of its terms and doubts; the same page
in English is written from the English originals. Every question is a
block with its state and its own Send (`review.js` runs them); only what
we are unsure of is asked, the rest folded away.
"""

import html
import json

# How sure a guess is, the least sure first: the order of the questions.
SURENESS = ("unsure", "likely", "sure")


def esc(text):
    """Text for HTML."""
    return html.escape(text, quote=True)


class Text:
    """The words of one review page: its interface (`ui`, the keys of
    review_site.UI), what each interface string is for (`contexts`, by
    path prefix), and per term its meaning, our reason and the notes of its
    alternatives (`terms`), and per doubtful string why we ask (`doubts`)."""

    def __init__(self, ui, contexts, terms, doubts, html_keys):
        self.ui = ui
        self.contexts = contexts
        self.terms = terms
        self.doubts = doubts
        self.html_keys = html_keys

    def say(self, key, **values):
        """`key`'s text with its `{name}`s filled in, ready for HTML: a key
        of markup as written (its values escaped), any other escaped."""
        text = self.ui[key]
        if key in self.html_keys:
            for name, value in values.items():
                text = text.replace("{" + name + "}", esc(str(value)))
            return text
        for name, value in values.items():
            text = text.replace("{" + name + "}", str(value))
        return esc(text)

    def context(self, path):
        """What the string at `path` is for (its first matching prefix)."""
        for prefix, text in self.contexts.items():
            if path == prefix or path.startswith(prefix + ".") or path.startswith(prefix + "["):
                return text
        raise KeyError(f"no context for {path}")


def question(tx, kind, item, title, body, now=None, asked=True):
    """One question as a block with its state, its own Send and a note;
    one we are sure of is not `asked`, and is left out of the count."""
    extra = f' data-now="{esc(now)}"' if now is not None else ""
    folded = "" if asked else " sure-one"
    return (f'<section class="q st-open{folded}" data-kind="{kind}" data-item="{esc(item)}"{extra}>'
            f'<div class="q-head"><h3 class="q-title">{title}</h3>'
            f'<button type="button" class="q-state" data-reset="{tx.say("reset")}">{tx.say("state_open")}</button></div>'
            f'{body}<div class="q-foot"><button type="button" class="send">{tx.say("send")}</button>'
            '<span class="q-note" role="status"></span></div></section>')


def sureness(tx, level):
    """How sure we are, as a tag."""
    return f'<span class="sure c-{level}">{tx.say("sure_" + level)}</span>'


def term_card(tx, term):
    """A term's multiple choice, our guess marked but not chosen."""
    item = term["id"]
    words = tx.terms[item]
    name = f"term|{item}|choice"
    options = [("current", term["current"], tx.say("now_word"))]
    if term.get("previous"):
        options.append(("previous", term["previous"], tx.say("before_word")))
    options += [(f"alt{n}", alt["text"], esc(words["notes"][n])) for n, alt in enumerate(term["choices"])]
    pick = term["pick"]
    rows = "".join(
        f'<label class="opt"><input type="radio" name="{name}" value="{esc(value)}">'
        f'<span><strong>{esc(text)}</strong>'
        + (f' <span class="guess">{tx.say("our_guess")}</span>' if value == pick else "")
        + f' <span class="note">{note}</span></span></label>'
        for value, text, note in options
    )
    rows += (f'<label class="opt"><input type="radio" name="{name}" value="other">'
             f'<span>{tx.say("another_word")}</span></label>')
    guessed = {value: text for value, text, _ in options}[pick]
    example = (f'<p class="what">{tx.say("used_on_page")} “{esc(term["example"])}”</p>'
               if term.get("example") else "")
    title = f'{esc(term["en"])} {sureness(tx, term["confidence"])}'
    body = (f'<p class="what">{esc(words["what"])}</p>{example}'
            f'<p class="hint">{tx.say("our_guess_line")} <strong>{esc(guessed)}</strong>. {esc(words["why"])}</p>{rows}'
            f'<div class="hidden" data-when="choice=other"><input type="text" name="term|{item}|other" '
            f'aria-label="{tx.say("your_word")}"></div>'
            f'<textarea name="term|{item}|note" placeholder="{tx.say("why_optional")}" '
            f'aria-label="{tx.say("why_optional")}"></textarea>')
    return question(tx, "term", item, title, body, asked=term["confidence"] != "sure")


def string_card(tx, item, direction):
    """An interface string to keep or to change."""
    path = item["path"]
    name = f"string|{esc(path)}|verdict"
    before = ""
    if item["before"]:
        before = (f'<label><input type="radio" name="{name}" value="before">'
                  f'<span>{tx.say("wording_before")} <span dir="{direction}">{esc(item["before"])}</span></span></label>')
    doubt = item["doubt"]
    why = f'<p class="hint">{tx.say("why_we_ask")} {esc(tx.doubts[path])}</p>' if doubt else ""
    body = (f'<p class="now" dir="{direction}">{esc(item["now"])}</p>'
            f'<p class="what">{esc(tx.context(path))}</p>{why}'
            f'<div class="row" role="radiogroup" aria-label="{esc(item["en"])}">'
            f'<label><input type="radio" name="{name}" value="ok"><span>{tx.say("reads_right")}</span></label>'
            f'<label><input type="radio" name="{name}" value="change"><span>{tx.say("should_change")}</span></label>'
            f'{before}</div>'
            f'<div class="hidden" data-when="verdict=change"><textarea dir="{direction}" name="string|{esc(path)}|text" '
            f'aria-label="{tx.say("better_wording")}">{esc(item["now"])}</textarea>'
            f'<p class="note">{tx.say("keep_slots")}</p></div>')
    title = f'<span class="en" lang="en" dir="ltr">{esc(item["en"])}</span>'
    if doubt:
        title += " " + sureness(tx, doubt["confidence"])
    return question(tx, "string", path, title, body, now=item["now"], asked=bool(doubt))


def language_page(data, sitekey, tx, back=None):
    """A language's review page, written in that language; in English when
    `back` is given: the link back to the page in its language, in that
    language."""
    english = back is not None
    code, direction = data["code"], data["dir"]
    language = data["english"] if english else data["own"]
    page_lang, page_dir = ("en", "ltr") if english else (code, direction)
    terms = sorted(data["terms"], key=lambda t: SURENESS.index(t["confidence"]))
    asked_terms = [t for t in terms if t["confidence"] != "sure"]
    sure_terms = [t for t in terms if t["confidence"] == "sure"]
    doubtful = sorted((s for s in data["strings"] if s["doubt"]),
                      key=lambda s: SURENESS.index(s["doubt"]["confidence"]))
    sure_strings = [s for s in data["strings"] if not s["doubt"]]
    count = len(asked_terms) + len(doubtful)
    sure_html = ("".join(term_card(tx, t) for t in sure_terms)
                 + "".join(string_card(tx, s, direction) for s in sure_strings))
    page = f"../pages/capabilities.{code}.html"
    on_page = question(tx, "general", "page", tx.say("page_question"),
                       f'<p class="what">{tx.say("page_hint")}</p>'
                       f'<textarea dir="{direction}" name="general|page|text" aria-label="{tx.say("page_question")}"></textarea>')
    general = question(tx, "general", "all", tx.say("else_question"),
                       f'<textarea dir="{direction}" name="general|all|text" aria-label="{tx.say("else_question")}"></textarea>')
    other = (f'<a href="./" lang="{code}" dir="{direction}">{esc(back)}</a>' if english
             else f'<a href="en.html" lang="en">{tx.say("in_english")}</a>')
    translated = "" if english else f'<p class="muted">{tx.say("translated")}</p>'
    payload = json.dumps(dict(data, sitekey=sitekey, ui=tx.ui), ensure_ascii=False).replace("</", "<\\/")
    return f"""<!doctype html>
<html lang="{page_lang}" dir="{page_dir}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex">
<title>{tx.say("title", language=language)}</title>
<link rel="stylesheet" href="../assets/review.css">
</head>
<body>
<main>
<p class="muted top-links"><a href="../">{tx.say("all_languages")}</a> · {other}</p>
<h1>{tx.say("heading", language=language)}</h1>
<p>{tx.say("intro", language=language, count=count)}</p>
{translated}
<p>{tx.say("page_link", language=language, page=page, english_page="../pages/capabilities.html")}</p>
<section class="me" id="me">
<h2>{tx.say("me_heading")}</h2>
<p class="muted">{tx.say("me_intro")}</p>
<label>{tx.say("me_name")}<input type="text" name="name" autocomplete="name"></label>
<label>{tx.say("me_contact")}<input type="email" name="contact" autocomplete="email"></label>
<label>{tx.say("me_level", language=language)}
<select name="level"><option value="">{tx.say("level_choose")}</option><option value="native">{tx.say("level_native")}</option><option value="fluent">{tx.say("level_fluent")}</option><option value="learning">{tx.say("level_learning")}</option></select></label>
</section>
<h2>{tx.say("terms_heading", count=len(asked_terms))}</h2>
<p class="muted">{tx.say("terms_intro")}</p>
<div lang="{code}">{"".join(term_card(tx, t) for t in asked_terms)}</div>
<h2>{tx.say("strings_heading", count=len(doubtful))}</h2>
<p class="muted">{tx.say("strings_intro")}</p>
<div lang="{code}">{"".join(string_card(tx, s, direction) for s in doubtful)}</div>
<details class="sure-ones"><summary>{tx.say("sure_summary", count=len(sure_terms) + len(sure_strings))}</summary>
<p class="muted">{tx.say("sure_intro")}</p>
<div lang="{code}">{sure_html}</div></details>
<h2>{tx.say("more_heading")}</h2>
<p class="muted">{tx.say("more_intro", page=page)}</p>
<div lang="{code}">{on_page}{general}</div>
</main>
<div class="bar"><span id="sent"></span><label><input type="checkbox" id="only-open"> {tx.say("only_open")}</label><div id="check"></div></div>
<script type="application/json" id="data">{payload}</script>
<script src="../assets/review.js"></script>
</body>
</html>
"""


def index_page(languages):
    """The languages to choose from: (code, own name, English name,
    direction) each."""
    items = "".join(
        f'<li><a href="{code}/" lang="{code}"><span dir="{direction}">{esc(own)}</span>'
        f'<small lang="en">{esc(english)}</small></a></li>'
        for code, own, english, direction in languages
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
<p>md2html makes web pages from Markdown, with its buttons, tips and messages in seventeen languages. None of the sixteen translations was written by a native speaker. If one of these is your language, tell us what reads wrong: each page is in its own language, asks only what we are unsure of, one question at a time. A minute helps; no account needed.</p>
<ul class="langs">{items}</ul>
<p class="muted">md2html is open source: <a href="https://github.com/githubrobbi/md2html">github.com/githubrobbi/md2html</a>.</p>
</main>
</body>
</html>
"""
