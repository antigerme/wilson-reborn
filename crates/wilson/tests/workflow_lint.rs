// SPDX-License-Identifier: GPL-3.0-or-later
//! Guards invariants of the CI/release workflows so fixed problems can't silently
//! regress. Runs in normal `cargo test` (so CI enforces it).

use std::path::PathBuf;

fn workflow(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github/workflows")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Regression guard for the v0.1.1 changelog duplication: if more than one release job
/// sets `generate_release_notes`, `action-gh-release` appends the changelog once per job.
#[test]
fn release_generates_notes_in_exactly_one_job() {
    let yml = workflow("release.yml");
    let n = yml.matches("generate_release_notes: true").count();
    assert_eq!(
        n, 1,
        "exactly one release job may set `generate_release_notes: true` (found {n}); \
         more than one duplicates the changelog in the GitHub Release"
    );
}

/// The release artifacts must never bundle the copyright game data — only the binaries.
#[test]
fn release_does_not_ship_game_data() {
    let yml = workflow("release.yml");
    for forbidden in ["RESOURCE.", "embed-data", ".wav", "dist.zip"] {
        assert!(
            !yml.contains(forbidden),
            "release.yml must not reference `{forbidden}` (copyright data stays out of \
             public release artifacts)"
        );
    }
}
