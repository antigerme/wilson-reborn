// SPDX-License-Identifier: GPL-3.0-or-later
//! Wilson Reborn — the Johnny Castaway screensaver, as a live window.
//!
//! Runs the [`wilson_engine`] runtime on the **original** game data and presents each
//! composited frame with `softbuffer` in a `winit` window. Any key or mouse input quits,
//! like a real screensaver. Runs fullscreen by default (use `--windowed` for dev).
//!
//! It needs the original Johnny Castaway data (`RESOURCE.MAP` + `RESOURCE.001`):
//! - `wilson --data <dir>` — load the data from `<dir>`.
//! - `wilson` — auto-detects the data in the working directory or next to the executable.
//! - `wilson --windowed --mute --speed <pct> --scale fit|stretch|integer|extend
//!   --filter nearest|linear|xbr --dedither` — options (`extend` fills widescreen).
//! - Windows screensaver verbs: `/s` (show), `/c` (config), `/p <hwnd>` (preview embedded
//!   in the configuration pane — Windows only).

#[cfg(not(feature = "embed-data"))]
mod assets;
mod audio;
mod config;
#[cfg(feature = "embed-data")]
mod embedded;
mod scale;
mod state;
mod stats;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wilson_engine::{clock, Director, Show};
use winit::event::{ElementState, Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, WindowBuilder};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Help first, so `-h`/`--help` (Unix) and `/?`/`/help` (Windows) always work and
    // never touch the data/window.
    if wants_help(&args) {
        print_help();
        return;
    }

    // Load options (file → defaults), seed a default file on first run, then let CLI
    // flags override for this run only.
    let mut cfg = config::Config::load();
    if config::config_file().is_some_and(|p| !p.exists()) {
        cfg.save();
    }
    cfg.apply_args(&args);

    // Windows screensaver verbs.
    let action = screensaver_action(&args);
    match action {
        Action::Configure => {
            print_config_info(&cfg);
            return;
        }
        Action::Preview(_) if !cfg!(windows) => {
            eprintln!("Wilson Reborn: the /p preview is only supported on Windows.");
            return;
        }
        _ => {}
    }
    let preview_parent: Option<isize> = match action {
        Action::Preview(hwnd) => Some(hwnd),
        _ => None,
    };
    let is_preview = preview_parent.is_some();

    // Original data + sounds: embedded (self-contained `embed-data` build) or loaded
    // from disk (`--data`/auto-detect).
    #[cfg(feature = "embed-data")]
    let (archive, palette, audio) = {
        let (archive, palette) = embedded::archive_and_palette();
        let audio = audio::Audio::from_sounds(embedded::sound_bytes(), cfg.mute);
        (archive, palette, audio)
    };
    #[cfg(not(feature = "embed-data"))]
    let (archive, palette, audio) = {
        let data_arg = args
            .windows(2)
            .find(|w| w[0] == "--data")
            .map(|w| w[1].clone());
        let Some(data_dir) = assets::find_data_dir(data_arg.as_deref()) else {
            eprintln!(
                "Wilson Reborn needs the original Johnny Castaway data files \
                 (RESOURCE.MAP + RESOURCE.001).\n\
                 Pass --data <dir>, set WILSON_DATA_DIR, or place the files in the current \
                 directory or next to the executable.\nSearched:"
            );
            for c in assets::data_candidates(data_arg.as_deref()) {
                eprintln!("  {}", c.display());
            }
            return;
        };
        let (archive, palette) = match assets::load(&data_dir) {
            Ok(loaded) => loaded,
            Err(e) => {
                eprintln!("Could not load data from {}: {e}", data_dir.display());
                return;
            }
        };
        // Sound effects come from the data dir (originals carry `soundN.wav`); the player
        // degrades to silence without the `audio` feature, a device, the files, or mute.
        let audio = audio::Audio::new(Some(data_dir.as_path()), cfg.mute);
        (archive, palette, audio)
    };

    let clock = clock::now();
    // Resume the 11-day story arc from the last session, if we saved one, and apply the
    // configured day/night cycle.
    let director = match state::DayState::load() {
        Some(s) => Director::new(s.current_day, s.stored_yday),
        None => Director::new(1, clock.yday),
    }
    .with_daynight(cfg.daynight);
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, seed);

    // Persist the story day whenever it advances, so the arc carries over to the next
    // run. `None` until the first frame establishes today's day.
    let mut last_saved: Option<(u8, i32)> = None;

    // Lifetime statistics: count this session and accumulate runtime.
    let mut stats = stats::Stats::load();
    stats.sessions += 1;
    stats.note_day(show.day_state().0);
    stats.save();
    let base_secs = stats.total_secs;
    let session_start = Instant::now();
    let mut last_flush = Instant::now();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let mut builder = WindowBuilder::new().with_title("Wilson Reborn — Johnny Castaway");
    builder = if let Some(hwnd) = preview_parent {
        apply_preview(builder, hwnd) // embedded preview pane (Windows /p)
    } else if cfg.windowed {
        builder.with_inner_size(winit::dpi::LogicalSize::new(640.0, 480.0))
    } else {
        builder.with_fullscreen(Some(Fullscreen::Borderless(None)))
    };
    let window = Rc::new(builder.build(&event_loop).expect("failed to create window"));
    let context = softbuffer::Context::new(window.clone()).expect("softbuffer context");
    let mut surface =
        softbuffer::Surface::new(&context, window.clone()).expect("softbuffer surface");

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent { event, .. } => match event {
                // The preview pane must keep running on hover/keypress; only a real
                // close ends it. In normal/fullscreen mode, any input quits.
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::MouseInput { .. } if !is_preview => elwt.exit(),
                WindowEvent::KeyboardInput { event: key, .. }
                    if !is_preview && key.state == ElementState::Pressed =>
                {
                    elwt.exit();
                }
                WindowEvent::RedrawRequested => {
                    let size = window.inner_size();
                    if let (Some(w), Some(h)) =
                        (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                    {
                        surface.resize(w, h).expect("resize surface");
                        // Refresh the wall clock so the story day rolls over at midnight
                        // even within a single long-running session.
                        show.set_clock(clock::now());
                        let frame = show.next_frame(&archive);
                        for &id in &frame.sounds {
                            audio.play(id);
                        }
                        let (day, yday) = show.day_state();
                        if last_saved != Some((day, yday)) {
                            state::DayState {
                                current_day: day,
                                stored_yday: yday,
                            }
                            .save();
                            last_saved = Some((day, yday));
                        }
                        // Update lifetime stats, flushing to disk occasionally.
                        stats.note_day(day);
                        if last_flush.elapsed() >= Duration::from_secs(30) {
                            stats.total_secs = base_secs + session_start.elapsed().as_secs();
                            stats.save();
                            last_flush = Instant::now();
                        }
                        let rgba = frame.surface.to_rgba(&palette);
                        // Optional: smooth the dithered sea/sky before scaling.
                        let rgba = if cfg.dedither {
                            wilson_engine::dedither(&rgba, 640, 480)
                        } else {
                            rgba
                        };
                        let mut buffer = surface.buffer_mut().expect("surface buffer");
                        scale::scale_rgba_to_argb(
                            &rgba,
                            640,
                            480,
                            &mut buffer,
                            size.width as usize,
                            size.height as usize,
                            cfg.scale,
                            cfg.filter,
                        );
                        buffer.present().expect("present");
                        let delay = Duration::from_millis(cfg.frame_delay_ms(frame.delay_ticks));
                        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
                    }
                }
                _ => {}
            },
            // Pace the animation: redraw only when the per-frame timer elapses (or on
            // the initial start), not on every loop iteration. Requesting a redraw on
            // every `AboutToWait` would preempt the `WaitUntil` deadline and run the
            // engine as fast as the CPU spins (the screensaver played far too fast).
            Event::NewEvents(StartCause::Init | StartCause::ResumeTimeReached { .. }) => {
                window.request_redraw();
            }
            Event::LoopExiting => {
                stats.total_secs = base_secs + session_start.elapsed().as_secs();
                stats.save();
            }
            _ => {}
        })
        .expect("event loop error");
}

