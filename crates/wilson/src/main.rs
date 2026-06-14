// SPDX-License-Identifier: GPL-3.0-or-later
//! Wilson Reborn — the Johnny Castaway screensaver, as a live window.
//!
//! Runs the [`wilson_engine`] runtime and presents each composited frame with
//! `softbuffer` in a `winit` window (nearest-neighbour upscaled). Any key or mouse
//! input quits, like a real screensaver.
//!
//! Usage:
//! - `wilson` — run with the built-in recreated demo assets.
//! - `wilson --data <dir>` — run with the user's original `RESOURCE.*` files.
//! - Windows screensaver verbs `/s` (show), `/p` (preview), `/c` (config) are accepted.

mod assets;
mod audio;
mod clock;
mod scale;
mod state;

use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use wilson_engine::{Director, Show};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Windows screensaver verbs: /c = configure (none yet), /p = preview (skip).
    if args
        .iter()
        .any(|a| a.to_ascii_lowercase().starts_with("/c"))
    {
        eprintln!("Wilson Reborn: configuration dialog is not implemented yet.");
        return;
    }
    if args
        .iter()
        .any(|a| a.to_ascii_lowercase().starts_with("/p"))
    {
        return;
    }

    let data_dir = args
        .windows(2)
        .find(|w| w[0] == "--data")
        .map(|w| w[1].clone());

    let (archive, palette) = match &data_dir {
        Some(dir) => match assets::load_real(std::path::Path::new(dir)) {
            Ok(loaded) => loaded,
            Err(e) => {
                eprintln!("Could not load data from {dir}: {e}\nFalling back to demo assets.");
                assets::demo_archive()
            }
        },
        None => assets::demo_archive(),
    };

    let clock = clock::now();
    // Resume the 11-day story arc from the last session, if we saved one.
    let director = match state::DayState::load() {
        Some(s) => Director::new(s.current_day, s.stored_yday),
        None => Director::new(1, clock.yday),
    };
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, seed);

    // Sound effects are loaded from the data dir (originals carry `soundN.wav`); the
    // player degrades to silence without the `audio` feature, a device, or the files.
    let audio = audio::Audio::new(data_dir.as_deref().map(std::path::Path::new));

    // Persist the story day whenever it advances, so the arc carries over to the next
    // run. `None` until the first frame establishes today's day.
    let mut last_saved: Option<(u8, i32)> = None;

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let window = Rc::new(
        WindowBuilder::new()
            .with_title("Wilson Reborn — Johnny Castaway")
            .with_inner_size(winit::dpi::LogicalSize::new(640.0, 480.0))
            .build(&event_loop)
            .expect("failed to create window"),
    );
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
                        let rgba = frame.surface.to_rgba(&palette);
                        let mut buffer = surface.buffer_mut().expect("surface buffer");
                        scale::scale_rgba_to_argb_fit(
                            &rgba,
                            640,
                            480,
                            &mut buffer,
                            size.width as usize,
                            size.height as usize,
                        );
                        buffer.present().expect("present");
                        let delay = Duration::from_millis(u64::from(frame.delay_ticks) * 20);
                        elwt.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
                    }
                }
                _ => {}
            },
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        })
        .expect("event loop error");
}
