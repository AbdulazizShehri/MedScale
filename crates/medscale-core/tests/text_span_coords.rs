use medscale_core::text::{TextCoordError, byte_to_scalar_offset, scalar_to_byte_offset};

#[test]
fn ascii_roundtrip() {
    let bytes = b"abc";
    assert_eq!(byte_to_scalar_offset(bytes, 0).unwrap(), 0);
    assert_eq!(byte_to_scalar_offset(bytes, 3).unwrap(), 3);
    assert_eq!(scalar_to_byte_offset(bytes, 2).unwrap(), 2);
}

#[test]
fn multibyte_scalar_boundary() {
    let bytes = "é".as_bytes(); // 2 bytes, 1 scalar
    assert_eq!(byte_to_scalar_offset(bytes, 0).unwrap(), 0);
    assert_eq!(byte_to_scalar_offset(bytes, 2).unwrap(), 1);
    assert_eq!(
        byte_to_scalar_offset(bytes, 1),
        Err(TextCoordError::NotScalarBoundary)
    );
}

#[test]
fn out_of_bounds_byte() {
    assert_eq!(
        byte_to_scalar_offset(b"a", 2),
        Err(TextCoordError::OutOfBounds)
    );
}

#[test]
fn out_of_bounds_scalar() {
    assert_eq!(
        scalar_to_byte_offset(b"a", 2),
        Err(TextCoordError::OutOfBounds)
    );
}

#[test]
fn invalid_utf8_fails_closed() {
    assert_eq!(
        byte_to_scalar_offset(&[0xff], 0),
        Err(TextCoordError::InvalidUtf8)
    );
}
