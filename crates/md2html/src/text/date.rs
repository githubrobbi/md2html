// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC

//! Dates: the day a moment falls on, and a date as an author writes it.

/// A day of the calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Date {
    /// The year, from year 0.
    pub(crate) year: u64,
    /// The month.
    pub(crate) month: Month,
    /// The day of the month, from 1.
    pub(crate) day: u64,
}

/// A month of the year.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Month {
    /// The first.
    January,
    /// The second.
    February,
    /// The third.
    March,
    /// The fourth.
    April,
    /// The fifth.
    May,
    /// The sixth.
    June,
    /// The seventh.
    July,
    /// The eighth.
    August,
    /// The ninth.
    September,
    /// The tenth.
    October,
    /// The eleventh.
    November,
    /// The twelfth.
    December,
}

impl Month {
    /// The twelve, in the order of the year.
    pub(crate) const ALL: [Self; 12] = [
        Self::January,
        Self::February,
        Self::March,
        Self::April,
        Self::May,
        Self::June,
        Self::July,
        Self::August,
        Self::September,
        Self::October,
        Self::November,
        Self::December,
    ];

    /// The month's number, January 1.
    pub(crate) const fn number(self) -> u8 {
        match self {
            Self::January => 1,
            Self::February => 2,
            Self::March => 3,
            Self::April => 4,
            Self::May => 5,
            Self::June => 6,
            Self::July => 7,
            Self::August => 8,
            Self::September => 9,
            Self::October => 10,
            Self::November => 11,
            Self::December => 12,
        }
    }

    /// The month a number names, if it names one.
    fn numbered(number: u64) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|month| u64::from(month.number()) == number)
    }

    /// The month `steps` months after March, round the year.
    const fn after_march(steps: u64) -> Self {
        match steps % 12 {
            0 => Self::March,
            1 => Self::April,
            2 => Self::May,
            3 => Self::June,
            4 => Self::July,
            5 => Self::August,
            6 => Self::September,
            7 => Self::October,
            8 => Self::November,
            9 => Self::December,
            10 => Self::January,
            _ => Self::February,
        }
    }

    /// This month's entry in a list of the twelve in the order of the year.
    pub(crate) const fn of<'list>(self, twelve: &[&'list str; 12]) -> &'list str {
        let [
            january,
            february,
            march,
            april,
            may,
            june,
            july,
            august,
            september,
            october,
            november,
            december,
        ] = *twelve;
        match self {
            Self::January => january,
            Self::February => february,
            Self::March => march,
            Self::April => april,
            Self::May => may,
            Self::June => june,
            Self::July => july,
            Self::August => august,
            Self::September => september,
            Self::October => october,
            Self::November => november,
            Self::December => december,
        }
    }
}

/// The days of an era: every 400 years of the calendar have as many.
const ERA_DAYS: u64 = 146_097;

/// The date `secs` seconds after the epoch falls on.
pub(crate) fn civil(secs: u64) -> Date {
    // Days counted from 0000-03-01, so that a leap day ends its year, in
    // eras of 400 years.
    let days = secs / 86_400 + 719_468;
    let era = days / ERA_DAYS;
    let day_of_era = days % ERA_DAYS;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // Months counted from March, every five of them 153 days long.
    let from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * from_march + 2) / 5 + 1;
    // January and February close the year that began in March.
    Date {
        year: era * 400 + year_of_era + u64::from(from_march >= 10),
        month: Month::after_march(from_march),
        day,
    }
}

/// A `YYYY-MM-DD` date at the start of `text`; what follows the day's
/// digits is not read.
pub(crate) fn iso_date(text: &str) -> Option<Date> {
    let mut parts = text.trim().splitn(3, '-');
    let year = parts.next()?.parse::<u64>().ok()?;
    let month = parts
        .next()?
        .parse::<u64>()
        .ok()
        .and_then(Month::numbered)?;
    let digits: String = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let day = digits
        .parse::<u64>()
        .ok()
        .filter(|day| (1..=31).contains(day))?;
    Some(Date { year, month, day })
}

#[cfg(test)]
mod tests {
    use super::{Date, Month, civil, iso_date};

    /// The date of the numbers, for writing the cases short.
    ///
    /// # Panics
    ///
    /// Panics when `number` is not a month's number, 1 to 12.
    fn date(year: u64, number: u64, day: u64) -> Date {
        Date {
            year,
            month: Month::numbered(number).unwrap(),
            day,
        }
    }

