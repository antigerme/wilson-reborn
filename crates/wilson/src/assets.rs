// SPDX-License-Identifier: GPL-3.0-or-later
//! Loading the **original** Johnny Castaway data (`RESOURCE.MAP` + `RESOURCE.001`).
//!
//! Wilson Reborn plays the original game data — there is no bundled/recreated art. The
//! files are located via `--data <dir>`, or auto-detected in the working directory or
//! next to the executable.

use std::path::{Path, PathBuf};

use wilson_dgds::{find_ci, Archive, Palette, ResourceMap};

/// Load the original `RESOURCE.MAP` + its data file from `dir` (file names are matched
/// case-insensitively, so `resource.map` / `RESOURCE.001` etc. all work).
pub fn load(dir: &Path) -> Result<(Archive, Palette), String> {
    let map_path = find_ci(dir, "RESOURCE.MAP")
        .ok_or_else(|| format!("RESOURCE.MAP: not found in {dir:?}"))?;
    let map_bytes = std::fs::read(&map_path).map_err(|e| format!("RESOURCE.MAP: {e}"))?;
    let map = ResourceMap::parse(&map_bytes).map_err(|e| e.to_string())?;
    let data_path = find_ci(dir, &map.data_file_name)
        .ok_or_else(|| format!("{}: not found in {dir:?}", map.data_file_name))?;
    let archive_bytes =
        std::fs::read(&data_path).map_err(|e| format!("{}: {e}", map.data_file_name))?;
    let archive = Archive::parse(&map_bytes, &archive_bytes).map_err(|e| e.to_string())?;
    let palette = archive
        .palette()
        .cloned()
        .ok_or_else(|| "no palette (PAL) found in the data".to_string())?;
    Ok((archive, palette))
}

/// The directories searched for the data, in priority order: the explicit `--data`
/// path, `$WILSON_DATA_DIR`, then the working directory and the executable's directory
/// (each also probed for a `data/` subdirectory).
pub fn data_candidates(explicit: Option<&str>) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(dir) = explicit {
        bases.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::var_os("WILSON_DATA_DIR") {
        bases.push(PathBuf::from(dir));
    }
    if let Ok(cwd) = std::env::current_dir() {
        bases.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            bases.push(parent.to_path_buf());
        }
    }
    // Each base, then a `data/` subdirectory of it.
    let mut out = Vec::with_capacity(bases.len() * 2);
    for base in bases {
        out.push(base.clone());
        out.push(base.join("data"));
    }
    out
}

/// Whether `path` looks like a zip — by `.zip` extension or the `PK\x03\x04` magic.
fn is_zip(path: &Path) -> bool {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        return true;
    }
    use std::io::Read;
    let mut magic = [0u8; 4];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && &magic == b"PK\x03\x04"
}

/// True if `dir` has a usable `RESOURCE.MAP` + its named data file (not the installer's
/// 35-byte "has been compressed" stub).
fn dir_has_usable_data(dir: &Path) -> bool {
    let Some(map_path) = find_ci(dir, "RESOURCE.MAP") else {
        return false;
    };
    // The data file's name comes from the MAP (usually RESOURCE.001); also try that name
    // directly, so a quirky/garbage MAP doesn't hide a present RESOURCE.001.
    let big = |name: &str| {
        find_ci(dir, name)
            .and_then(|p| std::fs::metadata(p).ok())
            .is_some_and(|m| m.len() > 1000)
    };
    if big("RESOURCE.001") {
        return true;
    }
    std::fs::read(&map_path)
        .ok()
        .and_then(|b| ResourceMap::parse(&b).ok())
        .is_some_and(|m| big(&m.data_file_name))
}

/// True if `dir` holds the original *installer's* compressed data (e.g. `RESOURCE.00$`),
/// whose proprietary compression we cannot read.
fn dir_has_installer(dir: &Path) -> bool {
    find_ci(dir, "RESOURCE.00$").is_some()
}

/// Extract `zip_path` into a per-zip temp directory (reused across runs) and return it.
fn extract_zip_cached(zip_path: &Path) -> Result<PathBuf, String> {
    let stem = zip_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");
    let len = std::fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0);
    let dest = std::env::temp_dir().join(format!("wilson-reborn-{stem}-{len}"));
    if dir_has_usable_data(&dest) || dir_has_installer(&dest) {
        return Ok(dest); // already extracted on a previous run
    }
    let file = std::fs::File::open(zip_path).map_err(|e| format!("{}: {e}", zip_path.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("{}: not a readable zip: {e}", zip_path.display()))?;
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    zip.extract(&dest)
        .map_err(|e| format!("extracting {}: {e}", zip_path.display()))?;
    Ok(dest)
}