/// What the screensaver was asked to do (from the Windows `/s` `/p` `/c` verbs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// Show the screensaver (default).
    Show,
    /// Print/show the configuration (`/c`).
    Configure,
    /// Render the small preview embedded in the parent window handle (`/p <hwnd>`).
    Preview(isize),
}

/// Detect the screensaver action in `args`. Verbs may use `/` or `-`, be upper/lower
/// case, and carry an HWND either after a colon (`/p:1234`) or as the next argument
/// (`/p 1234`).
fn screensaver_action(args: &[String]) -> Action {
    let mut i = 1;
    while i < args.len() {
        let low = args[i].to_ascii_lowercase();
        if let Some(body) = low.strip_prefix('/').or_else(|| low.strip_prefix('-')) {
            let mut chars = body.chars();
            let verb = chars.next();
            let rest = chars.as_str(); // text after the verb letter
            let clean = rest.is_empty() || rest.starts_with(':');
            match verb {
                Some('c') if clean => return Action::Configure,
                Some('s') if clean => return Action::Show,
                Some('p') if clean => {
                    let hwnd = rest
                        .strip_prefix(':')
                        .and_then(parse_hwnd)
                        .or_else(|| args.get(i + 1).and_then(|n| parse_hwnd(n)))
                        .unwrap_or(0);
                    return Action::Preview(hwnd);
                }
                _ => {}
            }
        }
        i += 1;
    }
    Action::Show
}

