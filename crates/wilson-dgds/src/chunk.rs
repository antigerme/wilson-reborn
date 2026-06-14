// SPDX-License-Identifier: GPL-3.0-or-later
//! DGDS chunk container primitives.
//!
//! A DGDS resource is a sequence of chunks. Each chunk header is a 4-byte tag whose
//! 4th byte is `':'`, followed by a little-endian `u32` size. The top bit of the size
//! (`0x8000_0000`) flags a *container* chunk that has no payload of its own (its
//! children follow inline).

use crate::decompress::decompress;
use crate::error::{DgdsError, Result};
use crate::reader::Reader;

/// A parsed chunk header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// The raw 4-byte tag, e.g. `b"SCR:"`.
    pub tag: [u8; 4],
    /// Payload size in bytes (with the container bit cleared).
    pub size: u32,
    /// Whether this is a container chunk (no payload; children follow).
    pub container: bool,
}

impl ChunkHeader {
    /// Read and validate a chunk header at the reader's current position.
    pub fn read(r: &mut Reader) -> Result<Self> {
        let tag = r.tag()?;
        if tag[3] != b':' {
            return Err(DgdsError::Malformed(format!(
                "invalid chunk tag {:?} (4th byte must be ':')",
                String::from_utf8_lossy(&tag)
            )));
        }
        let raw = r.u32()?;
        let container = raw & 0x8000_0000 != 0;
        let size = raw & 0x7FFF_FFFF;
        Ok(ChunkHeader {
            tag,
            size,
            container,
        })
    }

    /// The 3-letter tag (without the trailing colon) as a string.
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.tag[..3]).into_owned()
    }
}

/// Read a *packed block* and return its decompressed contents.
///
/// Layout (as used by `SCR:`/`BIN:`/`TT3:` payloads): a `u32` total size, then a
/// 1-byte compression method, a `u32` uncompressed size, and `total - 5` bytes of
/// compressed body.
pub fn read_packed_block(r: &mut Reader) -> Result<Vec<u8>> {
    let total = r.u32()? as usize;
    if total < 5 {
        return Err(DgdsError::Malformed(format!(
            "packed block total size {total} < 5"
        )));
    }
    let method = r.u8()?;
    let unpacked_size = r.u32()? as usize;
    let body = r.take(total - 5)?;
    decompress(method, body, unpacked_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_plain() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"SCR:");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        let mut r = Reader::new(&bytes);
        let h = ChunkHeader::read(&mut r).unwrap();
        assert_eq!(h.tag, *b"SCR:");
        assert_eq!(h.size, 16);
        assert!(!h.container);
        assert_eq!(h.name(), "SCR");
    }

    #[test]
    fn header_container_bit() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PAL:");
        bytes.extend_from_slice(&(0x8000_0000u32 | 32).to_le_bytes());
        let mut r = Reader::new(&bytes);
        let h = ChunkHeader::read(&mut r).unwrap();
        assert!(h.container);
        assert_eq!(h.size, 32);
    }

    #[test]
    fn header_rejects_bad_tag() {
        let bytes = *b"SCRX\x00\x00\x00\x00";
        let mut r = Reader::new(&bytes);
        assert!(matches!(
            ChunkHeader::read(&mut r),
            Err(DgdsError::Malformed(_))
        ));
    }

    #[test]
    fn packed_block_rle() {
        // body = RLE for 3x 0xAB; total = body_len(2) + 5 = 7.
        let body = [0x83u8, 0xAB];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&((body.len() + 5) as u32).to_le_bytes());
        bytes.push(1); // method = RLE
        bytes.extend_from_slice(&3u32.to_le_bytes()); // uncompressed size
        bytes.extend_from_slice(&body);
        let mut r = Reader::new(&bytes);
        assert_eq!(read_packed_block(&mut r).unwrap(), vec![0xAB, 0xAB, 0xAB]);
    }
}
