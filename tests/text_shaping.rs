use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{Ghe2d, LineLayout, TextAlign, TextOptions};

fn font(name: &str) -> LoadFont {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets/fonts")
        .join(name);
    LoadFont::from_bytes(std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}")))
        .expect("valid font")
}

const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };

/// White text at `size`, positioned at `(x, y)`.
fn t(name: &str, s: &str, x: f32, y: f32, size: f32) -> TextOptions {
    TextOptions::new(font(name), s).at(x, y).size(size).color(WHITE)
}

fn painted_pixels(g: &Ghe2d) -> usize {
    g.image.pixels().filter(|p| p.0[3] != 0).count()
}

fn painted_bounds(g: &Ghe2d) -> Option<(u32, u32, u32, u32)> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in g.image.enumerate_pixels() {
        if p.0[3] != 0 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    (min_x != u32::MAX).then_some((min_x, min_y, max_x, max_y))
}

#[test]
fn arabic_renders_with_presentation_form_font() {
    let mut g = Ghe2d::new(600, 160);
    g.draw_text(t("Cairo-Regular.ttf", "مرحبا بالعالم", 20.0, 20.0, 60.0));
    assert!(painted_pixels(&g) > 200, "Cairo should rasterize Arabic glyphs");
}

#[test]
fn arabic_renders_with_opentype_only_font() {
    // Ruwudu ships almost no legacy presentation-form glyphs — before the rustybuzz
    // switch this produced only .notdef boxes.
    let mut g = Ghe2d::new(600, 160);
    g.draw_text(t("Ruwudu-Regular.ttf", "مرحبا بالعالم", 20.0, 20.0, 60.0));
    assert!(painted_pixels(&g) > 200, "Ruwudu should rasterize Arabic glyphs via OpenType shaping");
}

#[test]
fn shaped_advances_match_rasterization_scale() {
    // Regression guard: `rustybuzz` advances must be scaled by `size / (ascent - descent)`
    // to line up with `rusttype`'s rasterization. A wrong basis spreads glyphs far apart.
    let mut g = Ghe2d::new(1200, 200);
    g.draw_text(t("Cairo-Regular.ttf", "ببببب", 20.0, 20.0, 100.0));
    let (min_x, _, max_x, _) = painted_bounds(&g).expect("something rendered");
    let width = max_x - min_x;
    assert!(width > 60, "expected the word to render (got {width}px)");
    assert!(width < 5 * 100, "5 connected letters must be well under 5 ems wide (got {width}px)");
}

#[test]
fn latin_renders() {
    let mut g = Ghe2d::new(600, 120);
    g.draw_text(t("Rubik-Regular.ttf", "Hello 123", 20.0, 20.0, 48.0));
    assert!(painted_pixels(&g) > 100);
}

#[test]
fn text_edge_cases_do_not_panic() {
    let mut g = Ghe2d::new(64, 64);
    let f = font("Cairo-Regular.ttf");
    let base = |s: String| TextOptions::new(f.clone(), s).color(WHITE);
    g.draw_text(base(String::new()).size(12.0));
    g.draw_text(base("🙂".into()).size(12.0));
    g.draw_text(base("a\nب\nc".into()).size(12.0));
    g.draw_text(base("سعر 250 ريال order #42".into()).size(12.0));
    g.draw_text(base("x".repeat(3000)).size(8.0));
    g.draw_text(base("نص طويل ".repeat(400)).size(8.0));
    g.draw_text(base("clipped far off".into()).at(5000.0, 5000.0).size(24.0));
    // constrained edge cases
    g.draw_text(base("tiny box".into()).size(40.0).width(1.0));
    g.draw_text(base("tiny box".into()).size(40.0).height(1.0));
    g.draw_text(base("supercalifragilisticexpialidocious".into()).size(40.0).width(20.0).wrap(true));
    g.draw_text(base("no height here".into()).size(20.0).line_layout(LineLayout::SpaceEvenly));
    g.draw_text(base("no height here".into()).size(20.0).line_layout(LineLayout::Middle(4.0)));
}

fn rightmost_painted_x(g: &Ghe2d) -> u32 {
    g.image
        .enumerate_pixels()
        .filter(|(_, _, p)| p.0[3] != 0)
        .map(|(x, _, _)| x)
        .max()
        .unwrap_or(0)
}

#[test]
fn mixed_direction_line_lays_out_all_runs() {
    let only_arabic = {
        let mut g = Ghe2d::new(700, 100);
        g.draw_text(t("Cairo-Regular.ttf", "عربي", 10.0, 10.0, 40.0));
        rightmost_painted_x(&g)
    };
    let mixed = {
        let mut g = Ghe2d::new(700, 100);
        g.draw_text(t("Cairo-Regular.ttf", "عربي 12 END", 10.0, 10.0, 40.0));
        rightmost_painted_x(&g)
    };
    assert!(mixed > only_arabic + 30, "the Latin/number runs must be laid out as their own visual runs");
}

// ---- Part 4: box layout -------------------------------------------------------

