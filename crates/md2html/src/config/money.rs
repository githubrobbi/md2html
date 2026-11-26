// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! A page's currency and the exchange rates its currency switch converts
//! with.
//!
//! The currency is a code of three letters; the rates are those of one day,
//! in a file the author gives, in the shape the European Central Bank's
//! rates are served in
//! (`{"base":"JPY","date":"2026-11-26","rates":{"USD":0.006279}}`).
//!
//! The rates are kept as the text they are written in: the page's script
//! does the arithmetic, so a rate is checked to be a number above zero and
//! carried over digit for digit.

use crate::scan::Scan;
use crate::text::{Date, iso_date, json_str_into, put};

/// How deep a rates file may nest what it holds besides the rates.
const MAX_DEPTH: usize = 16;

/// A currency's code: three letters, as ISO 4217 writes them (`JPY`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Currency([u8; 3]);

impl Currency {
    /// The code in `text`, in either case (`jpy` is `JPY`).
    ///
    /// # Errors
    ///
    /// When `text` is not three letters of A to Z.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        match text.trim().as_bytes() {
            &[first, second, third]
                if [first, second, third].iter().all(u8::is_ascii_alphabetic) =>
            {
                Ok(Self([
                    first.to_ascii_uppercase(),
                    second.to_ascii_uppercase(),
                    third.to_ascii_uppercase(),
                ]))
            }
            _ => Err(format!(
                "currency: `{}` is not a currency's code of three letters (JPY, USD, EUR)",
                text.trim()
            )),
        }
    }

    /// The code as text.
    pub(crate) fn as_str(&self) -> &str {
        // Three ASCII letters, checked when the code was read.
        core::str::from_utf8(&self.0).unwrap_or("XXX")
    }
}

/// The rates of one day: what one unit of `base` buys of each currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rates {
    /// The currency the rates are of.
    pub(crate) base: Currency,
    /// The day they are of, when the file says.
    pub(crate) date: Option<Date>,
    /// Each currency with its rate, as written, in the file's order.
    pub(crate) quotes: Vec<(Currency, String)>,
}

impl Rates {
    /// The rates in `text`.
    ///
    /// # Errors
    ///
    /// When `text` is not one JSON object, has no `base` or no `rates`, a
    /// code is not one of three letters, a rate is not a number above
    /// zero, or `date` is not a day written `2026-11-26`.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut scan = Scan::new(text.trim_start_matches('\u{feff}'));
        let mut base = None;
        let mut date = None;
        let mut quotes = None;
        let mut json = Reader {
            scan: &mut scan,
            depth: 0,
        };
        json.object(|field, key| {
            match key.as_str() {
                "base" => base = Some(Currency::parse(&field.string()?)?),
                "date" => {
                    let day = field.string()?;
                    date = Some(iso_date(&day).ok_or_else(|| {
                        format!("rates: the date `{day}` is not a day written 2026-11-26")
                    })?);
                }
                "rates" => {
                    let mut list = Vec::new();
                    field.object(|quote, code| {
                        let currency = Currency::parse(&code)?;
                        let rate = quote.number()?;
                        if !rate.chars().any(|ch| matches!(ch, '1'..='9')) || rate.starts_with('-')
                        {
                            return Err(format!(
                                "rates: {code} is {rate}; a rate is a number above zero"
                            ));
                        }
                        list.push((currency, rate));
                        Ok(())
                    })?;
                    quotes = Some(list);
                }
                _ => field.skip()?,
            }
            Ok(())
        })?;
        json.space();
        if !json.scan.is_done() {
            return Err(String::from("rates: something follows the object"));
        }
        Ok(Self {
            base: base.ok_or_else(|| String::from("rates: the file names no `base` currency"))?,
            date,
            quotes: quotes.ok_or_else(|| String::from("rates: the file holds no `rates`"))?,
        })
    }

    /// Whether the rates convert from `currency`: it is their base, or
    /// one of them.
    pub(crate) fn covers(&self, currency: Currency) -> bool {
        self.base == currency || self.quotes.iter().any(|(code, _)| *code == currency)
    }

    /// The rates as the page's script reads them, as the fields of a JSON
    /// object: the base, the day and each rate.
    pub(crate) fn json_fields_into(&self, out: &mut String) {
        out.push_str("\"base\":");
        json_str_into(out, self.base.as_str());
        if let Some(day) = self.date {
            put!(
                out,
                ",\"date\":\"{:04}-{:02}-{:02}\"",
                day.year(),
                day.month().number(),
                day.day()
            );
        }
        out.push_str(",\"rates\":{");
        for (place, (code, rate)) in self.quotes.iter().enumerate() {
            if place > 0 {
                out.push(',');
            }
            json_str_into(out, code.as_str());
            put!(out, ":{rate}");
        }
        out.push('}');
    }
}