/// The message shown when no usable data source is found.
const NEED_DATA_MSG: &str = "Wilson Reborn needs the original Johnny Castaway data \
    (RESOURCE.MAP + RESOURCE.001). Pass --data <dir-or-zip>, set WILSON_DATA_DIR, or place \
    the files (or the Internet Archive scrantic-run.zip) in the current directory or next to \
    the executable. Get them at \
    https://archive.org/download/johnny-castaway-screensaver/scrantic-run.zip";

/// Turn a candidate path into a ready data directory: a folder with the data is used
/// as-is; a `.zip` (e.g. `scrantic-run.zip`) is extracted to a temp dir; the original
/// installer's compressed data is detected and reported. `Ok(None)` means "not a data
/// source" (skip it); `Err` means "recognised but unusable" (e.g. the installer).
fn prepare(path: &Path) -> Result<Option<PathBuf>, String> {
    if path.is_file() && is_zip(path) {
        let dir = extract_zip_cached(path)?;
        return prepare(&dir); // now a directory
    }
    if path.is_dir() {
        if dir_has_usable_data(path) {
            return Ok(Some(path.to_path_buf()));
        }
        if dir_has_installer(path) {
            return prepare_installer(path).map(Some);
        }
    }
    Ok(None)
}

/// Decompress an original *installer* directory (its `RESOURCE.00$` is PKWARE-DCL-imploded)
/// into a temp dir holding a plain `RESOURCE.MAP` + `RESOURCE.001` (and `SCRANTIC.SCR` for
/// sound). Reused across runs.
fn prepare_installer(dir: &Path) -> Result<PathBuf, String> {
    let map = find_ci(dir, "RESOURCE.MAP")
        .ok_or_else(|| "installer is missing RESOURCE.MAP".to_string())?;
    let r00 = find_ci(dir, "RESOURCE.00$")
        .ok_or_else(|| "installer is missing RESOURCE.00$".to_string())?;
    let len = std::fs::metadata(&r00).map(|m| m.len()).unwrap_or(0);
    let dest = std::env::temp_dir().join(format!("wilson-reborn-installer-{len}"));
    if dir_has_usable_data(&dest) {
        return Ok(dest); // already decompressed on a previous run
    }
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    std::fs::copy(&map, dest.join("RESOURCE.MAP")).map_err(|e| format!("RESOURCE.MAP: {e}"))?;
    let comp = std::fs::read(&r00).map_err(|e| format!("RESOURCE.00$: {e}"))?;
    let data = wilson_dgds::decompress_installer(&comp).ok_or_else(|| {
        "RESOURCE.00$: unexpected installer compression (could not decompress)".to_string()
    })?;
    std::fs::write(dest.join("RESOURCE.001"), &data).map_err(|e| e.to_string())?;
    // Sound: the effects are WAVs inside the (also compressed) executable.
    if let Some(scc) = find_ci(dir, "SCRANTIC.SC$") {
        if let Some(exe) = std::fs::read(&scc)
            .ok()
            .and_then(|c| wilson_dgds::decompress_installer(&c))
        {
            let _ = std::fs::write(dest.join("SCRANTIC.SCR"), exe);
        }
    }
    Ok(dest)
}

/// Candidate data paths (folders and zips), in priority order. Reuses [`data_candidates`]
/// (the explicit `--data` path — which may be a `.zip` — plus the dir search) and also
/// probes for the Internet Archive zips next to each.
fn resolve_candidates(explicit: Option<&str>) -> Vec<PathBuf> {
    let dirs = data_candidates(explicit);
    let mut zips: Vec<PathBuf> = Vec::new();
    for d in &dirs {
        zips.push(d.join("scrantic-run.zip"));
        zips.push(d.join("scrantic-installer.zip"));
    }
    let mut v = dirs;
    v.extend(zips);
    v
}

