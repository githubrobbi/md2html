// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC
//
// The review page of one language (scripts/i18n/review_site.py writes it,
// review.css styles it). Every question is a block with a state, sent on
// its own: someone with a minute answers two and leaves, and both count.
//
//   not answered   nothing chosen yet
//   not sent       an answer, not sent yet (or changed since it was sent)
//   sent           the answer on the server is the one shown; its tag
//                  resets the question, and withdraws the answer sent
//
// About the reviewer (name, how to reach them, how well they know the
// language) is asked once, at the top, and goes with every answer. All of
// it is kept in this browser, so a reviewer can stop and come back. The
// first send of a visit passes Cloudflare Turnstile once; the server
// answers with a session that lets the next sends go at once.
'use strict';

const D = JSON.parse(document.getElementById('data').textContent);
const KEYS = { client: 'md2html-review-client', me: 'md2html-review-me', session: 'md2html-review-session', state: `md2html-review-${D.code}` };
const store = {
  get(key, fallback) {
    try {
      const raw = localStorage.getItem(key);
      return raw === null ? fallback : JSON.parse(raw);
    } catch {
      return fallback;
    }
  },
  set(key, value) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // A browser that keeps nothing still sends; it only forgets.
    }
  },
};

/** This browser's anonymous id: a reviewer's later answer replaces their earlier one. */
function client() {
  let id = store.get(KEYS.client, null);
  if (!id) {
    id = crypto.randomUUID();
    store.set(KEYS.client, id);
  }
  return id;
}

// ── About the reviewer ────────────────────────────────────────────────
const meForm = document.getElementById('me');
function me() {
  return store.get(KEYS.me, {});
}
for (const el of meForm.querySelectorAll('[name]')) {
  el.value = me()[el.name] ?? '';
  el.addEventListener('input', () => {
    const now = me();
    now[el.name] = el.value;
    store.set(KEYS.me, now);
  });
}

// ── The questions ─────────────────────────────────────────────────────
let state = store.get(KEYS.state, {});
const cards = [...document.querySelectorAll('.q')];
const keyOf = (card) => `${card.dataset.kind}:${card.dataset.item}`;
const same = (a, b) => JSON.stringify(a ?? null) === JSON.stringify(b ?? null);

/** The answer a card shows now. */
function read(card) {
  const answer = {};
  for (const el of card.querySelectorAll('[name]')) {
    const field = el.name.split('|')[2];
    if (el.type === 'radio') {
      if (el.checked) answer[field] = el.value;
    } else if (el.value.trim()) {
      answer[field] = el.value;
    }
  }
  return answer;
}

/** Whether an answer says enough to be sent. */
function complete(card, a) {
  switch (card.dataset.kind) {
    case 'term':
      return Boolean(a.choice) && (a.choice !== 'other' || Boolean(a.other?.trim()));
    case 'string':
      return Boolean(a.verdict) && (a.verdict !== 'change' || (Boolean(a.text?.trim()) && a.text !== card.dataset.now));
    default:
      return Boolean(a.text?.trim());
  }
}

/** Shows `answer` in a card (nothing: as the page came). */
function fill(card, answer) {
  for (const el of card.querySelectorAll('[name]')) {
    const field = el.name.split('|')[2];
    if (el.type === 'radio') el.checked = answer?.[field] === el.value;
    else el.value = answer?.[field] ?? el.defaultValue;
  }
}

// The page's own words (review_site.UI, in the page's language).
const T = D.ui;
const fillIn = (text, values) => Object.entries(values).reduce((out, [name, value]) => out.replace(`{${name}}`, value), text);
const LABELS = { open: T.state_open, ready: T.state_ready, changed: T.state_changed, sent: T.state_sent };

/** A card's state, its tag, its button and its open boxes. */
function paint(card) {
  const kept = state[keyOf(card)] ?? {};
  const answer = read(card);
  const done = complete(card, answer);
  let now = 'open';
  if (kept.sent && same(kept.sent, answer)) now = 'sent';
  else if (done) now = kept.sent ? 'changed' : 'ready';
  else if (kept.sent) now = 'changed';
  card.classList.remove('st-open', 'st-ready', 'st-changed', 'st-sent');
  card.classList.add(`st-${now}`);
  const tag = card.querySelector('.q-state');
  tag.textContent = LABELS[now];
  tag.title = now === 'sent' ? T.reset_title : '';
  card.querySelector('.send').disabled = now === 'sent';
  for (const box of card.querySelectorAll('[data-when]')) {
    const [field, value] = box.dataset.when.split('=');
    box.classList.toggle('hidden', answer[field] !== value);
  }
}

