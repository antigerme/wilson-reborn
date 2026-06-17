// SPDX-License-Identifier: GPL-3.0-or-later
//! Decompression for DGDS packed chunks.
//!
//! Methods: `0` = stored, `1` = RLE, `2` = LZW. The RLE and LZW routines are faithful
//! ports of the original SCRANTIC algorithms as implemented in
//! `repos/jc_reborn/uncompress.c` (variable-width LZW, 9..=12 bits, LSB-first, with
//! `0x100` as the clear code).

use crate::error::{DgdsError, Result};

/// Stored (no compression).
pub const METHOD_NONE: u8 = 0;
/// Run-length encoding.
pub const METHOD_RLE: u8 = 1;
/// Lempel–Ziv–Welch.
pub const METHOD_LZW: u8 = 2;

/// Decompress `input` into exactly `out_size` bytes using `method`.
pub fn decompress(method: u8, input: &[u8], out_size: usize) -> Result<Vec<u8>> {
    match method {
        METHOD_NONE => {
            if input.len() < out_size {
                return Err(DgdsError::eof("decompress(none)", out_size, input.len()));
            }
            Ok(input[..out_size].to_vec())
        }
        METHOD_RLE => rle(input, out_size),
        METHOD_LZW => lzw(input, out_size),
        other => Err(DgdsError::UnknownCompression(other)),
    }
}

/// RLE decompression (method 1).
///
/// Control byte: if the high bit is set, the low 7 bits are a repeat count for the
/// following value byte; otherwise the control byte is a literal count.
pub fn rle(input: &[u8], out_size: usize) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(out_size);
    let mut i = 0usize;

    while out.len() < out_size {
        let control = *input
            .get(i)
            .ok_or_else(|| DgdsError::eof("rle: control", i + 1, input.len()))?;
        i += 1;

        if control & 0x80 != 0 {
            let length = (control & 0x7F) as usize;
            let value = *input
                .get(i)
                .ok_or_else(|| DgdsError::eof("rle: value", i + 1, input.len()))?;
            i += 1;
            let take = length.min(out_size - out.len());
            out.resize(out.len() + take, value);
        } else {
            let length = control as usize;
            let slice = input
                .get(i..i + length)
                .ok_or_else(|| DgdsError::eof("rle: literal", i + length, input.len()))?;
            i += length;
            let take = length.min(out_size - out.len());
            out.extend_from_slice(&slice[..take]);
        }
    }

    Ok(out)
}

/// LZW decompression (method 2). Faithful port of `uncompressLZW`.
pub fn lzw(input: &[u8], out_size: usize) -> Result<Vec<u8>> {
    if out_size == 0 {
        return Ok(Vec::new());
    }

    let mut prefix = [0u16; 4096];
    let mut append = [0u8; 4096];
    let mut stack = [0u8; 4096];
    let mut sp = 0usize;

    let mut n_bits: u32 = 9;
    let mut free_entry: u32 = 257;
    let mut bitpos: u32 = 0;

    let mut br = BitReader::new(input);

    let mut oldcode = br.get_bits(n_bits);
    let mut lastbyte = oldcode;

    let mut out = Vec::with_capacity(out_size);
    out.push(oldcode as u8);

    while !br.exhausted() {
        let newcode = br.get_bits(n_bits);
        bitpos += n_bits;

        if newcode == 256 {
            // Clear: realign the bit stream to the next (n_bits<<3) boundary, reset.
            let nbits3 = n_bits << 3;
            let nskip = (nbits3 - ((bitpos - 1) % nbits3)) - 1;
            br.skip_bits(nskip);
            n_bits = 9;
            free_entry = 256;
            bitpos = 0;
            continue;
        }

        let mut code = newcode;

        if u32::from(code) >= free_entry {
            // KwKwK case: the code is not yet in the table.
            push(&mut stack, &mut sp, lastbyte as u8)?;
            code = oldcode;
        }

        while code > 255 {
            if code > 4095 {
                break;
            }
            push(&mut stack, &mut sp, append[code as usize])?;
            code = prefix[code as usize];
        }

        push(&mut stack, &mut sp, code as u8)?;
        lastbyte = code;

        while sp > 0 {
            sp -= 1;
            if out.len() >= out_size {
                return Ok(out);
            }
            out.push(stack[sp]);
        }

        if free_entry < 4096 {
            prefix[free_entry as usize] = oldcode;
            append[free_entry as usize] = lastbyte as u8;
            free_entry += 1;
            if free_entry >= (1u32 << n_bits) && n_bits < 12 {
                n_bits += 1;
                bitpos = 0;
            }
        }

        oldcode = newcode;
    }

    Ok(out)
}

#[inline]
fn push(stack: &mut [u8; 4096], sp: &mut usize, value: u8) -> Result<()> {
    if *sp >= stack.len() {
        return Err(DgdsError::Malformed("LZW decode stack overflow".into()));
    }
    stack[*sp] = value;
    *sp += 1;
    Ok(())
}

