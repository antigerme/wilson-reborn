// SPDX-License-Identifier: GPL-3.0-or-later
//! Animation script (`.TTM`) resources.
//!
//! Layout (per `repos/jc_reborn/resource.c`):
//! - `VER:` tag, `u32` size, 5-byte version string
//! - `PAG:` tag, `u32` page count, 2 unknown bytes
//! - `TT3:` tag, packed block of TTM bytecode
//! - `TTI:` tag, 4 unknown bytes
//! - `TAG:` tag, `u32` size, `u16` tag count, then `(u16 id, NUL-terminated string)`
//!
//! This exposes the decompressed bytecode and the tag table; decoding the opcode
//! stream into instructions is a later phase.

use crate::chunk::read_packed_block;
use crate::error::Result;
use crate::reader::Reader;
use crate::resource::Tag;

/// A parsed `.TTM` resource (animation script).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ttm {
    /// Version string (e.g. `"1.20"`).
    pub version: String,
    /// Page count recorded in the `PAG:` chunk.
    pub num_pages: u32,
    /// Decompressed TTM bytecode (the `TT3:` block).
    pub bytecode: Vec<u8>,
    /// Named entry points ("scenes") into the bytecode.
    pub tags: Vec<Tag>,
}

impl Ttm {
    /// Parse a `.TTM` resource body.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);

        r.expect_tag("TTM resource", b"VER:")?;
        let _version_size = r.u32()?;
        let version = r.fixed_str(5)?;

        r.expect_tag("TTM resource", b"PAG:")?;
        let num_pages = r.u32()?;
        let _pag_unknown1 = r.u8()?;
        let _pag_unknown2 = r.u8()?;

        r.expect_tag("TTM resource", b"TT3:")?;
        let bytecode = read_packed_block(&mut r)?;

        r.expect_tag("TTM resource", b"TTI:")?;
        r.skip(4)?;

        r.expect_tag("TTM resource", b"TAG:")?;
        let _tag_size = r.u32()?;
        let num_tags = r.u16()? as usize;
        let mut tags = Vec::with_capacity(num_tags);
        for _ in 0..num_tags {
            let id = r.u16()?;
            let description = r.cstr(40)?;
            tags.push(Tag { id, description });
        }

        Ok(Ttm {
            version,
            num_pages,
            bytecode,
            tags,
        })
    }
}

/// Operands of a [`TtmInstruction`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtmArgs {
    /// Zero or more 16-bit operands (raw values; some are interpreted as signed
    /// coordinates by the runtime).
    Words(Vec<u16>),
    /// A single string operand (opcodes whose low nibble is `0xF`).
    Str(String),
}

/// One decoded TTM instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtmInstruction {
    /// The full 16-bit opcode (its low nibble encodes the operand count).
    pub opcode: u16,
    /// The decoded operands.
    pub args: TtmArgs,
}

impl TtmInstruction {
    /// Human-readable mnemonic, or `"UNKNOWN"` if not recognised.
    pub fn name(&self) -> &'static str {
        ttm_opcode_name(self.opcode)
    }
}

impl Ttm {
    /// Decode this resource's bytecode into a list of instructions.
    pub fn instructions(&self) -> Result<Vec<TtmInstruction>> {
        decode_ttm(&self.bytecode)
    }
}

