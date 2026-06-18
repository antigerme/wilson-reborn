// SPDX-License-Identifier: GPL-3.0-or-later
//! The bundled Wilson Reborn application icon (our own original art) must exist and be a
//! valid `.ico`, so the Windows build can embed it. (The *original* Johnny Castaway icon
//! is never committed — it is extracted from the user's own data only in `embed-data`
//! builds; see `build.rs`.)

use std::path::Path;

#[test]
fn bundled_icon_is_a_valid_ico() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/wilson.ico");
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("assets/wilson.ico must exist ({e}): {}", path.display()));
    assert!(
        bytes.len() > 256,
        "icon should be non-trivial, got {} bytes",
        bytes.len()
    );
    // ICONDIR: reserved=0, type=1 (icon), idCount>=1.
    assert_eq!(
        &bytes[0..4],
        &[0x00, 0x00, 0x01, 0x00],
        "valid .ico header (type = icon)"
    );
    let count = u16::from_le_bytes([bytes[4], bytes[5]]);
    assert!(count >= 1, "the .ico must contain at least one image");
}
