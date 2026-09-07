use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::{Ghe2d, Ghe2dError};

#[test]
fn zero_sized_image_errors_instead_of_panicking() {
    let g = Ghe2d::new(0, 0);
    let r = g.to_png_buffer(CompressionType::Fast, FilterType::NoFilter);
    assert!(matches!(r, Err(Ghe2dError::PngEncode)), "got {r:?}");
}

#[test]
fn small_image_encodes_ok() {
    let mut g = Ghe2d::new(16, 16);
    g.draw_rect(0, 0, 16, 16, Rgba::new(0, 128, 255, 255));
    let bytes = g
        .to_png_buffer(CompressionType::Fast, FilterType::NoFilter)
        .expect("encode should succeed");
    assert_eq!(&bytes[1..4], b"PNG");
}