/// Decode a TTM bytecode stream into instructions.
///
/// Each instruction is a 16-bit opcode whose low nibble is the operand count; a
/// count of `0xF` denotes a single NUL-terminated string operand padded to an even
/// number of bytes (mirrors `repos/jc_reborn/dump.c`).
pub fn decode_ttm(bytecode: &[u8]) -> Result<Vec<TtmInstruction>> {
    let mut r = Reader::new(bytecode);
    let mut out = Vec::new();
    while !r.is_empty() {
        let opcode = r.u16()?;
        let num_args = (opcode & 0x000F) as usize;
        let args = if num_args == 0x0F {
            let mut bytes = Vec::new();
            let mut count = 0usize;
            loop {
                let b = r.u8()?;
                count += 1;
                if b == 0 {
                    break;
                }
                bytes.push(b);
            }
            if count % 2 == 1 {
                r.skip(1)?; // pad to an even number of bytes
            }
            TtmArgs::Str(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            let mut words = Vec::with_capacity(num_args);
            for _ in 0..num_args {
                words.push(r.u16()?);
            }
            TtmArgs::Words(words)
        };
        out.push(TtmInstruction { opcode, args });
    }
    Ok(out)
}

/// Mnemonic for a full 16-bit TTM opcode (per `repos/jc_reborn/dump.c`).
pub fn ttm_opcode_name(opcode: u16) -> &'static str {
    match opcode {
        0x001F => "SAVE_BACKGROUND",
        0x0080 => "DRAW_BACKGROUND",
        0x0110 => "PURGE",
        0x0FF0 => "UPDATE",
        0x1021 => "SET_DELAY",
        0x1051 => "SET_BMP_SLOT",
        0x1061 => "SET_PALETTE_SLOT",
        0x1101 => "LOCAL_TAG",
        0x1111 => "TAG",
        0x1121 => "TTM_UNKNOWN_1",
        0x1201 => "GOTO_TAG",
        0x2002 => "SET_COLORS",
        0x2012 => "SET_FRAME1",
        0x2022 => "TIMER",
        0x4004 => "SET_CLIP_ZONE",
        0x4110 => "FADE_OUT",
        0x4120 => "FADE_IN",
        0x4204 => "COPY_ZONE_TO_BG",
        0x4214 => "SAVE_IMAGE1",
        0xA002 => "DRAW_PIXEL",
        0xA054 => "SAVE_ZONE",
        0xA064 => "RESTORE_ZONE",
        0xA0A4 => "DRAW_LINE",
        0xA104 => "DRAW_RECT",
        0xA404 => "DRAW_CIRCLE",
        0xA504 => "DRAW_SPRITE",
        0xA510 => "DRAW_SPRITE1",
        0xA524 => "DRAW_SPRITE_FLIP",
        0xA530 => "DRAW_SPRITE3",
        0xA601 => "CLEAR_SCREEN",
        0xB606 => "DRAW_SCREEN",
        0xC020 => "LOAD_SAMPLE",
        0xC030 => "SELECT_SAMPLE",
        0xC040 => "DESELECT_SAMPLE",
        0xC051 => "PLAY_SAMPLE",
        0xC060 => "STOP_SAMPLE",
        0xF01F => "LOAD_SCREEN",
        0xF02F => "LOAD_IMAGE",
        0xF05F => "LOAD_PALETTE",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packed_none(body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&((body.len() + 5) as u32).to_le_bytes());
        out.push(0);
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn parse_ttm() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VER:");
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(b"1.20\0");
        bytes.extend_from_slice(b"PAG:");
        bytes.extend_from_slice(&3u32.to_le_bytes()); // num pages
        bytes.extend_from_slice(&[0, 0]); // pag unknown
        bytes.extend_from_slice(b"TT3:");
        bytes.extend_from_slice(&packed_none(&[0xF0, 0x0F, 0x20, 0x10]));
        bytes.extend_from_slice(b"TTI:");
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(b"TAG:");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // tag size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // num tags
        bytes.extend_from_slice(&7u16.to_le_bytes()); // tag id
        bytes.extend_from_slice(b"scene seven\0");

        let ttm = Ttm::parse(&bytes).unwrap();
        assert_eq!(ttm.version, "1.20");
        assert_eq!(ttm.num_pages, 3);
        assert_eq!(ttm.bytecode, vec![0xF0, 0x0F, 0x20, 0x10]);
        assert_eq!(ttm.tags.len(), 1);
        assert_eq!(ttm.tags[0].id, 7);
        assert_eq!(ttm.tags[0].description, "scene seven");
    }

    #[test]
    fn decode_words_and_string() {
        let mut code = Vec::new();
        // LOAD_IMAGE "AB" -> opcode 0xF02F, "AB\0" + 1 pad byte (odd -> even)
        code.extend_from_slice(&0xF02Fu16.to_le_bytes());
        code.extend_from_slice(b"AB\0\0");
        // SET_DELAY 5 -> 0x1021, arg 5
        code.extend_from_slice(&0x1021u16.to_le_bytes());
        code.extend_from_slice(&5u16.to_le_bytes());
        // DRAW_SPRITE 10 20 3 4 -> 0xA504 + 4 words
        code.extend_from_slice(&0xA504u16.to_le_bytes());
        for v in [10u16, 20, 3, 4] {
            code.extend_from_slice(&v.to_le_bytes());
        }
        // UPDATE -> 0x0FF0 (no args)
        code.extend_from_slice(&0x0FF0u16.to_le_bytes());

        let ins = decode_ttm(&code).unwrap();
        assert_eq!(ins.len(), 4);
        assert_eq!(ins[0].opcode, 0xF02F);
        assert_eq!(ins[0].name(), "LOAD_IMAGE");
        assert_eq!(ins[0].args, TtmArgs::Str("AB".to_string()));
        assert_eq!(ins[1].name(), "SET_DELAY");
        assert_eq!(ins[1].args, TtmArgs::Words(vec![5]));
        assert_eq!(ins[2].name(), "DRAW_SPRITE");
        assert_eq!(ins[2].args, TtmArgs::Words(vec![10, 20, 3, 4]));
        assert_eq!(ins[3].name(), "UPDATE");
        assert_eq!(ins[3].args, TtmArgs::Words(vec![]));
    }

    #[test]
    fn decode_even_string_has_no_padding() {
        // LOAD_SCREEN "OCEAN00.SCR" -> 11 chars + NUL = 12 bytes (even, no pad)
        let mut code = Vec::new();
        code.extend_from_slice(&0xF01Fu16.to_le_bytes());
        code.extend_from_slice(b"OCEAN00.SCR\0");
        code.extend_from_slice(&0x0FF0u16.to_le_bytes()); // UPDATE follows immediately

        let ins = decode_ttm(&code).unwrap();
        assert_eq!(ins.len(), 2);
        assert_eq!(ins[0].args, TtmArgs::Str("OCEAN00.SCR".to_string()));
        assert_eq!(ins[1].name(), "UPDATE");
    }

    #[test]
    fn unknown_opcode_still_consumes_args() {
        // Unknown opcode 0x9992 (low nibble 2 -> 2 args), then UPDATE.
        let mut code = Vec::new();
        code.extend_from_slice(&0x9992u16.to_le_bytes());
        code.extend_from_slice(&1u16.to_le_bytes());
        code.extend_from_slice(&2u16.to_le_bytes());
        code.extend_from_slice(&0x0FF0u16.to_le_bytes());

        let ins = decode_ttm(&code).unwrap();
        assert_eq!(ins.len(), 2);
        assert_eq!(ins[0].name(), "UNKNOWN");
        assert_eq!(ins[0].args, TtmArgs::Words(vec![1, 2]));
        assert_eq!(ins[1].name(), "UPDATE");
    }
}
