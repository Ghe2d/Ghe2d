//! Rounded rectangles, ellipses and free paths.

use ghe2d::utility::Rgba;
use ghe2d::{EllipseOptions, Ghe2d, PathOptions, RectOptions};

const RED: Rgba = Rgba {
    r: 220,
    g: 40,
    b: 40,
    a: 255,
};

fn alpha(g: &Ghe2d, x: u32, y: u32) -> u8 {
    g.image.get_pixel(x, y).0[3]
}

fn area(g: &Ghe2d) -> f64 {
    g.image.pixels().map(|p| p.0[3] as f64 / 255.0).sum()
}

#[test]
fn a_rounded_rectangle_has_round_corners_and_straight_sides() {
    let mut g = Ghe2d::new(80, 80);
    g.draw_rect(RectOptions::solid(10.0, 10.0, 60.0, 40.0, RED).radius(14.0));
    assert_eq!(alpha(&g, 10, 10), 0, "the corner is cut");
    assert_eq!(alpha(&g, 69, 10), 0);
    assert_eq!(alpha(&g, 10, 49), 0);
    assert_eq!(alpha(&g, 69, 49), 0);
    assert_eq!(
        alpha(&g, 40, 10),
        255,
        "the middle of the top side is square"
    );
    assert_eq!(alpha(&g, 10, 30), 255);
    assert_eq!(alpha(&g, 40, 30), 255);
    assert_eq!(alpha(&g, 9, 30), 0);
    // The area is the rectangle's minus four corners: 4 r^2 (1 - pi/4).
    let expect = 60.0 * 40.0 - 4.0 * 14.0 * 14.0 * (1.0 - std::f64::consts::FRAC_PI_4);
    assert!((area(&g) - expect).abs() < 6.0, "{} vs {expect}", area(&g));
}

#[test]
fn a_radius_larger_than_half_the_side_makes_a_pill() {
    let mut g = Ghe2d::new(100, 60);
    g.draw_rect(RectOptions::solid(10.0, 10.0, 80.0, 30.0, RED).radius(1000.0));
    assert!(alpha(&g, 10, 25) >= 250, "the middle of the end is on");
    assert_eq!(alpha(&g, 10, 10), 0);
    assert_eq!(alpha(&g, 50, 10), 255);
    let expect = 50.0 * 30.0 + std::f64::consts::PI * 15.0 * 15.0;
    assert!((area(&g) - expect).abs() < 8.0, "{} vs {expect}", area(&g));
}

#[test]
fn a_rounded_rectangle_turns_with_its_rotation() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_rect(
        RectOptions::solid(40.0, 50.0, 40.0, 20.0, RED)
            .radius(6.0)
            .rotation(90.0),
    );
    // Turned about its centre (60, 60): now 20 wide and 40 tall.
    assert_eq!(alpha(&g, 60, 42), 255);
    assert_eq!(alpha(&g, 60, 77), 255);
    assert_eq!(alpha(&g, 44, 60), 0);
    let expect = 40.0 * 20.0 - 4.0 * 36.0 * (1.0 - std::f64::consts::FRAC_PI_4);
    assert!((area(&g) - expect).abs() < 5.0, "{} vs {expect}", area(&g));
}

#[test]
fn a_rounded_rectangle_can_have_only_a_stroke() {
    let mut g = Ghe2d::new(80, 80);
    g.draw_rect(
        RectOptions::new(60.0, 40.0)
            .at(10.0, 10.0)
            .radius(10.0)
            .no_fill()
            .stroke(RED, 4.0),
    );
    assert_eq!(alpha(&g, 40, 30), 0, "empty inside");
    assert_eq!(alpha(&g, 40, 10), 255, "the line is centred on the edge");
    assert_eq!(alpha(&g, 40, 8), 255);
    assert_eq!(alpha(&g, 40, 6), 0);
    assert_eq!(alpha(&g, 40, 11), 255);
    assert_eq!(alpha(&g, 40, 13), 0);
}

#[test]
fn an_ellipse_fills_its_box() {
    let mut g = Ghe2d::new(120, 80);
    g.draw_ellipse(EllipseOptions::in_box(10.0, 10.0, 100.0, 50.0).fill(RED));
    assert_eq!(alpha(&g, 60, 35), 255);
    assert_eq!(alpha(&g, 12, 35), 255, "near the left end");
    assert_eq!(alpha(&g, 60, 11), 255, "near the top");
    assert_eq!(alpha(&g, 12, 12), 0, "the corner of the box is empty");
    assert_eq!(alpha(&g, 109, 59), 0);
    let expect = std::f64::consts::PI * 50.0 * 25.0;
    assert!(
        (area(&g) - expect).abs() < expect * 0.015,
        "{} vs {expect}",
        area(&g)
    );
}

#[test]
fn an_ellipse_turned_a_quarter_swaps_its_width_and_height() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_ellipse(
        EllipseOptions::new(40.0, 15.0)
            .at(60.0, 60.0)
            .rotation(90.0)
            .fill(RED),
    );
    assert_eq!(alpha(&g, 60, 22), 255);
    assert_eq!(alpha(&g, 60, 97), 255);
    assert_eq!(alpha(&g, 25, 60), 0);
    let expect = std::f64::consts::PI * 40.0 * 15.0;
    assert!(
        (area(&g) - expect).abs() < expect * 0.01,
        "{} vs {expect}",
        area(&g)
    );
}

#[test]
fn an_ellipse_with_equal_radii_matches_a_circle() {
    let mut e = Ghe2d::new(80, 80);
    e.draw_ellipse(EllipseOptions::new(25.0, 25.0).at(40.0, 40.0).fill(RED));
    let mut c = Ghe2d::new(80, 80);
    c.draw_circle(ghe2d::CircleOptions::solid(40.0, 40.0, 25.0, RED));
    assert!(
        (area(&e) - area(&c)).abs() < 3.0,
        "{} vs {}",
        area(&e),
        area(&c)
    );
}

#[test]
fn a_path_is_a_polygon_from_any_points_and_a_triangle_helper_exists() {
    let mut g = Ghe2d::new(100, 100);
    g.draw_path(PathOptions::triangle(20.0, 20.0, 60.0, 60.0).fill(RED));
    assert_eq!(alpha(&g, 50, 22), 255, "the apex is on the top middle");
    assert_eq!(alpha(&g, 50, 78), 255);
    assert_eq!(alpha(&g, 22, 78), 255, "the base corner");
    assert_eq!(alpha(&g, 22, 30), 0, "outside the slope");
    assert_eq!(alpha(&g, 78, 30), 0);
    assert!((area(&g) - 1800.0).abs() < 25.0, "{}", area(&g));

    // An open path draws a line only.
    let mut o = Ghe2d::new(100, 100);
    o.draw_path(
        PathOptions::new([(10.0, 10.0), (90.0, 10.0), (90.0, 90.0)])
            .closed(false)
            .no_fill()
            .stroke(RED, 4.0),
    );
    assert_eq!(alpha(&o, 50, 10), 255);
    assert_eq!(alpha(&o, 90, 50), 255);
    assert_eq!(alpha(&o, 50, 50), 0);
    assert_eq!(alpha(&o, 50, 90), 0, "not closed back to the start");
    // Too few points and an empty path draw nothing, and do not panic.
    let mut e = Ghe2d::new(10, 10);
    e.draw_path(PathOptions::new([]));
    e.draw_path(PathOptions::new([(1.0, 1.0)]));
    e.draw_path(PathOptions::new([(1.0, 1.0), (f32::NAN, 2.0), (3.0, 3.0)]).opacity(0.5));
    assert_eq!(area(&e), 0.0);
}
