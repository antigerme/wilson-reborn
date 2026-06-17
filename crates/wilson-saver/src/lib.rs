// SPDX-License-Identifier: GPL-3.0-or-later
//! C FFI for the macOS screensaver (`.saver`) bundle.
//!
//! A `.saver` is a loadable bundle subclassing `ScreenSaverView`, not a winit app, so it
//! can't reuse the `wilson` binary. This crate exposes the [`wilson_engine`] runtime over
//! a tiny C ABI that the Objective-C `ScreenSaverView` calls each `animateOneFrame`:
//! create a context, pull 640×480 RGBA frames, free it.
//!
//! Data: a screensaver can't take `--data`, so the context looks for the original game
//! files (`RESOURCE.MAP` + data) in `$WILSON_DATA_DIR` and then
//! `~/Library/Application Support/WilsonReborn/`. (A future `embed-data` variant can bake
//! them in for personal all-in-one builds.)

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use wilson_dgds::{find_ci, Archive, Palette, ResourceMap};
use wilson_engine::{clock, Director, Show};

/// The frame size the engine renders; the caller scales it to the view (4:3 letterbox).
pub const WILSON_WIDTH: u32 = 640;
/// The frame height the engine renders.
pub const WILSON_HEIGHT: u32 = 480;

const FRAME_BYTES: usize = (WILSON_WIDTH * WILSON_HEIGHT * 4) as usize;

/// Opaque runtime handle handed across the FFI.
pub struct WilsonCtx {
    archive: Archive,
    palette: Palette,
    show: Show,
}

/// Build the ordered list of directories to search for the original data.
fn data_dirs_from(data_env: Option<PathBuf>, home: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = data_env {
        dirs.push(d);
    }
    if let Some(h) = home {
        dirs.push(h.join("Library/Application Support/WilsonReborn"));
    }
    dirs
}

fn data_dirs() -> Vec<PathBuf> {
    data_dirs_from(
        std::env::var_os("WILSON_DATA_DIR").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

/// Load the archive + palette from the first directory that has valid data. File names
/// are matched case-insensitively (`resource.map` / `RESOURCE.001` / … all work).
fn load_data() -> Option<(Archive, Palette)> {
    for dir in data_dirs() {
        let Some(map) = find_ci(&dir, "RESOURCE.MAP").and_then(|p| std::fs::read(p).ok()) else {
            continue;
        };
        let Ok(rm) = ResourceMap::parse(&map) else {
            continue;
        };
        let Some(data) = find_ci(&dir, &rm.data_file_name).and_then(|p| std::fs::read(p).ok())
        else {
            continue;
        };
        let Ok(archive) = Archive::parse(&map, &data) else {
            continue;
        };
        if let Some(palette) = archive.palette().cloned() {
            return Some((archive, palette));
        }
    }
    None
}

/// Create the runtime, loading the original data. Returns null if the data isn't found
/// (the `ScreenSaverView` then shows a "data missing" message instead of animating).
///
/// The returned pointer must be released with [`wilson_saver_free`].
#[no_mangle]
pub extern "C" fn wilson_saver_new() -> *mut WilsonCtx {
    let Some((archive, palette)) = load_data() else {
        return std::ptr::null_mut();
    };
    let clock = clock::now();
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    let director = Director::new(1, clock.yday);
    let show = Show::new(
        &archive,
        &palette,
        WILSON_WIDTH as u16,
        WILSON_HEIGHT as u16,
        director,
        clock,
        seed,
    );
    Box::into_raw(Box::new(WilsonCtx {
        archive,
        palette,
        show,
    }))
}

/// Advance one frame and write 640×480 RGBA into `out` (`out_len` must be ≥ 640·480·4).
/// Returns the suggested delay until the next frame, in milliseconds (0 on error).
///
/// # Safety
/// `ctx` must come from [`wilson_saver_new`]; `out` must be valid for `out_len` writable
/// bytes.
#[no_mangle]
pub unsafe extern "C" fn wilson_saver_next_frame(
    ctx: *mut WilsonCtx,
    out: *mut u8,
    out_len: usize,
) -> u32 {
    if ctx.is_null() || out.is_null() || out_len < FRAME_BYTES {
        return 0;
    }
    let ctx = &mut *ctx;
    // Refresh the wall clock so the day rolls over even within a long session.
    ctx.show.set_clock(clock::now());
    let frame = ctx.show.next_frame(&ctx.archive);
    let rgba = frame.surface.to_rgba(&ctx.palette);
    let n = rgba.len().min(FRAME_BYTES);
    std::slice::from_raw_parts_mut(out, FRAME_BYTES)[..n].copy_from_slice(&rgba[..n]);
    u32::from(frame.delay_ticks) * 20 // one engine tick = 20 ms
}

/// Release a runtime handle. A null pointer is ignored.
///
/// # Safety
/// `ctx` must come from [`wilson_saver_new`] (or be null) and must not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn wilson_saver_free(ctx: *mut WilsonCtx) {
    if !ctx.is_null() {
        drop(Box::from_raw(ctx));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dirs_order_and_home_path() {
        let dirs = data_dirs_from(
            Some(PathBuf::from("/explicit")),
            Some(PathBuf::from("/home/u")),
        );
        assert_eq!(dirs[0], PathBuf::from("/explicit")); // explicit override first
        assert_eq!(
            dirs[1],
            PathBuf::from("/home/u/Library/Application Support/WilsonReborn")
        );
        // No HOME → only the explicit dir; no env → empty.
        assert_eq!(data_dirs_from(Some(PathBuf::from("/x")), None).len(), 1);
        assert!(data_dirs_from(None, None).is_empty());
    }

    #[test]
    fn ffi_is_null_safe() {
        let mut buf = vec![0u8; FRAME_BYTES];
        // Stepping or freeing a null context must be a safe no-op.
        unsafe {
            assert_eq!(
                wilson_saver_next_frame(std::ptr::null_mut(), buf.as_mut_ptr(), buf.len()),
                0
            );
            wilson_saver_free(std::ptr::null_mut());
        }
    }

    #[test]
    fn new_and_frame_with_real_data() {
        // Gated on the original data (skipped in CI). Uses $WILSON_DATA_DIR via data_dirs.
        if std::env::var_os("WILSON_DATA_DIR").is_none() {
            eprintln!("WILSON_DATA_DIR not set — skipping FFI real-data test");
            return;
        }
        let ctx = wilson_saver_new();
        assert!(!ctx.is_null(), "expected to load data from WILSON_DATA_DIR");
        let mut buf = vec![0u8; FRAME_BYTES];
        unsafe {
            let delay = wilson_saver_next_frame(ctx, buf.as_mut_ptr(), buf.len());
            assert!(delay > 0, "frame should report a positive delay");
            assert!(buf.iter().any(|&b| b != 0), "frame should not be all-zero");
            wilson_saver_free(ctx);
        }
    }
}
