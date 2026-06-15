// SPDX-License-Identifier: GPL-3.0-or-later
//! User configuration: runtime options read from a small text file (and overridable by
//! CLI flags), stored next to the day state in the per-user state directory.
//!
//! All options have sane defaults, so a missing/partial/unreadable file is fine — the
//! app just runs with defaults. CLI flags (`--windowed`, `--mute`, `--speed`, `--scale`)
//! win over the file but are not persisted, so they stay one-off.

use std::path::PathBuf;

use wilson_engine::DayNight;

use crate::scale::{Filter, ScaleMode};

/// Minimum/maximum playback speed (percent of the original timing).
const SPEED_MIN: u32 = 25;
const SPEED_MAX: u32 = 400;

/// The app's runtime options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    /// Run in a 640×480 window instead of fullscreen (handy for development).
    pub windowed: bool,
    /// Disable sound effects.
    pub mute: bool,
    /// Playback speed, percent of the original timing (100 = original), clamped.
    pub speed: u32,
    /// How the frame is scaled into the window.
    pub scale: ScaleMode,
    /// How upscaled pixels are sampled (nearest = crisp/retro, linear = smooth,
    /// xbr = edge-directed "HD" — smooth and sharp).
    pub filter: Filter,
    /// How the day/night cycle is driven (original 8-hour vs real 24-hour).
    pub daynight: DayNight,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            windowed: false,
            mute: false,
            speed: 100,
            scale: ScaleMode::Fit,
            filter: Filter::default(),
            daynight: DayNight::Original,
        }
    }
}

impl Config {
    /// Load the config from disk, falling back to defaults for anything missing.
    pub fn load() -> Config {
        config_file()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Config::parse(&t))
            .unwrap_or_default()
    }

    /// Persist the config (best-effort; any I/O error is ignored).
    pub fn save(&self) {
        let Some(path) = config_file() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, self.serialize());
    }

    /// Parse a config file, starting from defaults and overriding recognised keys.
    fn parse(text: &str) -> Config {
        let mut c = Config::default();
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
                "windowed" => {
                    if let Some(b) = parse_bool(value) {
                        c.windowed = b;
                    }
                }
                "mute" => {
                    if let Some(b) = parse_bool(value) {
                        c.mute = b;
                    }
                }
                "speed" => {
                    if let Ok(n) = value.parse::<u32>() {
                        c.speed = n.clamp(SPEED_MIN, SPEED_MAX);
                    }
                }
                "scale" => {
                    if let Some(m) = ScaleMode::parse(value) {
                        c.scale = m;
                    }
                }
                "filter" => {
                    if let Some(f) = Filter::parse(value) {
                        c.filter = f;
                    }
                }
                "daynight" => {
                    if let Some(m) = DayNight::parse(value) {
                        c.daynight = m;
                    }
                }
                _ => {}
            }
        }
        c
    }

    fn serialize(&self) -> String {
        format!(
            "# Wilson Reborn — configuration.\n\
             # windowed: true runs in a window instead of fullscreen.\n\
             windowed={}\n\
             # mute: true disables sound effects.\n\
             mute={}\n\
             # speed: playback speed percent ({SPEED_MIN}–{SPEED_MAX}; 100 = original).\n\
             speed={}\n\
             # scale: how the picture fills the window — fit | stretch | integer.\n\
             scale={}\n\
             # filter: pixel sampling — nearest (crisp/retro) | linear (smooth) | xbr (HD).\n\
             filter={}\n\
             # daynight: day/night cycle — original (8h, as in 1992) | real24h (wall clock).\n\
             daynight={}\n",
            self.windowed,
            self.mute,
            self.speed,
            self.scale.as_str(),
            self.filter.as_str(),
            self.daynight.as_str(),
        )
    }

    /// Apply CLI overrides: `--windowed`, `--mute`, `--speed <pct>`, `--scale <mode>`,
    /// `--filter <nearest|linear|xbr>`. Unknown flags are ignored (`--data` is elsewhere).
    pub fn apply_args(&mut self, args: &[String]) {
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--windowed" => self.windowed = true,
                "--mute" => self.mute = true,
                "--speed" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                        self.speed = n.clamp(SPEED_MIN, SPEED_MAX);
                        i += 1;
                    }
                }
                "--scale" => {
                    if let Some(m) = args.get(i + 1).and_then(|v| ScaleMode::parse(v)) {
                        self.scale = m;
                        i += 1;
                    }
                }
                "--filter" => {
                    if let Some(f) = args.get(i + 1).and_then(|v| Filter::parse(v)) {
                        self.filter = f;
                        i += 1;
                    }
                }
                "--daynight" => {
                    if let Some(m) = args.get(i + 1).and_then(|v| DayNight::parse(v)) {
                        self.daynight = m;
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }

    /// How long to show a frame of `ticks` engine ticks, in milliseconds, scaled by
    /// the configured speed (1 tick = 20 ms at 100%).
    pub fn frame_delay_ms(&self, ticks: u16) -> u64 {
        u64::from(ticks) * 20 * 100 / u64::from(self.speed)
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" | "on" => Some(true),
        "false" | "no" | "0" | "off" => Some(false),
        _ => None,
    }
}

