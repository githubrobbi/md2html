// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
// The article as md2html wrote it is the article Chrome reads: every element
// nested where its tags put it, none ended early by a tag inside it (a <div>
// in md2html's <p>, a <li> in its list item, an <a> in its link), none moved
// out (out of a table, out of a drawing). The tags of the page's <article>
// are read as written, each element inside the last one open, and compared
// with the elements of Chrome's article, in order and depth. Exits 1 on any
// difference.
// usage: node nesting.js page.html...
const { chromium } = require('playwright-core');
const fs = require('fs');
const path = require('path');

const VOID = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta',
  'param', 'source', 'track', 'wbr']);
const RAW = new Set(['script', 'style', 'textarea']);
// Chrome adds a body to a table that has rows and no body, and a column
// group around a bare column: they are left out on both sides.
const ADDED = new Set(['tbody', 'colgroup']);

// The article's elements as its tags nest them: [depth, name] in order.
function asWritten(html) {
  const start = html.indexOf('<article');
  const end = html.lastIndexOf('</article>');
  if (start < 0 || end < start) throw new Error('no article');
  const text = html.slice(start, end + '</article>'.length);
  const tag = /<!--[\s\S]*?-->|<(\/?)([a-zA-Z][a-zA-Z0-9-]*)((?:[^>"']|"[^"]*"|'[^']*')*?)(\/?)>/g;
  const out = [];
  const open = [];
  let depth = 0;
  let m;
  while ((m = tag.exec(text))) {
    if (m[0].startsWith('<!--')) continue;
    const [, closing, rawName, , selfClosing] = m;
    const name = rawName.toLowerCase();
    if (closing) {
      const at = open.lastIndexOf(name);
      if (at < 0) throw new Error(`</${name}> closes nothing at ${m.index}`);
      if (at !== open.length - 1) throw new Error(`</${name}> crosses <${open[open.length - 1]}> at ${m.index}`);
      open.pop();
      if (!ADDED.has(name)) depth--;
      continue;
    }
    if (!ADDED.has(name)) out.push([depth, name]);
    if (VOID.has(name) || selfClosing) continue;
    if (RAW.has(name)) {
      const close = text.toLowerCase().indexOf(`</${name}`, tag.lastIndex);
      tag.lastIndex = close < 0 ? text.length : close;
      // Its end tag is read next.
    }
    open.push(name);
    if (!ADDED.has(name)) depth++;
  }
  if (open.length) throw new Error(`left open: ${open.join(' ')}`);
  return out;
}

(async () => {
  const files = process.argv.slice(2).map(p => path.resolve(p));
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  let failed = false;
  for (const file of files) {
    const written = asWritten(fs.readFileSync(file, 'utf8'));
    const page = await browser.newPage();
    await page.goto('file://' + file);
    const read = await page.evaluate(added => {
      const out = [];
      const walk = (el, depth) => {
        const name = el.localName;
        const skip = added.includes(name);
        if (!skip) out.push([depth, name]);
        for (const kid of el.children) walk(kid, skip ? depth : depth + 1);
      };
      const article = document.querySelector('main > article') || document.querySelector('article');
      walk(article, 0);
      return out;
    }, [...ADDED]);
    await page.close();
    const show = ([d, n]) => '  '.repeat(d) + n;
    let at = 0;
    while (at < written.length && at < read.length && written[at][0] === read[at][0] && written[at][1] === read[at][1]) at++;
    const label = file.split('/').slice(-2).join('/');
    if (at === written.length && at === read.length) {
      console.log(`PASS ${label}: ${written.length} elements nest as written`);
    } else {
      failed = true;
      console.log(`FAIL ${label}: element ${at} of ${written.length} written, ${read.length} read`);
      console.log('  written: ' + written.slice(Math.max(0, at - 3), at + 4).map(show).join(' | '));
      console.log('  read:    ' + read.slice(Math.max(0, at - 3), at + 4).map(show).join(' | '));
    }
  }
  await browser.close();
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); process.exit(1); });