    /// Seconds since 1970 fall on the right calendar date, leap days and
    /// century years included.
    ///
    /// # Panics
    ///
    /// Panics when any of eighteen moments, among them the first and last
    /// second of a day, 29 February in 1972, 2000 and 2400, and the day after
    /// 28 February in 2100, which is not a leap year, falls on another date; or
    /// when the last second a `u64` counts does not give a year past 584
    /// billion with a day from 1 to 31.
    #[test]
    fn days_fall_on_their_dates() {
        for (secs, (year, month, day)) in [
            (0, (1970, 1, 1)),
            (86_399, (1970, 1, 1)),
            (86_400, (1970, 1, 2)),
            (68_083_200, (1972, 2, 28)),
            (68_169_600, (1972, 2, 29)),
            (68_256_000, (1972, 3, 1)),
            (946_598_400, (1999, 12, 31)),
            (951_782_400, (2000, 2, 29)),
            (951_868_800, (2000, 3, 1)),
            (978_220_800, (2000, 12, 31)),
            (978_307_200, (2001, 1, 1)),
            (1_700_000_000, (2023, 11, 14)),
            (1_706_659_200, (2024, 1, 31)),
            (1_714_435_200, (2024, 4, 30)),
            (4_107_456_000, (2100, 2, 28)),
            (4_107_542_400, (2100, 3, 1)),
            (13_574_563_200, (2400, 2, 29)),
            (13_601_001_600, (2400, 12, 31)),
        ] {
            assert_eq!(civil(secs), date(year, month, day), "{secs}");
        }
        let last = civil(u64::MAX);
        assert!(last.year > 584_000_000_000 && (1..=31).contains(&last.day));
    }

    /// Over a whole 400-year cycle, each day follows the one before it on the
    /// calendar.
    ///
    /// # Panics
    ///
    /// Panics when, for any of the 146,097 days from 1 January 1970, the date
    /// is neither the next day of the same month nor the 1st of the next month,
    /// naming the two dates; a skipped or repeated day, a 31st where a month
    /// has 30, or a month out of order fails it.
    #[test]
    fn every_day_of_an_era_has_a_date() {
        let mut previous = civil(0);
        for days in 1..=146_097 {
            let today = civil(days * 86_400);
            let next_month = today.month.number() == previous.month.number() % 12 + 1;
            assert!(
                (today.day == previous.day + 1 && today.month == previous.month)
                    || (today.day == 1 && next_month),
                "{previous:?} then {today:?}"
            );
            previous = today;
        }
    }

    /// Each month has its number and its place in a list of names.
    ///
    /// # Panics
    ///
    /// Panics when a month does not pick its own entry from a list of twelve
    /// names, when its number does not lead back to it, or when 0 or 13 is
    /// taken as a month.
    #[test]
    fn months_have_numbers_and_names() {
        let names = [
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12",
        ];
        for month in Month::ALL {
            assert_eq!(month.of(&names), month.number().to_string());
            assert_eq!(Month::numbered(month.number().into()), Some(month));
        }
        assert_eq!(Month::numbered(0), None);
        assert_eq!(Month::numbered(13), None);
    }

    /// An ISO date is read with what may follow it, and refused when any part
    /// is missing or out of range.
    ///
    /// # Panics
    ///
    /// Panics when 29 February 2024, a date with one-digit parts and space
    /// around it, or a date followed by a time or a word is not read; or when
    /// an empty text, a year alone, a year and month, a month of 0 or 13, a day
    /// of 0 or 32, a part that is not a number, or a negative year is read as a
    /// date.
    #[test]
    fn iso_dates_are_read_strictly() {
        assert_eq!(iso_date("2024-02-29"), Some(date(2024, 2, 29)));
        assert_eq!(iso_date(" 2024-1-5 \n"), Some(date(2024, 1, 5)));
        assert_eq!(iso_date("1999-12-31T23:59"), Some(date(1999, 12, 31)));
        assert_eq!(iso_date("2024-12-01 noon"), Some(date(2024, 12, 1)));
        for bad in [
            "",
            "2024",
            "2024-02",
            "2024-00-10",
            "2024-13-01",
            "2024-01-00",
            "2024-01-32",
            "x-01-01",
            "2024-x-01",
            "2024-01-x",
            "-1-01-01",
        ] {
            assert_eq!(iso_date(bad), None, "{bad}");
        }
    }
}
