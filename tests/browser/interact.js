// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// Click through rendered pages the way a reader would, in Chrome: search by
// keyboard and mouse, theme, text size, outline and drawer, figure, footnote,
// glossary, copy, print, phone width. Exits 1 on any failed check.
// usage: node interact.js page.html...
const { chromium } = require('playwright-core');
const files = process.argv.slice(2).map(p => require('path').resolve(p));
let failures = 0;
let results = [];
function check(name, ok, detail) { if (!ok) failures++; results.push((ok ? 'PASS ' : 'FAIL ') + name + (ok || !detail ? '' : ' :: ' + detail)); }
(async () => {
  const browser = await chromium.launch({ channel: 'chrome' });
  for (const file of files) {
  results = [];
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 }, permissions: ['clipboard-read', 'clipboard-write'] });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto('file://' + file);
  await page.waitForTimeout(200);
  const mod = process.platform === 'darwin' ? 'Meta' : 'Control';

  // Keyboard: the first Tab reaches the skip link.
  await page.keyboard.press('Tab');
  check('first Tab lands on the skip link', await page.evaluate(() => document.activeElement.classList.contains('skip')));

  // Search by keyboard.
  await page.keyboard.press(mod + '+k');
  check('Cmd/Ctrl-K opens search', await page.evaluate(() => document.getElementById('find').open));
  check('the search field has the focus', await page.evaluate(() => document.activeElement.id === 'q'));
  const word = await page.evaluate(() => (document.querySelector('article h2, article p').textContent.match(/[A-Za-zÄÖÜäöüß]{5,}/) || ['the'])[0]);
  await page.keyboard.type(word);
  await page.waitForTimeout(100);
  const n = await page.evaluate(() => document.querySelectorAll('#hits li[role=option]').length);
  check('typing finds results', n > 0, 'hits=' + n);
  check('the result count is announced', (await page.textContent('#hits-n')).length > 0);
  check('the combobox points at the chosen result', await page.evaluate(() => document.getElementById('q').getAttribute('aria-activedescendant') === 'hit-0'));
  await page.keyboard.press('ArrowDown');
  check('ArrowDown moves the choice', await page.evaluate(() => document.getElementById('q').getAttribute('aria-activedescendant') === 'hit-1' || document.querySelectorAll('#hits li[role=option]').length < 2));
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(150);
  check('Enter closes search and jumps', await page.evaluate(() => !document.getElementById('find').open && location.hash.length > 1), await page.evaluate(() => location.hash));
  check('the words found are lit where it lands', await page.evaluate(() => document.querySelectorAll('mark.hit').length > 0));
  await page.keyboard.press('/');
  check('/ opens search', await page.evaluate(() => document.getElementById('find').open));
  await page.keyboard.press('Escape');
  check('Escape closes search', await page.evaluate(() => !document.getElementById('find').open));
  await page.click('#open-find');
  check('the Search button opens search', await page.evaluate(() => document.getElementById('find').open));
  await page.keyboard.type('zzqqxx');
  await page.waitForTimeout(80);
  check('no result says so', (await page.textContent('#hits')).length > 5);
  await page.mouse.click(5, 5);
  check('a click on the backdrop closes search', await page.evaluate(() => !document.getElementById('find').open));

  // Theme: system -> light -> dark -> system.
  const modes = [];
  for (let i = 0; i < 3; i++) { await page.click('#theme'); modes.push(await page.evaluate(() => document.documentElement.getAttribute('data-theme') || 'system')); }
  check('the theme cycles light, dark, system', modes.join(',') === 'light,dark,system', modes.join(','));
  // Text size.
  await page.click('#size-up');
  check('+ makes the text larger', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1.15'));
  await page.keyboard.press(mod + '+Equal');
  check('Cmd/Ctrl + steps it again', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1.3'));
  await page.click('#size');
  check('the number returns to 100%', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1'));
  // Remembered across a reload.
  await page.click('#theme');
  await page.reload();
  check('the theme is remembered', await page.evaluate(() => document.documentElement.getAttribute('data-theme') === 'light'));
  await page.click('#theme'); await page.click('#theme');

  // The outline.
  const hasToc = await page.evaluate(() => !document.documentElement.classList.contains('no-toc'));
  if (hasToc) {
    await page.click('#toggle');
    check('the switch hides the outline', await page.evaluate(() => document.documentElement.getAttribute('data-toc') === 'off'));
    await page.hover('#edge'); await page.waitForTimeout(350);
    check('resting on the rail opens the drawer', await page.evaluate(() => document.querySelector('nav.toc').classList.contains('open')));
    await page.keyboard.press('Escape');
    check('Escape closes the drawer', await page.evaluate(() => !document.querySelector('nav.toc').classList.contains('open')));
    await page.keyboard.press(mod + '+Backslash');
    check('Cmd/Ctrl-\\ brings it back', await page.evaluate(() => document.documentElement.getAttribute('data-toc') !== 'off'));
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight / 2));
    await page.waitForTimeout(150);
    check('the outline marks where the reader is', await page.evaluate(() => !!document.querySelector('nav.toc a.on[aria-current]')));
    check('the bar names the chapter', (await page.textContent('#crumb')).trim().length > 0);
  }

  // A figure.
  if (await page.$('figure.fig > a')) {
    await page.click('figure.fig > a');
    await page.waitForTimeout(300);
    check('a click enlarges the figure', await page.evaluate(() => { const d = document.querySelector('dialog.zoom'); return !!d && d.open; }));
    await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    check('Escape puts it back', await page.evaluate(() => !document.querySelector('dialog.zoom').open));
  }
  // A footnote.
  if (await page.$('a.fnref')) {
    const ref = await page.$eval('a.fnref', a => a.id);
    await page.click('a.fnref');
    await page.waitForTimeout(100);
    check('a footnote number jumps to its note', await page.evaluate(() => location.hash.startsWith('#fn-')));
    check("the note's back returns to that reference", await page.evaluate(r => document.querySelector(location.hash + ' a.back').getAttribute('href') === '#' + r, ref));
  }
  // A glossary word: its meaning on focus, gone on Escape.
  if (await page.$('a.term')) {
    await page.focus('a.term');
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Tab');
    const shown = await page.evaluate(() => getComputedStyle(document.querySelector('a.term'), '::after').display);
    check("a glossary word shows its meaning on keyboard focus", shown === 'block', shown);
    await page.keyboard.press('Escape');
    check('Escape takes the meaning away', await page.evaluate(() => !document.activeElement.classList.contains('term')));
  }
  // Copy.
  if (await page.$('.copy')) {
    await page.hover('.code'); await page.click('.copy'); await page.waitForTimeout(150);
    const clip = await page.evaluate(() => navigator.clipboard.readText());
    check('copy puts the code on the clipboard', clip.length > 0);
    check('the button says it copied', await page.evaluate(() => document.querySelector('.copy').classList.contains('done')));
  }
  // A heading's anchor.
  check('every heading in the outline has its anchor', await page.evaluate(() => [...document.querySelectorAll('nav.toc a[href^="#"]')].every(a => document.getElementById(decodeURIComponent(a.getAttribute('href').slice(1))))));
  // Print.
  await page.emulateMedia({ media: 'print' });
  const pr = await page.evaluate(() => [getComputedStyle(document.querySelector('header.bar')).display, getComputedStyle(document.querySelector('nav.toc')).display, matchMedia('print').matches].join(','));
  check('print hides the controls', pr.startsWith('none,none'), pr);
  await page.emulateMedia({ media: 'screen' });
  // Phone width.
  await page.setViewportSize({ width: 390, height: 800 });
  await page.waitForTimeout(100);
  check('no sideways scroll at phone width', await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), String(await page.evaluate(() => document.documentElement.scrollWidth)));
  if (hasToc) {
    await page.click('#toggle');
    check('on a phone the switch opens the outline over the page', await page.evaluate(() => document.querySelector('nav.toc').classList.contains('open')));
    await page.click('nav.toc ol a');
    check('choosing a chapter closes it', await page.evaluate(() => !document.querySelector('nav.toc').classList.contains('open')));
  }
  check('no script error', errors.length === 0, errors.join(' | '));
  console.log(file.split('/').slice(-3).join('/') + '\n  ' + results.join('\n  '));
  await ctx.close();
  }
  await browser.close();
  process.exit(failures ? 1 : 0);
})().catch(e => { console.log(results.join('\n')); console.error('CRASH', e.message.split('\n')[0]); process.exit(1); });
