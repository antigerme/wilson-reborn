// SPDX-License-Identifier: GPL-3.0-or-later
//! Build script for the `wilson` app.
//!
//! * **Windows icon (always, Windows targets):** embed an application icon into the
//!   executable. By default it is Wilson Reborn's own icon (`assets/wilson.ico`, original
//!   art — safe to ship). In a personal `embed-data` build it instead uses the *original*
//!   Johnny Castaway icon taken from the user's own `SCRANTIC.EXE`/`.SCR` (or a
//!   `SCRANTIC.ICO`) at build time — copyright, so it is never committed nor shipped in the
//!   public release binaries.
//! * **Data embedding (`embed-data` feature):** embed the user's original data
//!   (`WILSON_EMBED_DATA`) into the binary for a self-contained build. The data is read
//!   only at build time from the local path — never written into the repository.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-env-changed=WILSON_EMBED_DATA");

    embed_windows_icon();

    if env::var_os("CARGO_FEATURE_EMBED_DATA").is_some() {
        embed_data();
    }
}

/// Embed the Windows application icon. No-op unless the *target* OS is Windows (so it is
/// correct even when cross-compiling to Windows from another host).
fn embed_windows_icon() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let default_ico = Path::new(&manifest).join("assets").join("wilson.ico");
    println!("cargo:rerun-if-changed={}", default_ico.display());

    // Default: our own (non-copyright) icon, in every Windows binary.
    let mut ico = default_ico;

    // Personal build: prefer the ORIGINAL icon from the user's own data (never committed) —
    // a `SCRANTIC.ICO` if provided, otherwise extracted from the NE `SCRANTIC.EXE`/`.SCR`.
    if env::var_os("CARGO_FEATURE_EMBED_DATA").is_some() {
        if let Some(dir) = env::var_os("WILSON_EMBED_DATA") {
            let dir = Path::new(&dir);
            let provided = dir.join("SCRANTIC.ICO");
            if provided.is_file() {
                ico = provided;
            } else if let Some(bytes) = ["SCRANTIC.EXE", "SCRANTIC.SCR"]
                .iter()
                .find_map(|n| fs::read(dir.join(n)).ok())
                .or_else(|| {
                    fs::read(dir.join("SCRANTIC.SC$"))
                        .ok()
                        .and_then(|c| wilson_dgds::decompress_installer(&c))
                })
            {
                if let Some(ico_bytes) = extract_ne_icon(&bytes) {
                    let p = Path::new(&env::var("OUT_DIR").unwrap()).join("original.ico");
                    if fs::write(&p, &ico_bytes).is_ok() {
                        ico = p;
                    }
                }
            }
        }
    }

    // Best-effort: a missing resource compiler (e.g. mingw `windres` on a cross build) must
    // not fail the build — warn and ship without the icon.
    // Embed the icon (best-effort: a missing resource compiler must not fail the build).
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        embed_icon_gnu(&ico);
    } else {
        let mut res = winres::WindowsResource::new();
        res.set_icon(&ico.to_string_lossy());
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed the Windows icon ({e}); building without it");
        }
    }
}

/// GNU/mingw target: compile the icon into a COFF object with the prefixed `windres` and
/// add that *object* directly to the link line. Linking it as a static lib can drop the
/// resource (nothing references it); an object passed straight to the linker is always kept.
fn embed_icon_gnu(ico: &Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc = out_dir.join("wilson_icon.rc");
    let obj = out_dir.join("wilson_icon_res.o");
    let ico_path = ico.to_string_lossy().replace('\\', "/");
    if fs::write(&rc, format!("1 ICON \"{ico_path}\"\n")).is_err() {
        return;
    }
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".into());
    let windres = format!("{arch}-w64-mingw32-windres");
    match std::process::Command::new(&windres)
        .arg("-O")
        .arg("coff")
        .arg("-i")
        .arg(&rc)
        .arg("-o")
        .arg(&obj)
        .status()
    {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", obj.display()),
        other => println!(
            "cargo:warning=could not embed the Windows icon via {windres} ({other:?}); building without it"
        ),
    }
}

