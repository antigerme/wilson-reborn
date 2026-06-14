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
}
