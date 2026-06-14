// SPDX-License-Identifier: GPL-3.0-or-later
//! Error type for the engine runtime.

use std::fmt;

/// Errors produced while running the SCRANTIC interpreters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// A resource referenced by a script was not found in the archive.
    ResourceNotFound(String),
    /// An error from the underlying DGDS layer (e.g. bytecode decoding).
    Dgds(wilson_dgds::DgdsError),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::ResourceNotFound(name) => write!(f, "resource not found: {name}"),
            EngineError::Dgds(e) => write!(f, "DGDS error: {e}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<wilson_dgds::DgdsError> for EngineError {
    fn from(e: wilson_dgds::DgdsError) -> Self {
        EngineError::Dgds(e)
    }
}

/// Result alias for the engine crate.
pub type Result<T> = std::result::Result<T, EngineError>;
