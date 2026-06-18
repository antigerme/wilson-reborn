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

/// The story arc is 11 days; `0` means "auto" (resume/today).
const MAX_STORY_DAY: u8 = 11;
/// Bounds for the story-mode per-day cadence, in real seconds.
const STORY_SECS_MIN: u32 = 5;
const STORY_SECS_MAX: u32 = 86_400;

/// Scene-transition style (between story runs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Transition {
    /// Hard cut — the faithful original behaviour (the original ships its dissolve disabled).
    #[default]
    None,
    /// The original's dormant LFSR tiled dissolve, resurrected (opt-in; KB10 §10.2).
    Dissolve,
}

impl Transition {
    fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "none" | "cut" | "hard" | "off" => Some(Transition::None),
            "dissolve" | "lfsr" | "tiled" => Some(Transition::Dissolve),
            _ => None,
        }
    }

    /// The canonical name (round-trips with [`Transition::parse`]).
    fn as_str(self) -> &'static str {
        match self {
            Transition::None => "none",
            Transition::Dissolve => "dissolve",
        }
    }
}

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
    /// xbr = edge-directed "HD" (dissolves dither), xbrz = edge-directed, keeps dither).
    pub filter: Filter,
    /// Smooth the dithered backgrounds (sea/sky) before scaling — off by default, since
    /// the dithering is the authentic 1992 look.
    pub dedither: bool,
    /// How the day/night cycle is driven (original 8-hour vs real 24-hour).
    pub daynight: DayNight,
    /// Show diagnostics: a per-second status line on stdout and an on-screen HUD overlay.
    pub debug: bool,
    /// Show the original's intro screen (`INTRO.SCR`) once at startup (default on, like the
    /// original's `Introduction` option). Disable with `--no-intro`.
    pub intro: bool,
    /// Start the 11-day story arc at this day (1–11); `0` = auto (resume the persisted day,
    /// or today). A one-off override (`--day N`), not persisted.
    pub day: u8,
    /// Story mode: play the whole 11-day arc in order (1 → 11 → 1 …) on a fixed cadence,
    /// instead of one day per real calendar day (`--story`). Starts at day 1.
    pub story: bool,
    /// In story mode, how many real seconds each story day is shown (`--story-secs`).
    pub story_secs: u32,
    /// Scene-transition style between story runs (default: hard cut, the faithful original).
    pub transition: Transition,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            windowed: false,
            mute: false,
            speed: 100,
            scale: ScaleMode::Fit,
            filter: Filter::default(),
            dedither: false,
            daynight: DayNight::Original,
            debug: false,
            intro: true,
            day: 0,
            story: false,
            story_secs: crate::timectl::DEFAULT_STORY_DAY_SECS,
            transition: Transition::None,
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
                "dedither" => {
                    if let Some(b) = parse_bool(value) {
                        c.dedither = b;
                    }
                }
                "debug" => {
                    if let Some(b) = parse_bool(value) {
                        c.debug = b;
                    }
                }
                "daynight" => {
                    if let Some(m) = DayNight::parse(value) {
                        c.daynight = m;
                    }
                }
                "intro" => {
                    if let Some(b) = parse_bool(value) {
                        c.intro = b;
                    }
                }
                "day" => {
                    if let Ok(n) = value.parse::<u8>() {
                        c.day = n.min(MAX_STORY_DAY);
                    }
                }
                "story" => {
                    if let Some(b) = parse_bool(value) {
                        c.story = b;
                    }
                }
                "story_secs" => {
                    if let Ok(n) = value.parse::<u32>() {
                        c.story_secs = n.clamp(STORY_SECS_MIN, STORY_SECS_MAX);
                    }
                }
                "transition" => {
                    if let Some(t) = Transition::parse(value) {
                        c.transition = t;
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
             # scale: window fit — fit | stretch | integer | extend (extend fills widescreen).\n\
             scale={}\n\
             # filter: pixel sampling — nearest (crisp/retro) | linear (smooth) | xbr (HD,\n\
             # dissolves dither) | xbrz (smooth edges, keeps dither).\n\
             filter={}\n\
             # dedither: true smooths the dithered sea/sky (default false = authentic look).\n\
             dedither={}\n\
             # daynight: day/night cycle — original (8h, as in 1992) | real24h (wall clock).\n\
             daynight={}\n\
             # debug: true shows diagnostics (stdout status line + on-screen HUD overlay).\n\
             debug={}\n\
             # intro: true shows the original's intro screen (INTRO.SCR) once at startup.\n\
             intro={}\n\
             # day: start the 11-day arc at this day (1–{MAX_STORY_DAY}); 0 = auto (resume/today).\n\
             day={}\n\
             # story: true plays the whole arc in order (day 1→11→1…) on a fixed cadence.\n\
             story={}\n\
             # story_secs: in story mode, real seconds per story day ({STORY_SECS_MIN}–{STORY_SECS_MAX}).\n\
             story_secs={}\n\
             # transition: scene transition — none (hard cut, faithful) | dissolve (the\n\
             # original's dormant LFSR tiled dissolve, resurrected).\n\
             transition={}\n",
            self.windowed,
            self.mute,
            self.speed,
            self.scale.as_str(),
            self.filter.as_str(),
            self.dedither,
            self.daynight.as_str(),
            self.debug,
            self.intro,
            self.day,
            self.story,
            self.story_secs,
            self.transition.as_str(),
        )
    }

    /// Apply CLI overrides: `--windowed`, `--mute`, `--dedither`, `--debug`, `--no-intro`,
    /// `--story`, `--speed <pct>`, `--scale <mode>`, `--filter <nearest|linear|xbr|xbrz>`,
    /// `--day <1-11>`, `--story-secs <s>`, `--transition <none|dissolve>`. Unknown flags are
    /// ignored.
    pub fn apply_args(&mut self, args: &[String]) {
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--windowed" => self.windowed = true,
                "--mute" => self.mute = true,
                "--dedither" => self.dedither = true,
                "--debug" => self.debug = true,
                "--no-intro" => self.intro = false,
                "--story" => self.story = true,
                "--day" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u8>().ok()) {
                        self.day = n.min(MAX_STORY_DAY);
                        i += 1;
                    }
                }
                "--story-secs" => {
                    if let Some(n) = args.get(i + 1).and_then(|v| v.parse::<u32>().ok()) {
                        self.story_secs = n.clamp(STORY_SECS_MIN, STORY_SECS_MAX);
                        i += 1;
                    }
                }
                "--transition" => {
                    if let Some(t) = args.get(i + 1).and_then(|v| Transition::parse(v)) {
                        self.transition = t;
                        i += 1;
                    }
                }
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
    /// the configured speed (1 tick = [`wilson_engine::MS_PER_TICK`] = 16 ms at 100%, the
    /// original's measured rate).
    pub fn frame_delay_ms(&self, ticks: u16) -> u64 {
        u64::from(ticks) * wilson_engine::MS_PER_TICK * 100 / u64::from(self.speed)
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
        assert_eq!(c.filter, Filter::Linear); // linear by default
        assert!(!c.dedither); // keep the authentic dither by default
        assert!(!c.debug); // no diagnostics by default
        assert!(c.intro); // the intro screen is shown by default (like the original)
        assert_eq!(c.day, 0); // auto: resume the persisted/real day
        assert!(!c.story); // real calendar by default, not story mode
        assert_eq!(c.story_secs, crate::timectl::DEFAULT_STORY_DAY_SECS);
        assert_eq!(c.transition, Transition::None); // hard cut by default (faithful)
    }

    #[test]
    fn parse_round_trips() {
        let c = Config {
            windowed: true,
            mute: true,
            speed: 250,
            scale: ScaleMode::Integer,
            filter: Filter::Nearest,
            dedither: true,
            daynight: DayNight::Real24h,
            debug: true,
            intro: false,
            day: 7,
            story: true,
            story_secs: 120,
            transition: Transition::Dissolve,
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
    fn story_options_are_clamped() {
        assert_eq!(Config::parse("day=99\n").day, MAX_STORY_DAY); // clamp to the 11-day arc
        assert_eq!(Config::parse("story_secs=1\n").story_secs, STORY_SECS_MIN);
        assert_eq!(
            Config::parse("story_secs=999999\n").story_secs,
            STORY_SECS_MAX
        );
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
            "--dedither",
            "--debug",
            "--daynight",
            "real24h",
            "--no-intro",
            "--story",
            "--day",
            "5",
            "--story-secs",
            "120",
            "--transition",
            "dissolve",
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
        assert!(c.dedither);
        assert!(c.debug);
        assert_eq!(c.daynight, DayNight::Real24h);
        assert!(!c.intro); // --no-intro
        assert!(c.story); // --story
        assert_eq!(c.day, 5); // --day 5
        assert_eq!(c.story_secs, 120); // --story-secs 120
        assert_eq!(c.transition, Transition::Dissolve); // --transition dissolve
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
        assert_eq!(c.frame_delay_ms(6), 96); // 6 ticks * 16 ms at 100% (the original's rate)
        c.speed = 200;
        assert_eq!(c.frame_delay_ms(6), 48); // twice as fast
        c.speed = 50;
        assert_eq!(c.frame_delay_ms(6), 192); // half speed
    }
}
