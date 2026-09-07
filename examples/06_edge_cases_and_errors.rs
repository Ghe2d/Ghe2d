//! 06 — verification harness: every hardened path returns an error, never a panic/abort.
//!
//! Exercises every `Ghe2dError` variant, the shape/text clipping guards, the WebP
//! dimension guard (which previously aborted the process), and the decode guards.
//!
//! Run: `cargo run --example 06_edge_cases_and_errors`

use std::path::Path;

use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::{ErrorMessage, Ghe2d, Ghe2dError};

fn check(label: &str, ok: bool) {
    println!("{} {label}", if ok { "OK  " } else { "FAIL" });
    assert!(ok, "case failed: {label}");
}

fn tiny_valid_png() -> Vec<u8> {
    let mut g = Ghe2d::new(24, 24);
    g.draw_rect(0, 0, 24, 24, Rgba::new(10, 20, 30, 255));
    g.to_png_buffer(CompressionType::Fast, FilterType::NoFilter).unwrap()
}

fn main() {
    // --- encoder guards -------------------------------------------------------
    check(
        "new(0,0).to_webp_buffer -> WebpEncode",
        matches!(Ghe2d::new(0, 0).to_webp_buffer(70.0), Err(Ghe2dError::WebpEncode)),
    );
    check(
        "new(0,0).to_png_buffer -> PngEncode",
        matches!(
            Ghe2d::new(0, 0).to_png_buffer(CompressionType::Fast, FilterType::NoFilter),
            Err(Ghe2dError::PngEncode)
        ),
    );
    check(
        "new(20000,4).to_webp_buffer -> ImageTooLarge (previously aborted the process)",
        matches!(Ghe2d::new(20_000, 4).to_webp_buffer(70.0), Err(Ghe2dError::ImageTooLarge)),
    );
    check(
        "new(4,20000).to_webp_buffer -> ImageTooLarge",
        matches!(Ghe2d::new(4, 20_000).to_webp_buffer(70.0), Err(Ghe2dError::ImageTooLarge)),
    );

    // --- shape clipping on a tiny canvas (must not panic) --------------------
    {
        let mut g = Ghe2d::new(8, 8);
        let c = Rgba::new(255, 255, 255, 255);
        g.draw_rect(100, 100, 9999, 9999, c.clone());
        g.draw_rect(4, 4, 9999, 9999, c.clone());
        g.draw_circle(0, 0, 5000, c.clone());
        g.draw_circle(4, 4, 0, c.clone());
        g.draw_circle(9999, 9999, 3, c.clone());
        check(
            "tiny-canvas shape clipping -> to_png_buffer Ok",
            g.to_png_buffer(CompressionType::Fast, FilterType::NoFilter).is_ok(),
        );
    }

    // --- text edge cases on a tiny canvas (must not panic) ------------------
    {
        let mut g = Ghe2d::new(8, 8);
        let font_bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/assets/fonts/Cairo-Regular.ttf"),
        )
        .expect("bundled font present");
        let font = ghe2d::font::LoadFont::from_bytes(font_bytes).expect("valid font");
        let w = Rgba::new(255, 255, 255, 255);
        g.draw_text(font.clone(), String::new(), 0, 0, 12, w.clone()); // empty
        g.draw_text(font.clone(), "🙂 اختبار".to_string(), 0, 0, 12, w.clone()); // emoji + RTL
        g.draw_text(font.clone(), "zero".to_string(), 0, 0, 0, w.clone()); // size 0
        g.draw_text(font.clone(), "x".repeat(4000), 0, 0, 8, w.clone()); // very long LTR
        g.draw_text(font.clone(), "نص طويل ".repeat(500), 0, 0, 8, w.clone()); // very long RTL
        check("tiny-canvas text edge cases -> no panic", true);
    }

    // --- font loading ------------------------------------------------------
    check(
        "LoadFont::new(missing) -> Io",
        matches!(
            ghe2d::font::LoadFont::new("does/not/exist-xyz.ttf".to_string()),
            Err(Ghe2dError::Io(_))
        ),
    );
    check(
        "LoadFont::from_bytes(garbage) -> FontLoad",
        matches!(
            ghe2d::font::LoadFont::from_bytes(vec![1, 2, 3, 4]),
            Err(Ghe2dError::FontLoad)
        ),
    );

    // --- buffer decode guards --------------------------------------------
    check(
        "new_by_load_from_buffer(empty) -> Image(BufferEmpty)",
        matches!(
            Ghe2d::new_by_load_from_buffer(Vec::new(), None, None, false),
            Err(Ghe2dError::Image(ErrorMessage::BufferEmpty))
        ),
    );
    check(
        "new_by_load_from_buffer(non-image) -> Image(FailedDecodeImage)",
        matches!(
            Ghe2d::new_by_load_from_buffer(b"not an image at all".to_vec(), None, None, false),
            Err(Ghe2dError::Image(ErrorMessage::FailedDecodeImage))
        ),
    );

    // --- zero-size resize is handled downstream, not a panic -------------
    {
        let png = tiny_valid_png();
        let zero = Ghe2d::new_by_load_from_buffer(png.clone(), Some(0), Some(0), false)
            .expect("0-size resize still constructs a Ghe2d");
        check(
            "resize to 0x0 -> to_webp_buffer Err (not panic)",
            zero.to_webp_buffer(70.0).is_err(),
        );
        check(
            "resize to 64x64 circle -> Ok",
            Ghe2d::new_by_load_from_buffer(png, Some(64), Some(64), true).is_ok(),
        );
    }

    println!("\nall edge cases handled without a panic or abort.");
}
