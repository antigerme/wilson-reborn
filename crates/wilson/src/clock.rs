// SPDX-License-Identifier: GPL-3.0-or-later
//! System clock → engine [`Clock`] (no external date crate).

use std::time::{SystemTime, UNIX_EPOCH};

use wilson_engine::Clock;

/// The current local-ish wall clock (uses UTC; good enough for day/holiday logic).
pub fn now() -> Clock {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    from_unix(secs)
}

fn from_unix(secs: u64) -> Clock {
    let days = (secs / 86_400) as i64;
    let hour = ((secs % 86_400) / 3_600) as u8;
    let (y, m, d) = civil_from_days(days);
    let yday = (days - days_from_civil(y, 1, 1)) as i32;
    Clock {
        yday,
        hour,
        month: m,
        day: d,
    }
}

/// Howard Hinnant's days→civil algorithm.
fn civil_from_days(z: i64) -> (i64, u8, u8) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u8; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8; // [1, 12]
    (y + i64::from(m <= 2), m, d)
}

/// Howard Hinnant's civil→days algorithm (inverse of [`civil_from_days`]).
fn days_from_civil(y: i64, m: u8, d: u8) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let m = i64::from(m);
    let d = i64::from(d);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch() {
        let c = from_unix(0);
        assert_eq!((c.month, c.day, c.hour, c.yday), (1, 1, 0, 0));
    }

    #[test]
    fn day_two_and_hour() {
        let c = from_unix(86_400 + 3 * 3_600);
        assert_eq!((c.month, c.day, c.hour, c.yday), (1, 2, 3, 1));
    }

    #[test]
    fn next_year_resets_yday() {
        // 1971-01-01 00:00 UTC (365 days after epoch; 1970 is not a leap year).
        let c = from_unix(365 * 86_400);
        assert_eq!((c.month, c.day, c.yday), (1, 1, 0));
    }

    #[test]
    fn a_known_date() {
        // 2000-03-01 00:00 UTC = 951_868_800.
        let c = from_unix(951_868_800);
        assert_eq!((c.month, c.day), (3, 1));
    }
}