/// LSB-first bit reader, mirroring `getByte`/`getBits` from the reference decoder.
struct BitReader<'a> {
    data: &'a [u8],
    in_offset: usize,
    max: usize,
    current: u8,
    nextbit: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut br = BitReader {
            data,
            in_offset: 0,
            max: data.len(),
            current: 0,
            nextbit: 0,
        };
        br.current = br.get_byte();
        br
    }

    fn get_byte(&mut self) -> u8 {
        if self.in_offset >= self.max {
            0
        } else {
            let b = self.data[self.in_offset];
            self.in_offset += 1;
            b
        }
    }

    fn get_bits(&mut self, n: u32) -> u16 {
        let mut x: u32 = 0;
        for i in 0..n {
            // Only the low 16 bits matter for real codes; skips ignore the value.
            if i < 16 && (self.current & (1 << self.nextbit)) != 0 {
                x |= 1 << i;
            }
            self.nextbit += 1;
            if self.nextbit > 7 {
                self.current = self.get_byte();
                self.nextbit = 0;
            }
        }
        x as u16
    }

    fn skip_bits(&mut self, n: u32) {
        let _ = self.get_bits(n);
    }

    fn exhausted(&self) -> bool {
        self.in_offset >= self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_passthrough() {
        let data = [1u8, 2, 3, 4, 5];
        assert_eq!(decompress(METHOD_NONE, &data, 3).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn none_too_short() {
        let data = [1u8, 2];
        assert!(decompress(METHOD_NONE, &data, 4).is_err());
    }

    #[test]
    fn unknown_method() {
        assert_eq!(
            decompress(7, &[0u8], 1),
            Err(DgdsError::UnknownCompression(7))
        );
    }

    #[test]
    fn rle_repeat() {
        // 0x83 -> high bit set, count 3, value 0xAA.
        let out = rle(&[0x83, 0xAA], 3).unwrap();
        assert_eq!(out, vec![0xAA, 0xAA, 0xAA]);
    }

    #[test]
    fn rle_literal() {
        // 0x03 -> literal of 3 bytes.
        let out = rle(&[0x03, 1, 2, 3], 3).unwrap();
        assert_eq!(out, vec![1, 2, 3]);
    }

    #[test]
    fn rle_mixed_and_bounded() {
        // literal 2 (10,11), repeat 4 of 0x07, literal 1 (99) — bounded to 5 bytes.
        let input = [0x02, 10, 11, 0x84, 0x07, 0x01, 99];
        let out = rle(&input, 5).unwrap();
        assert_eq!(out, vec![10, 11, 0x07, 0x07, 0x07]);
    }

    #[test]
    fn rle_truncated_input_errors() {
        assert!(rle(&[0x05, 1, 2], 5).is_err());
    }

    // --- LZW round-trip via a matching encoder ------------------------------

    struct BitWriter {
        bytes: Vec<u8>,
        cur: u8,
        nbit: u32,
    }
    impl BitWriter {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                cur: 0,
                nbit: 0,
            }
        }
        fn put(&mut self, val: u32, n: u32) {
            for i in 0..n {
                if (val >> i) & 1 != 0 {
                    self.cur |= 1 << self.nbit;
                }
                self.nbit += 1;
                if self.nbit > 7 {
                    self.bytes.push(self.cur);
                    self.cur = 0;
                    self.nbit = 0;
                }
            }
        }
        fn finish(mut self) -> Vec<u8> {
            if self.nbit > 0 {
                self.bytes.push(self.cur);
            }
            self.bytes
        }
    }

    /// Code width for the `i`-th emitted code, derived from the decoder's state:
    /// after reading code `i` the decoder's `free_entry` equals `257 + i`, and it
    /// widens when that reaches the next power of two. This avoids the classic LZW
    /// "early change" desync (the encoder must widen exactly when the decoder does).
    fn lzw_width(emit_index: usize) -> u32 {
        match emit_index {
            0..=255 => 9,
            256..=767 => 10,
            768..=1791 => 11,
            _ => 12,
        }
    }

    /// LZW encoder whose code widths and code assignments mirror the decoder exactly.
    fn lzw_encode(input: &[u8]) -> Vec<u8> {
        use std::collections::HashMap;
        if input.is_empty() {
            return Vec::new();
        }
        let mut writer = BitWriter::new();
        let mut dict: HashMap<(u16, u8), u16> = HashMap::new();
        let mut free_entry: u32 = 257;
        let mut emit_index: usize = 0;

        let mut current = u16::from(input[0]);
        for &c in &input[1..] {
            if let Some(&code) = dict.get(&(current, c)) {
                current = code;
            } else {
                writer.put(u32::from(current), lzw_width(emit_index));
                emit_index += 1;
                if free_entry < 4096 {
                    dict.insert((current, c), free_entry as u16);
                    free_entry += 1;
                }
                current = u16::from(c);
            }
        }
        writer.put(u32::from(current), lzw_width(emit_index));
        writer.finish()
    }

    fn roundtrip(input: &[u8]) {
        let encoded = lzw_encode(input);
        let decoded = lzw(&encoded, input.len()).unwrap();
        assert_eq!(decoded, input, "LZW round-trip mismatch");
    }

    #[test]
    fn lzw_roundtrip_small() {
        roundtrip(b"ABABABA");
        roundtrip(b"TOBEORNOTTOBEORTOBEORNOT");
        roundtrip(b"aaaaaaaaaaaaaaaaaaaa");
    }

    #[test]
    fn lzw_roundtrip_crosses_bit_width() {
        // 256 distinct bytes followed by repeats forces the table past 512 entries,
        // exercising the 9->10 (and beyond) code-width transitions.
        let mut input = Vec::new();
        for _ in 0..4 {
            input.extend(0u8..=255);
        }
        roundtrip(&input);
    }

    #[test]
    fn lzw_roundtrip_binary_pattern() {
        let mut input = Vec::new();
        let mut x: u8 = 0;
        for _ in 0..2000 {
            input.push(x);
            x = x.wrapping_mul(31).wrapping_add(7);
        }
        roundtrip(&input);
    }
}
