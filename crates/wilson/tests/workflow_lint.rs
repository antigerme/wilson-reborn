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

/// Both workflows must opt JS actions into Node 24. Node 20 is deprecated — GitHub forces
/// Node 24 from 2026-06-16 — so `actions/upload-artifact`, `softprops/action-gh-release`,
/// `Swatinem/rust-cache`, etc. otherwise log a deprecation warning on every run.
#[test]
fn workflows_force_node24_for_js_actions() {
    for name in ["ci.yml", "release.yml", "pages.yml"] {
        let yml = workflow(name);
        assert!(
            yml.contains("FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true"),
            "{name} must set env `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` \
             (Node 20 actions are deprecated)"
        );
    }
}

/// The publicly-published artifacts must never bundle the copyright game data — the release
/// binaries and the GitHub Pages site are bring-your-own only.
#[test]
fn public_artifacts_do_not_ship_game_data() {
    for name in ["release.yml", "pages.yml"] {
        let yml = workflow(name);
        for forbidden in ["RESOURCE.", "embed-data", ".wav", "dist.zip"] {
            assert!(
                !yml.contains(forbidden),
                "{name} must not reference `{forbidden}` (copyright data stays out of \
                 public artifacts)"
            );
        }
    }
}
