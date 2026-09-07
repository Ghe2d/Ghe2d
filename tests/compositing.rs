use ghe2d::utility::Rgba;
use ghe2d::{ComposeMode, ErrorMessage, Ghe2d, Ghe2dError, ImageOptions};

fn src_png(color: Rgba) -> Vec<u8> {
    let mut g = Ghe2d::new(20, 20);
    g.draw_rect(0, 0, 20, 20, color);
    g.encode(ghe2d::ExportOptions::png()).unwrap()
}

fn blue_base() -> Ghe2d {
    let mut g = Ghe2d::new(40, 40);
    g.draw_rect(0, 0, 40, 40, Rgba::new(0, 0, 255, 255));
    g
}

#[test]
fn normal_replaces_destination_pixels() {
    let mut g = blue_base();
    g.draw_image_bytes(src_png(Rgba::new(255, 0, 0, 128)), ImageOptions::new().at(5, 5)).unwrap();
    assert_eq!(g.image.get_pixel(10, 10).0, [255, 0, 0, 128], "Normal should copy the source pixel verbatim");
    assert_eq!(g.image.get_pixel(35, 35).0, [0, 0, 255, 255], "outside the placement is untouched");
}

#[test]
fn overlay_and_blend_differ_from_normal_and_each_other() {
    let semi = src_png(Rgba::new(255, 0, 0, 128));

    let mut n = blue_base();
    n.draw_image_bytes(semi.clone(), ImageOptions::new().at(5, 5).compose(ComposeMode::Normal)).unwrap();
    let mut o = blue_base();
    o.draw_image_bytes(semi.clone(), ImageOptions::new().at(5, 5).compose(ComposeMode::Overlay)).unwrap();
    let mut b = blue_base();
    b.draw_image_bytes(semi, ImageOptions::new().at(5, 5).compose(ComposeMode::Blend)).unwrap();

    let (np, op, bp) = (n.image.get_pixel(10, 10).0, o.image.get_pixel(10, 10).0, b.image.get_pixel(10, 10).0);
    assert_ne!(op, np, "Overlay must blend, not replace");
    assert_ne!(bp, np, "Blend must blend, not replace");
    assert_ne!(op, bp, "Overlay and Blend use different math");
    // both keep some blue from the base and add some red from the source
    assert!(op[2] > 0 && op[0] > 0);
}

#[test]
fn circle_option_masks_corners() {
    let g = Ghe2d::from_bytes(src_png(Rgba::new(0, 200, 0, 255)), ImageOptions::new().size(20, 20).circle(true)).unwrap();
    assert_eq!(g.image.get_pixel(0, 0).0[3], 0, "corner should be masked out");
    assert_eq!(g.image.get_pixel(10, 10).0[3], 255, "centre should be opaque");
}

#[test]
fn off_canvas_placement_does_not_panic() {
    let mut g = blue_base();
    let s = src_png(Rgba::new(255, 255, 255, 255));
    g.draw_image_bytes(s.clone(), ImageOptions::new().at(1000, 1000)).unwrap();
    g.draw_image_bytes(s.clone(), ImageOptions::new().at(35, 35).compose(ComposeMode::Blend)).unwrap();
    g.draw_image_bytes(s, ImageOptions::new().at(38, 0).compose(ComposeMode::Overlay)).unwrap();
}

#[test]
fn bad_buffers_error_cleanly() {
    assert!(matches!(
        Ghe2d::from_bytes(Vec::new(), ImageOptions::new()),
        Err(Ghe2dError::Image(ErrorMessage::BufferEmpty))
    ));
    assert!(matches!(
        Ghe2d::from_bytes(b"not an image at all".to_vec(), ImageOptions::new()),
        Err(Ghe2dError::Image(ErrorMessage::FailedDecodeImage))
    ));
    let mut g = blue_base();
    assert!(matches!(
        g.draw_image_bytes(vec![1, 2, 3], ImageOptions::new()),
        Err(Ghe2dError::Image(ErrorMessage::FailedDecodeImage))
    ));
}
