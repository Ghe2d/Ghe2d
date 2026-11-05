//! Shadow, glow, opacity, rotation and clipping, for every kind of drawable.

use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{
    CircleOptions, EllipseOptions, FitImageOptions, Ghe2d, Glow, GroupOptions, ImageFit,
    LineOptions, PathOptions, PolygonOptions, RectOptions, RhombusOptions, Shadow, StarOptions,
    TextOptions,
};

fn font() -> LoadFont {
    let p =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/assets/fonts/Rubik-Regular.ttf");
    LoadFont::from_bytes(std::fs::read(p).expect("font")).expect("valid font")
}

fn px(g: &Ghe2d, x: u32, y: u32) -> [u8; 4] {
    g.image.get_pixel(x, y).0
}

fn alpha(g: &Ghe2d, x: u32, y: u32) -> u8 {
    px(g, x, y)[3]
}

fn painted(g: &Ghe2d) -> usize {
    g.image.pixels().filter(|p| p.0[3] != 0).count()
}

fn bounds(g: &Ghe2d) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in g.image.enumerate_pixels() {
        if p.0[3] != 0 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0, y0, x1, y1)
}

const RED: Rgba = Rgba {
    r: 220,
    g: 40,
    b: 40,
    a: 255,
};

// ----------------------------------------------------------------------------- shadow

#[test]
fn a_hard_shadow_is_the_shape_shifted_and_coloured() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_rect(
        RectOptions::solid(30.0, 30.0, 20.0, 20.0, RED)
            .shadow(Shadow::new(Rgba::rgb(0, 0, 255)).offset(10.0, 10.0)),
    );
    assert_eq!(
        px(&g, 35, 35),
        [220, 40, 40, 255],
        "the rectangle is on top"
    );
    assert_eq!(px(&g, 55, 55), [0, 0, 255, 255], "shadow only");
    assert_eq!(
        px(&g, 55, 32),
        [0, 0, 0, 0],
        "above the shadow, right of the rectangle: nothing"
    );
    assert_eq!(alpha(&g, 25, 25), 0);
    assert_eq!(
        alpha(&g, 61, 61),
        0,
        "the shadow stops where the shifted shape stops"
    );
}

#[test]
fn a_blurred_shadow_fades_with_the_distance_and_stays_inside_its_reach() {
    let mut g = Ghe2d::new(160, 160);
    g.draw_rect(
        RectOptions::solid(60.0, 60.0, 40.0, 40.0, Rgba::WHITE)
            .shadow(Shadow::new(Rgba::BLACK).blur(16.0)),
    );
    // Right of the rectangle, along the middle row: the alpha only goes down.
    let row: Vec<u8> = (100..140).map(|x| alpha(&g, x, 80)).collect();
    assert!(row[0] > 60, "right at the edge it is dark: {}", row[0]);
    assert!(
        row.windows(2).all(|w| w[0] >= w[1]),
        "never grows away from the shape: {row:?}"
    );
    assert!(
        row[10] > 0 && row[10] < row[0],
        "in the middle of the blur: {}",
        row[10]
    );
    assert_eq!(
        *row.last().unwrap(),
        0,
        "beyond 1.5 x the blur radius there is nothing"
    );
    // It is symmetric around the shape.
    for d in [2u32, 8, 14] {
        let (l, r) = (alpha(&g, 60 - 1 - d, 80), alpha(&g, 100 + d, 80));
        assert!(l.abs_diff(r) <= 1, "left {l} right {r} at {d}");
    }
}

#[test]
fn the_shadow_opacity_multiplies_the_colour_alpha() {
    let mut g = Ghe2d::new(80, 80);
    g.draw_rect(
        RectOptions::solid(20.0, 20.0, 10.0, 10.0, RED).shadow(
            Shadow::new(Rgba::new(0, 0, 0, 200))
                .offset(30.0, 30.0)
                .opacity(0.5),
        ),
    );
    let a = alpha(&g, 55, 55) as i32;
    assert!((a - 100).abs() <= 1, "200 x 0.5 = 100, got {a}");
}