#[test]
fn center_align_balances_padding() {
    let mut g = Ghe2d::new(400, 80);
    g.draw_text(t("Rubik-Regular.ttf", "hi", 0.0, 10.0, 40.0).width(400.0).align(TextAlign::Center));
    let (min_x, _, max_x, _) = painted_bounds(&g).expect("rendered");
    let left_pad = min_x as i32;
    let right_pad = 400 - max_x as i32;
    assert!((left_pad - right_pad).abs() < 20, "center: pads {left_pad} vs {right_pad}");
}

#[test]
fn right_align_hugs_the_right_edge() {
    let mut g = Ghe2d::new(400, 80);
    g.draw_text(t("Rubik-Regular.ttf", "end", 0.0, 10.0, 40.0).width(400.0).align(TextAlign::Right));
    let (_, _, max_x, _) = painted_bounds(&g).expect("rendered");
    assert!(max_x > 360, "right-aligned text should reach the right edge (max_x={max_x})");
}

#[test]
fn arabic_default_align_is_right() {
    let mut g = Ghe2d::new(400, 90);
    g.draw_text(t("Cairo-Regular.ttf", "مرحبا", 0.0, 10.0, 40.0).width(400.0)); // no explicit align
    let (min_x, _, max_x, _) = painted_bounds(&g).expect("rendered");
    assert!(max_x > 360 && min_x > 150, "Arabic with a width but no align should hug the right");
}

#[test]
fn shrink_to_fit_keeps_text_within_width() {
    let mut g = Ghe2d::new(300, 200);
    // 120px would be ~600px wide; must shrink to fit 260.
    g.draw_text(t("Rubik-Regular.ttf", "shrink me down", 10.0, 10.0, 120.0).width(260.0));
    let (min_x, _, max_x, _) = painted_bounds(&g).expect("rendered");
    assert!(max_x - min_x <= 260, "shrunk text width {} must be <= 260", max_x - min_x);
    assert!(max_x <= 300, "must not spill past the canvas");
}

#[test]
fn wrap_produces_multiple_lines() {
    let one_line_h = {
        let mut g = Ghe2d::new(600, 400);
        g.draw_text(t("Rubik-Regular.ttf", "word", 10.0, 10.0, 30.0));
        let (_, min_y, _, max_y) = painted_bounds(&g).expect("rendered");
        max_y - min_y
    };
    let mut g = Ghe2d::new(600, 400);
    g.draw_text(
        t("Rubik-Regular.ttf", "one two three four five six seven eight nine ten", 10.0, 10.0, 30.0)
            .width(180.0)
            .wrap(true),
    );
    let (min_x, min_y, max_x, max_y) = painted_bounds(&g).expect("rendered");
    assert!(max_y - min_y > one_line_h * 2, "wrapped text should span several lines");
    assert!(max_x - min_x <= 180 + 5, "each wrapped line should fit the width");
}

#[test]
fn gap_increases_line_spacing() {
    let measure = |layout: LineLayout| {
        let mut g = Ghe2d::new(400, 400);
        g.draw_text(t("Rubik-Regular.ttf", "aaa\nbbb", 10.0, 10.0, 30.0).line_layout(layout));
        let (_, min_y, _, max_y) = painted_bounds(&g).expect("rendered");
        max_y - min_y
    };
    let h0 = measure(LineLayout::Gap(0.0));
    let h1 = measure(LineLayout::Gap(30.0));
    assert!(h1 >= h0 + 22 && h1 <= h0 + 38, "Gap(30) should add ~30px (h0={h0}, h1={h1})");
}

#[test]
fn middle_layout_centers_the_block_vertically() {
    let mut g = Ghe2d::new(200, 400);
    g.draw_text(
        t("Rubik-Regular.ttf", "l1\nl2\nl3", 10.0, 0.0, 24.0)
            .height(400.0)
            .line_layout(LineLayout::Middle(0.0)),
    );
    let (_, min_y, _, max_y) = painted_bounds(&g).expect("rendered");
    let top_pad = min_y as i32;
    let bot_pad = 400 - max_y as i32;
    assert!((top_pad - bot_pad).abs() < 30, "middle: top {top_pad} vs bottom {bot_pad}");
}

#[test]
fn space_between_pins_first_and_last_lines() {
    let mut g = Ghe2d::new(200, 400);
    g.draw_text(
        t("Rubik-Regular.ttf", "top\nmid\nbot", 10.0, 0.0, 24.0)
            .height(400.0)
            .line_layout(LineLayout::SpaceBetween),
    );
    let (_, min_y, _, max_y) = painted_bounds(&g).expect("rendered");
    assert!(min_y < 40, "first line should sit near the top (min_y={min_y})");
    assert!(max_y > 360, "last line should sit near the bottom (max_y={max_y})");
}

#[test]
fn default_path_unchanged_by_options_struct() {
    // A bare TextOptions (no width/height/align, default LineLayout) must render the same
    // block the old positional call did.
    let mut g = Ghe2d::new(500, 120);
    g.draw_text(t("Cairo-Regular.ttf", "مرحبا بالعالم", 20.0, 20.0, 60.0));
    let (min_x, _, max_x, _) = painted_bounds(&g).expect("rendered");
    assert!((18..60).contains(&min_x), "starts near x=20 (min_x={min_x})");
    assert!(max_x - min_x > 150 && max_x - min_x < 500, "natural width (got {})", max_x - min_x);
}
