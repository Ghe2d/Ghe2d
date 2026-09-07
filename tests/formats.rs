use ghe2d::utility::Rgba;
use ghe2d::{ExportOptions, Ghe2d, Ghe2dError, ImageFormat, ImageOptions};

/// 64x64 canvas: opaque blue on the left half, fully transparent on the right half.
fn canvas() -> Ghe2d {
    let mut g = Ghe2d::new(64, 64);
    g.draw_rect(0, 0, 32, 64, Rgba::new(0, 0, 255, 255));
    g
}

#[test]
fn encode_writes_the_right_magic_bytes() {
    let g = canvas();

    let png = g.encode(ExportOptions::png()).unwrap();
    assert_eq!(&png[0..4], b"\x89PNG");

    let jpeg = g.encode(ExportOptions::jpeg(80)).unwrap();
    assert_eq!(&jpeg[0..3], &[0xFF, 0xD8, 0xFF]);

    let webp = g.encode(ExportOptions::webp(80.0)).unwrap();
    assert_eq!(&webp[0..4], b"RIFF");
    assert_eq!(&webp[8..12], b"WEBP");
}

#[test]
fn jpeg_flattens_transparency_over_the_background() {
    let g = canvas();
    let on_white = g.encode(ExportOptions::jpeg(95).background(Rgba::new(255, 255, 255, 255))).unwrap();
    let on_black = g.encode(ExportOptions::jpeg(95).background(Rgba::new(0, 0, 0, 255))).unwrap();

    // Decode both and sample the (right half = originally transparent) region.
    let w = Ghe2d::from_bytes(on_white, ImageOptions::new()).unwrap();
    let b = Ghe2d::from_bytes(on_black, ImageOptions::new()).unwrap();
    let wp = w.image.get_pixel(50, 32).0;
    let bp = b.image.get_pixel(50, 32).0;
    assert!(wp[0] > 220 && wp[1] > 220 && wp[2] > 220, "transparent-over-white should be near white: {wp:?}");
    assert!(bp[0] < 40 && bp[1] < 40 && bp[2] < 40, "transparent-over-black should be near black: {bp:?}");
}

#[test]
fn detect_format_recognizes_encoded_buffers() {
    let g = canvas();
    assert!(matches!(Ghe2d::detect_format(&g.encode(ExportOptions::png()).unwrap()), Some(ImageFormat::Png)));
    assert!(matches!(Ghe2d::detect_format(&g.encode(ExportOptions::jpeg(80)).unwrap()), Some(ImageFormat::Jpeg)));
    assert!(matches!(Ghe2d::detect_format(&g.encode(ExportOptions::webp(80.0)).unwrap()), Some(ImageFormat::WebP)));
    assert_eq!(Ghe2d::detect_format(b"not an image"), None);
    assert_eq!(Ghe2d::detect_format(&[]), None);
}

#[test]
fn source_format_round_trips() {
    let jpeg = canvas().encode(ExportOptions::jpeg(85)).unwrap();
    let loaded = Ghe2d::from_bytes(jpeg, ImageOptions::new()).unwrap();
    assert!(matches!(loaded.source_format(), Some(ImageFormat::Jpeg)));
    assert_eq!((loaded.width(), loaded.height()), (64, 64));

    assert_eq!(Ghe2d::new(10, 10).source_format(), None);
}

#[test]
fn webp_dimension_guard_returns_err_not_abort() {
    let g = Ghe2d::new(20_000, 4);
    assert!(matches!(g.encode(ExportOptions::webp(70.0)), Err(Ghe2dError::ImageTooLarge)));
}

#[test]
fn zero_sized_canvas_errors_per_format() {
    let g = Ghe2d::new(0, 0);
    assert!(matches!(g.encode(ExportOptions::png()), Err(Ghe2dError::PngEncode)));
    assert!(matches!(g.encode(ExportOptions::jpeg(80)), Err(Ghe2dError::JpegEncode)));
    assert!(matches!(g.encode(ExportOptions::webp(70.0)), Err(Ghe2dError::WebpEncode)));
}

#[test]
fn detection_ignores_a_lying_file_extension() {
    // A real WebP written to a path that ends in `.png`. Loading is content-sniffed
    // (never extension-based), so it must still be read as WebP with no error.
    let webp = canvas().encode(ExportOptions::webp(80.0)).unwrap();
    let liar = std::env::temp_dir().join("ghe2d_liar_extension.png");
    std::fs::write(&liar, &webp).unwrap();

    let bytes = std::fs::read(&liar).unwrap();
    let _ = std::fs::remove_file(&liar);

    assert!(matches!(Ghe2d::detect_format(&bytes), Some(ImageFormat::WebP)));
    let loaded = Ghe2d::from_bytes(bytes, ImageOptions::new()).expect("decodes despite the .png name");
    assert!(matches!(loaded.source_format(), Some(ImageFormat::WebP)));
    assert_eq!((loaded.width(), loaded.height()), (64, 64));
}

#[test]
fn small_canvas_encodes_and_decodes_for_every_format() {
    let mut g = Ghe2d::new(24, 24);
    g.draw_rect(2, 2, 20, 20, Rgba::new(200, 120, 40, 255));
    for opts in [ExportOptions::png(), ExportOptions::jpeg(85), ExportOptions::webp(80.0)] {
        let bytes = g.encode(opts).unwrap_or_else(|e| panic!("{opts:?}: {e}"));
        let back = Ghe2d::from_bytes(bytes, ImageOptions::new()).unwrap();
        assert_eq!((back.width(), back.height()), (24, 24));
    }
}
