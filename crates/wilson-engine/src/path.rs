// SPDX-License-Identifier: GPL-3.0-or-later
//! Pathfinding between the island's six spots (A–F).
//!
//! Faithful port of `calcpath.c` + `calcpath_data.h` (`repos/jc_reborn`). Movement
//! uses a **second-order** adjacency table `walk_matrix[prev][cur][next]`: which next
//! spot is reachable depends on where Johnny came from (this shapes natural-looking
//! turns). The first hop uses the "from any spot" row. `calc_paths` enumerates all
//! simple routes; `calc_path` picks one at random — matching the reference engine,
//! which acknowledges this is a plausible reconstruction rather than the original.

use crate::rng::Rng;

/// Number of island spots (A–F).
pub const NUM_OF_NODES: usize = 6;
/// Sentinel for "no previous node" (used for the first hop).
const UNDEF_NODE: u8 = 6;

/// `walk_matrix[prev][cur][next] != 0` ⇒ Johnny may go `cur → next` having arrived
/// from `prev`. Index `[6]` ("from any spot") is used for the first hop.
static WALK_MATRIX: [[[u8; NUM_OF_NODES]; NUM_OF_NODES]; NUM_OF_NODES + 1] = [
    // from A
    [
        [0, 0, 0, 0, 0, 0], // A
        [0, 0, 1, 0, 0, 0], // B
        [0, 0, 0, 1, 0, 0], // C
        [0, 0, 0, 0, 0, 0], // D
        [0, 0, 0, 1, 0, 1], // E
        [0, 0, 0, 0, 0, 0], // F
    ],
    // from B
    [
        [0, 0, 0, 0, 1, 0], // A
        [0, 0, 0, 0, 0, 0], // B
        [0, 0, 0, 1, 0, 0], // C
        [0, 0, 0, 0, 0, 0], // D
        [0, 0, 0, 0, 0, 0], // E
        [0, 0, 0, 0, 0, 0], // F
    ],
    // from C
    [
        [0, 0, 0, 0, 1, 0], // A
        [1, 0, 0, 0, 0, 0], // B
        [0, 0, 0, 0, 0, 0], // C
        [0, 0, 0, 0, 1, 0], // D
        [0, 0, 0, 0, 0, 0], // E
        [0, 0, 0, 0, 0, 0], // F
    ],
    // from D
    [
        [0, 0, 0, 0, 0, 0], // A
        [0, 0, 0, 0, 0, 0], // B
        [1, 1, 0, 0, 0, 1], // C
        [0, 0, 0, 0, 0, 0], // D
        [1, 0, 0, 0, 0, 0], // E
        [0, 0, 0, 0, 1, 0], // F
    ],
    // from E
    [
        [0, 1, 1, 0, 0, 0], // A
        [0, 0, 0, 0, 0, 0], // B
        [0, 0, 0, 0, 0, 0], // C
        [0, 0, 1, 0, 0, 0], // D
        [0, 0, 0, 0, 0, 0], // E
        [0, 0, 0, 1, 0, 0], // F
    ],
    // from F
    [
        [0, 0, 0, 0, 0, 0], // A
        [0, 0, 0, 0, 0, 0], // B
        [0, 0, 0, 1, 0, 0], // C
        [0, 0, 0, 0, 0, 0], // D
        [1, 0, 0, 0, 0, 0], // E
        [0, 0, 0, 0, 0, 0], // F
    ],
    // from any spot
    [
        [0, 1, 1, 0, 1, 1], // A
        [1, 0, 1, 0, 0, 0], // B
        [1, 1, 0, 1, 1, 1], // C
        [0, 0, 1, 0, 1, 1], // D
        [1, 0, 1, 1, 0, 1], // E
        [1, 0, 1, 1, 1, 0], // F
    ],
];

fn recurse(
    prev: u8,
    cur: u8,
    dst: u8,
    visited: &mut [bool; NUM_OF_NODES],
    path: &mut Vec<u8>,
    out: &mut Vec<Vec<u8>>,
) {
    if cur == dst {
        out.push(path.clone());
        return;
    }
    for next in 0..NUM_OF_NODES as u8 {
        if WALK_MATRIX[prev as usize][cur as usize][next as usize] != 0 && !visited[next as usize] {
            visited[next as usize] = true;
            path.push(next);
            recurse(cur, next, dst, visited, path, out);
            path.pop();
            visited[next as usize] = false;
        }
    }
}

/// Enumerate every simple route from `from` to `to` (each a list of spots).
pub fn calc_paths(from: u8, to: u8) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    if from as usize >= NUM_OF_NODES || to as usize >= NUM_OF_NODES {
        return out;
    }
    let mut visited = [false; NUM_OF_NODES];
    visited[from as usize] = true;
    let mut path = vec![from];
    recurse(UNDEF_NODE, from, to, &mut visited, &mut path, &mut out);
    out
}

/// Pick one random route from `from` to `to` (falls back to `[from]` if none exist).
pub fn calc_path(from: u8, to: u8, rng: &mut Rng) -> Vec<u8> {
    let paths = calc_paths(from, to);
    if paths.is_empty() {
        return vec![from];
    }
    paths[rng.below(paths.len() as u32) as usize].clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn validate(path: &[u8], from: u8, to: u8) {
        assert_eq!(path[0], from, "path must start at `from`");
        assert_eq!(*path.last().unwrap(), to, "path must end at `to`");

        let mut seen = HashSet::new();
        for &n in path {
            assert!(seen.insert(n), "path must be simple (no repeats): {path:?}");
        }

        let mut prev = UNDEF_NODE;
        for w in path.windows(2) {
            assert!(
                WALK_MATRIX[prev as usize][w[0] as usize][w[1] as usize] != 0,
                "illegal hop {} -> {} (from {prev}) in {path:?}",
                w[0],
                w[1]
            );
            prev = w[0];
        }
    }

    #[test]
    fn every_pair_is_reachable_and_valid() {
        for from in 0..NUM_OF_NODES as u8 {
            for to in 0..NUM_OF_NODES as u8 {
                let paths = calc_paths(from, to);
                assert!(!paths.is_empty(), "no path from {from} to {to}");
                for p in &paths {
                    validate(p, from, to);
                }
            }
        }
    }

    #[test]
    fn same_spot_is_trivial() {
        assert_eq!(calc_paths(3, 3), vec![vec![3]]);
    }

    #[test]
    fn calc_path_is_deterministic_and_valid() {
        let mut rng = Rng::new(123);
        for _ in 0..50 {
            let p = calc_path(4, 5, &mut rng); // E -> F
            validate(&p, 4, 5);
        }
        // Same seed -> same first choice.
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(calc_path(0, 3, &mut a), calc_path(0, 3, &mut b));
    }
}