#[test]
fn a_text_has_a_shadow_made_from_its_glyphs() {
    let mut plain = Ghe2d::new(300, 100);
    plain.draw_text(
        TextOptions::new(font(), "Shadow")
            .at(20.0, 20.0)
            .size(48.0)
            .color(Rgba::WHITE),
    );
    let mut shaded = Ghe2d::new(300, 100);
    shaded.draw_text(
        TextOptions::new(font(), "Shadow")
            .at(20.0, 20.0)
            .size(48.0)
            .color(Rgba::WHITE)
            .shadow(Shadow::new(Rgba::BLACK).offset(6.0, 6.0).blur(4.0)),
    );
    assert!(
        painted(&shaded) > painted(&plain),
        "the shadow paints more pixels"
    );
    let dark = shaded
        .image
        .pixels()
        .filter(|p| p.0[3] > 100 && p.0[0] < 60)
        .count();
    assert!(dark > 50, "a dark shadow shows: {dark}");
    // The text itself is where it was: every pixel it covers fully is still white and opaque (give or take a rounding level).
    for ((x, y, p), q) in plain.image.enumerate_pixels().zip(shaded.image.pixels()) {
        if p.0 == [255, 255, 255, 255] {
            assert!(
                q.0.iter().all(|c| *c >= 253),
                "the glyph pixel ({x}, {y}) changed: {:?}",
                q.0
            );
        }
    }
}

#[test]
fn every_kind_of_drawable_takes_a_shadow() {
    let shadow = Shadow::new(Rgba::BLACK).offset(0.0, 12.0).blur(4.0);
    let mut cases: Vec<(&str, Ghe2d)> = Vec::new();
    let mut add = |name: &'static str, f: &dyn Fn(&mut Ghe2d)| {
        let mut g = Ghe2d::new(160, 160);
        f(&mut g);
        cases.push((name, g));
    };
    add("rect", &|g| {
        g.draw_rect(RectOptions::solid(50.0, 40.0, 60.0, 40.0, RED).shadow(shadow));
    });
    add("rounded", &|g| {
        g.draw_rect(
            RectOptions::solid(50.0, 40.0, 60.0, 40.0, RED)
                .radius(12.0)
                .shadow(shadow),
        );
    });
    add("circle", &|g| {
        g.draw_circle(CircleOptions::solid(80.0, 60.0, 25.0, RED).shadow(shadow));
    });
    add("ellipse", &|g| {
        g.draw_ellipse(
            EllipseOptions::new(35.0, 20.0)
                .at(80.0, 60.0)
                .fill(RED)
                .shadow(shadow),
        );
    });
    add("polygon", &|g| {
        g.draw_polygon(
            PolygonOptions::new(28.0)
                .at(80.0, 60.0)
                .sides(6)
                .fill(RED)
                .shadow(shadow),
        );
    });
    add("star", &|g| {
        g.draw_star(
            StarOptions::new(30.0)
                .at(80.0, 60.0)
                .fill(RED)
                .shadow(shadow),
        );
    });
    add("rhombus", &|g| {
        g.draw_rhombus(
            RhombusOptions::new(60.0, 40.0)
                .at(80.0, 60.0)
                .fill(RED)
                .shadow(shadow),
        );
    });
    add("path", &|g| {
        g.draw_path(
            PathOptions::triangle(50.0, 40.0, 60.0, 40.0)
                .fill(RED)
                .shadow(shadow),
        );
    });
    add("line", &|g| {
        g.draw_line(
            LineOptions::new(40.0, 60.0, 120.0, 60.0)
                .color(RED)
                .width(6.0)
                .shadow(shadow),
        );
    });
    add("text", &|g| {
        g.draw_text(
            TextOptions::new(font(), "Aa")
                .at(50.0, 30.0)
                .size(48.0)
                .color(RED)
                .shadow(shadow),
        );
    });
    add("image", &|g| {
        let src = image::RgbaImage::from_pixel(10, 10, image::Rgba([220, 40, 40, 255]));
        g.draw_image_fit(
            &src,
            FitImageOptions::new(50.0, 40.0, 60.0, 40.0).shadow(shadow),
        );
    });
    for (name, g) in &cases {
        let b = bounds(g);
        let own = g
            .image
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[3] > 0 && p.0[0] > 150 && p.0[1] < 90)
            .map(|(_, y, _)| y)
            .max()
            .unwrap_or_else(|| panic!("{name}: nothing red drawn"));
        assert!(
            b.3 > own + 6,
            "{name}: the shadow reaches below the shape ({} vs {own})",
            b.3
        );
        let shade = g
            .image
            .pixels()
            .filter(|p| p.0[3] > 80 && p.0[0] < 60 && p.0[1] < 60)
            .count();
        assert!(shade > 20, "{name}: dark shadow pixels: {shade}");
    }
}

