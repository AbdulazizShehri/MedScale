//! RawByte ↔ UnicodeScalar conversions (UTF-8), fail-closed on bounds.

use thiserror::Error;

/// Coordinate conversion errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextCoordError {
    #[error("offset out of bounds")]
    OutOfBounds,
    #[error("offset is not on a UTF-8 scalar boundary")]
    NotScalarBoundary,
    #[error("invalid UTF-8 in source bytes")]
    InvalidUtf8,
}

/// Converts a raw byte offset into a Unicode scalar offset.
pub fn byte_to_scalar_offset(bytes: &[u8], byte_offset: usize) -> Result<usize, TextCoordError> {
    if byte_offset > bytes.len() {
        return Err(TextCoordError::OutOfBounds);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| TextCoordError::InvalidUtf8)?;
    if byte_offset == bytes.len() {
        return Ok(text.chars().count());
    }
    if !text.is_char_boundary(byte_offset) {
        return Err(TextCoordError::NotScalarBoundary);
    }
    Ok(text[..byte_offset].chars().count())
}

/// Converts a Unicode scalar offset into a raw byte offset.
pub fn scalar_to_byte_offset(bytes: &[u8], scalar_offset: usize) -> Result<usize, TextCoordError> {
    let text = std::str::from_utf8(bytes).map_err(|_| TextCoordError::InvalidUtf8)?;
    let mut counted = 0_usize;
    for (byte_idx, _) in text.char_indices() {
        if counted == scalar_offset {
            return Ok(byte_idx);
        }
        counted += 1;
    }
    if counted == scalar_offset {
        return Ok(text.len());
    }
    Err(TextCoordError::OutOfBounds)
}
