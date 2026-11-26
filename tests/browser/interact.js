// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// Click through rendered pages the way a reader would, in Chrome: search by
// keyboard and mouse, theme, text size, outline and drawer, figure, diagram,
// footnote, glossary, copy, print, phone width. Exits 1 on any failed check.
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
  const word = await page.evaluate(() => (document.querySelector('article h2, article p').textContent.match(/\p{L}{4,}/u) || ['the'])[0]);
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

  // The reading settings: a panel behind "Aa" with text size, theme and the ways to keep the page.
  await page.click('#aa');
  check('Aa opens the reading settings', await page.evaluate(() => document.getElementById('prefs').matches(':popover-open')));
  const theme = async set => { await page.click(`.themes button[data-set="${set}"]`); return page.evaluate(() => document.documentElement.getAttribute('data-theme') || 'system'); };
  const modes = [await theme('light'), await theme('dark'), await theme('')];
  check('the theme picks light, dark, system', modes.join(',') === 'light,dark,system', modes.join(','));
  check('the theme in force is pressed', await page.evaluate(() => document.querySelector('.themes [aria-pressed=true]').dataset.set === ''));
  await page.click('#size-up');
  check('+ makes the text larger', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1.15'));
  await page.keyboard.press('Escape');
  check('Escape closes the panel', !(await page.evaluate(() => document.getElementById('prefs').matches(':popover-open'))));
  await page.keyboard.press(mod + '+Equal');
  check('Cmd/Ctrl + steps it again', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1.3'));
  await page.click('#aa');
  await page.click('#size');
  check('the number returns to 100%', await page.evaluate(() => document.documentElement.style.getPropertyValue('--zoom') === '1'));
  await page.keyboard.press('Escape');
  await page.click('#save');
  check('the save button opens its own menu', await page.evaluate(() => document.getElementById('keep').matches(':popover-open')));
  check('it offers printing', await page.evaluate(() => !!document.querySelector('#keep #print')));
  await page.keyboard.press('Escape');
  check('Aa holds only the display settings', await page.evaluate(() => !document.querySelector('#prefs .keep')));
  await page.click('#aa');
  await theme('light');
  await page.reload();
  check('the theme is remembered', await page.evaluate(() => document.documentElement.getAttribute('data-theme') === 'light'));
  await page.click('#aa'); await theme(''); await page.keyboard.press('Escape');
  check('every page credits md2html with its version', await page.evaluate(() => /md2html/.test(document.querySelector('.credit a')?.href || '') && /\d+\.\d+\.\d+/.test(document.querySelector('.credit').textContent)));

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
    const shape = await page.evaluate(() => { const i = document.querySelector('dialog.zoom img'); const r = i.getBoundingClientRect(); return [r.width / r.height, i.naturalWidth / i.naturalHeight]; });
    check('the enlargement keeps the image\'s shape', Math.abs(shape[0] / shape[1] - 1) < 0.03, shape.map(x => x.toFixed(2)).join(' vs '));
    await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    check('Escape puts it back', await page.evaluate(() => !document.querySelector('dialog.zoom').open));
  }
  // A diagram enlarges as a picture does: its drawing, copied into the
  // enlargement, in the page's colours, by the pointer and by the keyboard.
  const drawing = 'article figure.diagram > svg';
  if (await page.$(drawing)) {
    const zoomed = () => page.evaluate(() => { const d = document.querySelector('dialog.zoom'); return !!d && d.open && !!d.querySelector('.diagram:not([hidden]) > svg'); });
    await page.evaluate(s => document.querySelector(s).scrollIntoView({ block: 'center' }), drawing);
    await page.click(drawing);
    await page.waitForTimeout(300);
    check('a click enlarges the diagram, its drawing in the enlargement', await zoomed());
    const shape = await page.evaluate(() => { const s = document.querySelector('dialog.zoom .diagram > svg'); if (!s) return null; const r = s.getBoundingClientRect(), v = s.viewBox.baseVal; return [r.width / r.height, v.width / v.height]; });
    check("the enlargement keeps the diagram's shape", !!shape && Math.abs(shape[0] / shape[1] - 1) < 0.03, JSON.stringify(shape));
    check('no name stands twice in the page', await page.evaluate(() => { const ids = [...document.querySelectorAll('[id]')].map(e => e.id); return ids.length === new Set(ids).size; }));
    const mid = await page.evaluate(() => { const s = document.querySelector('dialog.zoom .diagram'); if (!s) return null; const r = s.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; });
    if (mid) { await page.mouse.click(mid[0], mid[1]); await page.waitForTimeout(300); }
    check('a click on the drawing brings it a step closer', await page.evaluate(() => { const s = document.querySelector('dialog.zoom .diagram'); return !!s && /scale\(2\)/.test(s.style.transform); }));
    // The page's colours in the copy, light and dark: the theme changes while it is open.
    const fills = () => page.evaluate(s => { const c = document.querySelector('dialog.zoom .diagram .shape, dialog.zoom .diagram .actor rect'); const o = document.querySelector(s.replace('> svg', '') + ' .shape, ' + s.replace('> svg', '') + ' .actor rect'); return c && o ? [getComputedStyle(c).fill, getComputedStyle(o).fill, getComputedStyle(c.closest('.diagram')).backgroundColor] : null; }, drawing);
    const light = await fills();
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark')); await page.waitForTimeout(80);
    const dark = await fills();
    check("the enlargement takes the dark theme's colours", !!light && !!dark && dark[0] === dark[1] && dark[0] !== light[0] && dark[2] !== light[2], JSON.stringify({ light, dark }));
    await page.evaluate(() => document.documentElement.removeAttribute('data-theme'));
    await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    check('Escape puts the diagram back, and the focus with it', await page.evaluate(s => { const d = document.querySelector('dialog.zoom'); return !!d && !d.open && document.querySelector(s).parentNode.contains(document.activeElement); }, drawing));
    // The keyboard: Tab reaches the diagram's button, Enter opens it.
    const button = await page.$('article figure.diagram > button');
    check("Tab reaches the diagram's button, named in the page's words", !!button && await button.evaluate(b => b.textContent.trim().length > 0));
    if (button) {
      await button.focus(); await page.keyboard.press('Shift+Tab'); await page.keyboard.press('Tab');
      check("the diagram's button shows when it has the focus", await button.evaluate(b => document.activeElement === b && getComputedStyle(b).opacity === '1'));
      await page.keyboard.press('Enter'); await page.waitForTimeout(300);
      check('Enter on it enlarges the diagram', await zoomed());
      await page.keyboard.press('Escape'); await page.waitForTimeout(300);
      check('Escape gives the focus back to the button', await button.evaluate(b => document.activeElement === b && !document.querySelector('dialog.zoom').open));
    }
    // A diagram its author titled: the title under it on the page, styled
    // as a picture's caption, and under the enlargement as a picture's is.
    const titled = 'article figure.diagram > figcaption';
    if (/showcase\.html$/.test(file)) check('the showcase titles its diagrams', !!(await page.$(titled)));
    if (await page.$(titled)) {
      const said = await page.evaluate(s => { const c = document.querySelector(s), st = getComputedStyle(c), svg = c.parentNode.querySelector(':scope > svg'); return { text: c.textContent, size: st.fontSize, below: c.getBoundingClientRect().top >= svg.getBoundingClientRect().bottom - 1 }; }, titled);
      check("a titled diagram's caption stands under it, at a caption's size", said.text.trim().length > 0 && said.below && said.size === '13.5px', JSON.stringify(said));
      await page.evaluate(s => document.querySelector(s).parentNode.scrollIntoView({ block: 'center' }), titled);
      await page.click('article figure.diagram:has(> figcaption) > svg');
      await page.waitForTimeout(300);
      const note = await page.evaluate(() => { const p = document.querySelector('dialog.zoom p'); return p && document.querySelector('dialog.zoom').open ? { text: p.textContent, shown: getComputedStyle(p).display !== 'none' && getComputedStyle(p).opacity === '1' } : null; });
      check("the enlargement of a titled diagram shows its caption", !!note && note.text === said.text && note.shown, JSON.stringify(note));
      await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    }
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
  // The language switch: on a page with translations in every browser; on
  // a page alone only where the browser translates on the device, and then
  // with the languages to translate into.
  const versions = await page.evaluate(() => document.querySelectorAll('#langs ul.authored a').length);
  const translates = await page.evaluate(() => 'Translator' in self);
  if (await page.$('#lang')) {
    check('the globe shows only with something to choose', await page.evaluate(() => !document.getElementById('lang').hidden) === (versions > 1 || translates));
  }
  if (await page.$('#lang') && (versions > 1 || translates)) {
    await page.click('#lang');
    await page.waitForTimeout(100);
    check('the language switch opens', await page.evaluate(() => document.getElementById('langs').matches(':popover-open')));
    check('it lists the versions, this one marked', await page.evaluate(() => !!document.querySelector('#langs a[aria-current]')));
    check('it offers another language', await page.evaluate(() => document.querySelectorAll('#langs ul.authored a').length > 1 || !document.querySelector('#langs .device').hidden));
    check('every version link names its language', await page.evaluate(() => [...document.querySelectorAll('#langs ul.authored a')].every(a => a.getAttribute('hreflang') && a.getAttribute('lang'))));
    await page.keyboard.press('Escape');
    check('Escape closes the switch', !(await page.evaluate(() => document.getElementById('langs').matches(':popover-open'))));
  }
  // Sharing: the # beside a heading copies the section's link; a selection offers a passage link.
  await page.evaluate(() => { window.__copied = []; if (navigator.clipboard) navigator.clipboard.writeText = t => { window.__copied.push(t); return Promise.resolve(); }; });
  const anchor = await page.$('article h2 a.anchor');
  if (anchor) {
    await page.hover('article h2'); await anchor.click(); await page.waitForTimeout(120);
    const copied = await page.evaluate(() => window.__copied.pop() || '');
    check('the # copies the section\'s link', /#[^#]+$/.test(copied) && copied.startsWith(page.url().split('#')[0]), copied);
    check('a notice says the link was copied', await page.evaluate(() => { const t = document.getElementById('toast'); return !!t && t.classList.contains('on') && t.getAttribute('role') === 'status'; }));
  }
  const para = await page.$('article p');
  if (para) {
    // The steps above scroll the page (the heading's # is clicked where it
    // stands): the paragraph is brought into view before its place is read,
    // or the drag starts outside the page and selects the controls instead.
    await para.evaluate(p => p.scrollIntoView({ block: 'center' }));
    const box = await para.boundingBox();
    await page.mouse.move(box.x + 4, box.y + 6); await page.mouse.down(); await page.mouse.move(box.x + Math.min(box.width - 4, 260), box.y + 6); await page.mouse.up();
    const chip = await page.waitForSelector('#passage:not([hidden])', { timeout: 3000 }).catch(() => null);
    check('a selection offers a link to the passage', !!chip);
    if (chip) {
      await chip.click(); await page.waitForTimeout(80);
      const link = await page.evaluate(() => window.__copied.pop() || '');
      check('the passage link is a text fragment', link.includes('#:~:text='), link);
    }
    await page.mouse.click(5, box.y + box.height + 40);
  }
  // A table row: its share symbol appears at its end and copies a link to it.
  const rowAt = await page.evaluate(() => { const tr = [...document.querySelectorAll('article tbody tr[id]')].find(t => { const r = t.getBoundingClientRect(); return r.top > 120 && r.bottom < innerHeight - 40; }); if (!tr) { const any = document.querySelector('article tbody tr[id]'); if (!any) return null; any.scrollIntoView({ block: 'center' }); } const t = tr || document.querySelector('article tbody tr[id]'); const r = t.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2, t.id]; });
  if (rowAt) {
    await page.mouse.move(rowAt[0], rowAt[1]); await page.waitForTimeout(150);
    const btn = await page.$('#rowshare:not([hidden])');
    check('a table row offers its link', !!btn);
    if (btn) { await btn.click(); await page.waitForTimeout(100); const link = await page.evaluate(() => window.__copied.pop() || ''); check('the row link names the row', link.endsWith('#' + encodeURIComponent(rowAt[2])) || decodeURIComponent(link).endsWith('#' + rowAt[2]), link); }
  }
  // At 150% text size (the body is zoomed), a diagram's or a code block's share symbol still sits on it.
  for (const sel of ['article figure.diagram[id]', 'article .code[id]']) {
    const has = await page.evaluate(s => !!document.querySelector(s), sel);
    if (!has) continue;
    await page.evaluate(() => { try { localStorage.setItem('iv-size', '150'); } catch (e) {} document.documentElement.style.setProperty('--zoom', '1.5'); });
    for (let k = 0; k < 3; k++) { await page.evaluate(s => { const el = document.querySelector(s); scrollTo(0, el.getBoundingClientRect().top + scrollY - 140); }, sel); await page.waitForTimeout(250); }
    const r = await page.evaluate(s => { const b = document.querySelector(s).getBoundingClientRect(); return [b.left, b.top, b.right, b.bottom]; }, sel);
    await page.mouse.move((r[0] + r[2]) / 2, r[1] + Math.min(40, (r[3] - r[1]) / 2), { steps: 3 }); await page.waitForTimeout(150);
    const btn = await page.evaluate(() => { const b = document.getElementById('rowshare'); if (!b || b.hidden) return null; const q = b.getBoundingClientRect(); return [q.left, q.top]; });
    check(`at 150%, ${sel.split(' ')[1]} shows its share symbol on it`, !!btn && btn[0] >= r[0] - 2 && btn[0] <= r[2] && btn[1] >= r[1] - 2 && btn[1] <= r[3], JSON.stringify({ r, btn }));
    await page.evaluate(() => { try { localStorage.removeItem('iv-size'); } catch (e) {} document.documentElement.style.setProperty('--zoom', '1'); });
  }
  // A checklist item is one line of text: its words do not split into columns.
  check('a checklist item reads as one paragraph', await page.evaluate(() => [...document.querySelectorAll('li.task')].every(li => getComputedStyle(li).display !== 'flex')));
  // Every table, figure, diagram, code block and callout inside the reading column.
  const outside = await page.evaluate(() => {
    const p = document.querySelector('article > p'); if (!p) return [];
    const col = p.getBoundingClientRect();
    return [...document.querySelectorAll('article .scroll, article figure, article .diagram, article .code, article .callout')]
      .filter(e => { const r = e.getBoundingClientRect(); return r.width && (r.left < col.left - 2 || r.right > innerWidth); })
      .map(e => e.className);
  });
  check('every block sits in the reading column', outside.length === 0, outside.join(', '));
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
