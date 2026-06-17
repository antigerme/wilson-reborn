// SPDX-License-Identifier: GPL-3.0-or-later
//! Extract the original digital sound effects from the original `SCRANTIC.EXE` /
//! `SCRANTIC.SCR`.
//!
//! The 1992 Johnny Castaway sound effects are **not** in `RESOURCE.001` — they are stored
//! as 23 standard **WAV/RIFF** chunks (mono, 11025 Hz, 8-bit PCM) embedded directly in the
//! original executable. This lets the app play sound straight from the original
//! distribution (e.g. `scrantic-run.zip`) without needing separately-extracted
//! `soundN.wav` files.
//!
//! The 23 effects are ids `0..=24` (the originals skip 11 and 13). They are *not* stored in
//! id order in the executable, but each effect's PCM `data` length is **unique**, so we map
//! each extracted chunk to its id by that length — robust to the chunk ordering. The
//! lengths were verified byte-for-byte against the canonical original (the data payloads
//! are identical to the `soundN.wav` shipped by the JCOS/jc_reborn projects).

/// `(pcm_data_length, sound_id)` for each of the 23 original effects. Verified against the
/// canonical `SCRANTIC.EXE`; the `data`-chunk length uniquely identifies each effect.
const SOUND_DATA_LEN_TO_ID: &[(usize, u16)] = &[
    (10262, 0),
    (11072, 1),
    (1488, 2),
    (7392, 3),
    (4992, 4),
    (2816, 5),
    (15744, 6),
    (14976, 7),
    (2304, 8),
    (3040, 9),
    (20224, 10),
    (5438, 12),
    (11328, 14),
    (2838, 15),
    (7604, 16),
    (4253, 17),
    (13943, 18),
    (3288, 19),
    (7215, 20),
    (4838, 21),
    (1292, 22),
    (1515, 23),
    (9672, 24),
];

/// The length of a WAV's `data` sub-chunk (the PCM payload), or `None` if absent/malformed.
fn wav_data_len(wav: &[u8]) -> Option<usize> {
    let mut p = 12; // skip "RIFF" + size + "WAVE"
    while p + 8 <= wav.len() {
        let size = u32::from_le_bytes(wav[p + 4..p + 8].try_into().ok()?) as usize;
        if &wav[p..p + 4] == b"data" {
            return Some(size);
        }
        p += 8 + size + (size & 1); // chunks are word-aligned
    }
    None
}

/// Scan `exe` (the bytes of `SCRANTIC.EXE`/`.SCR`) for the embedded WAV sound effects and
/// return them indexed by sound id (`0..=24`; absent ids — including 11 and 13 — are
/// `None`). Returns an all-`None` vec if no recognised effects are found.
pub fn sounds_from_scrantic_exe(exe: &[u8]) -> Vec<Option<Vec<u8>>> {
    let mut out = vec![None; 25];
    let mut i = 0usize;
    while i + 12 <= exe.len() {
        if &exe[i..i + 4] == b"RIFF" && &exe[i + 8..i + 12] == b"WAVE" {
            let riff_size = u32::from_le_bytes(exe[i + 4..i + 8].try_into().unwrap()) as usize + 8;
            if let Some(end) = i.checked_add(riff_size).filter(|&e| e <= exe.len()) {
                let wav = &exe[i..end];
                if let Some(dlen) = wav_data_len(wav) {
                    if let Some(&(_, id)) =
                        SOUND_DATA_LEN_TO_ID.iter().find(|&&(len, _)| len == dlen)
                    {
                        let slot = id as usize;
                        if slot < out.len() && out[slot].is_none() {
                            out[slot] = Some(wav.to_vec());
                        }
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid WAV (mono 11025 Hz 8-bit) with `data_len` payload bytes.
    fn make_wav(data_len: usize) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
        w.extend_from_slice(b"WAVE");
        w.extend_from_slice(b"fmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes()); // PCM
        w.extend_from_slice(&1u16.to_le_bytes()); // mono
        w.extend_from_slice(&11025u32.to_le_bytes());
        w.extend_from_slice(&11025u32.to_le_bytes()); // byte rate
        w.extend_from_slice(&1u16.to_le_bytes()); // block align
        w.extend_from_slice(&8u16.to_le_bytes()); // bits
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(data_len as u32).to_le_bytes());
        w.resize(w.len() + data_len, 0x80u8); // silent 8-bit PCM payload
        w
    }

    #[test]
    fn extracts_and_maps_embedded_wavs_by_length() {
        // A fake "EXE": junk, then sound id 2's WAV (data len 1488), junk, id 24 (9672).
        let mut exe = vec![0u8; 64];
        exe.extend_from_slice(&make_wav(1488)); // -> id 2
        exe.extend_from_slice(&[0xCCu8; 16]);
        exe.extend_from_slice(&make_wav(9672)); // -> id 24
        let sounds = sounds_from_scrantic_exe(&exe);
        assert_eq!(sounds.len(), 25);
        assert!(sounds[2].is_some(), "id 2 (len 1488) extracted");
        assert!(sounds[24].is_some(), "id 24 (len 9672) extracted");
        assert!(sounds[0].is_none() && sounds[11].is_none() && sounds[13].is_none());
        // The extracted bytes are a complete, replayable RIFF/WAVE.
        let w = sounds[2].as_ref().unwrap();
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(wav_data_len(w), Some(1488));
    }

    #[test]
    fn no_sounds_in_unrelated_bytes() {
        assert!(sounds_from_scrantic_exe(&[0u8; 1000])
            .iter()
            .all(Option::is_none));
        // A RIFF that isn't WAVE, or has an unknown data length, is ignored.
        assert!(sounds_from_scrantic_exe(&make_wav(99))
            .iter()
            .all(Option::is_none));
    }

    /// Gated on real data: if `WILSON_DATA_DIR` has the original `SCRANTIC.EXE`/`.SCR`,
    /// confirm we extract exactly the 23 original effects (ids 0..=24, skipping 11 & 13).
    #[test]
    fn extracts_real_scrantic_exe_if_present() {
        let Some(dir) = std::env::var_os("WILSON_DATA_DIR") else {
            return;
        };
        let dir = std::path::Path::new(&dir);
        let Some(path) =
            crate::find_ci(dir, "SCRANTIC.EXE").or_else(|| crate::find_ci(dir, "SCRANTIC.SCR"))
        else {
            return;
        };
        let bytes = std::fs::read(path).unwrap();
        let sounds = sounds_from_scrantic_exe(&bytes);
        let n = sounds.iter().filter(|s| s.is_some()).count();
        println!("extracted {n} sound effects from the original executable");
        assert_eq!(n, 23, "expected 23 original sound effects in SCRANTIC.EXE");
        assert!(
            sounds[11].is_none() && sounds[13].is_none(),
            "11 & 13 are absent"
        );
        assert!(sounds[0].is_some() && sounds[24].is_some());
    }
}