// ----------------------------------------------------------------------------- glow

#[test]
fn a_glow_surrounds_the_shape_on_every_side_and_is_not_offset() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_circle(
        CircleOptions::solid(60.0, 60.0, 20.0, Rgba::WHITE)
            .glow(Glow::new(Rgba::rgb(255, 0, 0)).blur(14.0)),
    );
    let (l, r, t, b) = (
        alpha(&g, 36, 60),
        alpha(&g, 83, 60),
        alpha(&g, 60, 36),
        alpha(&g, 60, 83),
    );
    assert!(l > 20 && r > 20 && t > 20 && b > 20, "{l} {r} {t} {b}");
    assert!(
        l.abs_diff(r) <= 2 && t.abs_diff(b) <= 2,
        "centred: {l} {r} {t} {b}"
    );
    assert_eq!(alpha(&g, 60, 5), 0, "the glow does not go on for ever");
    assert_eq!(
        px(&g, 60, 60),
        [255, 255, 255, 255],
        "the shape is on top of its glow"
    );
    let halo = px(&g, 36, 60);
    assert!(
        halo[0] > 200 && halo[1] < 40,
        "the halo is the glow colour: {halo:?}"
    );
}

#[test]
fn a_larger_intensity_makes_a_denser_glow() {
    let at = |intensity: f32| {
        let mut g = Ghe2d::new(120, 120);
        g.draw_circle(
            CircleOptions::solid(60.0, 60.0, 20.0, Rgba::WHITE).glow(
                Glow::new(Rgba::rgb(255, 0, 0))
                    .blur(14.0)
                    .intensity(intensity),
            ),
        );
        alpha(&g, 36, 60)
    };
    let (zero, one, three) = (at(0.0), at(1.0), at(3.0));
    assert_eq!(zero, 0, "no intensity, no glow");
    assert!(one > 0 && three > one, "{zero} {one} {three}");
}

#[test]
fn the_shadow_is_under_the_glow_and_both_are_under_the_shape() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_rect(
        RectOptions::solid(40.0, 40.0, 20.0, 20.0, RED)
            .shadow(Shadow::new(Rgba::rgb(0, 0, 255)))
            .glow(Glow::new(Rgba::rgb(0, 255, 0)).blur(0.0)),
    );
    // Same place, so the glow (a hard copy of the shape) hides the shadow, and the shape hides both.
    assert_eq!(px(&g, 50, 50), [220, 40, 40, 255]);
    assert_eq!(alpha(&g, 70, 70), 0);
}

// ----------------------------------------------------------------------------- opacity

#[test]
fn opacity_fades_the_whole_drawable() {
    let mut g = Ghe2d::new(40, 40);
    g.draw_rect(RectOptions::solid(10.0, 10.0, 20.0, 20.0, RED).opacity(0.5));
    let a = alpha(&g, 20, 20) as i32;
    assert!((a - 128).abs() <= 1, "{a}");
    assert_eq!(
        &px(&g, 20, 20)[..3],
        &[220, 40, 40],
        "the colour is not changed"
    );

    // Over an opaque ground the colours mix.
    let mut g = Ghe2d::new(40, 40);
    g.draw_rect(RectOptions::solid(0.0, 0.0, 40.0, 40.0, Rgba::WHITE));
    g.draw_rect(RectOptions::solid(10.0, 10.0, 20.0, 20.0, Rgba::BLACK).opacity(0.25));
    let v = px(&g, 20, 20);
    assert_eq!(v[3], 255);
    assert!((v[0] as i32 - 191).abs() <= 1, "{v:?}");
}

#[test]
fn opacity_fades_the_shadow_with_the_shape() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_rect(
        RectOptions::solid(20.0, 20.0, 20.0, 20.0, RED)
            .shadow(Shadow::new(Rgba::BLACK).offset(30.0, 30.0))
            .opacity(0.5),
    );
    let a = alpha(&g, 60, 60) as i32;
    assert!((a - 128).abs() <= 1, "the shadow is faded too: {a}");
}

#[test]
fn zero_opacity_draws_nothing() {
    let mut g = Ghe2d::new(40, 40);
    g.draw_circle(CircleOptions::solid(20.0, 20.0, 10.0, RED).opacity(0.0));
    assert_eq!(painted(&g), 0);
}