// The questions asked: those we are sure of, folded away, and the free
// words at the end (optional) are not counted.
const asked = cards.filter((card) => !card.classList.contains('sure-one') && card.dataset.kind !== 'general');

function count() {
  const sent = asked.filter((card) => card.classList.contains('st-sent')).length;
  document.getElementById('sent').textContent = fillIn(T.sent_count, { sent, count: asked.length });
}

function remember(card) {
  const key = keyOf(card);
  state[key] = { ...(state[key] ?? {}), draft: read(card) };
  store.set(KEYS.state, state);
}

for (const card of cards) {
  const kept = state[keyOf(card)];
  if (kept) fill(card, kept.draft ?? kept.sent);
  paint(card);
  card.addEventListener('input', () => {
    remember(card);
    paint(card);
    count();
  });
  card.querySelector('.send').addEventListener('click', () => send(card));
  card.querySelector('.q-state').addEventListener('click', () => {
    if (card.classList.contains('st-sent')) reset(card);
  });
}
count();

document.getElementById('only-open').addEventListener('change', (e) => {
  document.body.classList.toggle('only-open', e.target.checked);
});

// ── Sending ───────────────────────────────────────────────────────────
let widget = null;
let waiting = null;
let ready = null;

/** Turnstile, loaded once, on the first send. */
function turnstileReady() {
  ready ??= new Promise((resolve, reject) => {
    window.onTurnstile = resolve;
    const script = document.createElement('script');
    script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit&onload=onTurnstile';
    script.async = true;
    script.onerror = () => reject(new Error('the spam check could not load'));
    document.head.append(script);
  });
  return ready;
}

/** A fresh Turnstile token; it shows a check only when Cloudflare asks for one. */
async function token() {
  await turnstileReady();
  return new Promise((resolve, reject) => {
    waiting = { resolve, reject };
    if (widget === null) {
      widget = window.turnstile.render('#check', {
        sitekey: D.sitekey,
        execution: 'execute',
        appearance: 'interaction-only',
        callback: (t) => { waiting?.resolve(t); waiting = null; },
        'error-callback': () => { waiting?.reject(new Error('the spam check did not pass')); waiting = null; },
      });
    } else {
      window.turnstile.reset(widget);
    }
    window.turnstile.execute(widget);
  });
}

/** The session the server gave this visit, or a new one after a Turnstile check. */
async function session() {
  const kept = store.get(KEYS.session, null);
  if (kept && kept.client === client() && kept.expires > Date.now() / 1000 + 60) return kept.token;
  const response = await fetch('/api/session', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ token: await token(), client: client() }),
  });
  if (!response.ok) throw new Error(await response.text());
  const fresh = await response.json();
  store.set(KEYS.session, { ...fresh, client: client() });
  return fresh.token;
}

/** Sends one card's answer (null withdraws it); a lapsed session is renewed once. */
async function post(card, answer) {
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const response = await fetch('/api/answer', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        session: await session(), client: client(), lang: D.code,
        kind: card.dataset.kind, item: card.dataset.item, answer, reviewer: me(),
      }),
    });
    if (response.status === 401) {
      store.set(KEYS.session, null);
      continue;
    }
    if (!response.ok) throw new Error(await response.text());
    return;
  }
  throw new Error('the spam check did not pass');
}

function say(card, text, error) {
  const note = card.querySelector('.q-note');
  note.textContent = text;
  note.classList.toggle('error', Boolean(error));
}

// What a card still needs before it can be sent, in words.
const MISSING = { term: T.missing_term, string: T.missing_string, general: T.missing_text };

async function send(card) {
  const answer = read(card);
  if (!complete(card, answer)) {
    say(card, MISSING[card.dataset.kind] ?? MISSING.general, true);
    return;
  }
  const button = card.querySelector('.send');
  button.disabled = true;
  say(card, T.sending);
  try {
    await post(card, answer);
    state[keyOf(card)] = { sent: answer, draft: answer };
    store.set(KEYS.state, state);
    say(card, T.arrived);
  } catch (err) {
    say(card, fillIn(T.failed, { error: err.message }), true);
  }
  paint(card);
  count();
}

async function reset(card) {
  say(card, T.withdrawing);
  try {
    await post(card, null);
    delete state[keyOf(card)];
    store.set(KEYS.state, state);
    fill(card, null);
    say(card, T.withdrawn);
  } catch (err) {
    say(card, fillIn(T.not_withdrawn, { error: err.message }), true);
  }
  paint(card);
  count();
}
