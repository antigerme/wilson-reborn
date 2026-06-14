// SPDX-License-Identifier: GPL-3.0-or-later
//! Tiny cross-session persistence of the story day, so the 11-day arc resumes where
//! it left off instead of restarting at day 1 on every launch.
//!
//! The engine's [`Director`](wilson_engine::Director) tracks `current_day` (1–11) and
//! `stored_yday` (the day-of-year it last advanced). We persist that pair to a small
//! text file in a per-user state directory and load it at startup. Everything here is
//! best-effort: any missing/unreadable/unwritable file degrades to "no persistence"
//! (the arc simply starts at day 1) — it never panics.

use std::path::{Path, PathBuf};

/// The persisted story progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayState {
    /// Current story day (1–11).
    pub current_day: u8,
    /// Day-of-year the day was last advanced (to detect calendar changes).
    pub stored_yday: i32,
}

impl DayState {
    /// Load persisted progress, or `None` if absent/unreadable.
    pub fn load() -> Option<DayState> {
        Self::load_from(&state_file()?)
    }

    /// Persist progress (best-effort; any I/O error is ignored).
    pub fn save(&self) {
        if let Some(path) = state_file() {
            self.save_to(&path);
        }
    }

    fn load_from(path: &Path) -> Option<DayState> {
        Self::parse(&std::fs::read_to_string(path).ok()?)
    }

    fn save_to(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, self.serialize());
    }

    fn parse(text: &str) -> Option<DayState> {
        let mut current_day = None;
        let mut stored_yday = None;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line.split_once('=')?;
            match key.trim() {
                "current_day" => current_day = value.trim().parse().ok(),
                "stored_yday" => stored_yday = value.trim().parse().ok(),
                _ => {}
            }
        }
        Some(DayState {
            current_day: current_day?,
            stored_yday: stored_yday?,
        })
    }

    fn serialize(&self) -> String {
        format!(
            "# Wilson Reborn — story progress (the 11-day arc).\ncurrent_day={}\nstored_yday={}\n",
            self.current_day, self.stored_yday
        )
    }
}

/// The per-user state directory (zero-dep platform resolution).
///
/// Windows: `%APPDATA%\WilsonReborn`. Otherwise XDG: `$XDG_STATE_HOME/wilson-reborn`
/// or `$HOME/.local/state/wilson-reborn`.
fn state_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join("WilsonReborn"))
    } else if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        Some(PathBuf::from(xdg).join("wilson-reborn"))
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state/wilson-reborn"))
    }
}

fn state_file() -> Option<PathBuf> {
    Some(state_dir()?.join("state.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_serialize_round_trip() {
        let s = DayState {
            current_day: 7,
            stored_yday: 165,
        };
        let parsed = DayState::parse(&s.serialize()).expect("round-trips");
        assert_eq!(parsed, s);
    }

    #[test]
    fn parse_ignores_comments_and_blanks_and_unknown_keys() {
        let text = "# a comment\n\n  current_day = 3 \nfoo=bar\nstored_yday=10\n";
        assert_eq!(
            DayState::parse(text),
            Some(DayState {
                current_day: 3,
                stored_yday: 10,
            })
        );
    }

    #[test]
    fn parse_rejects_missing_or_bad_fields() {
        assert_eq!(DayState::parse("current_day=3\n"), None); // no stored_yday
        assert_eq!(DayState::parse("current_day=x\nstored_yday=1\n"), None); // not a number
        assert_eq!(DayState::parse(""), None);
    }

    #[test]
    fn save_to_then_load_from_round_trips() {
        let mut path = std::env::temp_dir();
        path.push(format!("wilson_state_test_{}.txt", std::process::id()));
        let s = DayState {
            current_day: 9,
            stored_yday: 200,
        };
        s.save_to(&path);
        let loaded = DayState::load_from(&path).expect("loads what we saved");
        assert_eq!(loaded, s);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_from_missing_file_is_none() {
        let path = std::env::temp_dir().join("wilson_state_does_not_exist_12345.txt");
        let _ = std::fs::remove_file(&path);
        assert_eq!(DayState::load_from(&path), None);
    }
}