// ----------------------------------------------------------------------------- rotation

#[test]
fn a_group_turns_clockwise_about_the_centre_of_its_box() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_group(
        GroupOptions::new(30.0, 40.0, 40.0, 20.0).rotation(90.0),
        |l, ox, oy| {
            l.draw_rect(RectOptions::solid(ox, oy, 40.0, 20.0, RED));
        },
    );
    let (x0, y0, x1, y1) = bounds(&g);
    assert_eq!(
        (x0, y0, x1, y1),
        (40, 30, 59, 69),
        "the 40 x 20 box is now 20 x 40 around (50, 50)"
    );
    assert_eq!(px(&g, 50, 35), [220, 40, 40, 255]);
    assert_eq!(
        alpha(&g, 35, 50),
        0,
        "where the box was, left of the middle, nothing now"
    );
}

#[test]
fn a_turn_of_180_flips_a_picture_and_its_marker_moves_to_the_other_corner() {
    let mut src = image::RgbaImage::from_pixel(20, 20, image::Rgba([0, 0, 255, 255]));
    for y in 0..10 {
        for x in 0..10 {
            src.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }
    let mut g = Ghe2d::new(60, 60);
    g.draw_image_fit(
        &src,
        FitImageOptions::new(20.0, 20.0, 20.0, 20.0)
            .fit(ImageFit::Fill)
            .rotation(180.0),
    );
    assert_eq!(
        px(&g, 22, 22)[2],
        255,
        "the red corner is not top-left any more"
    );
    assert_eq!(px(&g, 37, 37), [255, 0, 0, 255]);
}

#[test]
fn a_rotated_text_stands_up_and_a_text_turned_back_lies_down() {
    let mut flat = Ghe2d::new(300, 300);
    flat.draw_text(
        TextOptions::new(font(), "Rotate me")
            .at(80.0, 130.0)
            .size(40.0)
            .color(Rgba::WHITE),
    );
    let (a, b, c, d) = bounds(&flat);
    assert!(c - a > 2 * (d - b), "flat: {:?}", (a, b, c, d));
    let mut up = Ghe2d::new(300, 300);
    up.draw_text(
        TextOptions::new(font(), "Rotate me")
            .at(80.0, 130.0)
            .size(40.0)
            .color(Rgba::WHITE)
            .rotation(90.0),
    );
    let (a, b, c, d) = bounds(&up);
    assert!(d - b > 2 * (c - a), "standing: {:?}", (a, b, c, d));
    // The same amount of ink, within the resampling.
    let ink = |g: &Ghe2d| g.image.pixels().map(|p| p.0[3] as u64).sum::<u64>() as f64;
    let ratio = ink(&up) / ink(&flat);
    assert!((0.93..1.07).contains(&ratio), "ink ratio {ratio}");
    // A turn of 360 is no turn.
    let mut full = Ghe2d::new(300, 300);
    full.draw_text(
        TextOptions::new(font(), "Rotate me")
            .at(80.0, 130.0)
            .size(40.0)
            .color(Rgba::WHITE)
            .rotation(360.0),
    );
    let worst = flat
        .image
        .pixels()
        .zip(full.image.pixels())
        .map(|(a, b)| (0..4).map(|i| a.0[i].abs_diff(b.0[i])).max().unwrap())
        .max()
        .unwrap();
    assert!(
        worst <= 3,
        "a turn of 360 is no turn (largest difference {worst} levels)"
    );
}

#[test]
fn a_turned_rectangle_keeps_its_area() {
    let area = |deg: f32| {
        let mut g = Ghe2d::new(200, 200);
        g.draw_group(
            GroupOptions::new(60.0, 80.0, 80.0, 40.0).rotation(deg),
            |l, ox, oy| {
                l.draw_rect(RectOptions::solid(ox, oy, 80.0, 40.0, Rgba::WHITE));
            },
        );
        g.image.pixels().map(|p| p.0[3] as f64 / 255.0).sum::<f64>()
    };
    for deg in [0.0, 17.0, 45.0, 90.0, 133.0, 270.0] {
        let a = area(deg);
        assert!((a - 3200.0).abs() < 40.0, "{deg}: {a}");
    }
}

// ----------------------------------------------------------------------------- clip

#[test]
fn a_clipped_group_cuts_what_goes_past_its_box_and_an_unclipped_one_does_not() {
    let draw = |clip: bool, margin: f32| {
        let mut g = Ghe2d::new(100, 100);
        g.draw_group(
            GroupOptions::new(30.0, 30.0, 40.0, 40.0)
                .clip(clip)
                .margin(margin),
            |l, ox, oy| {
                l.draw_rect(RectOptions::solid(ox - 10.0, oy - 10.0, 60.0, 60.0, RED));
            },
        );
        g
    };
    let clipped = draw(true, 20.0);
    assert_eq!(bounds(&clipped), (30, 30, 69, 69));
    assert_eq!(px(&clipped, 29, 50)[3], 0);
    let free = draw(false, 20.0);
    assert_eq!(bounds(&free), (20, 20, 79, 79));
    // The room past the box is the margin (2 px by itself): what is farther is not kept.
    let tight = draw(false, 0.0);
    assert_eq!(bounds(&tight), (28, 28, 71, 71));
}

#[test]
fn a_clipped_box_with_a_fraction_edge_is_cut_anti_aliased() {
    let mut g = Ghe2d::new(60, 60);
    g.draw_group(
        GroupOptions::new(10.0, 10.0, 20.5, 20.0).clip(true),
        |l, _ox, _oy| {
            l.draw_rect(RectOptions::solid(0.0, 0.0, 200.0, 200.0, Rgba::WHITE));
        },
    );
    let edge = alpha(&g, 30, 20) as i32;
    assert!((edge - 128).abs() <= 2, "half covered: {edge}");
    assert_eq!(alpha(&g, 31, 20), 0);
}

#[test]
fn effects_of_a_clipped_group_follow_the_clipped_shape() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_group(
        GroupOptions::new(30.0, 30.0, 40.0, 40.0)
            .clip(true)
            .shadow(Shadow::new(Rgba::BLACK).offset(30.0, 0.0)),
        |l, ox, oy| {
            l.draw_rect(RectOptions::solid(ox - 20.0, oy, 100.0, 40.0, RED));
        },
    );
    // The shadow is the 40 wide box shifted by 30, not the 100 wide rectangle.
    assert_eq!(alpha(&g, 85, 50), 255);
    assert_eq!(alpha(&g, 101, 50), 0);
}

