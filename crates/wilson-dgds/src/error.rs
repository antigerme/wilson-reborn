// SPDX-License-Identifier: GPL-3.0-or-later
//! Error type for DGDS parsing/decompression.

use std::fmt;

/// Errors produced while reading or decoding DGDS resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DgdsError {
    /// Not enough bytes left to satisfy a read.
    UnexpectedEof {
        /// Human-readable description of what was being read.
        context: &'static str,
        /// Number of bytes the read required (absolute end position).
        needed: usize,
        /// Number of bytes actually available.
        available: usize,
    },
    /// A 4-byte chunk tag did not match what the format requires.
    BadTag {
        /// What was being parsed when the tag was checked.
        context: &'static str,
        /// The expected tag.
        expected: [u8; 4],
        /// The tag actually found.
        found: [u8; 4],
    },
    /// Unsupported/unknown compression method byte.
    UnknownCompression(u8),
    /// The data was structurally invalid in some other way.
    Malformed(String),
}

impl DgdsError {
    /// Convenience constructor for [`DgdsError::UnexpectedEof`].
    pub(crate) fn eof(context: &'static str, needed: usize, available: usize) -> Self {
        DgdsError::UnexpectedEof {
            context,
            needed,
            available,
        }
    }
}

fn tag_to_string(tag: &[u8; 4]) -> String {
    String::from_utf8_lossy(tag).into_owned()
}

impl fmt::Display for DgdsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DgdsError::UnexpectedEof {
                context,
                needed,
                available,
            } => write!(
                f,
                "unexpected end of data while reading {context}: need {needed} byte(s), have {available}"
            ),
            DgdsError::BadTag {
                context,
                expected,
                found,
            } => write!(
                f,
                "bad chunk tag while parsing {context}: expected {:?}, found {:?}",
                tag_to_string(expected),
                tag_to_string(found)
            ),
            DgdsError::UnknownCompression(m) => {
                write!(f, "unknown compression method {m}")
            }
            DgdsError::Malformed(msg) => write!(f, "malformed DGDS data: {msg}"),
        }
    }
}

impl std::error::Error for DgdsError {}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, DgdsError>;
