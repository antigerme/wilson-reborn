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

/// Resolve the data directory: the explicit `--data` path if given, otherwise the first
/// of the working directory or the executable's directory that contains `RESOURCE.MAP`.
pub fn find_data_dir(explicit: Option<&str>) -> Option<PathBuf> {
    if let Some(dir) = explicit {
        return Some(PathBuf::from(dir));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    candidates
        .into_iter()
        .find(|c| c.join("RESOURCE.MAP").is_file())
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
    fn explicit_dir_is_used_verbatim() {
        assert_eq!(
            find_data_dir(Some("/some/where")),
            Some(PathBuf::from("/some/where"))
        );
    }
}
