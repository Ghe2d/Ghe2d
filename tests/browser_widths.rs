//! Text widths against a browser's real layout.
//!
//! The expected widths were measured in Chrome as the width of an inline-block `<span>` with
//! `white-space: pre` (the DOM layout that is actually painted), with the same four font
//! files that ship in `examples/assets/fonts`, at the CSS font-size listed per font.
//! ghe2d draws at `LoadFont::css_to_size(css_px)`. (`canvas.measureText` is not the reference:
//! it shapes word by word, so it ignores kerning across spaces and the script-dependent space
//! glyph some Arabic fonts use.)

use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::text_width;

fn font(name: &str) -> LoadFont {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/assets/fonts").join(name);
    LoadFont::from_bytes(std::fs::read(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))).expect("valid font")
}

const SENTENCES: [&str; 7] = [
    "بسم الله الرحمن الرحيم",
    "تم شحن رصيدك بنجاح، شكرا لثقتك بنا",
    "السعر النهائي ١٢٠ ريال شامل الضريبة",
    "The quick brown fox jumps over the lazy dog",
    "Total: 129.99 SAR (VAT included)",
    "WAVE Typography AVATAR To Ty",
    "Mixed عربي and English معا 123",
];

/// (file, CSS font-size in px, browser width of each sentence in px)
const CASES: [(&str, f32, [f32; 7]); 4] = [
    ("Cairo-Regular.ttf", 40.0, [355.77, 582.48, 575.47, 742.73, 543.89, 543.05, 542.44]),
    ("Tajawal-Bold.ttf", 32.0, [321.45, 539.02, 517.81, 619.47, 438.28, 438.28, 435.64]),
    ("Rubik-Regular.ttf", 36.0, [344.78, 555.27, 519.14, 740.39, 531.98, 531.05, 505.56]),
    ("Ruwudu-Regular.ttf", 48.0, [349.92, 582.25, 561.52, 892.14, 688.81, 733.78, 612.94]),
];

#[test]
fn widths_match_browser_within_one_percent() {
    for (file, css, widths) in CASES {
        let f = font(file);
        for (s, want) in SENTENCES.iter().zip(widths) {
            let got = text_width(&f, &[], s, f.css_to_size(css));
            let ratio = got / want;
            assert!(
                (ratio - 1.0).abs() <= 0.01,
                "{file} @{css}px {s:?}: ghe2d {got:.2} vs browser {want:.2} (ratio {ratio:.4})"
            );
        }
    }
}

#[test]
fn css_size_and_conversion_agree() {
    let f = font("Ruwudu-Regular.ttf");
    let s = f.css_to_size(48.0);
    assert!((f.size_to_css(s) - 48.0).abs() < 1e-3);
    let a = ghe2d::TextOptions::new(f.clone(), "x").css_size(48.0);
    let b = ghe2d::TextOptions::new(f.clone(), "x").size(s);
    assert_eq!(ghe2d::measure_text(&a).size, ghe2d::measure_text(&b).size);
    // Ruwudu: ascent - descent is about 1.608 em.
    assert!((s / 48.0 - 1.608).abs() < 0.01, "ratio {}", s / 48.0);
}

#[test]
fn space_takes_the_script_of_the_preceding_text() {
    // Ruwudu has a narrower Arabic space (via locl). A space after Arabic text must use it
    // even when the bidi algorithm puts that space in a left-to-right run.
    let f = font("Ruwudu-Regular.ttf");
    let px = f.css_to_size(48.0);
    let mixed = text_width(&f, &[], "عربي and", px);
    let with_arabic_space = text_width(&f, &[], "عربي ", px) + text_width(&f, &[], "and", px);
    let with_latin_space = text_width(&f, &[], "عربي", px) + text_width(&f, &[], " and", px);
    assert!((mixed - with_arabic_space).abs() < 0.5, "{mixed} vs {with_arabic_space}");
    assert!((mixed - with_latin_space).abs() > 5.0, "fixture font should have two space widths");
}