/// The path to the config file (next to the day state), if a home dir is known.
pub fn config_file() -> Option<PathBuf> {
    Some(crate::state::state_dir()?.join("config.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_screensaver_friendly() {
        let c = Config::default();
        assert!(!c.windowed); // fullscreen
        assert!(!c.mute);
        assert_eq!(c.speed, 100);
        assert_eq!(c.scale, ScaleMode::Fit);
        assert_eq!(c.filter, Filter::Linear); // smooth by default
    }

    #[test]
    fn parse_round_trips() {
        let c = Config {
            windowed: true,
            mute: true,
            speed: 250,
            scale: ScaleMode::Integer,
            filter: Filter::Nearest,
            daynight: DayNight::Real24h,
        };
        assert_eq!(Config::parse(&c.serialize()), c);
    }

    #[test]
    fn parse_is_lenient() {
        let c = Config::parse(
            "# comment\n\nwindowed=yes\nmute=off\nbogus=1\nscale=stretch\nfilter=crisp\ndaynight=24h\n",
        );
        assert!(c.windowed);
        assert!(!c.mute);
        assert_eq!(c.scale, ScaleMode::Stretch);
        assert_eq!(c.filter, Filter::Nearest); // "crisp" synonym
        assert_eq!(c.daynight, DayNight::Real24h);
        assert_eq!(c.speed, 100); // untouched → default
    }

    #[test]
    fn speed_is_clamped() {
        assert_eq!(Config::parse("speed=5\n").speed, SPEED_MIN);
        assert_eq!(Config::parse("speed=9999\n").speed, SPEED_MAX);
    }

    #[test]
    fn cli_overrides_win() {
        let mut c = Config::default();
        let args: Vec<String> = [
            "x",
            "--windowed",
            "--mute",
            "--speed",
            "200",
            "--scale",
            "integer",
            "--filter",
            "nearest",
            "--daynight",
            "real24h",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        c.apply_args(&args);
        assert!(c.windowed);
        assert!(c.mute);
        assert_eq!(c.speed, 200);
        assert_eq!(c.scale, ScaleMode::Integer);
        assert_eq!(c.filter, Filter::Nearest);
        assert_eq!(c.daynight, DayNight::Real24h);
    }

    #[test]
    fn cli_speed_is_clamped_and_bad_values_ignored() {
        let mut c = Config::default();
        c.apply_args(&["x".into(), "--speed".into(), "1000".into()]);
        assert_eq!(c.speed, SPEED_MAX);
        let mut c2 = Config::default();
        c2.apply_args(&["x".into(), "--speed".into(), "abc".into()]);
        assert_eq!(c2.speed, 100); // unchanged
    }

    #[test]
    fn frame_delay_scales_with_speed() {
        let mut c = Config::default();
        assert_eq!(c.frame_delay_ms(6), 120); // 6 ticks * 20 ms at 100%
        c.speed = 200;
        assert_eq!(c.frame_delay_ms(6), 60); // twice as fast
        c.speed = 50;
        assert_eq!(c.frame_delay_ms(6), 240); // half speed
    }
}
