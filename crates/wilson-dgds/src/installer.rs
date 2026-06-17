// SPDX-License-Identifier: GPL-3.0-or-later
//! Decompress the original Sierra/Dynamix 1992 **installer** files (`RESOURCE.00$`,
//! `SCRANTIC.SC$`; magic `65 5d 13 8c`) into the plain `RESOURCE.001` / `SCRANTIC.SCR`.
//!
//! The outer file is a thin Dynamix wrapper (42-byte header) around a **PKWARE Data
//! Compression Library** stream ("DCL implode" / "explode"). The explode algorithm here is
//! a faithful, `#![forbid(unsafe_code)]`-clean port of Mark Adler's public-domain
//! `blast.c` (zlib `contrib/blast`, zlib licence — compatible with this crate's
//! GPL-3.0-or-later). Verified byte-for-byte against the originals (the decompressed output
//! md5-matches the `RESOURCE.001`/`SCRANTIC.SCR` from `scrantic-run.zip`).
//!
//! This is the *installer's* compression — unrelated to the per-chunk RLE/LZW used **inside**
//! `RESOURCE.001` (see [`crate::decompress`]).

const MAGIC: u32 = 0x8C13_5D65;
const MAXBITS: usize = 13;

/// A canonical-Huffman decode table, built from `blast.c`'s compact "rep" arrays: each rep
/// byte encodes `(count-1)` in its high nibble and the code length in its low nibble.
struct Huff {
    count: [i32; MAXBITS + 1],
    symbol: Vec<u16>,
}

impl Huff {
    fn new(rep: &[u8]) -> Huff {
        let mut lengths: Vec<u8> = Vec::new();
        for &b in rep {
            let run = (b >> 4) + 1;
            let len = b & 0x0f;
            lengths.extend(std::iter::repeat_n(len, run as usize));
        }
        let mut count = [0i32; MAXBITS + 1];
        for &l in &lengths {
            count[l as usize] += 1;
        }
        let mut offs = [0i32; MAXBITS + 2];
        for length in 1..MAXBITS {
            offs[length + 1] = offs[length] + count[length];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offs[l as usize] as usize] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Huff { count, symbol }
    }
}

// Compact code-length tables, verbatim from blast.c.
#[rustfmt::skip]
const LITLEN: &[u8] = &[
    11, 124, 8, 7, 28, 7, 188, 13, 76, 4, 10, 8, 12, 10, 12, 10, 8, 23, 8,
    9, 7, 6, 7, 8, 7, 6, 55, 8, 23, 24, 12, 11, 7, 9, 11, 12, 6, 7, 22, 5,
    7, 24, 6, 11, 9, 6, 7, 22, 7, 11, 38, 7, 9, 8, 25, 11, 8, 11, 9, 12,
    8, 12, 5, 38, 5, 38, 5, 11, 7, 5, 6, 21, 6, 10, 53, 8, 7, 24, 10, 27,
    44, 253, 253, 253, 252, 252, 252, 13, 12, 45, 12, 45, 12, 61, 12, 45,
    44, 173,
];
const LENLEN: &[u8] = &[2, 35, 36, 53, 38, 23];
const DISTLEN: &[u8] = &[2, 20, 53, 230, 247, 151, 248];
const LENBASE: [i32; 16] = [3, 2, 4, 5, 6, 7, 8, 9, 10, 12, 16, 24, 40, 72, 136, 264];
const LENEXTRA: [i32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];

/// Bit reader + output accumulator for the explode loop (ports blast.c's state).
struct Blast<'a> {
    data: &'a [u8],
    inpos: usize,
    bitbuf: u32,
    bitcnt: i32,
    out: Vec<u8>,
}

impl Blast<'_> {
    /// Read `need` bits LSB-first (blast.c `bits()`).
    fn bits(&mut self, need: i32) -> Option<u32> {
        let mut val = self.bitbuf;
        while self.bitcnt < need {
            val |= u32::from(*self.data.get(self.inpos)?) << self.bitcnt;
            self.inpos += 1;
            self.bitcnt += 8;
        }
        self.bitbuf = val >> need;
        self.bitcnt -= need;
        Some(val & ((1u32 << need) - 1))
    }

    /// Decode one symbol with the (bit-reversed) Huffman table `h` (blast.c `decode()`).
    fn decode(&mut self, h: &Huff) -> Option<u16> {
        let mut bitbuf = self.bitbuf;
        let mut left = self.bitcnt;
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        let mut length = 1usize;
        loop {
            while left != 0 {
                left -= 1;
                code |= ((bitbuf & 1) ^ 1) as i32; // PKWARE stores codes bit-reversed
                bitbuf >>= 1;
                let count = h.count[length];
                if code < first + count {
                    self.bitbuf = bitbuf;
                    self.bitcnt = (self.bitcnt - length as i32) & 7;
                    return h.symbol.get((index + (code - first)) as usize).copied();
                }
                index += count;
                first += count;
                first <<= 1;
                code <<= 1;
                length += 1;
            }
            left = (MAXBITS as i32 + 1) - length as i32;
            if left == 0 {
                return None;
            }
            bitbuf = u32::from(*self.data.get(self.inpos)?);
            self.inpos += 1;
            if left > 8 {
                left = 8;
            }
        }
    }
}

