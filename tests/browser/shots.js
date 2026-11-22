// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// The screenshots of the capabilities page (docs/capabilities.md), taken from the
// showcase and the README in Chrome. usage: node shots.js <repo>
const { chromium } = require('playwright-core');
const path = require('path');
const repo = path.resolve(process.argv[2] || '../..');
const out = n => path.join(repo, 'docs/capabilities', n + '.jpg');
const showcase = 'file://' + path.join(repo, 'docs/showcase.html');
(async () => {
  const b = await chromium.launch({ channel: 'chrome' });
  async function open(theme, w, h, url = showcase) {
    const ctx = await b.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 2 });
    await ctx.addInitScript(t => { try { if (t) localStorage.setItem('iv-theme', t); } catch (e) {} }, theme);
    const p = await ctx.newPage(); await p.goto(url); await p.waitForTimeout(300); return [ctx, p];
  }
  const shot = async (p, name, opts = {}) => p.screenshot({ path: out(name), type: 'jpeg', quality: 78, ...opts });
  let [c, p] = await open('light', 1280, 800); await shot(p, 'light'); await c.close();
  [c, p] = await open('dark', 1280, 800);
  await p.evaluate(() => document.getElementById('3-code-maths-and-tables').scrollIntoView()); await p.waitForTimeout(200);
  await shot(p, 'dark-code'); await c.close();
  [c, p] = await open('light', 390, 844); await shot(p, 'phone'); await c.close();
  [c, p] = await open('light', 1280, 800);
  await p.keyboard.press('Meta+k'); await p.keyboard.type('callout'); await p.waitForTimeout(200); await shot(p, 'search');
  await p.keyboard.press('Enter'); await p.waitForTimeout(400); await shot(p, 'search-landed'); await c.close();
  [c, p] = await open('light', 1280, 800);
  await p.click('figure.fig > a'); await p.waitForTimeout(600); await shot(p, 'zoom'); await c.close();
  [c, p] = await open('light', 1280, 800);
  await p.evaluate(() => document.querySelector('a.fnref').scrollIntoView({ block: 'center' })); await p.hover('a.fnref'); await p.waitForTimeout(200);
  const r = await p.evaluate(() => { const e = document.querySelector('a.fnref').getBoundingClientRect(); return { x: Math.max(0, e.x - 420), y: Math.max(0, e.y - 120), width: 760, height: 260 }; });
  await shot(p, 'footnote', { clip: r });
  await p.evaluate(() => document.querySelector('a.term').scrollIntoView({ block: 'center' })); await p.hover('a.term'); await p.waitForTimeout(200);
  const t = await p.evaluate(() => { const e = document.querySelector('a.term').getBoundingClientRect(); return { x: Math.max(0, e.x - 300), y: Math.max(0, e.y - 100), width: 760, height: 260 }; });
  await shot(p, 'glossary', { clip: t }); await c.close();
  // Aa: the text size and the theme.
  [c, p] = await open('light', 1280, 800);
  await p.click('#aa'); await p.waitForTimeout(250);
  const pr = await p.evaluate(() => { const e = document.getElementById('prefs').getBoundingClientRect(); return { x: Math.max(0, e.x - 420), y: 0, width: Math.min(1280 - Math.max(0, e.x - 420), e.width + 440), height: e.bottom + 24 }; });
  await shot(p, 'panel', { clip: pr }); await c.close();
  // A selection offers a link to the passage.
  [c, p] = await open('light', 1280, 800);
  await p.evaluate(() => { const el = document.querySelector('article > p'); el.scrollIntoView({ block: 'center' }); });
  await p.waitForTimeout(200);
  const pp = await p.evaluate(() => { const r = document.querySelector('article > p').getBoundingClientRect(); return [r.x, r.y]; });
  await p.mouse.move(pp[0] + 4, pp[1] + 10); await p.mouse.down(); await p.mouse.move(pp[0] + 330, pp[1] + 10); await p.mouse.up(); await p.waitForTimeout(250);
  const sh = await p.evaluate(() => { const r = document.querySelector('article > p').getBoundingClientRect(); return { x: Math.max(0, r.x - 40), y: Math.max(0, r.y - 80), width: Math.min(900, 1280 - Math.max(0, r.x - 40)), height: 170 }; });
  await shot(p, 'share', { clip: sh }); await c.close();
  [c, p] = await open('light', 1000, 1300); await p.emulateMedia({ media: 'print' }); await p.waitForTimeout(200); await shot(p, 'print'); await c.close();
  await b.close();
  console.log('shots written to ' + path.join(repo, 'docs/capabilities'));
})().catch(e => { console.error(e); process.exit(1); });
