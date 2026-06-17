// SPDX-License-Identifier: GPL-3.0-or-later
//! Filesystem helpers for locating the original data files.

use std::path::{Path, PathBuf};

/// Find a file named `name` in `dir`, matching **case-insensitively** (ASCII).
///
/// The original DOS files are upper-case (`RESOURCE.MAP`, `RESOURCE.001`), but user
/// copies often differ in case (`resource.map`, `Resource.001`); on case-sensitive
/// filesystems (Linux) an exact lookup then fails. This tries an exact match first (fast
/// path) and otherwise scans the directory for the first case-insensitive match.
pub fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|f| f.eq_ignore_ascii_case(name))
        {
            let path = entry.path();
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_case_insensitively() {
        let dir = std::env::temp_dir().join(format!("wilson_ci_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("resource.001"), b"data").unwrap();

        // Any casing of the request finds the file (regardless of the host FS's own
        // case sensitivity), and the returned path reads back the right bytes.
        for req in ["resource.001", "RESOURCE.001", "Resource.001"] {
            let found = find_ci(&dir, req).unwrap_or_else(|| panic!("not found: {req}"));
            assert_eq!(fs::read(&found).unwrap(), b"data");
        }
        assert!(find_ci(&dir, "MISSING.MAP").is_none());

        let _ = fs::remove_dir_all(&dir);
    }
}