/// Decompress a raw PKWARE DCL ("implode") stream.
fn explode(data: &[u8]) -> Option<Vec<u8>> {
    let litcode = Huff::new(LITLEN);
    let lencode = Huff::new(LENLEN);
    let distcode = Huff::new(DISTLEN);
    let mut s = Blast {
        data,
        inpos: 0,
        bitbuf: 0,
        bitcnt: 0,
        out: Vec::new(),
    };
    let lit = s.bits(8)?;
    let dictbits = s.bits(8)? as i32;
    if lit > 1 || !(4..=6).contains(&dictbits) {
        return None;
    }
    loop {
        if s.bits(1)? != 0 {
            // length/distance back-reference
            let sym = s.decode(&lencode)? as usize;
            let length = LENBASE[sym] + s.bits(LENEXTRA[sym])? as i32;
            if length == 519 {
                break; // end-of-stream marker
            }
            let shift = if length == 2 { 2 } else { dictbits };
            let dist = (i32::from(s.decode(&distcode)?) << shift) + s.bits(shift)? as i32 + 1;
            if dist as usize > s.out.len() {
                return None;
            }
            let start = s.out.len() - dist as usize;
            for i in 0..length as usize {
                let b = s.out[start + i]; // overlapping copy is intentional
                s.out.push(b);
            }
        } else {
            // literal
            let byte = if lit != 0 {
                s.decode(&litcode)? as u8
            } else {
                s.bits(8)? as u8
            };
            s.out.push(byte);
        }
    }
    Some(s.out)
}

/// Decompress an original Dynamix installer file (the whole `*.00$`/`*.SC$` bytes) into the
/// plain resource it carries (e.g. `RESOURCE.00$` → `RESOURCE.001`). Returns `None` if the
/// bytes aren't a recognised Dynamix-compressed file or the stream is malformed.
pub fn decompress_installer(buf: &[u8]) -> Option<Vec<u8>> {
    if buf.len() < 30 || u32::from_le_bytes(buf[0..4].try_into().ok()?) != MAGIC {
        return None;
    }
    let payload_size =
        usize::from(buf[14]) | (usize::from(buf[15]) << 8) | (usize::from(buf[16]) << 16);
    let payload_off = 29 + usize::from(buf[28]) + 1; // header + filename + 1 pad byte
    let end = payload_off.checked_add(payload_size)?.min(buf.len());
    explode(buf.get(payload_off..end)?)
}

/// Whether `buf` starts with the Dynamix installer-compression magic.
pub fn is_installer_compressed(buf: &[u8]) -> bool {
    buf.len() >= 4 && buf[0..4] == MAGIC.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_installer_bytes() {
        assert!(decompress_installer(&[0u8; 64]).is_none());
        assert!(!is_installer_compressed(b"PK\x03\x04"));
        assert!(is_installer_compressed(&[0x65, 0x5d, 0x13, 0x8c, 0, 0]));
    }

    #[test]
    fn huff_table_builds() {
        // Sanity: the literal table expands to 256 symbols (full byte alphabet).
        let h = Huff::new(LITLEN);
        assert_eq!(h.symbol.len(), 256);
    }

    /// Gated byte-exact check: set `WILSON_INSTALLER_IN`=path to a `.00$`/`.SC$` and
    /// `WILSON_INSTALLER_OUT`=path to its expected decompressed file.
    #[test]
    fn decompresses_real_installer_if_present() {
        let (Some(inp), Some(exp)) = (
            std::env::var_os("WILSON_INSTALLER_IN"),
            std::env::var_os("WILSON_INSTALLER_OUT"),
        ) else {
            return;
        };
        let comp = std::fs::read(inp).unwrap();
        let want = std::fs::read(exp).unwrap();
        let got = decompress_installer(&comp).expect("decompress installer file");
        assert_eq!(got.len(), want.len(), "decompressed length");
        assert!(got == want, "decompressed bytes match the original exactly");
    }
}
