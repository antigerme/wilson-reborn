// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional **time controls** (quality-of-life), all opt-in — the default is plain
//! real-wall-clock time, identical to before.
//!
//! - **Jump to a day** (`--day N`): start the 11-day story arc at day `N` instead of the
//!   persisted/real day. Handled in `main` (the engine's [`wilson_engine::Director`] already
//!   takes a starting day); this module just documents it alongside the others.
//! - **Story mode** (`--story`): play the whole arc in order (day 1 → 11 → 1 …) on a fixed
//!   cadence, without waiting real days — see [`story_clock`]. Combine with `--speed` to also
//!   accelerate the animation itself.
//!
//! These synthesize the [`Clock`] the engine consumes, so the engine stays pure (it is always
//! just told "what time is it"); nothing about the runtime semantics changes.

use wilson_engine::Clock;

/// Default real seconds each story day is shown in story mode (the arc advances one day per
/// this span). ~1.5 min/day ⇒ the full 11-day arc in ~16 min.
pub const DEFAULT_STORY_DAY_SECS: u32 = 90;

/// Synthesize the engine clock for **story mode** from the elapsed runtime.
///
/// Each story day lasts `day_secs` real seconds, so the director's `advance_day` steps the
/// arc 1 → 11 → 1 … (it bumps the day whenever the `yday` we feed changes, at run
/// boundaries). Within a day the `hour` sweeps 0→23 so the day/night cycle is visible.
/// Holidays keep the **real** `month`/`day` (so they still fire on the real date, not a fake
/// one). At `elapsed_secs == 0` this is day-index 0, hour 0 — pair it with a director started
/// at day 1 (`stored_yday == 0`) so the first run plays day 1.
pub fn story_clock(real: Clock, elapsed_secs: u64, day_secs: u32) -> Clock {
    let day_secs = u64::from(day_secs.max(1));
    let day_index = (elapsed_secs / day_secs) as i32;
    let within = (elapsed_secs % day_secs) as f64 / day_secs as f64; // 0.0..1.0 through the day
    let hour = (within * 24.0) as u8; // 0..=23 (within < 1.0 ⇒ never 24)
    Clock {
        yday: day_index,
        hour: hour.min(23),
        month: real.month,
        day: real.day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Clock {
        Clock {
            yday: 200,
            hour: 9,
            month: 6,
            day: 14,
        }
    }

    #[test]
    fn story_starts_at_day_index_zero_hour_zero() {
        // Paired with a director at day 1 / stored_yday 0, this keeps the first run on day 1.
        let c = story_clock(base(), 0, 90);
        assert_eq!(c.yday, 0);
        assert_eq!(c.hour, 0);
    }

    #[test]
    fn story_advances_one_day_index_per_day_secs() {
        // The yday we feed must change once per `day_secs` so the arc steps exactly one day.
        assert_eq!(story_clock(base(), 89, 90).yday, 0);
        assert_eq!(story_clock(base(), 90, 90).yday, 1);
        assert_eq!(story_clock(base(), 180, 90).yday, 2);
        assert_eq!(story_clock(base(), 90 * 11, 90).yday, 11); // arc wraps (director clamps)
    }

    #[test]
    fn story_hour_sweeps_the_day_for_daynight() {
        // Quarter through the story day ⇒ ~06:00; just before the end ⇒ ~23:00.
        assert_eq!(story_clock(base(), 0, 100).hour, 0);
        assert_eq!(story_clock(base(), 25, 100).hour, 6);
        assert_eq!(story_clock(base(), 50, 100).hour, 12);
        assert_eq!(story_clock(base(), 99, 100).hour, 23);
        assert!(story_clock(base(), 1_000_000, 100).hour <= 23); // always a valid hour
    }

    #[test]
    fn story_keeps_the_real_calendar_date_for_holidays() {
        let c = story_clock(base(), 12_345, 90);
        assert_eq!((c.month, c.day), (6, 14)); // holidays still fire on the real date
    }

    #[test]
    fn story_day_secs_zero_does_not_divide_by_zero() {
        // Defensive: a degenerate cadence is clamped to 1 s/day, not a panic.
        let c = story_clock(base(), 5, 0);
        assert_eq!(c.yday, 5);
    }
}
