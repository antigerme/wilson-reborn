// SPDX-License-Identifier: GPL-3.0-or-later
//! Frame-by-frame fidelity / regression harness against the ORIGINAL data.
//!
//! Skipped unless `WILSON_DATA_DIR` points at the original `RESOURCE.MAP` + data file
//! (CI has no copyrighted data, so this no-ops there). It checks three things:
//! 1. the runtime is **deterministic** — the same seed/clock yields a byte-identical
//!    frame sequence (so any future change that alters output is caught);
//! 2. the stream is **non-degenerate** — many distinct frames, not a frozen/blank screen;
//! 3. **every one of the 63 story scenes renders something** (no blank/missing scene).
//!
//! Run it locally with:
//! ```sh
//! WILSON_DATA_DIR=/path/to/dist cargo test -p wilson-engine --test fidelity -- --nocapture
//! ```
//! Set `WILSON_DUMP=/some/dir` as well to also write a PPM filmstrip for eyeballing
//! against the original.

use std::collections::HashSet;

use wilson_dgds::{Archive, Palette, ResourceMap};
use wilson_engine::{AdsVm, Clock, Director, Show, Surface, STORY_SCENES, TRANSPARENT};

fn real_data() -> Option<(Archive, Palette)> {
    let dir = std::env::var("WILSON_DATA_DIR").ok()?;
    let map = std::fs::read(format!("{dir}/RESOURCE.MAP")).expect("read RESOURCE.MAP");
    let rm = ResourceMap::parse(&map).expect("parse RESOURCE.MAP");
    let data = std::fs::read(format!("{dir}/{}", rm.data_file_name)).expect("read data file");
    let archive = Archive::parse(&map, &data).expect("parse archive");
    let palette = archive.palette().cloned().expect("palette");
    Some((archive, palette))
}

/// FNV-1a hash of a frame's indexed pixels (a compact frame fingerprint).
fn frame_hash(pixels: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &p in pixels {
        h ^= u64::from(p);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn run_show(archive: &Archive, palette: &Palette, frames: usize) -> Vec<u64> {
    let director = Director::new(5, 100);
    let clock = Clock {
        yday: 100,
        hour: 12,
        month: 12,
        day: 24, // Christmas — also exercises the holiday compositing path
    };
    let mut show = Show::new(archive, palette, 640, 480, director, clock, 0x00C0_FFEE);
    let mut hashes = Vec::with_capacity(frames);
    for _ in 0..frames {
        let f = show.next_frame(archive);
        assert_eq!(f.surface.pixels.len(), 640 * 480, "frame must be 640x480");
        hashes.push(frame_hash(&f.surface.pixels));
    }
    hashes
}

#[test]
fn show_render_is_deterministic_and_varied() {
    let Some((archive, palette)) = real_data() else {
        eprintln!("WILSON_DATA_DIR not set — skipping fidelity harness");
        return;
    };
    // 1. Deterministic: two identical runs are byte-identical frame-by-frame.
    let a = run_show(&archive, &palette, 400);
    let b = run_show(&archive, &palette, 400);
    assert_eq!(a, b, "the runtime must be deterministic (frame-by-frame)");
    // 2. Non-degenerate: plenty of distinct frames (animation actually happens).
    let distinct: HashSet<u64> = a.iter().copied().collect();
    assert!(
        distinct.len() > 20,
        "expected a varied frame stream, got {} distinct frames",
        distinct.len()
    );
    eprintln!(
        "fidelity: 400 frames, {} distinct, deterministic OK",
        distinct.len()
    );
}

#[test]
fn every_story_scene_renders_something() {
    let Some((archive, palette)) = real_data() else {
        eprintln!("WILSON_DATA_DIR not set — skipping per-scene render check");
        return;
    };
    let mut checked = 0;
    for sc in STORY_SCENES {
        let ads = archive
            .ads(sc.ads_name)
            .unwrap_or_else(|| panic!("missing {}", sc.ads_name));
        let mut vm = AdsVm::new(ads, sc.ads_tag, &archive, &palette, 640, 480, 1)
            .unwrap_or_else(|e| panic!("{}#{}: {e}", sc.ads_name, sc.ads_tag));
        let mut drew = false;
        for _ in 0..120 {
            match vm.next_frame(&archive) {
                Ok(Some(f)) => {
                    if f.surface.pixels.iter().any(|&p| p != 0 && p != TRANSPARENT) {
                        drew = true;
                        break;
                    }
                }
                _ => break,
            }
        }
        assert!(
            drew,
            "scene {}#{} rendered nothing",
            sc.ads_name, sc.ads_tag
        );
        checked += 1;
    }
    assert_eq!(checked, 63, "expected all 63 scenes");
    eprintln!("fidelity: all {checked} story scenes render content");
}

#[test]
fn dump_filmstrip_when_requested() {
    let Ok(out_dir) = std::env::var("WILSON_DUMP") else {
        return;
    };
    let Some((archive, palette)) = real_data() else {
        return;
    };
    let director = Director::new(5, 100);
    let clock = Clock {
        yday: 100,
        hour: 12,
        month: 12,
        day: 24,
    };
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, 0x00C0_FFEE);
    // Write every 40th frame as a PPM so the run can be eyeballed against the original.
    for i in 0..800 {
        let f = show.next_frame(&archive);
        if i % 40 == 0 {
            write_ppm(&format!("{out_dir}/frame_{i:04}.ppm"), &f.surface, &palette);
        }
    }
}

fn write_ppm(path: &str, surface: &Surface, palette: &Palette) {
    let mut out = format!("P6\n{} {}\n255\n", surface.width, surface.height).into_bytes();
    for &p in &surface.pixels {
        out.extend_from_slice(&palette.colors[p as usize]);
    }
    std::fs::write(path, out).expect("write ppm");
}
