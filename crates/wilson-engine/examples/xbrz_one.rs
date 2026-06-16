// SPDX-License-Identifier: GPL-3.0-or-later
//! Apply the engine's [`xbrz2x`] upscaler to one binary PPM (P6) image — the xBRZ
//! counterpart of `xbr_one`, for eyeballing xBRZ vs our xBR (Hyllian) offline.
//!
//! Usage: cargo run -p wilson-engine --example xbrz_one -- <in.ppm> <out.ppm>

use std::io::Write;

use wilson_engine::xbrz2x;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: xbrz_one <in.ppm> <out.ppm>");
        std::process::exit(2);
    }
    let (w, h, rgb) = read_ppm(&args[1]);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for px in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
    }
    let up = xbrz2x(&rgba, w, h);
    let mut out_rgb = Vec::with_capacity(w * 2 * h * 2 * 3);
    for px in up.chunks_exact(4) {
        out_rgb.extend_from_slice(&px[0..3]);
    }
    write_ppm(&args[2], w * 2, h * 2, &out_rgb);
    println!("{}x{} -> {}x{} ({})", w, h, w * 2, h * 2, args[2]);
}

/// Minimal binary PPM (P6) reader: returns `(width, height, rgb_bytes)`.
fn read_ppm(path: &str) -> (usize, usize, Vec<u8>) {
    let data = std::fs::read(path).expect("read ppm");
    let mut pos = 0;
    let mut tok = || -> String {
        while pos < data.len() && (data[pos].is_ascii_whitespace() || data[pos] == b'#') {
            if data[pos] == b'#' {
                while pos < data.len() && data[pos] != b'\n' {
                    pos += 1;
                }
            } else {
                pos += 1;
            }
        }
        let start = pos;
        while pos < data.len() && !data[pos].is_ascii_whitespace() {
            pos += 1;
        }
        String::from_utf8_lossy(&data[start..pos]).into_owned()
    };
    assert_eq!(tok(), "P6", "only binary PPM (P6) is supported");
    let w: usize = tok().parse().expect("width");
    let h: usize = tok().parse().expect("height");
    let _max: usize = tok().parse().expect("maxval");
    pos += 1; // single whitespace after maxval before the pixel data
    let body = data[pos..].to_vec();
    assert!(body.len() >= w * h * 3, "ppm body too short");
    (w, h, body)
}

fn write_ppm(path: &str, w: usize, h: usize, rgb: &[u8]) {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).expect("create ppm"));
    write!(f, "P6\n{w} {h}\n255\n").expect("ppm header");
    f.write_all(rgb).expect("ppm body");
}