/// Parse a window handle (decimal, or `0x`-prefixed hex), as Windows passes it.
fn parse_hwnd(s: &str) -> Option<isize> {
    let s = s.trim();
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(hex) => isize::from_str_radix(hex, 16).ok(),
        None => s.parse::<isize>().ok(),
    }
}

/// Apply the Windows preview-window settings: make the window a borderless child of the
/// preview pane (`hwnd`). A no-op on other platforms (where `/p` isn't used).
fn apply_preview(builder: WindowBuilder, hwnd: isize) -> WindowBuilder {
    #[cfg(windows)]
    {
        use std::num::NonZeroIsize;
        use winit::raw_window_handle::{RawWindowHandle, Win32WindowHandle};
        // The classic Windows preview pane is ~152×112 px.
        let b = builder
            .with_decorations(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(152u32, 112u32));
        if let Some(nz) = NonZeroIsize::new(hwnd) {
            let handle = RawWindowHandle::Win32(Win32WindowHandle::new(nz));
            // SAFETY: `hwnd` is the preview window handle Windows passed on the command
            // line; it is valid for the lifetime of the preview.
            return unsafe { b.with_parent_window(Some(handle)) };
        }
        b
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        builder
    }
}

/// Print the active configuration and where it lives (the textual `/c` dialog).
fn print_config_info(cfg: &config::Config) {
    let path = config::config_file()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown location)".to_string());
    println!("Wilson Reborn — configuration");
    println!("  file:     {path}");
    println!("  windowed: {}", cfg.windowed);
    println!("  mute:     {}", cfg.mute);
    println!("  speed:    {}%", cfg.speed);
    println!("  scale:    {}", cfg.scale.as_str());
    println!("  filter:   {}", cfg.filter.as_str());
    println!("  dedither: {}", cfg.dedither);
    println!("  daynight: {}", cfg.daynight.as_str());
    println!("  stats:    {}", stats::Stats::load().summary());
    println!(
        "Edit the file above, or pass --windowed/--mute/--dedither/--speed <pct>/\
         --scale <mode>/--filter <nearest|linear|xbr>/--daynight <original|real24h>."
    );
}

/// Whether `args` ask for help, accepting both Unix (`-h`, `--help`) and Windows
/// (`/?`, `/help`) conventions (case-insensitive).
fn wants_help(args: &[String]) -> bool {
    args.iter().skip(1).any(|a| {
        matches!(
            a.to_ascii_lowercase().as_str(),
            "-h" | "-help" | "--help" | "help" | "/?" | "/h" | "/help"
        )
    })
}

