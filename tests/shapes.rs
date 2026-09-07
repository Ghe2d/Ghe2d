use ghe2d::utility::Rgba;
use ghe2d::Ghe2d;

// Every shape that extends past the canvas must draw its in-bounds part and
// silently clip the rest -- never panic.

#[test]
fn shapes_off_canvas_do_not_panic() {
    let mut g = Ghe2d::new(10, 10);
    let c = Rgba::new(255, 255, 255, 255);

    g.draw_rect(8, 8, 50, 50, c.clone()); // straddles the bottom-right edge
    g.draw_rect(1000, 1000, 5, 5, c.clone()); // origin fully outside
    g.draw_rect(0, 0, 0, 0, c.clone()); // zero-sized
    g.draw_circle(5, 5, 100, c.clone()); // radius past every edge
    g.draw_circle(2, 2, 0, c.clone()); // zero radius
    g.draw_circle(1000, 1000, 3, c.clone()); // center outside
    g.draw_circle(0, 0, 4, c); // center at corner, most of it clipped
}

#[test]
fn shapes_on_zero_canvas_do_not_panic() {
    let mut g = Ghe2d::new(0, 0);
    let c = Rgba::new(1, 2, 3, 4);
    g.draw_rect(0, 0, 5, 5, c.clone());
    g.draw_circle(0, 0, 5, c);
}

#[test]
fn in_bounds_rect_is_actually_drawn() {
    let mut g = Ghe2d::new(10, 10);
    g.draw_rect(2, 2, 3, 3, Rgba::new(10, 20, 30, 255));
    assert_eq!(g.image.get_pixel(3, 3).0, [10, 20, 30, 255]);
    // outside the rect is untouched
    assert_eq!(g.image.get_pixel(9, 9).0, [0, 0, 0, 0]);
}
