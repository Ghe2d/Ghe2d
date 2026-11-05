//! Text in a box: keeping the size, aligning without shrinking, and clean colour on a
//! transparent canvas.

use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{measure_text, Ghe2d, LineLayout, TextAlign, TextOptions};

fn font() -> LoadFont {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/assets/fonts/Rubik-Regular.ttf");
    LoadFont::from_bytes(std::fs::read(p).expect("font")).expect("valid font")
}

fn ink_bounds(g: &Ghe2d) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in g.image.enumerate_pixels() {
        if p.0[3] > 20 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0, y0, x1, y1)
}

#[test]
fn text_shrinks_to_fit_by_default_and_keeps_its_size_when_told_to() {
    let base = || {
        TextOptions::new(font(), "A line that is far too long for its box")
            .size(40.0)
            .width(120.0)
    };
    let shrunk = measure_text(&base());
    assert!(shrunk.size < 40.0 && shrunk.width <= 120.5, "{shrunk:?}");
    let kept = measure_text(&base().shrink_to_fit(false));
    assert_eq!(kept.size, 40.0);
    assert!(kept.width > 120.0, "it runs out of the box: {}", kept.width);
    // Wrapping still wraps when it does not shrink.
    let wrapped = measure_text(&base().shrink_to_fit(false).wrap(true));
    assert_eq!(wrapped.size, 40.0);
    assert!(wrapped.lines.len() > 1);
}

#[test]
fn a_text_that_is_not_shrunk_is_still_aligned_inside_its_box() {
    let at = |align: TextAlign| {
        let mut g = Ghe2d::new(400, 80);
        g.draw_text(
            TextOptions::new(font(), "Mid")
                .at(50.0, 10.0)
                .size(40.0)
                .color(Rgba::WHITE)
                .width(300.0)
                .align(align)
                .shrink_to_fit(false),
        );
        ink_bounds(&g)
    };
    let (l0, _, r0, _) = at(TextAlign::Left);
    let (l1, _, r1, _) = at(TextAlign::Center);
    let (l2, _, r2, _) = at(TextAlign::Right);
    assert!(l0 <= 56, "left starts at the box: {l0}");
    assert!(
        ((l1 + r1) as i32 - 400).abs() <= 6,
        "centred on x = 200: {l1} {r1}"
    );
    assert!(r2 >= 344, "right ends at the box: {r2}");
    assert!(r0 < l1 && r1 < l2 + 100);
}

#[test]
fn middle_layout_centres_a_line_in_a_taller_box() {
    let mut g = Ghe2d::new(200, 200);
    g.draw_text(
        TextOptions::new(font(), "Hx")
            .at(0.0, 50.0)
            .size(30.0)
            .color(Rgba::WHITE)
            .width(200.0)
            .height(100.0)
            .line_layout(LineLayout::Middle(0.0)),
    );
    let (_, y0, _, y1) = ink_bounds(&g);
    let mid = (y0 + y1) as f32 / 2.0;
    assert!((mid - 100.0).abs() < 8.0, "{mid}");
}

#[test]
fn text_on_a_transparent_canvas_keeps_the_colour_at_its_soft_edges() {
    let mut g = Ghe2d::new(300, 100);
    g.draw_text(
        TextOptions::new(font(), "Clean edges")
            .at(10.0, 20.0)
            .size(48.0)
            .color(Rgba::rgb(250, 200, 20)),
    );
    let mut soft = 0;
    for p in g.image.pixels() {
        if p.0[3] > 0 {
            assert_eq!(
                &p.0[..3],
                &[250, 200, 20],
                "a pixel with alpha {} lost the colour",
                p.0[3]
            );
            if p.0[3] < 255 {
                soft += 1;
            }
        }
    }
    assert!(soft > 100, "there are soft edge pixels to check: {soft}");
}

#[test]
fn a_semi_transparent_text_colour_never_thins_an_opaque_canvas() {
    let mut g = Ghe2d::new(200, 80);
    g.draw_rect(ghe2d::RectOptions::solid(
        0.0,
        0.0,
        200.0,
        80.0,
        Rgba::WHITE,
    ));
    g.draw_text(
        TextOptions::new(font(), "Ghost")
            .at(10.0, 10.0)
            .size(48.0)
            .color(Rgba::new(0, 0, 0, 128)),
    );
    assert!(g.image.pixels().all(|p| p.0[3] == 255));
    let darkest = g.image.pixels().map(|p| p.0[0]).min().unwrap();
    assert!(
        (120..140).contains(&darkest),
        "half black over white is about 127: {darkest}"
    );
}
