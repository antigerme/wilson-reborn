// SPDX-License-Identifier: GPL-3.0-or-later
//! Optional validation against the ORIGINAL (copyrighted) Johnny Castaway data.
//!
//! Skipped unless `WILSON_DATA_DIR` points at a directory containing `RESOURCE.MAP`
//! and the data file it references (e.g. `RESOURCE.001`). CI never has the data, so
//! this test no-ops there; run it locally with:
//!
//! ```sh
//! WILSON_DATA_DIR=/path/to/dist cargo test -p wilson-dgds --test real_data -- --nocapture
//! ```

use wilson_dgds::{Archive, ResourceMap};

#[test]
fn parses_and_decodes_real_data_if_present() {
    let Ok(dir) = std::env::var("WILSON_DATA_DIR") else {
        eprintln!("WILSON_DATA_DIR not set — skipping real-data validation");
        return;
    };

    let map = std::fs::read(format!("{dir}/RESOURCE.MAP")).expect("read RESOURCE.MAP");
    let rm = ResourceMap::parse(&map).expect("parse RESOURCE.MAP");
    let data = std::fs::read(format!("{dir}/{}", rm.data_file_name)).expect("read data file");

    let archive = Archive::parse(&map, &data).expect("parse the real archive");
    assert!(!archive.bitmaps.is_empty(), "expected BMP resources");
    assert!(!archive.screens.is_empty(), "expected SCR resources");
    assert!(!archive.ttms.is_empty(), "expected TTM resources");
    assert!(!archive.ads.is_empty(), "expected ADS resources");
    assert!(archive.palette().is_some(), "expected a palette");

    // Decompression + bytecode decoding must succeed on every script.
    for (name, ttm) in &archive.ttms {
        ttm.instructions()
            .unwrap_or_else(|e| panic!("TTM {name} failed to decode: {e}"));
    }
    for (name, ads) in &archive.ads {
        ads.instructions()
            .unwrap_or_else(|e| panic!("ADS {name} failed to decode: {e}"));
    }

    eprintln!(
        "real data OK: {} bmp, {} scr, {} ttm, {} ads",
        archive.bitmaps.len(),
        archive.screens.len(),
        archive.ttms.len(),
        archive.ads.len()
    );
}
