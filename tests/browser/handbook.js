// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
// A handbook's pages in Chrome, opened from file:// the way a reader opens
// them: every page shows the handbook's contents with itself marked, the
// next and previous links walk the contents' order from the first chapter
// to the last, an entry of the contents opens its page, and the outline of
// the page's own sections still follows the reader. Exits 1 on any failed
// check.
// usage: node handbook.js first-chapter.html
const { chromium } = require('playwright-core');
const path = require('path');
const first = path.resolve(process.argv[2] || '');
let failures = 0;
const results = [];
function check(name, ok, detail) { if (!ok) failures++; results.push((ok ? 'PASS ' : 'FAIL ') + name + (ok || !detail ? '' : ' :: ' + detail)); }
(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto('file://' + first);
  await page.waitForTimeout(150);
  // The contents, as the first chapter lists them: where each entry leads.
  const order = await page.evaluate(() => [...document.querySelectorAll('nav.toc ol.book a')].map(a => a.href));
  check('the first chapter shows the contents', order.length > 1, 'entries=' + order.length);
  check('the first entry is the first chapter', order[0] === page.url(), order[0] + ' vs ' + page.url());
  // Next, from the first chapter to the last: each page marks itself, and
  // the walk follows the contents.
  const walked = [];
  for (let step = 0; step < order.length + 1; step++) {
    const here = page.url();
    walked.push(here);
    const marked = await page.evaluate(() => {
      const a = document.querySelector('nav.toc ol.book a[aria-current=page]');
      return a ? a.href : '';
    });
    check('page ' + (step + 1) + ' marks itself in the contents', marked === here, marked + ' vs ' + here);
    const next = await page.$('nav.pages a.next');
    if (!next) break;
    await Promise.all([page.waitForNavigation(), next.click()]);
    await page.waitForTimeout(60);
  }
  check('next walks the contents, first to last', JSON.stringify(walked) === JSON.stringify(order), walked.length + ' pages walked, ' + order.length + ' listed');
  // Previous leads back.
  const prev = await page.$('nav.pages a.prev');
  if (prev && walked.length > 1) {
    await Promise.all([page.waitForNavigation(), prev.click()]);
    check('previous leads back one page', page.url() === walked[walked.length - 2], page.url());
  }
  // An entry of the contents opens its page.
  const target = order[Math.floor(order.length / 2)];
  await Promise.all([page.waitForNavigation(), page.evaluate(t => [...document.querySelectorAll('nav.toc ol.book a')].find(a => a.href === t).click(), target)]);
  check('an entry of the contents opens its page', page.url() === target, page.url() + ' vs ' + target);
  // The outline of the page's own sections still follows the reader.
  const heads = await page.evaluate(() => document.querySelectorAll('article h2[id]').length);
  if (heads > 1) {
    await page.evaluate(() => { const h = [...document.querySelectorAll('article h2[id]')].pop(); scrollTo(0, h.offsetTop - 40); });
    await page.waitForTimeout(150);
    const lit = await page.evaluate(() => { const a = document.querySelector('nav.toc > ol:not(.book) a.on'); return a ? a.getAttribute('href') : ''; });
    const last = await page.evaluate(() => '#' + [...document.querySelectorAll('article h2[id]')].pop().id);
    check('the outline marks the section read, under the contents', lit === last, lit + ' vs ' + last);
    check('the crumb names it', (await page.textContent('#crumb')).length > 0);
  }
  check('no script error', errors.length === 0, errors.join(' | '));
  await browser.close();
  console.log(results.join('\n'));
  console.log(failures ? failures + ' failed' : 'handbook: all passed');
  process.exit(failures ? 1 : 0);
})().catch(e => { console.error(e); process.exit(1); });
