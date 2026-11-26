---
title: Amounts in the reader's currency
description: A page whose amounts are in yen, with the exchange rates of one day beside it, so its reader can see them in their own currency.
kicker: Currency
currency: JPY
rates: currency.rates.json
---

<!--
SPDX-License-Identifier: MIT OR Apache-2.0
Copyright (c) 2026 Sky, LLC
-->

# Amounts in the reader's currency

This page is a budget in yen, and it asks for a currency switch. The button beside the globe offers your currency: the one of the language you read in first (US dollars in English, euros in German), every other currency of the rates under "More currencies". The page shows its amounts as written until you pick one.

## A month in Tokyo

A student's month costs about ¥215,000. The room is ¥85,000, paid in cash, and food is planned at ¥19,600 a week; the konbini alone ran ¥37,000–48,000 a month this autumn.

| Line | A month |
|---|---:|
| Room, single | ¥85,000 |
| Phone (eSIM) | ¥4,600 |
| Eating out | ¥35,141 |
| Groceries and konbini | ¥28,719 |
| Shopping | ¥8,630 |
| Entertainment | ¥3,549 |
| Cash for food | ¥25,000 |
| **All in** | **¥190,639** |

Written the Japanese way, 85,000円 is the same amount and converts the same. A plain number is not an amount: 2026 is a year, 211 a room.

## How the page asks for it

Two lines at the top of the Markdown name the page's currency and a file of exchange rates beside it:

```yaml
currency: JPY
rates: currency.rates.json
```

The rates are those of one day, in the shape the European Central Bank's rates are served in; one command fetches them at build time:

```sh
curl -s 'https://api.frankfurter.dev/v1/latest?base=JPY' > currency.rates.json
```

md2html puts the rates and their day into the page. Nothing is fetched while it is read, so the page works offline, and the amounts are converted at the rates the author built it with, which the page names.

## What the reader sees

- **Nothing changes until they ask.** The amounts are the author's; the switch only offers.
- **Their language's currency first.** Read in English, US dollars; translated into German on the device, euros.
- **The currency moves with the language when it is that language's own.** Dollars read in English become euros once the page is in German, and dollars again back in English. Yen picked in English stay yen in German, and the amounts as written stay as written.
- **The original a hover away.** A converted amount shows the one written under the pointer; a note under the title names the rate and its day, with a way back.
- **Kept for the next page.** The choice travels like the language does.
- **Everything converts.** The text, the tables, ranges like ¥37,000–48,000, and what the page's own scripts write later, a calculator's results among them. Code stays as written: `¥1,000` in code is text.
