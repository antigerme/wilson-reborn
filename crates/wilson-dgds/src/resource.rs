// SPDX-License-Identifier: GPL-3.0-or-later
//! The `RESOURCE.MAP` index and `RESOURCE.001` archive layout (Johnny Castaway).
//!
//! `RESOURCE.MAP` layout (per `repos/jc_reborn/resource.c`):
//! - 6 unknown bytes
//! - 13-byte NUL-terminated data file name (e.g. `RESOURCE.001`)
//! - `u16` entry count
//! - per entry: `u32` decompressed length, `u32` offset (into the data file)
//!
//! Each entry in the data file starts with a 13-byte NUL-terminated resource name
//! (whose extension gives the type, e.g. `.ADS`) and a `u32` size, followed by the
//! chunked resource body.
//!
//! Note: in the broader DGDS family (Rise of the Dragon / Heart of China) the first
//! `u32` per index entry is a filename *hash* rather than a length; this parser targets
//! the Johnny Castaway `RESOURCE.MAP` layout specifically.

use crate::error::Result;
use crate::reader::Reader;

/// One entry of the `RESOURCE.MAP` index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceMapEntry {
    /// Decompressed length recorded in the index.
    pub length: u32,
    /// Byte offset of the entry within the data archive.
    pub offset: u32,
}

/// The parsed `RESOURCE.MAP` index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceMap {
    /// Name of the archive that holds the resources (e.g. `RESOURCE.001`).
    pub data_file_name: String,
    /// All index entries, in file order.
    pub entries: Vec<ResourceMapEntry>,
}

impl ResourceMap {
    /// Parse a `RESOURCE.MAP` byte buffer.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        r.skip(6)?;
        let data_file_name = r.fixed_str(13)?;
        let count = r.u16()? as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let length = r.u32()?;
            let offset = r.u32()?;
            entries.push(ResourceMapEntry { length, offset });
        }
        Ok(ResourceMap {
            data_file_name,
            entries,
        })
    }
}

/// The header of one resource inside the data archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEntry {
    /// Resource name, e.g. `ACTIVITY.ADS`.
    pub name: String,
    /// Size field that follows the name.
    pub size: u32,
    /// Offset at which the resource body (chunks) begins.
    pub body_offset: usize,
}

/// Read the resource header located at `offset` within the data archive.
pub fn read_entry_header(archive: &[u8], offset: u32) -> Result<ResourceEntry> {
    let mut r = Reader::new(archive);
    r.seek(offset as usize)?;
    let name = r.fixed_str(13)?;
    let size = r.u32()?;
    Ok(ResourceEntry {
        name,
        size,
        body_offset: r.position(),
    })
}

/// Return the resource type extension (including the dot), e.g. `".ADS"`.
pub fn resource_extension(name: &str) -> Option<&str> {
    name.rfind('.').map(|i| &name[i..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_map() -> Vec<u8> {
        let mut m = Vec::new();
        m.extend_from_slice(&[0, 0, 0, 2, 0, 0]); // 6 unknown bytes
        let mut name = b"RESOURCE.001".to_vec();
        name.push(0); // 13th byte (NUL terminator)
        m.extend_from_slice(&name);
        m.extend_from_slice(&2u16.to_le_bytes()); // 2 entries
        m.extend_from_slice(&100u32.to_le_bytes()); // entry0 length
        m.extend_from_slice(&0u32.to_le_bytes()); // entry0 offset
        m.extend_from_slice(&50u32.to_le_bytes()); // entry1 length
        m.extend_from_slice(&200u32.to_le_bytes()); // entry1 offset
        m
    }

    #[test]
    fn parse_map() {
        let map = ResourceMap::parse(&build_map()).unwrap();
        assert_eq!(map.data_file_name, "RESOURCE.001");
        assert_eq!(map.entries.len(), 2);
        assert_eq!(
            map.entries[0],
            ResourceMapEntry {
                length: 100,
                offset: 0
            }
        );
        assert_eq!(map.entries[1].offset, 200);
    }

    #[test]
    fn parse_entry_header() {
        let mut archive = vec![0u8; 8]; // padding before the entry
        let mut name = b"ACTIVITY.ADS".to_vec();
        name.push(0);
        archive.extend_from_slice(&name);
        archive.extend_from_slice(&1234u32.to_le_bytes());
        archive.extend_from_slice(b"BODYBYTES");

        let entry = read_entry_header(&archive, 8).unwrap();
        assert_eq!(entry.name, "ACTIVITY.ADS");
        assert_eq!(entry.size, 1234);
        assert_eq!(entry.body_offset, 8 + 13 + 4);
        assert_eq!(resource_extension(&entry.name), Some(".ADS"));
    }

    #[test]
    fn truncated_map_errors() {
        assert!(ResourceMap::parse(&[0, 0, 0]).is_err());
    }
}
