use ghe2d::utility::Rgba;
use ghe2d::{
    CircleOptions, Ghe2d, LineOptions, PolygonOptions, RectOptions, RhombusOptions, StarOptions,
};

const W: Rgba = Rgba::WHITE;

fn painted(g: &Ghe2d) -> usize {
    g.image.pixels().filter(|p| p.0[3] != 0).count()
}

fn painted_bounds(g: &Ghe2d) -> Option<(u32, u32, u32, u32)> {
    let (mut a, mut b, mut c, mut d) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in g.image.enumerate_pixels() {
        if p.0[3] != 0 {
            a = a.min(x);
            b = b.min(y);
            c = c.max(x);
            d = d.max(y);
        }
    }
    (a != u32::MAX).then_some((a, b, c, d))
}

#[test]
fn rect_fill_clips_and_blends() {
    let mut g = Ghe2d::new(10, 10);
    g.draw_rect(RectOptions::solid(8.0, 8.0, 50.0, 50.0, W)); // straddles the corner
    g.draw_rect(RectOptions::solid(1000.0, 1000.0, 5.0, 5.0, W)); // fully outside
    g.draw_rect(RectOptions::new(0.0, 0.0).fill(W)); // zero-size
    assert_eq!(g.image.get_pixel(9, 9).0, [255, 255, 255, 255]);
    assert_eq!(g.image.get_pixel(0, 0).0, [0, 0, 0, 0]);

    // fill with alpha blends over an opaque base instead of overwriting
    let mut h = Ghe2d::new(4, 4);
    h.draw_rect(RectOptions::solid(0.0, 0.0, 4.0, 4.0, Rgba::rgb(0, 0, 0)));
    h.draw_rect(RectOptions::solid(0.0, 0.0, 4.0, 4.0, Rgba::new(255, 255, 255, 128)));
    let px = h.image.get_pixel(1, 1).0;
    assert!(px[0] > 100 && px[0] < 200, "expected a blend, got {px:?}");
}

#[test]
fn polygon_side_count_and_center() {
    let mut g = Ghe2d::new(200, 200);
    g.draw_polygon(PolygonOptions::new(60.0).at(100.0, 100.0).sides(6).fill(W));
    let (x0, y0, x1, y1) = painted_bounds(&g).expect("rendered");
    // pointy-top hexagon r=60 at (100,100): top/bottom vertices give the full 2r in y,
    // the horizontal extent is r*sqrt(3) ~= 104.
    assert!((y0 as i32 - 40).abs() < 8 && (y1 as i32 - 160).abs() < 8, "y {y0}..{y1}");
    assert!(((x1 - x0) as i32 - 104).abs() < 12, "hex width {} px", x1 - x0);
    assert!(((x0 + x1) as i32 / 2 - 100).abs() < 4, "not centred on x=100");
    assert!(((y0 + y1) as i32 / 2 - 100).abs() < 4, "not centred on y=100");
}

#[test]
fn triangle_is_polygon_with_three_sides_by_default() {
    let mut tri = Ghe2d::new(120, 120);
    tri.draw_polygon(PolygonOptions::new(50.0).at(60.0, 60.0).fill(W));
    let mut hex = Ghe2d::new(120, 120);
    hex.draw_polygon(PolygonOptions::new(50.0).at(60.0, 60.0).sides(6).fill(W));
    // a triangle covers noticeably less area than a hexagon of the same radius
    assert!(painted(&tri) * 3 < painted(&hex) * 2, "tri {} hex {}", painted(&tri), painted(&hex));
}

#[test]
fn star_is_concave_so_covers_less_than_its_polygon() {
    let mut star = Ghe2d::new(200, 200);
    star.draw_star(StarOptions::new(80.0).at(100.0, 100.0).points(5).fill(W));
    let mut poly = Ghe2d::new(200, 200);
    poly.draw_polygon(PolygonOptions::new(80.0).at(100.0, 100.0).sides(5).fill(W));
    assert!(painted(&star) < painted(&poly), "star {} poly {}", painted(&star), painted(&poly));
    assert!(painted(&star) > 500, "star should still render");
}

