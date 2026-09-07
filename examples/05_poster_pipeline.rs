//! 05 — end-to-end poster: the "real" pipeline.
//!
//! Exercises: full composition (gradient + decoration + framed border), an in-memory logo
//! sprite composited via `draw_image_bytes` (`ImageOptions::circle` + `ComposeMode::Overlay`),
//! multi-font `draw_text`, `save` / `encode` with `ExportOptions` (PNG / JPEG / WebP), and
//! a WebP quality sweep printing the size/quality trade-off.
//!
//! Run: `cargo run --example 05_poster_pipeline`

use std::fs;
use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{ExportOptions, Ghe2d, Ghe2dError, ImageOptions, TextAlign, TextOptions};

fn asset(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
}
fn out_dir() -> PathBuf {
    let d = asset("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn make_logo() -> Result<Vec<u8>, Ghe2dError> {
    let mut l = Ghe2d::new(400, 400);
    l.draw_rect(0, 0, 400, 400, Rgba::new(255, 255, 255, 255));
    l.draw_circle(200, 200, 190, Rgba::new(24, 118, 210, 255));
    l.draw_circle(200, 200, 120, Rgba::new(255, 255, 255, 255));
    l.draw_circle(200, 200, 60, Rgba::new(255, 176, 32, 255));
    l.encode(ExportOptions::png())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const W: u32 = 1080;
    const H: u32 = 1350;
    let o = out_dir();
    let p = |name: &str| o.join(name).to_str().unwrap().to_owned();

    let fp = |n: &str| asset(&format!("examples/assets/fonts/{n}")).to_string_lossy().into_owned();
    let title_font = LoadFont::new(fp("Ruwudu-Regular.ttf"))?;
    let subtitle_font = LoadFont::new(fp("Cairo-Regular.ttf"))?;
    let footer_font = LoadFont::new(fp("Rubik-Regular.ttf"))?;

    let mut g = Ghe2d::new(W, H);

    // gradient
    for y in 0..H {
        let t = y as f32 / H as f32;
        g.draw_rect(0, y, W, 1, Rgba::new((12.0 + t * 20.0) as u8, (16.0 + t * 30.0) as u8, (40.0 + t * 70.0) as u8, 255));
    }
    // scattered decoration circles
    for i in 0..40u32 {
        let x = (i * 137) % W;
        let y = (i * 91) % H;
        g.draw_circle(x, y, 6 + (i % 5) * 6, Rgba::new(255, 255, 255, 18));
    }
    // framed border
    let frame = Rgba::new(220, 180, 90, 255);
    g.draw_rect(40, 40, W - 80, 6, frame);
    g.draw_rect(40, H - 46, W - 80, 6, frame);
    g.draw_rect(40, 40, 6, H - 80, frame);
    g.draw_rect(W - 46, 40, 6, H - 80, frame);

    // circle-cropped logo
    g.draw_image_bytes(
        make_logo()?,
        ImageOptions::new().at((W - 360) / 2, 150).size(360, 360).circle(true).compose(ghe2d::ComposeMode::Overlay),
    )?;

    // text — everything centered inside the framed content box [46, W-46]
    let content_x = 46.0;
    let content_w = (W - 92) as f32;
    g.draw_text(
        TextOptions::new(title_font, "منصة الصور")
            .at(content_x, 600.0).size(110.0).color(Rgba::new(245, 245, 250, 255))
            .width(content_w).align(TextAlign::Center),
    );
    g.draw_text(
        TextOptions::new(subtitle_font.clone(), "توليد صور احترافية — الإصدار v0.2.0")
            .at(content_x, 780.0).size(40.0).color(Rgba::new(210, 210, 230, 255))
            .width(content_w).align(TextAlign::Center),
    );
    g.draw_text(
        TextOptions::new(footer_font, "generated with ghe2d • 2026")
            .at(content_x, (H - 120) as f32).size(30.0).color(Rgba::new(180, 190, 210, 255))
            .width(content_w).align(TextAlign::Center),
    );
    // long tagline: wrapped inside the box instead of bleeding off the edge
    g.draw_text(
        TextOptions::new(subtitle_font, "سطر طويل يُلَفّ تلقائياً داخل حدود البوستر بدلاً من أن يتجاوز الحافة اليمنى")
            .at(content_x, 940.0).size(52.0).color(Rgba::new(255, 130, 130, 230))
            .width(content_w).wrap(true).align(TextAlign::Center),
    );

    g.save(&p("05_poster.png"), ExportOptions::png())?;
    g.save(&p("05_poster.webp"), ExportOptions::webp(80.0))?;
    g.save(&p("05_poster.jpg"), ExportOptions::jpeg(85))?;

    println!("webp quality sweep:");
    for q in [10.0f32, 40.0, 70.0, 100.0] {
        let bytes = g.encode(ExportOptions::webp(q))?;
        let name = format!("05_poster_q{}.webp", q as u32);
        fs::write(o.join(&name), &bytes)?;
        println!("  q={:>3} -> {:>8} bytes  ({name})", q as u32, bytes.len());
    }
    println!("wrote 05_* files to {}", o.display());
    Ok(())
}
