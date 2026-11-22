// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025-2026 Sky, LLC.
// Accessibility audit (axe-core, WCAG 2.0-2.2 A and AA, best practice) of rendered
// pages, in both themes and at phone width; exits 1 on any violation, any sideways
// scroll at phone width, or any script error.
// usage: node audit.js page.html...
const { chromium } = require('playwright-core');
const axeSource = require('fs').readFileSync(require.resolve('axe-core/axe.min.js'), 'utf8');
const pages = process.argv.slice(2).map(p => require('path').resolve(p));
let failed = false;
(async () => {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const totals = {};
  for (const file of pages) {
    for (const [theme, width] of [['light', 1400], ['dark', 1400], ['light', 390]]) {
      const ctx = await browser.newContext({ viewport: { width, height: 900 } });
      await ctx.addInitScript(t => { try { localStorage.setItem('iv-theme', t); } catch (e) {} }, theme);
      const page = await ctx.newPage();
      const errors = [];
      page.on('pageerror', e => errors.push('pageerror: ' + e.message));
      page.on('console', m => { if (m.type() === 'error') errors.push('console: ' + m.text()); });
      await page.goto('file://' + file);
      await page.waitForTimeout(300);
      await page.addScriptTag({ content: axeSource });
      const res = await page.evaluate(async () => await axe.run(document, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa', 'best-practice'] },
        resultTypes: ['violations'] }));
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      const label = `${file.split('/').slice(-3).join('/')} [${theme} ${width}]`;
      const v = res.violations.map(x => `${x.impact} ${x.id} x${x.nodes.length}: ${x.nodes[0].target.join(' ')} :: ${(x.nodes[0].failureSummary||'').split('\n')[1]||''}`);
      console.log(`\n== ${label}: ${res.violations.length} violation kinds${overflow > 0 ? `, OVERFLOW ${overflow}px` : ''}${errors.length ? ', ERRORS ' + errors.join(' | ') : ''}`);
      v.forEach(s => console.log('   ' + s.slice(0, 260)));
      res.violations.forEach(x => totals[x.id] = (totals[x.id] || 0) + x.nodes.length);
      if (res.violations.length || overflow > 0 || errors.length) failed = true;
      await ctx.close();
    }
  }
  console.log('\nTOTALS', JSON.stringify(totals));
  await browser.close();
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); process.exit(1); });