/// Print detailed, platform-appropriate usage. Unix shows `-h/--help`; Windows also
/// lists the screensaver verbs (`/s /c /p /?`). The `--…` options work on every platform.
fn print_help() {
    let help_flags = if cfg!(windows) {
        "/?, /help"
    } else {
        "-h, --help"
    };
    let cfg_path = config::config_file()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unknown location)".to_string());

    println!(
        "Wilson Reborn — a modern, portable clone of the 1992 \"Johnny Castaway\" \
         screensaver (Sierra/Dynamix).\n"
    );
    println!("USAGE:");
    println!("  wilson [OPTIONS]                  run the screensaver (fullscreen)");
    println!("  wilson --data <DIR> [OPTIONS]     use the original game data in <DIR>\n");
    println!("DATA (required):");
    println!("  Needs the original files RESOURCE.MAP + RESOURCE.001 (and soundN.wav for");
    println!("  sound). Located via --data <DIR>, the WILSON_DATA_DIR env var, the current");
    println!("  directory, or next to the executable; file names match case-insensitively.\n");
    println!("OPTIONS:");
    println!("  {help_flags:<33}show this help and exit");
    println!("  --windowed                       run in a 640x480 window (default: fullscreen)");
    println!("  --mute                           disable sound effects (default: sound on)");
    println!("  --speed <PCT>                    playback speed 25-400; 100 = original (default)");
    println!("  --scale <MODE>                   fit | stretch | integer | extend (default: fit)");
    println!("                                     extend = fill widescreen, no bars/distortion");
    println!("  --filter <nearest|linear|xbr>    pixel sampling (default: xbr):");
    println!("                                     nearest = crisp/retro,");
    println!("                                     linear  = smooth (bilinear),");
    println!("                                     xbr     = smooth + sharp (\"HD\")");
    println!("  --dedither                       smooth the dithered sea/sky (default: off)");
    println!("  --daynight <original|real24h>    day/night cycle (default: original 8h)");
    println!("  --data <DIR>                     game data folder (default: auto-detect)");
    if cfg!(windows) {
        println!("\nWINDOWS SCREENSAVER VERBS (as the OS invokes a .scr):");
        println!("  /s                               show the screensaver (same as no args)");
        println!("  /c                               show the configuration");
        println!("  /p <HWND>                        preview inside the configuration pane");
        println!("  /?, /help                        show this help");
    }
    println!("\nCONFIG FILE (persists options; flags override it for one run):");
    println!("  {cfg_path}");
    println!("\nEXAMPLES:");
    if cfg!(windows) {
        println!("  wilson.scr /c                    show the configuration");
        println!("  wilson.exe --windowed --speed 200");
    } else {
        println!("  wilson --data ~/jc --filter xbr");
        println!("  wilson --windowed --speed 200 --scale integer");
    }
    println!("\nFree software under GPL-3.0-or-later. Plays only your own original game data.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        std::iter::once("wilson")
            .chain(list.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn actions_are_detected() {
        assert_eq!(screensaver_action(&args(&["/c"])), Action::Configure);
        assert_eq!(screensaver_action(&args(&["-s"])), Action::Show);
        assert_eq!(screensaver_action(&args(&["/S:0"])), Action::Show);
        assert_eq!(screensaver_action(&args(&[])), Action::Show);
    }

    #[test]
    fn help_is_detected_per_platform() {
        // Unix and Windows conventions, case-insensitive.
        for f in [
            "-h", "--help", "-help", "help", "/?", "/h", "/help", "--HELP", "/Help",
        ] {
            assert!(wants_help(&args(&[f])), "should detect help flag {f:?}");
        }
        // Real options must not be mistaken for help.
        assert!(!wants_help(&args(&[])));
        assert!(!wants_help(&args(&["--windowed", "--filter", "xbr"])));
        assert!(!wants_help(&args(&["/c"])));
        assert!(!wants_help(&args(&["--data", "/some/dir"])));
    }

    #[test]
    fn preview_action_captures_hwnd() {
        // HWND as the next argument, or after a colon, or absent (0).
        assert_eq!(
            screensaver_action(&args(&["/p", "1234"])),
            Action::Preview(1234)
        );
        assert_eq!(
            screensaver_action(&args(&["/P:5678"])),
            Action::Preview(5678)
        );
        assert_eq!(screensaver_action(&args(&["-p"])), Action::Preview(0));
    }

    #[test]
    fn non_verbs_are_ignored() {
        assert_eq!(
            screensaver_action(&args(&["--data", "/some/dir"])),
            Action::Show
        );
        assert_eq!(
            screensaver_action(&args(&["--windowed", "--scale", "fit"])),
            Action::Show
        );
    }

    #[test]
    fn parse_hwnd_decimal_and_hex() {
        assert_eq!(parse_hwnd("1234"), Some(1234));
        assert_eq!(parse_hwnd("0x10"), Some(16));
        assert_eq!(parse_hwnd(" 42 "), Some(42));
        assert_eq!(parse_hwnd("nope"), None);
    }
}
