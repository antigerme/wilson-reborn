// SPDX-License-Identifier: GPL-3.0-or-later
//! Lifetime statistics: how many times Wilson has run, for how long in total, and the
//! highest story day the castaway has reached. Stored as a small text file next to the
//! config and day state.
//!
//! Best-effort, like the other persisted state: a missing/unreadable file just starts
//! the counters at zero, and saving never panics.

use std::path::PathBuf;

/// Persisted lifetime counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// How many times the screensaver has been shown.
    pub sessions: u64,
    /// Total time shown, in seconds (across all sessions).
    pub total_secs: u64,
    /// Highest story day reached (1–11), `0` if never run.
    pub max_day: u8,
}

impl Stats {
    /// Load the counters, or zeros if absent/unreadable.
    pub fn load() -> Stats {
        stats_file()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Stats::parse(&t))
            .unwrap_or_default()
    }

    /// Persist the counters (best-effort; any I/O error is ignored).
    pub fn save(&self) {
        let Some(path) = stats_file() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, self.serialize());
    }

    /// Raise [`Stats::max_day`] to `day` if it is higher (clamped to the 1–11 cycle).
    pub fn note_day(&mut self, day: u8) {
        if (1..=11).contains(&day) && day > self.max_day {
            self.max_day = day;
        }
    }

    fn parse(text: &str) -> Stats {
        let mut s = Stats::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "sessions" => {
                    if let Ok(n) = value.parse() {
                        s.sessions = n;
                    }
                }
                "total_secs" => {
                    if let Ok(n) = value.parse() {
                        s.total_secs = n;
                    }
                }
                "max_day" => {
                    if let Ok(n) = value.parse::<u8>() {
                        s.max_day = n.min(11);
                    }
                }
                _ => {}
            }
        }
        s
    }

    fn serialize(&self) -> String {
        format!(
            "# Wilson Reborn — lifetime statistics.\nsessions={}\ntotal_secs={}\nmax_day={}\n",
            self.sessions, self.total_secs, self.max_day
        )
    }

    /// A human-readable one-liner, e.g. `"3 sessions, 1h 5m total, reached day 7/11"`.
    pub fn summary(&self) -> String {
        format!(
            "{} session(s), {} total, reached day {}/11",
            self.sessions,
            format_duration(self.total_secs),
            self.max_day
        )
    }
}

/// Format a number of seconds as `"Hh Mm"` (or `"Mm Ss"` / `"Ss"` for short spans).
pub fn format_duration(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

fn stats_file() -> Option<PathBuf> {
    Some(crate::state::state_dir()?.join("stats.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips() {
        let s = Stats {
            sessions: 12,
            total_secs: 4000,
            max_day: 9,
        };
        assert_eq!(Stats::parse(&s.serialize()), s);
    }

    #[test]
    fn parse_is_lenient_and_clamps() {
        let s = Stats::parse("# c\n\nsessions=3\nbogus=x\nmax_day=99\n");
        assert_eq!(s.sessions, 3);
        assert_eq!(s.total_secs, 0);
        assert_eq!(s.max_day, 11); // clamped
    }

    #[test]
    fn note_day_keeps_the_maximum() {
        let mut s = Stats::default();
        s.note_day(5);
        s.note_day(3); // lower → ignored
        s.note_day(8);
        s.note_day(0); // invalid → ignored
        s.note_day(12); // out of range → ignored
        assert_eq!(s.max_day, 8);
    }

    #[test]
    fn duration_formats() {
        assert_eq!(format_duration(45), "45s");
        assert_eq!(format_duration(125), "2m 5s");
        assert_eq!(format_duration(3700), "1h 1m");
    }

    #[test]
    fn summary_mentions_the_counters() {
        let s = Stats {
            sessions: 2,
            total_secs: 3900,
            max_day: 7,
        };
        assert_eq!(s.summary(), "2 session(s), 1h 5m total, reached day 7/11");
    }
}
