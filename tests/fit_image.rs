//! Cover / contain / fill, resampling, and decoding for `draw_image_fit*`.

use std::time::Instant;

use ghe2d::{ExportOptions, FitImageOptions, Ghe2d, Ghe2dError, ImageFit};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

/// `w x h`, the left half red and the right half blue.
fn halves(w: u32, h: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(w, h, |x, _| image::Rgba(if x < w / 2 { RED } else { BLUE }))
}

fn px(g: &Ghe2d, x: u32, y: u32) -> [u8; 4] {
    g.image.get_pixel(x, y).0
}

#[test]
fn cover_fills_the_box_and_crops_the_overflow_from_the_centre() {
    // 200 x 100 into 100 x 100: nothing is scaled, the middle 100 columns show.
    let mut g = Ghe2d::new(120, 120);
    g.draw_image_fit(
        &halves(200, 100),
        FitImageOptions::new(10.0, 10.0, 100.0, 100.0),
    );
    assert_eq!(px(&g, 10, 60), RED);
    assert_eq!(px(&g, 59, 60), RED);
    assert_eq!(px(&g, 60, 60), BLUE);
    assert_eq!(px(&g, 109, 60), BLUE);
    assert_eq!(px(&g, 9, 60)[3], 0, "nothing outside the box");
    assert_eq!(px(&g, 110, 60)[3], 0);
    assert_eq!(px(&g, 60, 10)[3], 255);
    assert_eq!(
        px(&g, 60, 109)[3],
        255,
        "the box is covered to its last row"
    );
}

#[test]
fn cover_position_chooses_which_part_shows() {
    let draw = |at: f32| {
        let mut g = Ghe2d::new(100, 100);
        g.draw_image_fit(
            &halves(200, 100),
            FitImageOptions::new(0.0, 0.0, 100.0, 100.0).position(at, 0.5),
        );
        g
    };
    assert_eq!(
        (px(&draw(0.0), 5, 50), px(&draw(0.0), 95, 50)),
        (RED, RED),
        "the start"
    );
    assert_eq!(
        (px(&draw(1.0), 5, 50), px(&draw(1.0), 95, 50)),
        (BLUE, BLUE),
        "the end"
    );
}

#[test]
fn contain_keeps_the_whole_picture_and_centres_it() {
    // 200 x 100 into 100 x 100: scaled to 100 x 50, rows 25 to 74.
    let mut g = Ghe2d::new(100, 100);
    g.draw_image_fit(
        &halves(200, 100),
        FitImageOptions::new(0.0, 0.0, 100.0, 100.0).fit(ImageFit::Contain),
    );
    assert_eq!(px(&g, 10, 50), RED);
    assert_eq!(px(&g, 90, 50), BLUE);
    assert_eq!(px(&g, 50, 10)[3], 0, "empty above");
    assert_eq!(px(&g, 50, 90)[3], 0, "empty below");
    assert_eq!(px(&g, 50, 25)[3], 255);
    assert_eq!(px(&g, 50, 74)[3], 255);
    assert_eq!(px(&g, 50, 24)[3], 0);
    assert_eq!(px(&g, 50, 75)[3], 0);
    // Position at the start puts it on top.
    let mut top = Ghe2d::new(100, 100);
    top.draw_image_fit(
        &halves(200, 100),
        FitImageOptions::new(0.0, 0.0, 100.0, 100.0)
            .fit(ImageFit::Contain)
            .position(0.5, 0.0),
    );
    assert_eq!(px(&top, 50, 0)[3], 255);
    assert_eq!(px(&top, 50, 60)[3], 0);
}

#[test]
fn contain_makes_a_small_picture_bigger() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_image_fit(
        &halves(10, 10),
        FitImageOptions::new(0.0, 0.0, 100.0, 100.0).fit(ImageFit::Contain),
    );
    assert_eq!(px(&g, 5, 50), RED);
    assert_eq!(px(&g, 95, 50), BLUE);
    assert_eq!(px(&g, 0, 0)[3], 255);
    assert_eq!(px(&g, 99, 99)[3], 255);
}

#[test]
fn fill_stretches_to_the_whole_box() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_image_fit(
        &halves(200, 100),
        FitImageOptions::new(0.0, 0.0, 100.0, 40.0).fit(ImageFit::Fill),
    );
    assert_eq!(px(&g, 5, 20), RED);
    assert_eq!(px(&g, 95, 20), BLUE);
    assert_eq!(px(&g, 50, 39)[3], 255);
    assert_eq!(px(&g, 50, 40)[3], 0);
}

#[test]
fn a_box_with_a_fraction_edge_is_snapped_to_whole_pixels() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_image_fit(
        &halves(10, 10),
        FitImageOptions::new(10.4, 10.4, 20.2, 20.2).fit(ImageFit::Fill),
    );
    assert_eq!(px(&g, 9, 15)[3], 0);
    assert_eq!(px(&g, 10, 15)[3], 255);
    assert_eq!(px(&g, 30, 15)[3], 255);
    assert_eq!(px(&g, 31, 15)[3], 0);
}

