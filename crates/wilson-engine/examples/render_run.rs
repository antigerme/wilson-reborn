// SPDX-License-Identifier: GPL-3.0-or-later
//! `render_run` — headless renderer that turns a Wilson Reborn engine run into image
//! frames, for **visual validation** (by a human or an AI) without a display.
//!
//! The on-screen app is just a thin window that blits the engine's [`Surface`] frames
//! through the palette. This example drives the **same engine** ([`Show::next_frame`])
//! and writes the resulting frames to disk as PPM (P6) images — so what you see here is
//! exactly what the screensaver shows, but reproducible and scriptable.
//!
//! Because a full hour of frames would be ~25 GB, you choose how many frames to advance
//! and how often to *save* one (`save_every`). A typical use samples one frame every few
//! simulated seconds across a long run, then tiles them into a montage to eyeball.
//!
//! Usage:
//!   cargo run -p wilson-engine --example render_run -- \
//!       <data-dir> <out-dir> [total_frames] [save_every] [seed]
//!
//!   <data-dir>     folder with the original RESOURCE.MAP + RESOURCE.001 (+ soundN.wav)
//!   <out-dir>      where to write frame_000000.ppm, frame_000001.ppm, …
//!   total_frames   engine frames to advance (default 27000 ≈ 1 h at ~7.7 fps)
//!   save_every     save one frame every N advanced (default 225 ≈ one per ~30 s)
//!   seed           RNG seed for reproducibility (default 1)
//!
//! Make a montage (one image) or an mp4 from the saved frames with ffmpeg, e.g.:
//!   ffmpeg -pattern_type glob -i 'out/*.ppm' -vf 'scale=320:240,tile=8x8' montage.png
//!   ffmpeg -framerate 8 -pattern_type glob -i 'out/*.ppm' -pix_fmt yuv420p run.mp4

use std::io::Write;
use std::path::{Path, PathBuf};

use wilson_dgds::{find_ci, Archive, Palette, ResourceMap};
use wilson_engine::{Clock, Director, Show};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: render_run <data-dir> <out-dir> [total_frames] [save_every] [seed]\n\
             example: cargo run -p wilson-engine --example render_run -- ./data ./out 27000 225 1"
        );
        std::process::exit(2);
    }
    let data_dir = PathBuf::from(&args[1]);
    let out_dir = PathBuf::from(&args[2]);
    let total_frames: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(27_000);
    let save_every: u32 = args
        .get(4)
        .and_then(|s| s.parse().ok())
        .unwrap_or(225)
        .max(1);
    let seed: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

    let (archive, palette) = load_data(&data_dir).unwrap_or_else(|e| {
        eprintln!("error loading data from {}: {e}", data_dir.display());
        std::process::exit(1);
    });
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    // Start the 11-day arc at day 1, noon in June (no holiday), like a fresh run.
    let director = Director::new(1, 0);
    let clock = Clock {
        yday: 0,
        hour: 12,
        month: 6,
        day: 14,
    };
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, seed);

    // Telemetry: total intended playback time (Σ delay·16 ms) and how often each story
    // day was seen — so the summary shows the run actually progressed and was paced.
    let mut total_ms: u64 = 0;
    let mut day_hits = [0u32; 12]; // index by story day 1..=11
    let mut saved = 0u32;

    for i in 0..total_frames {
        let frame = show.next_frame(&archive);
        total_ms += u64::from(frame.delay_ticks) * wilson_engine::MS_PER_TICK;
        let day = show.day_state().0 as usize;
        if day < day_hits.len() {
            day_hits[day] += 1;
        }
        if i % save_every == 0 {
            let rgb = rgba_to_rgb(&frame.surface.to_rgba(&palette));
            write_ppm(
                &out_dir.join(format!("frame_{saved:06}.ppm")),
                640,
                480,
                &rgb,
            );
            saved += 1;
        }
    }

    println!(
        "rendered {total_frames} frames, saved {saved} to {}",
        out_dir.display()
    );
    println!(
        "intended playback: {:.1} s ({:.1} min), avg {:.0} ms/frame",
        total_ms as f64 / 1000.0,
        total_ms as f64 / 60_000.0,
        total_ms as f64 / f64::from(total_frames)
    );
    print!("story-day frames seen:");
    for (d, n) in day_hits.iter().enumerate().skip(1) {
        if *n > 0 {
            print!(" d{d}={n}");
        }
    }
    println!();
}

/// Load `RESOURCE.MAP` + its data file from `dir` (case-insensitive names) and return
/// the parsed [`Archive`] and its [`Palette`].
fn load_data(dir: &Path) -> Result<(Archive, Palette), String> {
    let map_path = find_ci(dir, "RESOURCE.MAP").ok_or("RESOURCE.MAP not found")?;
    let map = std::fs::read(&map_path).map_err(|e| e.to_string())?;
    let rm = ResourceMap::parse(&map).map_err(|e| e.to_string())?;
    let data_path = find_ci(dir, &rm.data_file_name).ok_or("data file not found")?;
    let data = std::fs::read(&data_path).map_err(|e| e.to_string())?;
    let archive = Archive::parse(&map, &data).map_err(|e| e.to_string())?;
    let palette = archive.palette().cloned().ok_or("no palette in data")?;
    Ok((archive, palette))
}

/// Drop the alpha byte from RGBA pixels, yielding packed RGB for PPM output.
fn rgba_to_rgb(rgba: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        rgb.extend_from_slice(&px[0..3]);
    }
    rgb
}

/// Write a binary PPM (P6) image — a trivial, dependency-free format ffmpeg/most tools
/// read directly.
fn write_ppm(path: &Path, w: u32, h: u32, rgb: &[u8]) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).expect("create ppm"));
    write!(f, "P6\n{w} {h}\n255\n").expect("ppm header");
    f.write_all(rgb).expect("ppm body");
}
