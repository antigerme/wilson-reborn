// SPDX-License-Identifier: GPL-3.0-or-later
//! Scene-sequencing script (`.ADS`) resources.
//!
//! Layout (per `repos/jc_reborn/resource.c`):
//! - `VER:` tag, `u32` size, 5-byte version string
//! - `ADS:` tag, 4 unknown bytes
//! - `RES:` tag, `u32` size, `u16` count, then `(u16 id, NUL-terminated name)`
//!   mapping a slot id to the `.TTM` file it drives
//! - `SCR:` tag, packed block of ADS bytecode
//! - `TAG:` tag, `u32` size, `u16` count, then `(u16 id, NUL-terminated string)`
//!
//! This exposes the decompressed bytecode, the resource (slot→TTM) table and the
//! tag table; decoding the opcode stream is a later phase.

use crate::chunk::read_packed_block;
use crate::error::Result;
use crate::reader::Reader;
use crate::resource::Tag;

/// A `(id, name)` entry from the `RES:` table: a slot id and the `.TTM` file it
/// refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdsRes {
    /// Slot identifier referenced by the ADS bytecode.
    pub id: u16,
    /// Name of the `.TTM` resource this slot loads.
    pub name: String,
}

/// A parsed `.ADS` resource (scene-sequencing script).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ads {
    /// Version string (e.g. `"1.20"`).
    pub version: String,
    /// The slot→TTM resource table.
    pub resources: Vec<AdsRes>,
    /// Decompressed ADS bytecode (the `SCR:` block).
    pub bytecode: Vec<u8>,
    /// Named entry points ("sequences") into the bytecode.
    pub tags: Vec<Tag>,
}

impl Ads {
    /// Parse an `.ADS` resource body.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);

        r.expect_tag("ADS resource", b"VER:")?;
        let _version_size = r.u32()?;
        let version = r.fixed_str(5)?;

        r.expect_tag("ADS resource", b"ADS:")?;
        r.skip(4)?;

        r.expect_tag("ADS resource", b"RES:")?;
        let _res_size = r.u32()?;
        let num_res = r.u16()? as usize;
        let mut resources = Vec::with_capacity(num_res);
        for _ in 0..num_res {
            let id = r.u16()?;
            let name = r.cstr(40)?;
            resources.push(AdsRes { id, name });
        }

        r.expect_tag("ADS resource", b"SCR:")?;
        let bytecode = read_packed_block(&mut r)?;

        r.expect_tag("ADS resource", b"TAG:")?;
        let _tag_size = r.u32()?;
        let num_tags = r.u16()? as usize;
        let mut tags = Vec::with_capacity(num_tags);
        for _ in 0..num_tags {
            let id = r.u16()?;
            let description = r.cstr(40)?;
            tags.push(Tag { id, description });
        }

        Ok(Ads {
            version,
            resources,
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
    fn parse_ads() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VER:");
        bytes.extend_from_slice(&5u32.to_le_bytes());
        bytes.extend_from_slice(b"1.20\0");
        bytes.extend_from_slice(b"ADS:");
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes.extend_from_slice(b"RES:");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // res size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // num res
        bytes.extend_from_slice(&1u16.to_le_bytes()); // res id
        bytes.extend_from_slice(b"FISHING.TTM\0");
        bytes.extend_from_slice(b"SCR:");
        bytes.extend_from_slice(&packed_none(&[0x05, 0x20, 0xFF, 0xFF]));
        bytes.extend_from_slice(b"TAG:");
        bytes.extend_from_slice(&0u32.to_le_bytes()); // tag size
        bytes.extend_from_slice(&1u16.to_le_bytes()); // num tags
        bytes.extend_from_slice(&1u16.to_le_bytes()); // tag id
        bytes.extend_from_slice(b"fish\0");

        let ads = Ads::parse(&bytes).unwrap();
        assert_eq!(ads.version, "1.20");
        assert_eq!(ads.resources.len(), 1);
        assert_eq!(ads.resources[0].id, 1);
        assert_eq!(ads.resources[0].name, "FISHING.TTM");
        assert_eq!(ads.bytecode, vec![0x05, 0x20, 0xFF, 0xFF]);
        assert_eq!(ads.tags[0].description, "fish");
    }
}