/// Extract the first icon from a 16-bit **NE** (Windows 3.x) executable as `.ico` bytes.
/// The original `SCRANTIC.EXE`/`.SCR` is an NE binary; this rebuilds a standard `.ico`
/// from its `RT_GROUP_ICON` (type 14) + `RT_ICON` (type 3) resources. Returns `None` if
/// the input is not a parseable NE executable with an icon.
fn extract_ne_icon(exe: &[u8]) -> Option<Vec<u8>> {
    let u16le = |o: usize| exe.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]]));
    let u32le = |o: usize| {
        exe.get(o..o + 4)
            .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    };
    let ne = u32le(0x3C)? as usize;
    if exe.get(ne..ne + 2)? != b"NE" {
        return None;
    }
    let mut pos = ne + u16le(ne + 0x24)? as usize; // resource table offset
    let align = u16le(pos)? as u32;
    pos += 2;
    let shift = |v: u16| (v as usize) << align;

    let mut icons: std::collections::HashMap<u16, &[u8]> = std::collections::HashMap::new();
    let mut group: Option<&[u8]> = None;
    loop {
        let rt_type = u16le(pos)?;
        pos += 2;
        if rt_type == 0 {
            break;
        }
        let count = u16le(pos)? as usize;
        pos += 2 + 4; // resource count word + reserved dword
        let is_int = rt_type & 0x8000 != 0;
        let tid = rt_type & 0x7FFF;
        for _ in 0..count {
            let off = shift(u16le(pos)?);
            let len = shift(u16le(pos + 2)?);
            let id = u16le(pos + 6)? & 0x7FFF;
            pos += 12;
            let blob = exe.get(off..off + len)?;
            if is_int && tid == 3 {
                icons.insert(id, blob);
            } else if is_int && tid == 14 && group.is_none() {
                group = Some(blob);
            }
        }
    }

    // GRPICONDIR: reserved(2), type(2), count(2), then count GRPICONDIRENTRY (14 bytes each).
    let g = group?;
    let want = u16::from_le_bytes([*g.get(4)?, *g.get(5)?]) as usize;
    let mut metas: Vec<(&[u8], &[u8])> = Vec::new();
    for e in g.get(6..)?.chunks_exact(14).take(want) {
        let n_id = u16::from_le_bytes([e[12], e[13]]) & 0x7FFF;
        metas.push((e, icons.get(&n_id)?));
    }
    // Rebuild .ico: ICONDIR header + ICONDIRENTRY[] (16 bytes) + concatenated images.
    let mut entries = Vec::new();
    let mut body = Vec::new();
    let mut offset = 6 + 16 * metas.len();
    for (e, img) in &metas {
        entries.extend_from_slice(&e[0..8]); // width,height,colors,reserved,planes,bitcount
        entries.extend_from_slice(&(img.len() as u32).to_le_bytes());
        entries.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += img.len();
        body.extend_from_slice(img);
    }
    let mut out = Vec::with_capacity(6 + entries.len() + body.len());
    out.extend_from_slice(&0u16.to_le_bytes()); // reserved
    out.extend_from_slice(&1u16.to_le_bytes()); // type = icon
    out.extend_from_slice(&(metas.len() as u16).to_le_bytes());
    out.extend_from_slice(&entries);
    out.extend_from_slice(&body);
    Some(out)
}

