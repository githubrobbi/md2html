<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# The author's tags inside md2html's

Each paragraph below puts a tag of the author's where HTML's parser would end an element of md2html's, or move one of md2html's out of the author's. `nesting.js` reads the page in Chrome and holds every element where its tags put it.

## In a paragraph

A block in a paragraph: <div>a division</div> and after it.

A paragraph in a paragraph: <p>inner</p> and after it.

A list in a paragraph: <ul><li>an item</li></ul> and after it.

A table in a paragraph: <table><tr><td>a cell</td></tr></table> and after it.

A heading in a paragraph: <h3>a heading</h3> and after it.

A disclosure in a paragraph: <details><summary>more</summary>hidden</details> and after it.

A rule in a paragraph: <hr> and after it, and a quote: <blockquote>quoted</blockquote> and after it.

*Emphasis around a block: <div>a division</div> and after it*, then plain.

## In a list

- An item with <li>an item of the author's</li> in it
- An item with <div><li>an item in a division</li></div> in it
- An item with <dl><dt>a term</dt><dd>its meaning</dd></dl> in it, which may stand

1. A numbered item with <li>another item</li> in it

## In a table

| Head <tr> row | Head <td> cell |
| --- | --- |
| A row <tr> inside | A cell <td> inside |
| A body <tbody> inside | A caption <caption>caption</caption> inside |
| A column <col> inside | A list <ul><li>in a cell</li></ul> which may stand |

## In a link and a heading

A link [with <a href="https://example.org/inner">a link of the author's</a> inside](https://example.org/outer) and after it.

An author's link <a href="https://example.org/a">with [a link of md2html's](https://example.org/b) inside, *emphasis* and a note[^1]</a> and after it.

### A heading with <h4>a heading</h4> inside it

## In the author's drawing and table

Text <svg viewBox="0 0 10 10"><text>`code` and *emphasis* in a drawing</text></svg> then <div>a division</div> after it.

Text <svg viewBox="0 0 10 10"> **strong** <div>a division</div></svg> after it.

<table><tr>

A paragraph of md2html's in the author's table, with *emphasis*.

</table>

<ruby>

A paragraph <rt>ruby text</rt> under a ruby.

</ruby>

> A quote with <li>an item</li> in it.

> [!NOTE]
> A callout with <li>an item</li> and <div>a division</div> in it.

[^1]: A note with <div>a division</div> in it.