/// A reader of the little JSON a rates file is.
struct Reader<'scan, 'text> {
    /// Where it is.
    scan: &'scan mut Scan<'text>,
    /// How many objects and arrays it is inside.
    depth: usize,
}

impl Reader<'_, '_> {
    /// Past the white space.
    fn space(&mut self) {
        self.scan
            .take_while(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r'));
    }

    /// `ch`, after any white space.
    ///
    /// # Errors
    ///
    /// When another character is there.
    fn expect(&mut self, ch: char) -> Result<(), String> {
        self.space();
        if self.scan.eat(ch) {
            Ok(())
        } else {
            Err(format!("rates: `{ch}` expected, at `{}`", self.near()))
        }
    }

    /// A few characters from here, to say where something is wrong.
    fn near(&self) -> String {
        self.scan.rest().chars().take(12).collect()
    }

    /// An object: `each` reads the value of each key.
    ///
    /// # Errors
    ///
    /// When it is not an object, or `each` fails.
    fn object<Each>(&mut self, mut each: Each) -> Result<(), String>
    where
        Each: FnMut(&mut Self, String) -> Result<(), String>,
    {
        self.expect('{')?;
        self.nested()?;
        self.space();
        if !self.scan.eat('}') {
            loop {
                let key = self.string()?;
                self.expect(':')?;
                each(self, key)?;
                self.space();
                if self.scan.eat('}') {
                    break;
                }
                self.expect(',')?;
            }
        }
        self.depth -= 1;
        Ok(())
    }

    /// One level deeper, unless that is too deep.
    ///
    /// # Errors
    ///
    /// When it is deeper than [`MAX_DEPTH`].
    fn nested(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(String::from("rates: the file nests too deep"));
        }
        Ok(())
    }

