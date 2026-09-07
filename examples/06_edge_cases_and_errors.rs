//! 06 — verification harness: every hardened path returns an error, never a panic/abort.
//!
//! Exercises every `Ghe2dError` variant, the shape/text clipping guards, the WebP
//! dimension guard (which previously aborted the process), the decode guards, and the
//! new `ExportOptions` / `ImageOptions` / `detect_format` surface.
//!
//! Run: `cargo run --example 06_edge_cases_and_errors`

use std::path::Path;

use ghe2d::utility::Rgba;
use ghe2d::{
    ErrorMessage, ExportOptions, Ghe2d, Ghe2dError, ImageFormat, ImageOptions, LineLayout,
    TextAlign, TextOptions,
};

fn check(label: &str, ok: bool) {
    println!("{} {label}", if ok { "OK  " } else { "FAIL" });
    assert!(ok, "case failed: {label}");
}

fn tiny_valid_png() -> Vec<u8> {
    let mut g = Ghe2d::new(24, 24);
    g.draw_rect(0, 0, 24, 24, Rgba::new(10, 20, 30, 255));
    g.encode(ExportOptions::png()).unwrap()
}

fn main() {
    // --- encoder guards -------------------------------------------------------
    check(
        "new(0,0).encode(webp) -> WebpEncode",
        matches!(Ghe2d::new(0, 0).encode(ExportOptions::webp(70.0)), Err(Ghe2dError::WebpEncode)),
    );
    check(
        "new(0,0).encode(png) -> PngEncode",
        matches!(Ghe2d::new(0, 0).encode(ExportOptions::png()), Err(Ghe2dError::PngEncode)),
    );
    check(
        "new(0,0).encode(jpeg) -> JpegEncode",
        matches!(Ghe2d::new(0, 0).encode(ExportOptions::jpeg(80)), Err(Ghe2dError::JpegEncode)),
    );
    check(
        "new(20000,4).encode(webp) -> ImageTooLarge (previously aborted the process)",
        matches!(Ghe2d::new(20_000, 4).encode(ExportOptions::webp(70.0)), Err(Ghe2dError::ImageTooLarge)),
    );
    check(
        "new(4,20000).encode(webp) -> ImageTooLarge",
        matches!(Ghe2d::new(4, 20_000).encode(ExportOptions::webp(70.0)), Err(Ghe2dError::ImageTooLarge)),
    );

    // --- format detection ---------------------------------------------------
    let png = tiny_valid_png();
    check("detect_format(png) -> Png", matches!(Ghe2d::detect_format(&png), Some(ImageFormat::Png)));
    check("detect_format(garbage) -> None", Ghe2d::detect_format(b"nope").is_none());
    check(
        "from_bytes(png).source_format() -> Png",
        matches!(
            Ghe2d::from_bytes(png.clone(), ImageOptions::new()).map(|g| g.source_format()),
            Ok(Some(ImageFormat::Png))
        ),
    );

    // --- shape clipping on a tiny canvas (must not panic) --------------------
    {
        let mut g = Ghe2d::new(8, 8);
        let c = Rgba::new(255, 255, 255, 255);
        g.draw_rect(100, 100, 9999, 9999, c);
        g.draw_rect(4, 4, 9999, 9999, c);
        g.draw_circle(0, 0, 5000, c);
        g.draw_circle(4, 4, 0, c);
        g.draw_circle(9999, 9999, 3, c);
        check("tiny-canvas shape clipping -> encode(png) Ok", g.encode(ExportOptions::png()).is_ok());
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
        let text = |s: String| TextOptions::new(font.clone(), s).color(w);
        g.draw_text(text(String::new()).size(12.0));
        g.draw_text(text("🙂 اختبار".into()).size(12.0));
        g.draw_text(text("zero".into()).size(0.0));
        g.draw_text(text("x".repeat(4000)).size(8.0));
        g.draw_text(text("نص طويل ".repeat(500)).size(8.0));
        g.draw_text(text("absurd width".into()).size(40.0).width(1.0));
        g.draw_text(text("absurd height".into()).size(40.0).height(1.0));
        g.draw_text(text("supercalifragilisticexpialidocious".into()).size(40.0).width(15.0).wrap(true));
        g.draw_text(text("no height".into()).size(20.0).line_layout(LineLayout::SpaceBetween));
        g.draw_text(text("no height".into()).size(20.0).line_layout(LineLayout::Middle(6.0)));
        g.draw_text(text("a\nb\nc".into()).size(10.0).height(4.0).line_layout(LineLayout::SpaceEvenly));
        g.draw_text(text("align".into()).size(12.0).width(60.0).align(TextAlign::Center));
        check("tiny-canvas text edge cases -> no panic", true);
    }

    // --- font loading ------------------------------------------------------
    check(
        "LoadFont::new(missing) -> Io",
        matches!(ghe2d::font::LoadFont::new("does/not/exist-xyz.ttf".to_string()), Err(Ghe2dError::Io(_))),
    );
    check(
        "LoadFont::from_bytes(garbage) -> FontLoad",
        matches!(ghe2d::font::LoadFont::from_bytes(vec![1, 2, 3, 4]), Err(Ghe2dError::FontLoad)),
    );

    // --- buffer decode guards --------------------------------------------
    check(
        "from_bytes(empty) -> Image(BufferEmpty)",
        matches!(
            Ghe2d::from_bytes(Vec::new(), ImageOptions::new()),
            Err(Ghe2dError::Image(ErrorMessage::BufferEmpty))
        ),
    );
    check(
        "from_bytes(non-image) -> Image(FailedDecodeImage)",
        matches!(
            Ghe2d::from_bytes(b"not an image at all".to_vec(), ImageOptions::new()),
            Err(Ghe2dError::Image(ErrorMessage::FailedDecodeImage))
        ),
    );

    // --- zero-size resize is handled downstream, not a panic -------------
    {
        let zero = Ghe2d::from_bytes(png.clone(), ImageOptions::new().size(0, 0))
            .expect("0-size resize still constructs a Ghe2d");
        check("resize to 0x0 -> encode(webp) Err (not panic)", zero.encode(ExportOptions::webp(70.0)).is_err());
        check(
            "resize to 64x64 circle -> Ok",
            Ghe2d::from_bytes(png, ImageOptions::new().size(64, 64).circle(true)).is_ok(),
        );
    }

    println!("\nall edge cases handled without a panic or abort.");
}
