use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::Ghe2d;

fn font(name: &str) -> LoadFont {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets/fonts")
        .join(name);
    LoadFont::from_bytes(std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}")))
        .expect("valid font")
}

fn painted_pixels(g: &Ghe2d) -> usize {
    g.image.pixels().filter(|p| p.0[3] != 0).count()
}

#[test]
fn arabic_renders_with_presentation_form_font() {
    let mut g = Ghe2d::new(600, 160);
    g.draw_text(font("Cairo-Regular.ttf"), "مرحبا بالعالم".to_string(), 20, 20, 60, Rgba::new(255, 255, 255, 255));
    assert!(painted_pixels(&g) > 200, "Cairo should rasterize Arabic glyphs");
}

#[test]
fn arabic_renders_with_opentype_only_font() {
    // Ruwudu ships almost no legacy presentation-form glyphs — before the rustybuzz
    // switch this produced only .notdef boxes. Real shaping must render actual glyphs.
    let mut g = Ghe2d::new(600, 160);
    g.draw_text(font("Ruwudu-Regular.ttf"), "مرحبا بالعالم".to_string(), 20, 20, 60, Rgba::new(255, 255, 255, 255));
    assert!(painted_pixels(&g) > 200, "Ruwudu should rasterize Arabic glyphs via OpenType shaping");
}

#[test]
fn shaped_advances_match_rasterization_scale() {
    // Regression guard: `rustybuzz` advances are design units and must be scaled by
    // `size / (ascent - descent)` to line up with `rusttype`'s rasterization. Getting the
    // scale basis wrong (e.g. dividing by units_per_em) spreads the glyphs far apart.
    // A connected 6-letter Arabic word must stay well under 6 ems wide.
    let mut g = Ghe2d::new(1200, 200);
    g.draw_text(font("Cairo-Regular.ttf"), "ببببب".to_string(), 20, 20, 100, Rgba::new(255, 255, 255, 255));
    let (mut min_x, mut max_x) = (u32::MAX, 0u32);
    for (x, _y, p) in g.image.enumerate_pixels() {
        if p.0[3] != 0 {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
        }
    }
    let width = max_x.saturating_sub(min_x);
    assert!(width > 60, "expected the word to actually render (got {width}px)");
    assert!(width < 5 * 100, "5 connected letters must be well under 5 ems wide (got {width}px)");
}

#[test]
fn latin_renders() {
    let mut g = Ghe2d::new(600, 120);
    g.draw_text(font("Rubik-Regular.ttf"), "Hello 123".to_string(), 20, 20, 48, Rgba::new(255, 255, 255, 255));
    assert!(painted_pixels(&g) > 100);
}

#[test]
fn text_edge_cases_do_not_panic() {
    let mut g = Ghe2d::new(64, 64);
    let f = font("Cairo-Regular.ttf");
    let w = Rgba::new(255, 255, 255, 255);
    g.draw_text(f.clone(), String::new(), 0, 0, 24, w.clone());
    g.draw_text(f.clone(), "🙂".to_string(), 0, 0, 24, w.clone());
    g.draw_text(f.clone(), "a\nب\nc".to_string(), 0, 0, 12, w.clone());
    g.draw_text(f.clone(), "سعر 250 ريال order #42".to_string(), 0, 0, 12, w.clone());
    g.draw_text(f.clone(), "x".repeat(3000), 0, 0, 8, w.clone());
    g.draw_text(f.clone(), "نص طويل ".repeat(400), 0, 0, 8, w.clone());
    g.draw_text(f, "clipped far off".to_string(), 5000, 5000, 24, w);
}

fn rightmost_painted_x(g: &Ghe2d) -> u32 {
    let mut r = 0;
    for (x, _y, p) in g.image.enumerate_pixels() {
        if p.0[3] != 0 && x > r {
            r = x;
        }
    }
    r
}

#[test]
fn mixed_direction_line_lays_out_all_runs() {
    // Base direction is RTL (first strong char is Arabic). With a correct bidi + shaping
    // pipeline the neutral / Latin / digit runs are laid out as extra visual runs, so the
    // whole line is meaningfully wider than the Arabic run alone. The old whole-buffer
    // reverse mangled and collapsed these.
    let f = font("Cairo-Regular.ttf");
    let w = Rgba::new(255, 255, 255, 255);

    let mut only_arabic = Ghe2d::new(700, 100);
    only_arabic.draw_text(f.clone(), "عربي".to_string(), 10, 10, 40, w.clone());

    let mut mixed = Ghe2d::new(700, 100);
    mixed.draw_text(f, "عربي 12 END".to_string(), 10, 10, 40, w);

    assert!(
        rightmost_painted_x(&mixed) > rightmost_painted_x(&only_arabic) + 30,
        "the Latin/number runs must be laid out as their own visual runs, not collapsed"
    );
}
