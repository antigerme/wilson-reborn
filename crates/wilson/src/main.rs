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
//! - `wilson --windowed --mute --speed <pct> --scale fit|stretch|integer` — options.
//! - Windows screensaver verbs `/s` (show), `/p` (preview), `/c` (config) are accepted.

mod assets;
mod audio;
mod clock;
mod config;
mod scale;
mod state;
mod stats;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wilson_engine::{Director, Show};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Fullscreen, WindowBuilder};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Load options (file → defaults), seed a default file on first run, then let CLI
    // flags override for this run only.
    let mut cfg = config::Config::load();
    if config::config_file().is_some_and(|p| !p.exists()) {
        cfg.save();
    }
    cfg.apply_args(&args);

    // Windows screensaver verbs.
    match screensaver_verb(&args) {
        Some('c') => {
            print_config_info(&cfg);
            return;
        }
        Some('p') => return, // preview window embedding is not supported yet
        _ => {}              // 's' or none → show
    }

    let data_arg = args
        .windows(2)
        .find(|w| w[0] == "--data")
        .map(|w| w[1].clone());

    let Some(data_dir) = assets::find_data_dir(data_arg.as_deref()) else {
        eprintln!(
            "Wilson Reborn needs the original Johnny Castaway data files \
             (RESOURCE.MAP + RESOURCE.001).\n\
             Pass --data <dir>, or place the files in the current directory or next to \
             the executable."
        );
        return;
    };
    let (archive, palette) = match assets::load(&data_dir) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("Could not load data from {}: {e}", data_dir.display());
            return;
        }
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

    // Sound effects are loaded from the data dir (originals carry `soundN.wav`); the
    // player degrades to silence without the `audio` feature, a device, the files, or
    // when muted.
    let audio = audio::Audio::new(Some(data_dir.as_path()), cfg.mute);

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
    builder = if cfg.windowed {
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
                WindowEvent::CloseRequested | WindowEvent::MouseInput { .. } => elwt.exit(),
                WindowEvent::KeyboardInput { event: key, .. }
                    if key.state == ElementState::Pressed =>
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
                        let mut buffer = surface.buffer_mut().expect("surface buffer");
                        scale::scale_rgba_to_argb(
                            &rgba,
                            640,
                            480,
                            &mut buffer,
                            size.width as usize,
                            size.height as usize,
                            cfg.scale,
                        );
                        buffer.present().expect("present");
                        let delay = Duration::from_millis(cfg.frame_delay_ms(frame.delay_ticks));
                        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
                    }
                }
                _ => {}
            },
            Event::AboutToWait => window.request_redraw(),
            Event::LoopExiting => {
                stats.total_secs = base_secs + session_start.elapsed().as_secs();
                stats.save();
            }
            _ => {}
        })
        .expect("event loop error");
}

/// Detect a Windows screensaver verb in `args`: `/c` (configure), `/p` (preview) or
/// `/s` (show). Verbs may use `/` or `-`, be upper/lower case, and carry an HWND
/// suffix (`/p:1234`). Returns the verb letter, or `None` if absent.
fn screensaver_verb(args: &[String]) -> Option<char> {
    for a in args.iter().skip(1) {
        let low = a.to_ascii_lowercase();
        let body = low.strip_prefix('/').or_else(|| low.strip_prefix('-'));
        if let Some(body) = body {
            let mut chars = body.chars();
            if let Some(c @ ('c' | 'p' | 's')) = chars.next() {
                let rest = chars.as_str();
                if rest.is_empty() || rest.starts_with(':') {
                    return Some(c);
                }
            }
        }
    }
    None
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
    println!("  daynight: {}", cfg.daynight.as_str());
    println!("  stats:    {}", stats::Stats::load().summary());
    println!(
        "Edit the file above, or pass --windowed/--mute/--speed <pct>/--scale <mode>/\
         --daynight <original|real24h>."
    );
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
    fn verbs_are_detected() {
        assert_eq!(screensaver_verb(&args(&["/c"])), Some('c'));
        assert_eq!(screensaver_verb(&args(&["/P:1234"])), Some('p'));
        assert_eq!(screensaver_verb(&args(&["-s"])), Some('s'));
        assert_eq!(screensaver_verb(&args(&["/S:0"])), Some('s'));
    }

    #[test]
    fn non_verbs_are_ignored() {
        assert_eq!(screensaver_verb(&args(&["--data", "/some/dir"])), None);
        assert_eq!(
            screensaver_verb(&args(&["--windowed", "--scale", "fit"])),
            None
        );
        assert_eq!(screensaver_verb(&args(&[])), None);
    }
}