/// Resolve `--data`/auto-detect into a ready data directory. Accepts a **folder** or a
/// **`.zip`** (extracted to a temp dir). Returns a clear error if nothing usable is found
/// (preferring a specific reason, e.g. "that's the installer", over the generic message).
pub fn resolve_data_dir(explicit: Option<&str>) -> Result<PathBuf, String> {
    let mut reason: Option<String> = None;
    for cand in resolve_candidates(explicit) {
        match prepare(&cand) {
            Ok(Some(dir)) => return Ok(dir),
            Ok(None) => {}
            Err(e) => {
                if reason.is_none() {
                    reason = Some(e);
                }
            }
        }
    }
    Err(reason.unwrap_or_else(|| NEED_DATA_MSG.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A temp dir unique to this test process + `tag`.
    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("wilson-test-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_data_dir_reports_resource_map() {
        let err = load(Path::new("/nonexistent/wilson/data")).unwrap_err();
        assert!(err.contains("RESOURCE.MAP"), "unexpected error: {err}");
    }

    #[test]
    fn detects_usable_dir_and_installer_and_zip_magic() {
        // A usable dir: RESOURCE.MAP + a >1000-byte RESOURCE.001.
        let good = tmp("good");
        std::fs::write(good.join("RESOURCE.MAP"), [0u8; 32]).unwrap();
        std::fs::write(good.join("RESOURCE.001"), vec![0u8; 2000]).unwrap();
        assert!(dir_has_usable_data(&good));
        assert!(!dir_has_installer(&good));

        // The installer: a compressed RESOURCE.00$ + the 35-byte stub.
        let inst = tmp("inst");
        std::fs::write(inst.join("RESOURCE.MAP"), [0u8; 32]).unwrap();
        std::fs::write(
            inst.join("RESOURCE.001"),
            b"Resource #1 has been compressed.\r\n",
        )
        .unwrap();
        std::fs::write(inst.join("RESOURCE.00$"), vec![0u8; 4000]).unwrap();
        assert!(!dir_has_usable_data(&inst), "stub data file is not usable");
        assert!(dir_has_installer(&inst));
        // The installer is recognised and decompression is attempted; on this garbage
        // RESOURCE.00$ that fails with a clear error (real data is covered by a gated test).
        let err = prepare(&inst).unwrap_err();
        assert!(
            err.contains("RESOURCE.00$") || err.to_lowercase().contains("installer"),
            "{err}"
        );

        // is_zip by magic, regardless of extension.
        let zpath = tmp("z").join("data.bin");
        std::fs::write(&zpath, b"PK\x03\x04rest").unwrap();
        assert!(is_zip(&zpath));
        assert!(!is_zip(&good.join("RESOURCE.MAP")));
    }

    #[test]
    fn data_zip_is_accepted_directly() {
        // Build a scrantic-run-style zip: RESOURCE.MAP + a >1000-byte RESOURCE.001.
        let dir = tmp("zipsrc");
        let zip_path = dir.join("scrantic-run.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut w = zip::ZipWriter::new(file);
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            w.start_file("RESOURCE.MAP", opts).unwrap();
            w.write_all(&[0u8; 32]).unwrap();
            w.start_file("RESOURCE.001", opts).unwrap();
            w.write_all(&vec![7u8; 2000]).unwrap();
            w.finish().unwrap();
        }
        // resolve_data_dir extracts the zip and returns a usable directory.
        let got = resolve_data_dir(Some(zip_path.to_str().unwrap())).expect("zip accepted");
        assert!(dir_has_usable_data(&got), "extracted dir has usable data");
        assert!(find_ci(&got, "RESOURCE.001").is_some());
    }

    #[test]
    fn candidates_prioritise_explicit_then_probe_data_subdir() {
        let c = data_candidates(Some("/explicit"));
        // The explicit dir and its data/ subdir come first, in that order.
        assert_eq!(c[0], PathBuf::from("/explicit"));
        assert_eq!(c[1], PathBuf::from("/explicit/data"));
        // The working directory is searched too (after the explicit path).
        assert!(c.iter().any(|p| p.ends_with("data")));
        assert!(c.len() >= 4);
    }

    #[test]
    fn finds_real_data_when_present() {
        // Gated: only runs when WILSON_DATA_DIR points at real data.
        if std::env::var_os("WILSON_DATA_DIR").is_some() {
            let found = resolve_data_dir(None).expect("auto-detect via WILSON_DATA_DIR");
            assert!(find_ci(&found, "RESOURCE.MAP").is_some());
            assert!(load(&found).is_ok());
        }
    }

    #[test]
    fn resolves_real_scrantic_zip_if_present() {
        // Gated end-to-end: `WILSON_TEST_ZIP=/path/scrantic-run.zip` → accept the zip
        // directly, load the graphics, and extract the 23 sounds from the embedded EXE.
        let Some(z) = std::env::var_os("WILSON_TEST_ZIP") else {
            return;
        };
        let dir = resolve_data_dir(Some(z.to_str().unwrap())).expect("resolve real zip");
        assert!(load(&dir).is_ok(), "load graphics from the extracted zip");
        let exe = find_ci(&dir, "SCRANTIC.EXE")
            .or_else(|| find_ci(&dir, "SCRANTIC.SCR"))
            .expect("zip carries SCRANTIC.EXE/.SCR");
        let bytes = std::fs::read(exe).unwrap();
        let n = wilson_dgds::sounds_from_scrantic_exe(&bytes)
            .iter()
            .filter(|s| s.is_some())
            .count();
        assert_eq!(n, 23, "23 sounds extracted from the EXE inside the zip");
    }

    #[test]
    fn resolves_real_installer_if_present() {
        // Gated: `WILSON_TEST_INSTALLER=<dir-or-zip>` (the scrantic-installer with the
        // compressed RESOURCE.00$) → decompress and load the graphics.
        let Some(p) = std::env::var_os("WILSON_TEST_INSTALLER") else {
            return;
        };
        let dir = resolve_data_dir(Some(p.to_str().unwrap())).expect("resolve installer");
        assert!(
            load(&dir).is_ok(),
            "load graphics from the decompressed installer"
        );
    }
}
