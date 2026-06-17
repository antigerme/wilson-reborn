// SPDX-License-Identifier: GPL-3.0-or-later
//! `audit` — deep content telemetry over a long run on the REAL data, to verify nothing
//! is missing: every sound id actually emitted, every ADS scene reached, every story day,
//! and that each holiday date triggers its holiday. Read-only; prints a report.
//!
//! Usage: WILSON_DATA_DIR=<dir> cargo run -p wilson-engine --example audit

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use wilson_dgds::{find_ci, Archive, Palette, ResourceMap};
use wilson_engine::{Clock, Director, Show};

fn main() {
    let dir = std::env::var("WILSON_DATA_DIR").expect("set WILSON_DATA_DIR to the real data dir");
    let (archive, palette) = load_data(Path::new(&dir)).expect("load real data");

    // --- 1) Long run at the default June date: collect sounds, scenes, days ---------
    let director = Director::new(1, 0);
    let clock = Clock {
        yday: 0,
        hour: 12,
        month: 6,
        day: 14,
    };
    let mut show = Show::new(&archive, &palette, 640, 480, director, clock, 7);
    let frames = 120_000u32; // ~ many full 11-day cycles
    let mut sounds: BTreeMap<u16, u32> = BTreeMap::new();
    let mut scenes: BTreeSet<(String, u16)> = BTreeSet::new();
    let mut days = [0u32; 12];
    let mut yday = 0i32;
    for i in 0..frames {
        // Advance the calendar day periodically so the 11-day story arc progresses
        // (the day rolls over when the wall-clock day changes), exercising every day's
        // scripted beats (Suzy/Mary/etc.) and their sounds.
        if i % 1500 == 1499 {
            yday += 1;
            show.set_clock(Clock {
                yday,
                hour: 12,
                month: 6,
                day: 14,
            });
        }
        let frame = show.next_frame(&archive);
        for &s in &frame.sounds {
            *sounds.entry(s).or_default() += 1;
        }
        let d = show.debug_info();
        if let Some((n, t)) = d.scene {
            scenes.insert((n.to_string(), t));
        }
        if (d.day as usize) < days.len() {
            days[d.day as usize] += 1;
        }
    }

    println!("=== LONG RUN ({frames} frames, seed 7) ===");
    println!(
        "sound ids emitted ({}): {:?}",
        sounds.len(),
        sounds.keys().collect::<Vec<_>>()
    );
    println!("  with counts: {sounds:?}");
    println!("distinct ADS scenes reached: {}", scenes.len());
    let scene_names: BTreeSet<&str> = scenes.iter().map(|(n, _)| n.as_str()).collect();
    println!("  unique .ADS files: {scene_names:?}");
    print!("story days seen:");
    for (d, n) in days.iter().enumerate().skip(1) {
        if *n > 0 {
            print!(" d{d}={n}");
        }
    }
    println!();

    // --- 1b) Multi-seed coverage: confirm rare scenes/visitors (Suzy, etc.) and all
    //         sounds are *reachable* (one seed only samples a subset) ----------------
    let mut all_scenes: BTreeSet<(String, u16)> = BTreeSet::new();
    let mut all_ads: BTreeSet<String> = BTreeSet::new();
    let mut all_sounds: BTreeSet<u16> = BTreeSet::new();
    for seed in 0..24u64 {
        let mut show = Show::new(
            &archive,
            &palette,
            640,
            480,
            Director::new(1, 0),
            clock,
            seed,
        );
        let mut yd = 0i32;
        for i in 0..40_000u32 {
            if i % 1200 == 1199 {
                yd += 1;
                show.set_clock(Clock {
                    yday: yd,
                    hour: 12,
                    month: 6,
                    day: 14,
                });
            }
            let f = show.next_frame(&archive);
            for &s in &f.sounds {
                all_sounds.insert(s);
            }
            if let Some((n, t)) = show.debug_info().scene {
                all_scenes.insert((n.to_string(), t));
                all_ads.insert(n.to_string());
            }
        }
    }
    println!("\n=== COVERAGE (24 seeds × 40k frames) ===");
    println!("distinct scenes reached: {}", all_scenes.len());
    println!("ADS files reached ({}): {all_ads:?}", all_ads.len());
    println!(
        "sound ids reachable ({}): {:?}",
        all_sounds.len(),
        all_sounds.iter().collect::<Vec<_>>()
    );

    // --- 1c) Climactic-scene pick frequency: is SUZY reachable (just rare)? ---------
    // plan_run picks the FINAL scene uniformly among (day==0 generic) ∪ (day==N), exactly
    // like jc_reborn. SUZY (day 3/9) competes with all generic FINALs → authentically rare.
    println!("\n=== CLIMACTIC PICK FREQUENCY pick_scene(day, FINAL) — 500k draws ===");
    for day in [3u8, 9u8] {
        let mut rng = wilson_engine::Rng::new(1);
        let draws = 500_000u32;
        let mut suzy = 0u32;
        let mut candidates: BTreeSet<(String, u16)> = BTreeSet::new();
        for _ in 0..draws {
            if let Some(s) =
                wilson_engine::story::pick_scene(day, wilson_engine::story::FINAL, 0, &mut rng)
            {
                if s.ads_name == "SUZY.ADS" {
                    suzy += 1;
                }
                candidates.insert((s.ads_name.to_string(), s.ads_tag));
            }
        }
        println!(
            "  day {day}: {} distinct FINAL candidates; SUZY picked {suzy}/{draws} ({:.3}%)",
            candidates.len(),
            100.0 * f64::from(suzy) / f64::from(draws)
        );
    }

    // --- 1d) Day-3-pinned Show: does SUZY actually PLAY (produce frames)? -----------
    // Director at day 3 with a fixed yday so the day never advances → every run is a
    // day-3 run. SUZY (~5% of day-3 finals) should show up if Show plays it correctly.
    {
        let mut show = Show::new(
            &archive,
            &palette,
            640,
            480,
            Director::new(3, 100),
            Clock {
                yday: 100,
                hour: 12,
                month: 6,
                day: 14,
            },
            99,
        );
        let mut scene_frames: BTreeMap<String, u32> = BTreeMap::new();
        let mut day_seen: BTreeSet<u8> = BTreeSet::new();
        for _ in 0..300_000u32 {
            let _ = show.next_frame(&archive);
            let d = show.debug_info();
            day_seen.insert(d.day);
            if let Some((n, _)) = d.scene {
                *scene_frames.entry(n.to_string()).or_default() += 1;
            }
        }
        println!("\n=== DAY-3-PINNED Show (300k frames) ===");
        println!("  days seen: {day_seen:?}");
        println!(
            "  SUZY.ADS frames: {}",
            scene_frames.get("SUZY.ADS").copied().unwrap_or(0)
        );
        println!("  per-ADS frame counts: {scene_frames:?}");
    }

    // --- 2) Each holiday date must trigger its holiday ------------------------------
    println!("\n=== HOLIDAYS (set the date, run 12k frames, observe) ===");
    for (mo, dy, label) in [
        (6u8, 14u8, "June (none)"),
        (10, 30, "Halloween"),
        (3, 16, "St Patrick"),
        (12, 24, "Christmas"),
        (12, 31, "New Year"),
        (1, 1, "New Year (Jan 1)"),
    ] {
        let clock = Clock {
            yday: 0,
            hour: 12,
            month: mo,
            day: dy,
        };
        let mut show = Show::new(&archive, &palette, 640, 480, Director::new(1, 0), clock, 7);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for _ in 0..12_000 {
            let _ = show.next_frame(&archive);
            seen.insert(format!("{:?}", show.debug_info().holiday));
        }
        println!("  {mo:>2}/{dy:<2} {label:<18} holidays observed: {seen:?}");
    }
}

fn load_data(dir: &Path) -> Result<(Archive, Palette), String> {
    let map_path = find_ci(dir, "RESOURCE.MAP").ok_or("RESOURCE.MAP not found")?;
    let map = std::fs::read(&map_path).map_err(|e| e.to_string())?;
    let rm = ResourceMap::parse(&map).map_err(|e| e.to_string())?;
    let data_path = find_ci(dir, &rm.data_file_name).ok_or("data file not found")?;
    let data = std::fs::read(&data_path).map_err(|e| e.to_string())?;
    let archive = Archive::parse(&map, &data).map_err(|e| e.to_string())?;
    let palette = archive.palette().cloned().ok_or("no palette")?;
    Ok((archive, palette))
}
