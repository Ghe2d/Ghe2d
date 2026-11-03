//! Font chain (fallback), weight/italic face selection, variable fonts, text measuring.
use ghe2d::font::{FontFamily, LoadFont};
use ghe2d::utility::Rgba;
use ghe2d::{measure_text, text_width, Ghe2d, TextAlign, TextOptions};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/fonts/");
// Cairo variable font (wght 200..1000, slnt), SIL Open Font License 1.1.
const CAIRO_VAR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Cairo-Variable.ttf");

fn load(p: &str) -> LoadFont {
    LoadFont::new(p.to_string()).expect("font loads")
}
fn ex(name: &str) -> LoadFont {
    load(&format!("{DIR}{name}"))
}

fn canvas() -> Ghe2d {
    let mut g = Ghe2d::new(500, 120);
    for p in g.image.pixels_mut() {
        *p = image::Rgba([255, 255, 255, 255]);
    }
    g
}

/// Sum of ink (darkness) and the horizontal ink extent `(min_x, max_x)`.
fn ink(g: &Ghe2d) -> (u64, Option<(u32, u32)>) {
    let mut sum = 0u64;
    let mut ext: Option<(u32, u32)> = None;
    for (x, _y, p) in g.image.enumerate_pixels() {
        let d = 255 - p.0[0] as u64;
        if d > 20 {
            ext = Some(match ext {
                None => (x, x),
                Some((a, b)) => (a.min(x), b.max(x)),
            });
        }
        sum += d;
    }
    (sum, ext)
}

fn black() -> Rgba {
    Rgba::new(0, 0, 0, 255)
}

/// A character `missing` lacks but `has` covers (Hebrew / Cyrillic / Greek candidates).
fn probe_char(missing: &LoadFont, has: &LoadFont) -> char {
    "אבגЖЯдλπ".chars().find(|c| !missing.has_glyph(*c) && has.has_glyph(*c)).expect("probe char")
}

#[test]
fn fallback_draws_missing_characters() {
    let cairo = ex("Cairo-Regular.ttf");
    let rubik = ex("Rubik-Regular.ttf");
    let c = probe_char(&cairo, &rubik);
    let text = format!("AB {c}{c}{c}");

    let mut plain = canvas();
    plain.draw_text(TextOptions::new(cairo.clone(), &text).at(5.0, 20.0).size(48.0).color(black()));
    let mut chained = canvas();
    chained.draw_text(
        TextOptions::new(cairo.clone(), &text).fallback(rubik.clone()).at(5.0, 20.0).size(48.0).color(black()),
    );
    assert!(ink(&plain).0 > 0 && ink(&chained).0 > 0);
    assert_ne!(plain.image.as_raw(), chained.image.as_raw(), "fallback changes the output");

    // The chain measures the missing characters with the fallback's advances (scaled to the
    // primary's pixels per em), not with the primary's .notdef box.
    let w_chain = text_width(&cairo, std::slice::from_ref(&rubik), &c.to_string(), 40.0);
    let w_plain = text_width(&cairo, &[], &c.to_string(), 40.0);
    assert!(w_chain > 0.0 && (w_chain - w_plain).abs() > 0.5, "{w_chain} vs {w_plain}");
}

#[test]
fn missing_glyph_is_visible_not_dropped() {
    let cairo = ex("Cairo-Regular.ttf");
    let rubik = ex("Rubik-Regular.ttf");
    let c = probe_char(&cairo, &rubik);
    let mut g = canvas();
    g.draw_text(TextOptions::new(cairo, c.to_string()).at(5.0, 20.0).size(60.0).color(black()));
    assert!(ink(&g).0 > 0, "notdef drawn when no font in the chain has the character");
}

