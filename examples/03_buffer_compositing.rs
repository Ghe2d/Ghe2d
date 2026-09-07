//! 03 — buffer round-trips and compositing (sync, self-generating).
//!
//! Exercises: `to_png_buffer`, `new_by_load_from_buffer` (plain / resized / circle-cropped),
//! `from_buffer_to_normal` and `from_buffer_to_overlay` (plain / circle / resized /
//! off-canvas → clipped), and the `is_circle` flag.
//!
//! Run: `cargo run --example 03_buffer_compositing`

use std::fs;
use std::path::PathBuf;

use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::{Ghe2d, Ghe2dError};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn make_sprite() -> Result<Vec<u8>, Ghe2dError> {
    let mut s = Ghe2d::new(300, 300);
    s.draw_rect(0, 0, 300, 300, Rgba::new(30, 30, 40, 255));
    s.draw_circle(150, 150, 130, Rgba::new(240, 90, 90, 255));
    s.draw_circle(150, 150, 80, Rgba::new(250, 220, 120, 255));
    s.draw_rect(120, 120, 60, 60, Rgba::new(60, 120, 240, 255));
    s.to_png_buffer(CompressionType::Default, FilterType::Adaptive)
}

fn make_photo() -> Result<Vec<u8>, Ghe2dError> {
    let mut p = Ghe2d::new(400, 400);
    for y in 0..400u32 {
        let t = y as f32 / 400.0;
        p.draw_rect(0, y, 400, 1, Rgba::new((t * 200.0) as u8, 40, (200.0 - t * 150.0) as u8, 255));
    }
    p.to_png_buffer(CompressionType::Fast, FilterType::NoFilter)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = out_dir();
    let p = |name: &str| o.join(name).to_str().unwrap().to_owned();

    let sprite = make_sprite()?;
    let photo = make_photo()?;

    // new_by_load_from_buffer: plain / resized / circle-cropped
    Ghe2d::new_by_load_from_buffer(photo.clone(), None, None, false)?.save(&p("03_a_plain.png"))?;
    Ghe2d::new_by_load_from_buffer(photo.clone(), Some(800), Some(600), false)?
        .save(&p("03_b_resized_800x600.png"))?;
    Ghe2d::new_by_load_from_buffer(sprite.clone(), Some(400), Some(400), true)?
        .save(&p("03_c_circle.png"))?;

    // compositing onto a fresh base
    let mut base = Ghe2d::new(900, 600);
    for y in 0..600u32 {
        base.draw_rect(0, y, 900, 1, Rgba::new(16, 18, 24, 255));
    }
    base.from_buffer_to_normal(sprite.clone(), 50, 50, Some(200), Some(200), false)?;
    base.from_buffer_to_normal(sprite.clone(), 300, 60, Some(180), Some(180), true)?; // circle + resize
    base.from_buffer_to_overlay(sprite.clone(), 520, 200, None, None, true)?; // circle, native size
    base.from_buffer_to_overlay(sprite.clone(), 820, 520, Some(160), Some(160), false)?; // off-canvas → clip

    base.save_with_png(&p("03_base_composite.png"), CompressionType::Default, FilterType::Adaptive)?;
    base.save_with_webp(&p("03_base_composite.webp"), 85.0)?;

    println!("wrote 03_* files to {}", o.display());
    Ok(())
}