#[test]
fn transparent_edges_do_not_get_a_dark_fringe_when_resampled() {
    // A white disc on a transparent ground, shrunk a lot.
    let src = image::RgbaImage::from_fn(200, 200, |x, y| {
        let (dx, dy) = (x as f32 - 100.0, y as f32 - 100.0);
        if (dx * dx + dy * dy).sqrt() < 80.0 {
            image::Rgba([255, 255, 255, 255])
        } else {
            image::Rgba([0, 0, 0, 0])
        }
    });
    let mut g = Ghe2d::new(50, 50);
    g.draw_image_fit(
        &src,
        FitImageOptions::new(0.0, 0.0, 50.0, 50.0).fit(ImageFit::Fill),
    );
    let mut edge = 0;
    for p in g.image.pixels() {
        if p.0[3] > 0 && p.0[3] < 255 {
            edge += 1;
            assert!(
                p.0[0] >= 250 && p.0[1] >= 250 && p.0[2] >= 250,
                "grey fringe: {:?}",
                p.0
            );
        }
    }
    assert!(edge > 20, "the soft edge is there: {edge}");
}

#[test]
fn an_image_over_a_picture_keeps_what_is_under_the_transparent_part() {
    let mut g = Ghe2d::new(40, 40);
    g.draw_image_fit(
        &halves(4, 4),
        FitImageOptions::new(0.0, 0.0, 40.0, 40.0).fit(ImageFit::Fill),
    );
    let hole = image::RgbaImage::from_fn(4, 4, |x, _| {
        if x < 2 {
            image::Rgba([0, 0, 0, 0])
        } else {
            image::Rgba([0, 255, 0, 255])
        }
    });
    g.draw_image_fit(
        &hole,
        FitImageOptions::new(0.0, 0.0, 40.0, 40.0).fit(ImageFit::Fill),
    );
    assert_eq!(
        px(&g, 5, 20),
        RED,
        "the transparent half shows what is under it"
    );
    assert_eq!(px(&g, 35, 20), [0, 255, 0, 255]);
}

#[test]
fn decoding_from_bytes_works_for_every_format_and_fails_cleanly_on_bad_bytes() {
    let mut src = Ghe2d::new(8, 8);
    src.image = halves(8, 8);
    for opts in [
        ExportOptions::png(),
        ExportOptions::webp(100.0),
        ExportOptions::jpeg(95),
    ] {
        let bytes = src.encode(opts).unwrap();
        let mut g = Ghe2d::new(40, 40);
        g.draw_image_fit_bytes(
            &bytes,
            FitImageOptions::new(0.0, 0.0, 40.0, 40.0).fit(ImageFit::Fill),
        )
        .unwrap();
        let (l, r) = (px(&g, 5, 20), px(&g, 35, 20));
        assert!(l[0] > 200 && l[2] < 60, "{l:?}");
        assert!(r[2] > 200 && r[0] < 60, "{r:?}");
    }
    let mut g = Ghe2d::new(10, 10);
    assert!(matches!(
        g.draw_image_fit_bytes(&[], FitImageOptions::new(0.0, 0.0, 5.0, 5.0)),
        Err(Ghe2dError::Image(_))
    ));
    assert!(matches!(
        g.draw_image_fit_bytes(
            b"not a picture at all",
            FitImageOptions::new(0.0, 0.0, 5.0, 5.0)
        ),
        Err(Ghe2dError::Image(_))
    ));
}

#[test]
fn a_box_far_bigger_than_the_canvas_only_costs_what_can_be_seen() {
    let src = halves(300, 200);
    let started = Instant::now();
    let mut g = Ghe2d::new(200, 100);
    g.draw_image_fit(
        &src,
        FitImageOptions::new(-50_000.0, -50_000.0, 100_000.0, 100_000.0).fit(ImageFit::Fill),
    );
    assert!(
        started.elapsed().as_secs() < 5,
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(
        px(&g, 100, 50)[3],
        255,
        "the middle of the canvas is on the picture"
    );
    // Zero or negative boxes, and an empty picture, draw nothing and do not panic.
    let mut z = Ghe2d::new(20, 20);
    z.draw_image_fit(&src, FitImageOptions::new(0.0, 0.0, 0.0, 10.0));
    z.draw_image_fit(&src, FitImageOptions::new(0.0, 0.0, -10.0, 10.0));
    z.draw_image_fit(
        &image::RgbaImage::new(0, 0),
        FitImageOptions::new(0.0, 0.0, 10.0, 10.0),
    );
    z.draw_image_fit(&src, FitImageOptions::new(f32::NAN, 0.0, 10.0, 10.0));
    assert_eq!(z.image.pixels().filter(|p| p.0[3] != 0).count(), 0);
}

#[test]
fn a_picture_turned_by_ninety_degrees_stays_in_its_box_centre() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_image_fit(
        &halves(40, 40),
        FitImageOptions::new(30.0, 30.0, 40.0, 40.0)
            .fit(ImageFit::Fill)
            .rotation(90.0),
    );
    // Red was the left half; turned clockwise it is the top half.
    assert_eq!(px(&g, 50, 35), RED);
    assert_eq!(px(&g, 50, 65), BLUE);
}