#[test]
fn variable_weight_changes_width_and_ink() {
    let var = load(CAIRO_VAR);
    assert!(var.is_variable());
    let (min, _def, max) = var.weight_axis().unwrap();
    assert!(min <= 200.0 && max >= 900.0, "{min}..{max}");

    let light = var.with_weight(200.0);
    let heavy = var.with_weight(900.0);
    assert_eq!(var.with_weight(5000.0).weight(), max, "clamped to the axis");

    let t = "Hello مرحبا";
    let wl = text_width(&light, &[], t, 40.0);
    let wh = text_width(&heavy, &[], t, 40.0);
    assert!(wh > wl + 1.0, "heavy wider: {wl} vs {wh}");

    let mut a = canvas();
    a.draw_text(TextOptions::new(var.clone(), t).at(5.0, 20.0).size(40.0).color(black()).weight(200.0));
    let mut b = canvas();
    b.draw_text(TextOptions::new(var.clone(), t).at(5.0, 20.0).size(40.0).color(black()).weight(900.0));
    let (ia, _) = ink(&a);
    let (ib, _) = ink(&b);
    assert!(ib as f64 > ia as f64 * 1.3, "heavy has clearly more ink: {ia} vs {ib}");
}

#[test]
fn weight_builder_matches_with_weight() {
    let var = load(CAIRO_VAR);
    let t = "مرحبا";
    let o = TextOptions::new(var.clone(), t).size(40.0).weight(700.0);
    let m = measure_text(&o);
    let w = text_width(&var.with_weight(700.0), &[], t, 40.0);
    assert!((m.width - w).abs() < 0.01);
}

#[test]
fn family_picks_by_weight_and_italic() {
    let mut fam = FontFamily::new();
    fam.add(ex("Rubik-Regular.ttf")).add(ex("Tajawal-Bold.ttf"));
    let p = fam.pick(700.0, false).unwrap();
    assert!(p.weight() >= 650.0, "bold face chosen, got {}", p.weight());
    let p = fam.pick(300.0, false).unwrap();
    assert!(p.weight() <= 450.0, "regular face chosen, got {}", p.weight());
    // No italic file: falls back to upright instead of failing.
    assert!(fam.pick(400.0, true).is_some());
    assert!(FontFamily::new().pick(400.0, false).is_none());

    fam.add(load(CAIRO_VAR));
    let v = fam.pick(850.0, false).unwrap();
    assert!(v.is_variable(), "variable face covers 850 exactly");
    assert_eq!(v.weight(), 850.0);
}

#[test]
fn measure_agrees_with_drawing() {
    let rubik = ex("Rubik-Regular.ttf");
    let cairo = ex("Cairo-Regular.ttf");
    let t = "Hello مرحبا World";
    let m = measure_text(&TextOptions::new(rubik.clone(), t).fallback(cairo.clone()).size(40.0));
    assert_eq!(m.lines.len(), 1);
    let mut g = canvas();
    g.draw_text(TextOptions::new(rubik, t).fallback(cairo).at(10.0, 20.0).size(40.0).color(black()));
    let (_, ext) = ink(&g);
    let (a, b) = ext.unwrap();
    let drawn = (b - a) as f32;
    // Ink extent sits inside the advance box, short only by the side bearings.
    assert!(drawn <= m.width + 1.0 && drawn > m.width - 12.0, "ink {drawn} vs advance {}", m.width);
}

#[test]
fn measured_wrapping_is_deterministic_and_fits() {
    let cairo = ex("Cairo-Regular.ttf");
    let t = "هذا نص عربي طويل يحتاج إلى التفاف على عدة أسطر داخل صندوق ضيق";
    let o = TextOptions::new(cairo.clone(), t).size(24.0).width(200.0).wrap(true).align(TextAlign::Right);
    let m1 = measure_text(&o);
    let m2 = measure_text(&o);
    assert_eq!(m1, m2);
    assert!(m1.lines.len() > 1);
    for l in &m1.lines {
        assert!(text_width(&cairo, &[], l, m1.size) <= 200.5, "line too wide: {l}");
    }
    assert!(m1.height > 0.0 && m1.width <= 200.5);
}