// ----------------------------------------------------------------------------- safety

#[test]
fn absurd_values_never_panic_and_never_paint_out_of_nothing() {
    let mut g = Ghe2d::new(50, 50);
    let weird = Shadow::new(Rgba::BLACK)
        .offset(f32::NAN, f32::INFINITY)
        .blur(f32::NAN)
        .opacity(f32::NAN);
    g.draw_rect(
        RectOptions::solid(10.0, 10.0, 20.0, 20.0, RED)
            .shadow(weird)
            .glow(Glow::new(Rgba::WHITE).blur(1e9).intensity(f32::INFINITY)),
    );
    g.draw_circle(CircleOptions::solid(f32::NAN, 5.0, 10.0, RED).opacity(0.5));
    g.draw_rect(RectOptions::solid(-1e9, -1e9, 2e9, 2e9, RED).opacity(0.5));
    g.draw_rect(RectOptions::solid(0.0, 0.0, 0.0, 10.0, RED).shadow(weird));
    g.draw_group(
        GroupOptions::new(f32::NAN, 0.0, 10.0, 10.0).rotation(f32::NAN),
        |_, _, _| {},
    );
    g.draw_group(
        GroupOptions::new(0.0, 0.0, 1e9, 1e9).rotation(33.0),
        |_, _, _| {},
    );
    g.draw_text(
        TextOptions::new(font(), "x")
            .at(f32::NAN, 0.0)
            .rotation(10.0),
    );
    let mut empty = Ghe2d::new(0, 0);
    empty.draw_rect(RectOptions::solid(0.0, 0.0, 10.0, 10.0, RED).opacity(0.5));
    // A huge box of one colour with the effects off screen stays cheap.
    let mut big = Ghe2d::new(100, 100);
    big.draw_rect(
        RectOptions::solid(-1e6, -1e6, 2e6, 2e6, RED).shadow(Shadow::new(Rgba::BLACK).blur(20.0)),
    );
    assert_eq!(px(&big, 50, 50), [220, 40, 40, 255]);
}
