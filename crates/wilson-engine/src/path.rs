// SPDX-License-Identifier: GPL-3.0-or-later
//! Pathfinding between the island's six spots (A–F).
//!
//! **Byte-faithful** to the original `SCRANTIC.EXE`: for each `(from, to)` pair the binary
//! stores a *route stream* — per cursor spot, a weighted list of next-spot choices (weights
//! sum to 100). Walking is a **step-wise weighted pick**: stand at `from`, roll against that
//! spot's section to choose the next spot, repeat until `to` is reached. The data is in
//! [`crate::calcpath_data::ROUTE_STREAMS`] (extracted by `docs/reverse-engineering/
//! extract_calcpath.py`; see KB10 §10.3). This replaces the earlier jc_reborn-reconstructed
//! second-order `WALK_MATRIX`, which diverged from the original on all 30 pairs.

use crate::calcpath_data::{Section, ROUTE_STREAMS};
use crate::rng::Rng;

/// Number of island spots (A–F).
pub const NUM_OF_NODES: usize = 6;

/// The weighted moves available at `cursor` in `stream` (the section `-cursor`), if any.
fn section(stream: &'static [Section], cursor: u8) -> Option<&'static [(u8, u8)]> {
    stream
        .iter()
        .find(|(c, _)| *c == cursor)
        .map(|(_, moves)| *moves)
}

/// Weighted random pick of the next spot from `cursor` (mirrors the original's `rng % Σw`
/// then cumulative-subtract). Returns `None` at a dead end (e.g. the destination's section).
fn pick_next(stream: &'static [Section], cursor: u8, rng: &mut Rng) -> Option<u8> {
    let moves = section(stream, cursor)?;
    let total: u32 = moves.iter().map(|&(_, w)| u32::from(w)).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.below(total);
    for &(next, w) in moves {
        let w = u32::from(w);
        if roll < w {
            return Some(next);
        }
        roll -= w;
    }
    moves.last().map(|&(next, _)| next)
}

/// Pick one route from `from` to `to`, the original's way: a step-wise weighted walk over the
/// `(from, to)` route stream. Falls back to `[from]` for a degenerate/unknown pair.
pub fn calc_path(from: u8, to: u8, rng: &mut Rng) -> Vec<u8> {
    let (f, t) = (from as usize, to as usize);
    if f >= NUM_OF_NODES || t >= NUM_OF_NODES || from == to {
        return vec![from];
    }
    let stream = ROUTE_STREAMS[f][t];
    let mut route = vec![from];
    let mut cur = from;
    // A simple route visits at most NUM_OF_NODES spots; the bound also guards bad data.
    while cur != to && route.len() <= NUM_OF_NODES {
        match pick_next(stream, cur, rng) {
            Some(next) if !route.contains(&next) => {
                route.push(next);
                cur = next;
            }
            _ => break,
        }
    }
    if cur == to {
        route
    } else {
        // The curated streams always reach `to` from `from`; this is a defensive fallback for
        // any pick that dead-ends, so callers always get a route that arrives at `to`.
        calc_paths(from, to)
            .into_iter()
            .next()
            .unwrap_or_else(|| vec![from])
    }
}

/// Enumerate every simple route from `from` to `to` allowed by the original's route stream
/// (each a list of spots). Used for validation and reachability.
pub fn calc_paths(from: u8, to: u8) -> Vec<Vec<u8>> {
    let (f, t) = (from as usize, to as usize);
    if f >= NUM_OF_NODES || t >= NUM_OF_NODES {
        return Vec::new();
    }
    if from == to {
        return vec![vec![from]];
    }
    let stream = ROUTE_STREAMS[f][t];
    let mut out = Vec::new();
    let mut path = vec![from];
    enumerate(stream, from, to, &mut path, &mut out);
    out
}

fn enumerate(
    stream: &'static [Section],
    cur: u8,
    to: u8,
    path: &mut Vec<u8>,
    out: &mut Vec<Vec<u8>>,
) {
    if cur == to {
        out.push(path.clone());
        return;
    }
    let Some(moves) = section(stream, cur) else {
        return;
    };
    let mut tried = Vec::new();
    for &(next, _w) in moves {
        if path.contains(&next) || tried.contains(&next) {
            continue;
        }
        tried.push(next);
        path.push(next);
        enumerate(stream, next, to, path, out);
        path.pop();
    }
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

        // Every hop must be a choice the original's stream offers at that cursor.
        let stream = ROUTE_STREAMS[from as usize][to as usize];
        for w in path.windows(2) {
            let moves = section(stream, w[0]).unwrap_or(&[]);
            assert!(
                moves.iter().any(|&(next, _)| next == w[1]),
                "illegal hop {} -> {} in {path:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn every_pair_is_reachable_and_valid() {
        for from in 0..NUM_OF_NODES as u8 {
            for to in 0..NUM_OF_NODES as u8 {
                if from == to {
                    continue;
                }
                let paths = calc_paths(from, to);
                assert!(!paths.is_empty(), "no path from {from} to {to}");
                for p in &paths {
                    validate(p, from, to);
                }
            }
        }
    }

    #[test]
    fn calc_path_reaches_every_destination() {
        // The weighted walk must always arrive at `to` (and yield a valid, simple route) for
        // every pair over many seeds — the core guarantee of the faithful route streams.
        let mut rng = Rng::new(20_260_618);
        for from in 0..NUM_OF_NODES as u8 {
            for to in 0..NUM_OF_NODES as u8 {
                if from == to {
                    continue;
                }
                for _ in 0..300 {
                    let p = calc_path(from, to, &mut rng);
                    validate(&p, from, to);
                }
            }
        }
    }

    #[test]
    fn same_spot_is_trivial() {
        assert_eq!(calc_paths(3, 3), vec![vec![3]]);
        let mut rng = Rng::new(1);
        assert_eq!(calc_path(3, 3, &mut rng), vec![3]);
    }

    #[test]
    fn calc_path_is_deterministic_and_valid() {
        let mut rng = Rng::new(123);
        for _ in 0..200 {
            let p = calc_path(4, 5, &mut rng); // E -> F
            validate(&p, 4, 5);
        }
        // Same seed -> same choice.
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        assert_eq!(calc_path(0, 3, &mut a), calc_path(0, 3, &mut b));
    }

    #[test]
    fn no_direct_5_to_3_hop_matches_the_original() {
        // The clearest divergence the byte-check found: the original never walks spot 5→3
        // (0-based 4→2) directly — it detours via spot 4 (0-based 3). Guard it.
        for to in 0..NUM_OF_NODES as u8 {
            for p in calc_paths(4, to) {
                for w in p.windows(2) {
                    assert!(
                        !(w[0] == 4 && w[1] == 2),
                        "unexpected direct 5->3 hop in {p:?}"
                    );
                }
            }
        }
    }
}
