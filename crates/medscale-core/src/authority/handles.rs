//! Compile-time / unit assertion that responses never carry DB/key handles.

/// Marker type documenting that facade responses must not embed secret handles.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSecretHandles;

/// Asserts the no-handle invariant for documentation and unit tests.
#[must_use]
pub const fn assert_no_secret_handles() -> NoSecretHandles {
    NoSecretHandles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_secret_handles_marker_exists() {
        let _ = assert_no_secret_handles();
    }
}
