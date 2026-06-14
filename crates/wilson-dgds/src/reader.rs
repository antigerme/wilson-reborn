// SPDX-License-Identifier: GPL-3.0-or-later
//! A small bounds-checked, little-endian cursor over a byte slice.

use crate::error::{DgdsError, Result};

/// A forward/seekable cursor that reads little-endian integers and fixed fields
/// from a byte slice, returning [`DgdsError::UnexpectedEof`] instead of panicking.
#[derive(Debug, Clone)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Create a reader positioned at the start of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current read position (byte offset).
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Number of bytes left to read.
    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Whether the cursor is at end-of-data.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Move the cursor to an absolute position.
    pub fn seek(&mut self, pos: usize) -> Result<()> {
        if pos > self.data.len() {
            return Err(DgdsError::eof("seek", pos, self.data.len()));
        }
        self.pos = pos;
        Ok(())
    }

    /// Advance the cursor by `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }

    /// Borrow the next `n` bytes and advance.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end =
            self.pos
                .checked_add(n)
                .ok_or(DgdsError::eof("take", usize::MAX, self.data.len()))?;
        if end > self.data.len() {
            return Err(DgdsError::eof("take", end, self.data.len()));
        }
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Read one byte.
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// Read a little-endian `u16`.
    pub fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Read a little-endian `u32`.
    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read a 4-byte tag (e.g. `b"SCR:"`).
    pub fn tag(&mut self) -> Result<[u8; 4]> {
        let b = self.take(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }

    /// Read a 4-byte tag and verify it matches `expected`.
    pub fn expect_tag(&mut self, context: &'static str, expected: &[u8; 4]) -> Result<()> {
        let found = self.tag()?;
        if &found == expected {
            Ok(())
        } else {
            Err(DgdsError::BadTag {
                context,
                expected: *expected,
                found,
            })
        }
    }

    /// Read a fixed-width field of `n` bytes and decode it as an ASCII/UTF-8 string,
    /// stopping at the first NUL byte (DGDS uses NUL-terminated fixed fields).
    pub fn fixed_str(&mut self, n: usize) -> Result<String> {
        let bytes = self.take(n)?;
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
}
