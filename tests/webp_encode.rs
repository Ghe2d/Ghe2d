use ghe2d::utility::Rgba;
use ghe2d::{Ghe2d, Ghe2dError};

#[test]
fn oversized_image_errors_instead_of_aborting() {
    // 20000 px wide exceeds WebP's 16383 limit. Before the fix this reached
    // `slice::from_raw_parts(NULL, 0)` and aborted the whole process.
    let g = Ghe2d::new(20_000, 4);
    let err = g.to_webp_buffer(70.0);
    assert!(matches!(err, Err(Ghe2dError::ImageTooLarge)), "got {err:?}");
}

#[test]
fn zero_sized_image_errors() {
    let g = Ghe2d::new(0, 0);
    assert!(matches!(g.to_webp_buffer(70.0), Err(Ghe2dError::WebpEncode)));
}

#[test]
fn small_image_encodes_ok() {
    let mut g = Ghe2d::new(32, 32);
    g.draw_rect(4, 4, 20, 20, Rgba::new(255, 0, 0, 255));
    let bytes = g.to_webp_buffer(70.0).expect("encode should succeed");
    assert!(!bytes.is_empty());
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WEBP");
}
