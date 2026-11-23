// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Sky, LLC.

//! The pipeline's clock: the system clock read as UTC, with no crate.
//!
//! The pipeline needs three things of time: a date for the changelog, a
//! stamp for a log file's name, and the moments a workflow started and
//! finished, kept in its state file. All three are UTC, so a run means the
//! same on every machine.

use core::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Seconds in a day.
const DAY: u64 = 86_400;

/// A moment in UTC, to the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    /// Seconds since 1970-01-01T00:00:00Z.
    secs: u64,
}

/// A moment's fields on the calendar and the clock.
struct Civil {
    /// The year.
    year: u64,
    /// The month, 1 to 12.
    month: u64,
    /// The day of the month, 1 to 31.
    day: u64,
    /// The hour, 0 to 23.
    hour: u64,
    /// The minute, 0 to 59.
    minute: u64,
    /// The second, 0 to 59.
    second: u64,
}

impl Stamp {
    /// This moment. A system clock set before 1970 reads as 1970.
    pub(crate) fn now() -> Self {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        Self { secs }
    }

    /// The calendar and clock fields (the days-to-civil algorithm of Howard
    /// Hinnant's `chrono`-compatible date library, for days from 1970 on).
    const fn civil(self) -> Civil {
        let days = self.secs / DAY + 719_468;
        let rest = self.secs % DAY;
        let era = days / 146_097;
        let day_of_era = days % 146_097;
        let year_of_era =
            (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let month = if shifted_month < 10 {
            shifted_month + 3
        } else {
            shifted_month - 9
        };
        Civil {
            year: year_of_era + era * 400 + if month <= 2 { 1 } else { 0 },
            month,
            day: day_of_year - (153 * shifted_month + 2) / 5 + 1,
            hour: rest / 3_600,
            minute: rest % 3_600 / 60,
            second: rest % 60,
        }
    }

    /// The date, as the changelog writes it: `2026-11-22`.
    pub(crate) fn date(self) -> String {
        let at = self.civil();
        format!("{:04}-{:02}-{:02}", at.year, at.month, at.day)
    }

    /// The moment as part of a file name: `20261122_153000`.
    pub(crate) fn compact(self) -> String {
        let at = self.civil();
        format!(
            "{:04}{:02}{:02}_{:02}{:02}{:02}",
            at.year, at.month, at.day, at.hour, at.minute, at.second
        )
    }

    /// The moment in RFC 3339, as the state file keeps it:
    /// `2026-11-22T15:30:00Z`.
    fn rfc3339(self) -> String {
        let at = self.civil();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            at.year, at.month, at.day, at.hour, at.minute, at.second
        )
    }

    /// Reads RFC 3339 in UTC: `2026-11-22T15:30:00Z`, with or without a
    /// fraction of a second (state files written before this clock carry one).
    fn parse(text: &str) -> Option<Self> {
        let (date, time) = text.split_once('T')?;
        let clock = time.strip_suffix('Z')?;
        let whole = clock
            .split_once('.')
            .map_or(clock, |(seconds, _fraction)| seconds);
        let mut date_fields = date.splitn(3, '-').map(str::parse::<u64>);
        let mut time_fields = whole.splitn(3, ':').map(str::parse::<u64>);
        let year = date_fields.next()?.ok()?;
        let month = date_fields.next()?.ok()?;
        let day = date_fields.next()?.ok()?;
        let hour = time_fields.next()?.ok()?;
        let minute = time_fields.next()?.ok()?;
        let second = time_fields.next()?.ok()?;
        let in_range = (1_970..=9_999).contains(&year)
            && (1..=12).contains(&month)
            && (1..=31).contains(&day)
            && hour < 24
            && minute < 60
            && second < 60;
        if !in_range {
            return None;
        }
        let shifted_year = if month <= 2 { year - 1 } else { year };
        let era = shifted_year / 400;
        let year_of_era = shifted_year % 400;
        let shifted_month = if month > 2 { month - 3 } else { month + 9 };
        let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        let days = (era * 146_097 + day_of_era).checked_sub(719_468)?;
        Some(Self {
            secs: days * DAY + hour * 3_600 + minute * 60 + second,
        })
    }
}

impl fmt::Display for Stamp {
    /// The moment for a person: `2026-11-22 15:30:00 UTC`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let at = self.civil();
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            at.year, at.month, at.day, at.hour, at.minute, at.second
        )
    }
}

impl Serialize for Stamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.rfc3339())
    }
}

impl<'de> Deserialize<'de> for Stamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text)
            .ok_or_else(|| D::Error::custom(format!("`{text}` is not a UTC moment in RFC 3339")))
    }
}

#[cfg(test)]
mod tests {
    use super::Stamp;

    /// The stamp of a number of seconds since 1970.
    const fn at(secs: u64) -> Stamp {
        Stamp { secs }
    }

    #[test]
    fn known_moments_read_as_their_dates() {
        assert_eq!(at(0).rfc3339(), "1970-01-01T00:00:00Z");
        assert_eq!(at(951_782_400).date(), "2000-02-29");
        assert_eq!(at(951_868_799).rfc3339(), "2000-02-29T23:59:59Z");
        assert_eq!(at(951_868_800).date(), "2000-03-01");
        assert_eq!(at(1_704_067_199).rfc3339(), "2023-12-31T23:59:59Z");
        assert_eq!(at(1_795_361_400).to_string(), "2026-11-22 15:30:00 UTC");
        assert_eq!(at(1_795_361_400).compact(), "20261122_153000");
        assert_eq!(at(4_102_444_800).date(), "2100-01-01");
    }

    #[test]
    fn every_day_of_four_centuries_goes_there_and_back() {
        for day in 0..146_097_u64 {
            let moment = at(day * 86_400 + 12 * 3_600 + 34 * 60 + 56);
            assert_eq!(Stamp::parse(&moment.rfc3339()), Some(moment));
        }
    }

    #[test]
    fn the_state_file_keeps_and_reads_moments() {
        let moment = at(1_795_361_400);
        let kept = serde_json::to_string(&moment).unwrap();
        assert_eq!(kept, "\"2026-11-22T15:30:00Z\"");
        assert_eq!(serde_json::from_str::<Stamp>(&kept).unwrap(), moment);
        // A state file written with a fraction of a second, as before this
        // clock.
        let older = "\"2026-11-22T15:30:00.123456789Z\"";
        assert_eq!(serde_json::from_str::<Stamp>(older).unwrap(), moment);
        for wrong in [
            "\"yesterday\"",
            "\"2026-13-01T00:00:00Z\"",
            "\"2026-11-22T15:30:00+01:00\"",
        ] {
            assert!(serde_json::from_str::<Stamp>(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn now_is_after_this_was_written() {
        assert!(Stamp::now().date().as_str() >= "2026-11-22");
    }
}
