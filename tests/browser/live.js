// SPDX-License-Identifier: MIT OR Apache-2.0
// Live pages, end to end in Chrome: a page reloads when its Markdown changes
// (a file with a time to live, `--serve`, a published page on a plain web
// server) and keeps the reader's place, falls back to the section before,
// or to the top when nothing is left. usage: node live.js [md2html] [workdir]
const { chromium } = require('playwright-core');
const os = require('os'), path = require('path');
const { execFileSync, spawn } = require('child_process');
const fs = require('fs');
const BIN = path.resolve(process.argv[2] || path.join(__dirname, '../../target/release/md2html'));
const DIR = process.argv[3] || fs.mkdtempSync(path.join(os.tmpdir(), 'md2html-live-'));
fs.mkdirSync(DIR, { recursive: true });
const para = (s, i) => `Paragraph ${i} of section ${s}: ` + 'words that fill a line and more words that fill a line. '.repeat(3);
const doc = (sections, extra = {}) => '# Live\n\nIntro.\n\n' + sections.map(s =>
  `## ${s}. Section ${s}\n\n` + (extra[s] || '') + [1, 2, 3, 4, 5, 6].map(i => para(s, i)).join('\n\n')).join('\n\n') + '\n';
const results = [];
const check = (name, ok, detail = '') => { results.push(ok); console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : ' :: ' + detail}`); };

async function topBlock(page) {
  return page.evaluate(() => { for (const el of document.querySelectorAll('article p, article h2')) { const r = el.getBoundingClientRect(); if (r.bottom > 66) return { text: el.textContent.slice(0, 40), top: Math.round(r.top), y: Math.round(scrollY) }; } return null; });
}
async function scrollTo(page, text) {
  await page.evaluate(t => { const el = [...document.querySelectorAll('article p')].find(p => p.textContent.startsWith(t)); scrollTo(0, el.getBoundingClientRect().top + scrollY - 90); }, text);
  await page.waitForTimeout(150);
}
async function waitReload(page, before, ms) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) { await page.waitForTimeout(250); let now; try { now = await page.evaluate(() => window.__mark); } catch (e) { continue; } if (now !== before) return true; }
  return false;
}

(async () => {
  const b = await chromium.launch({ channel: 'chrome' });
  // A. A file opened from disk, with a time to live of 5 s.
  const md = DIR + '/a.md', html = DIR + '/a.html';
  fs.writeFileSync(md, doc([1, 2, 3, 4]));
  execFileSync(BIN, [md, '-o', html, '-q', '--ttl', '5']);
  const p = await b.newPage({ viewport: { width: 1100, height: 700 } });
  await p.goto('file://' + html); await p.evaluate(() => { window.__mark = 1; });
  await scrollTo(p, 'Paragraph 3 of section 3');
  const before = await topBlock(p);
  // Content added above, and the paragraph itself edited slightly.
  fs.writeFileSync(md, doc([1, 2, 3, 4], { 1: 'A new paragraph.\n\nAnother new one.\n\nAnd a third.\n\n' }).replace('Paragraph 3 of section 3: words', 'Paragraph 3 of section 3: changed words'));
  execFileSync(BIN, [md, '-o', html, '-q', '--ttl', '5']);
  check('the page reloads itself when its time is up', await waitReload(p, 1, 9000));
  await p.waitForTimeout(400);
  const after = await topBlock(p);
  check('it keeps the reader\'s place with content added above', after && after.text === before.text && Math.abs(after.top - before.top) < 30 && after.y > before.y + 80, JSON.stringify({ before, after }));
  check('it says the page was updated', await p.evaluate(() => { const t = document.getElementById('toast'); return !!t && /Updated/.test(t.textContent); }));
  // B. The section is gone: the nearest heading before it.
  await p.evaluate(() => { window.__mark = 2; });
  await scrollTo(p, 'Paragraph 4 of section 3');
  fs.writeFileSync(md, doc([1, 2, 4]));
  execFileSync(BIN, [md, '-o', html, '-q', '--ttl', '5']);
  await waitReload(p, 2, 9000); await p.waitForTimeout(400);
  const gone = await topBlock(p);
  check('a section that is gone sends the reader to the one before', gone && /Section 2|section 2/.test(gone.text), JSON.stringify(gone));
  // C. Chaos: nothing is the same.
  await p.evaluate(() => { window.__mark = 3; });
  fs.writeFileSync(md, '# Other\n\n' + Array.from({ length: 40 }, (_, i) => `Totally different line ${i}.`).join('\n\n') + '\n');
  execFileSync(BIN, [md, '-o', html, '-q', '--ttl', '5']);
  await waitReload(p, 3, 9000); await p.waitForTimeout(400);
  check('a page changed beyond recognition starts from the top', await p.evaluate(() => scrollY < 5), String(await p.evaluate(() => scrollY)));
  await p.close();

  // D. md2html --serve: reloads at once on save.
  const smd = DIR + '/s.md';
  fs.writeFileSync(smd, doc([1, 2, 3]));
  const port = 8731;
  const srv = spawn(BIN, [smd, '--serve', '--port', String(port), '-q'], { stdio: 'ignore' });
  await new Promise(r => setTimeout(r, 800));
  const q = await b.newPage({ viewport: { width: 1100, height: 700 } });
  await q.goto(`http://127.0.0.1:${port}/s.html`); await q.evaluate(() => { window.__mark = 1; });
  await scrollTo(q, 'Paragraph 2 of section 2');
  const sb = await topBlock(q);
  const t0 = Date.now();
  fs.writeFileSync(smd, doc([1, 2, 3], { 1: 'Inserted above.\n\n' }));
  const fast = await waitReload(q, 1, 5000);
  check('--serve reloads the page on save, within 2 seconds', fast && Date.now() - t0 < 2500, `${Date.now() - t0} ms`);
  await q.waitForTimeout(400);
  const sa = await topBlock(q);
  check('--serve keeps the place too', sa && sa.text === sb.text && Math.abs(sa.top - sb.top) < 30 && sa.y > sb.y + 20, JSON.stringify({ sb, sa }));
  // The server answers this computer's names only, and nothing outside the folder.
  const foreign = await q.evaluate(async p => { try { const r = await fetch(`http://127.0.0.1:${p}/../../etc/hosts`); return r.status; } catch (e) { return 'err'; } }, port);
  check('--serve gives nothing outside the folder', foreign === 404, String(foreign));
  await q.close(); srv.kill();
  // E. A published page with --live, on an ordinary web server: it checks its fingerprint when its time is up.
  const wdir = DIR + '/www'; fs.mkdirSync(wdir, { recursive: true });
  const wmd = wdir + '/w.md';
  fs.writeFileSync(wmd, doc([1, 2, 3]));
  execFileSync(BIN, [wmd, '-q', '--ttl', '5']);
  const web = spawn('python3', ['-I', '-m', 'http.server', '8742', '--bind', '127.0.0.1', '--directory', wdir], { stdio: 'ignore' });
  await new Promise(r => setTimeout(r, 900));
  const w = await b.newPage({ viewport: { width: 1100, height: 700 } });
  await w.goto('http://127.0.0.1:8742/w.html'); await w.evaluate(() => { window.__mark = 1; });
  await scrollTo(w, 'Paragraph 2 of section 2');
  const wb = await topBlock(w);
  await w.waitForTimeout(6000);
  check('an unchanged published page does not reload', await w.evaluate(() => window.__mark === 1));
  fs.writeFileSync(wmd, doc([1, 2, 3], { 1: 'New above.\n\nAnd more.\n\n' }));
  execFileSync(BIN, [wmd, '-q', '--ttl', '5']);
  check('a changed published page reloads when its time is up', await waitReload(w, 1, 9000));
  await w.waitForTimeout(400);
  const wa = await topBlock(w);
  check('and keeps the place', wa && wa.text === wb.text && Math.abs(wa.top - wb.top) < 30, JSON.stringify({ wb, wa }));
  await w.close(); web.kill();
  await b.close();
  process.exit(results.every(Boolean) ? 0 : 1);
})();
