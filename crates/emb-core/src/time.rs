//! Calendar timestamps stored in design metadata.
//!
//! The library never reads the system clock: callers supply timestamps.

use std::fmt;

/// A validated calendar date and time of day (no time zone), years 0–9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

impl Timestamp {
    /// Creates a timestamp, or `None` if any field is out of range.
    ///
    /// ```
    /// use emb_core::Timestamp;
    ///
    /// assert!(Timestamp::new(2024, 2, 29, 23, 59, 59).is_some());
    /// assert!(Timestamp::new(2023, 2, 29, 0, 0, 0).is_none());
    /// ```
    #[must_use]
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        let valid = year <= 9999
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month)
            && hour < 24
            && minute < 60
            && second < 60;
        valid.then_some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }

    /// Converts seconds since the Unix epoch (UTC) to a timestamp.
    ///
    /// ```
    /// use emb_core::Timestamp;
    ///
    /// let t = Timestamp::from_unix_seconds(951_782_400).unwrap();
    /// assert_eq!(t.to_compact_string(), "20000229000000");
    /// ```
    #[must_use]
    pub fn from_unix_seconds(seconds: i64) -> Option<Self> {
        let days = seconds.div_euclid(86_400);
        let secs_of_day = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Self::new(
            u16::try_from(year).ok()?,
            u8::try_from(month).ok()?,
            u8::try_from(day).ok()?,
            u8::try_from(secs_of_day / 3600).ok()?,
            u8::try_from(secs_of_day % 3600 / 60).ok()?,
            u8::try_from(secs_of_day % 60).ok()?,
        )
    }

    /// Parses the compact `YYYYMMDDhhmmss` form.
    ///
    /// ```
    /// use emb_core::Timestamp;
    ///
    /// let t = Timestamp::parse_compact("20241231235958").unwrap();
    /// assert_eq!(t.to_compact_string(), "20241231235958");
    /// assert!(Timestamp::parse_compact("2024123123595").is_none());
    /// ```
    #[must_use]
    pub fn parse_compact(text: &str) -> Option<Self> {
        if text.len() != 14 || !text.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let field = |range: std::ops::Range<usize>| text.get(range)?.parse::<u16>().ok();
        let narrow = |range| field(range).and_then(|v| u8::try_from(v).ok());
        Self::new(
            field(0..4)?,
            narrow(4..6)?,
            narrow(6..8)?,
            narrow(8..10)?,
            narrow(10..12)?,
            narrow(12..14)?,
        )
    }

    /// Formats as `YYYYMMDDhhmmss`.
    #[must_use]
    pub fn to_compact_string(&self) -> String {
        format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's
/// `civil_from_days` algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_1970_01_01() {
        let t = Timestamp::from_unix_seconds(0).unwrap();
        assert_eq!(t.to_compact_string(), "19700101000000");
    }

    #[test]
    fn negative_unix_seconds_are_before_epoch() {
        let t = Timestamp::from_unix_seconds(-1).unwrap();
        assert_eq!(t.to_compact_string(), "19691231235959");
    }

    #[test]
    fn known_date_converts_exactly() {
        // 2026-10-22T19:04:05Z
        let t = Timestamp::from_unix_seconds(1_792_695_845).unwrap();
        assert_eq!(t.to_string(), "2026-10-22 19:04:05");
    }

    #[test]
    fn century_years_follow_gregorian_leap_rules() {
        assert!(Timestamp::new(1900, 2, 29, 0, 0, 0).is_none());
        assert!(Timestamp::new(2000, 2, 29, 0, 0, 0).is_some());
    }

    #[test]
    fn out_of_range_fields_are_rejected() {
        assert!(Timestamp::parse_compact("20241301000000").is_none());
        assert!(Timestamp::parse_compact("20241200000000").is_none());
        assert!(Timestamp::parse_compact("20241231240000").is_none());
        assert!(Timestamp::parse_compact("2024123123596a").is_none());
    }
}