#[test]
fn rhombus_hits_its_apexes() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_rhombus(RhombusOptions::new(80.0, 60.0).at(60.0, 60.0).fill(W));
    assert_ne!(g.image.get_pixel(60, 31).0[3], 0, "top apex");
    assert_ne!(g.image.get_pixel(60, 89).0[3], 0, "bottom apex");
    assert_ne!(g.image.get_pixel(21, 60).0[3], 0, "left apex");
    assert_eq!(g.image.get_pixel(21, 31).0[3], 0, "corner is outside the diamond");
}

#[test]
fn line_width_scales_the_painted_area() {
    let mut thin = Ghe2d::new(120, 40);
    thin.draw_line(LineOptions::new(10.0, 20.0, 110.0, 20.0).color(W).width(1.0));
    let mut thick = Ghe2d::new(120, 40);
    thick.draw_line(LineOptions::new(10.0, 20.0, 110.0, 20.0).color(W).width(8.0));
    let (t1, t8) = (painted(&thin), painted(&thick));
    assert!(t8 >= t1 * 3 && t8 <= t1 * 9, "thin {t1} thick {t8}");

    // a diagonal line lights both endpoints
    let mut d = Ghe2d::new(60, 60);
    d.draw_line(LineOptions::new(5.0, 5.0, 54.0, 54.0).color(W).width(3.0));
    assert_ne!(d.image.get_pixel(5, 5).0[3], 0);
    assert_ne!(d.image.get_pixel(54, 54).0[3], 0);
}

#[test]
fn stroke_only_leaves_the_interior_empty() {
    let mut g = Ghe2d::new(120, 120);
    g.draw_circle(CircleOptions::new(50.0).at(60.0, 60.0).no_fill().stroke(W, 4.0));
    assert_eq!(g.image.get_pixel(60, 60).0[3], 0, "centre must be untouched");
    assert_ne!(g.image.get_pixel(60, 11).0[3], 0, "rim near the top edge is painted");

    let mut h = Ghe2d::new(140, 140);
    h.draw_polygon(PolygonOptions::new(55.0).at(70.0, 70.0).sides(5).no_fill().stroke(W, 3.0));
    assert_eq!(h.image.get_pixel(70, 70).0[3], 0, "polygon centre must be untouched");
    assert!(painted(&h) > 100, "the outline should be drawn");
}

#[test]
fn degenerate_shapes_do_not_panic() {
    let mut g = Ghe2d::new(16, 16);
    g.draw_polygon(PolygonOptions::new(6.0).at(8.0, 8.0).sides(1).fill(W)); // clamped to 3
    g.draw_polygon(PolygonOptions::new(0.0).at(8.0, 8.0).fill(W)); // zero radius
    g.draw_star(StarOptions::new(6.0).at(8.0, 8.0).points(0).fill(W)); // clamped
    g.draw_star(StarOptions::new(9999.0).at(8.0, 8.0).points(7).fill(W)); // huge
    g.draw_circle(CircleOptions::solid(-50.0, -50.0, 20.0, W)); // off canvas
    g.draw_rhombus(RhombusOptions::new(0.0, 0.0).at(8.0, 8.0).fill(W));
    g.draw_line(LineOptions::new(0.0, 0.0, 0.0, 0.0).color(W).width(4.0)); // zero length
    g.draw_rect(RectOptions::new(20.0, 20.0).at(4.0, 4.0).rotation(37.0).fill(W).stroke(W, 2.0));
    let mut z = Ghe2d::new(0, 0);
    z.draw_polygon(PolygonOptions::new(10.0).fill(W));
    z.draw_line(LineOptions::new(0.0, 0.0, 5.0, 5.0).color(W));
}
