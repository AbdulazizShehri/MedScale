//! Tagged text spans and coordinate systems.

use serde::{Deserialize, Serialize};

/// Coordinate system tag for source/derived text offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateSystem {
    RawByte,
    UnicodeScalar,
}

/// Representation tag for the text being spanned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRepresentation {
    SourceBytes,
    DerivedText,
}

/// Tagged span into a representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub representation: TextRepresentation,
    pub coordinate_system: CoordinateSystem,
    pub start: u64,
    pub end: u64,
}

impl TextSpan {
    /// Returns true when `start <= end`.
    #[must_use]
    pub const fn is_ordered(&self) -> bool {
        self.start <= self.end
    }
}
