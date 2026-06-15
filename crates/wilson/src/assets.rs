// SPDX-License-Identifier: GPL-3.0-or-later
//! Loading the **original** Johnny Castaway data (`RESOURCE.MAP` + `RESOURCE.001`).
//!
//! Wilson Reborn plays the original game data — there is no bundled/recreated art. The
//! files are located via `--data <dir>`, or auto-detected in the working directory or
//! next to the executable.

use std::path::{Path, PathBuf};

use wilson_dgds::{Archive, Palette, ResourceMap};

/// Load the original `RESOURCE.MAP` + its data file from `dir`.
pub fn load(dir: &Path) -> Result<(Archive, Palette), String> {
    let map_bytes =
        std::fs::read(dir.join("RESOURCE.MAP")).map_err(|e| format!("RESOURCE.MAP: {e}"))?;
    let map = ResourceMap::parse(&map_bytes).map_err(|e| e.to_string())?;
    let archive_bytes = std::fs::read(dir.join(&map.data_file_name))
        .map_err(|e| format!("{}: {e}", map.data_file_name))?;
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

/// Resolve the data directory: the first [`data_candidates`] entry that actually
/// contains `RESOURCE.MAP`. Falls back to the explicit `--data` path (so [`load`] can
/// report a clear error) when nothing matches.
pub fn find_data_dir(explicit: Option<&str>) -> Option<PathBuf> {
    let candidates = data_candidates(explicit);
    if let Some(found) = candidates.iter().find(|c| c.join("RESOURCE.MAP").is_file()) {
        return Some(found.clone());
    }
    explicit.map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_data_dir_reports_resource_map() {
        let err = load(Path::new("/nonexistent/wilson/data")).unwrap_err();
        assert!(err.contains("RESOURCE.MAP"), "unexpected error: {err}");
    }

    #[test]
    fn explicit_dir_falls_back_when_no_data_found() {
        // Nothing on disk matches, so the explicit path is returned for a clear error.
        assert_eq!(
            find_data_dir(Some("/nonexistent/wilson/xyz")),
            Some(PathBuf::from("/nonexistent/wilson/xyz"))
        );
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
        if let Some(dir) = std::env::var_os("WILSON_DATA_DIR") {
            let found = find_data_dir(None).expect("auto-detect via WILSON_DATA_DIR");
            assert!(found.join("RESOURCE.MAP").is_file());
            assert!(load(&found).is_ok());
            let _ = dir;
        }
    }
}