/// Embed the user's original game data into the binary (the `embed-data` feature).
fn embed_data() {
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("embedded_data.rs");

    // The feature can be enabled without data (e.g. CI's `--all-features`). Emit a
    // compilable stub + a warning so the build still succeeds; the resulting binary
    // just won't run until built with WILSON_EMBED_DATA pointing at real data.
    let Some(dir) = env::var_os("WILSON_EMBED_DATA") else {
        println!(
            "cargo:warning=feature `embed-data` is enabled but WILSON_EMBED_DATA is unset; \
             building a stub (this binary will NOT run). Set WILSON_EMBED_DATA to embed the \
             original RESOURCE.* + soundN.wav."
        );
        fs::write(
            &out,
            "pub static MAP: &[u8] = &[];\npub static DATA: &[u8] = &[];\n\
             pub static SOUNDS: &[(u16, &[u8])] = &[];\n",
        )
        .expect("write embedded_data.rs stub");
        return;
    };
    let dir = Path::new(&dir);
    println!("cargo:rerun-if-changed={}", dir.display());

    let map_path = dir.join("RESOURCE.MAP");
    let map = fs::read(&map_path)
        .unwrap_or_else(|e| panic!("WILSON_EMBED_DATA: cannot read {}: {e}", map_path.display()));

    // RESOURCE.MAP: 6 unknown bytes, then a 13-byte NUL-padded data file name.
    let data_name: String = map
        .get(6..19)
        .map(|n| {
            n.iter()
                .take_while(|&&b| b != 0)
                .map(|&b| b as char)
                .collect()
        })
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| "RESOURCE.001".to_string());
    let out_parent = Path::new(&out).parent().unwrap().to_path_buf();

    // Resolve the data file. Normally RESOURCE.001 next to the MAP — but the original
    // *installer* stores it compressed as RESOURCE.00$; decompress that on the fly.
    let named = dir.join(&data_name);
    let data_path: PathBuf =
        if named.is_file() && fs::metadata(&named).map(|m| m.len()).unwrap_or(0) > 1000 {
            named
        } else if let Ok(comp) = fs::read(dir.join("RESOURCE.00$")) {
            let data = wilson_dgds::decompress_installer(&comp)
                .expect("WILSON_EMBED_DATA: RESOURCE.00$ present but could not be decompressed");
            let p = out_parent.join("RESOURCE.001");
            fs::write(&p, &data).expect("write decompressed RESOURCE.001");
            p
        } else {
            panic!(
                "WILSON_EMBED_DATA: data file {} not found (and no RESOURCE.00$ installer file)",
                named.display()
            );
        };

    // Embed whichever soundN.wav are present (0..=24; the originals skip 11 and 13).
    let mut entries: Vec<(u16, PathBuf)> = Vec::new();
    for id in 0u16..25 {
        let wav = dir.join(format!("sound{id}.wav"));
        if wav.is_file() {
            entries.push((id, wav));
        }
    }
    // Fallback: no soundN.wav — the effects are WAVs inside SCRANTIC.EXE/.SCR (or the
    // installer's compressed SCRANTIC.SC$). Extract them to OUT_DIR and embed.
    if entries.is_empty() {
        let exe_bytes = ["SCRANTIC.EXE", "SCRANTIC.SCR"]
            .iter()
            .find_map(|n| fs::read(dir.join(n)).ok())
            .or_else(|| {
                fs::read(dir.join("SCRANTIC.SC$"))
                    .ok()
                    .and_then(|c| wilson_dgds::decompress_installer(&c))
            });
        if let Some(bytes) = exe_bytes {
            for (id, slot) in wilson_dgds::sounds_from_scrantic_exe(&bytes)
                .iter()
                .enumerate()
            {
                if let Some(wav) = slot {
                    let path = out_parent.join(format!("sound{id}.wav"));
                    fs::write(&path, wav).expect("write extracted soundN.wav");
                    entries.push((id as u16, path));
                }
            }
        }
    }
    let mut sounds = String::from("&[");
    for (id, path) in &entries {
        sounds.push_str(&format!(
            "({id}, include_bytes!(r\"{}\")), ",
            path.display()
        ));
    }
    sounds.push(']');

    let code = format!(
        "// @generated by build.rs — embedded original data.\n\
         pub static MAP: &[u8] = include_bytes!(r\"{}\");\n\
         pub static DATA: &[u8] = include_bytes!(r\"{}\");\n\
         pub static SOUNDS: &[(u16, &[u8])] = {};\n",
        map_path.display(),
        data_path.display(),
        sounds,
    );
    fs::write(&out, code).expect("write embedded_data.rs");
}