    /// A string, its escapes read.
    ///
    /// # Errors
    ///
    /// When it is not a string, is not closed, or holds a control character
    /// or an escape JSON does not know.
    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.scan.bump() {
                None => return Err(String::from("rates: a string is not closed")),
                Some('"') => return Ok(out),
                Some('\\') => out.push(self.escape()?),
                Some(ch) if ch < ' ' => {
                    return Err(String::from("rates: a control character in a string"));
                }
                Some(ch) => out.push(ch),
            }
        }
    }

    /// The character an escape stands for, after its backslash.
    ///
    /// # Errors
    ///
    /// When JSON knows no such escape.
    fn escape(&mut self) -> Result<char, String> {
        let escaped = match self.scan.bump() {
            Some('"') => '"',
            Some('\\') => '\\',
            Some('/') => '/',
            Some('b') => '\u{8}',
            Some('f') => '\u{c}',
            Some('n') => '\n',
            Some('r') => '\r',
            Some('t') => '\t',
            Some('u') => {
                let hex = self.scan.take_chars(4);
                u32::from_str_radix(hex, 16)
                    .ok()
                    .filter(|_| hex.len() == 4)
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("rates: `\\u{hex}` is no character"))?
            }
            _ => return Err(String::from("rates: an unknown escape in a string")),
        };
        Ok(escaped)
    }

    /// A number, as written: `-`, digits, a fraction, an exponent.
    ///
    /// # Errors
    ///
    /// When it is not a number as JSON writes one.
    fn number(&mut self) -> Result<String, String> {
        self.space();
        let start = self.scan.mark();
        self.scan.eat('-');
        let whole = self.scan.take_while(|ch| ch.is_ascii_digit());
        let fine = whole == "0" || (!whole.is_empty() && !whole.starts_with('0'));
        let fraction = !self.scan.eat('.') || !self.digits().is_empty();
        let exponent = if self.scan.eat_if(|ch| matches!(ch, 'e' | 'E')).is_some() {
            self.scan.eat_if(|ch| matches!(ch, '+' | '-'));
            !self.digits().is_empty()
        } else {
            true
        };
        let text = self.scan.since(start);
        if fine && fraction && exponent {
            Ok(text.to_owned())
        } else {
            Err(format!("rates: `{}` is not a number", self.near_of(text)))
        }
    }

    /// The digits from here.
    fn digits(&mut self) -> &str {
        self.scan.take_while(|ch| ch.is_ascii_digit())
    }

    /// `text`, or what follows when it is empty.
    fn near_of(&self, text: &str) -> String {
        if text.is_empty() {
            self.near()
        } else {
            text.to_owned()
        }
    }

    /// Past one value of any kind: what the rates file holds beside the
    /// rates (`amount`, a provider's notes).
    ///
    /// # Errors
    ///
    /// When it is no value JSON writes.
    fn skip(&mut self) -> Result<(), String> {
        self.space();
        match self.scan.peek() {
            Some('{') => self.object(|inner, _| inner.skip()),
            Some('[') => {
                self.scan.bump();
                self.nested()?;
                self.space();
                if !self.scan.eat(']') {
                    loop {
                        self.skip()?;
                        self.space();
                        if self.scan.eat(']') {
                            break;
                        }
                        self.expect(',')?;
                    }
                }
                self.depth -= 1;
                Ok(())
            }
            Some('"') => self.string().map(drop),
            Some('t') if self.scan.eat_str("true") => Ok(()),
            Some('f') if self.scan.eat_str("false") => Ok(()),
            Some('n') if self.scan.eat_str("null") => Ok(()),
            _ => self.number().map(drop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Currency, Rates};

    /// The rates as the European Central Bank's are served: the base, the
    /// day, each rate as written; what else the file holds is passed over.
    ///
    /// # Panics
    ///
    /// When the file is not read, or a part of it is read wrong.
    #[test]
    fn a_rates_file_is_read_as_written() {
        let rates = Rates::parse(
            "{\"amount\":1.0,\"base\":\"JPY\",\"date\":\"2026-11-26\",\
             \"rates\":{\"EUR\":0.0057212,\"USD\":0.006279,\"IDR\":115.49,\"KRW\":8.443e0},\
             \"note\":[\"ECB\",{\"x\":null}]}\n",
        )
        .expect("read");
        assert_eq!(rates.base.as_str(), "JPY");
        assert_eq!(rates.date, crate::text::iso_date("2026-11-26"));
        let codes: Vec<_> = rates
            .quotes
            .iter()
            .map(|(code, rate)| (code.as_str(), rate.as_str()))
            .collect();
        assert_eq!(codes, [
            ("EUR", "0.0057212"),
            ("USD", "0.006279"),
            ("IDR", "115.49"),
            ("KRW", "8.443e0")
        ]);
        assert!(rates.covers(Currency::parse("jpy").expect("code")));
        assert!(rates.covers(Currency::parse("USD").expect("code")));
        assert!(!rates.covers(Currency::parse("GBP").expect("code")));
        let mut json = String::new();
        rates.json_fields_into(&mut json);
        assert_eq!(
            json,
            "\"base\":\"JPY\",\"date\":\"2026-11-26\",\"rates\":{\"EUR\":0.0057212,\"USD\":0.006279,\"IDR\":115.49,\"KRW\":8.443e0}"
        );
    }

    /// What is not a rates file is refused, with what is wrong.
    ///
    /// # Panics
    ///
    /// When a broken file is read, or refused without saying why.
    #[test]
    fn a_broken_rates_file_is_refused_with_the_reason() {
        for (text, why) in [
            ("", "`{` expected"),
            ("{\"rates\":{}}", "no `base`"),
            ("{\"base\":\"JPY\"}", "no `rates`"),
            ("{\"base\":\"YEN!\",\"rates\":{}}", "three letters"),
            ("{\"base\":\"JPY\",\"rates\":{\"USD\":0}}", "above zero"),
            ("{\"base\":\"JPY\",\"rates\":{\"USD\":-1}}", "above zero"),
            (
                "{\"base\":\"JPY\",\"rates\":{\"USD\":\"1\"}}",
                "not a number",
            ),
            ("{\"base\":\"JPY\",\"rates\":{\"USD\":01}}", "not a number"),
            ("{\"base\":\"JPY\",\"rates\":{\"USD\":1.}}", "not a number"),
            (
                "{\"base\":\"JPY\",\"date\":\"yesterday\",\"rates\":{}}",
                "not a day",
            ),
            ("{\"base\":\"JPY\",\"rates\":{}} x", "follows"),
            ("{\"base\":\"JPY\",\"rates\":{\"USD\":1}", "expected"),
            (
                &format!("{{\"a\":{}{}}}", "[".repeat(40), "]".repeat(40)),
                "too deep",
            ),
        ] {
            let error = Rates::parse(text).expect_err(text);
            assert!(error.contains(why), "{text}: {error}");
        }
    }

    /// A rate is carried into the page as the number it is written as, so
    /// nothing but a number reaches the script.
    ///
    /// # Panics
    ///
    /// When a rate that is no JSON number is read.
    #[test]
    fn only_a_number_reaches_the_page_as_a_rate() {
        for rate in ["1</script>", "1e", "Infinity", "NaN", "0x10", "1,5"] {
            let text = format!("{{\"base\":\"JPY\",\"rates\":{{\"USD\":{rate}}}}}");
            assert!(Rates::parse(&text).is_err(), "{rate}");
        }
    }
}
