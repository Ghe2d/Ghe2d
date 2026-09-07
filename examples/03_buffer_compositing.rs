//! 03 — buffer round-trips and compositing (sync, self-generating).
//!
//! Exercises: `encode` → `Ghe2d::from_bytes` (plain / resized / circle-cropped) and
//! `draw_image_bytes` with every `ComposeMode` (Normal / Overlay / Blend), `circle`,
//! resize, and off-canvas clipping.
//!
//! Run: `cargo run --example 03_buffer_compositing`

use std::path::PathBuf;

use ghe2d::utility::Rgba;
use ghe2d::{ComposeMode, ExportOptions, Ghe2d, Ghe2dError, ImageOptions};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn make_sprite() -> Result<Vec<u8>, Ghe2dError> {
    let mut s = Ghe2d::new(300, 300);
    // leave the border transparent so Blend / Overlay have something to show
    s.draw_circle(150, 150, 140, Rgba::new(30, 30, 40, 255));
    s.draw_circle(150, 150, 130, Rgba::new(240, 90, 90, 220));
    s.draw_circle(150, 150, 80, Rgba::new(250, 220, 120, 200));
    s.draw_rect(120, 120, 60, 60, Rgba::new(60, 120, 240, 255));
    s.encode(ExportOptions::png())
}

fn make_photo() -> Result<Vec<u8>, Ghe2dError> {
    let mut p = Ghe2d::new(400, 400);
    for y in 0..400u32 {
        let t = y as f32 / 400.0;
        p.draw_rect(0, y, 400, 1, Rgba::new((t * 200.0) as u8, 40, (200.0 - t * 150.0) as u8, 255));
    }
    p.encode(ExportOptions::png())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = out_dir();
    let p = |name: &str| o.join(name).to_str().unwrap().to_owned();

    let sprite = make_sprite()?;
    let photo = make_photo()?;

    // from_bytes: plain / resized / circle-cropped
    let a = Ghe2d::from_bytes(photo.clone(), ImageOptions::new())?;
    println!("loaded photo: {:?} {}x{}", a.source_format(), a.width(), a.height());
    a.save(&p("03_a_plain.png"), ExportOptions::png())?;
    Ghe2d::from_bytes(photo.clone(), ImageOptions::new().size(800, 600))?
        .save(&p("03_b_resized_800x600.png"), ExportOptions::png())?;
    Ghe2d::from_bytes(sprite.clone(), ImageOptions::new().size(400, 400).circle(true))?
        .save(&p("03_c_circle.png"), ExportOptions::png())?;

    // compositing onto a fresh base
    let mut base = Ghe2d::new(900, 600);
    for y in 0..600u32 {
        base.draw_rect(0, y, 900, 1, Rgba::new(16, 18, 24, 255));
    }
    base.draw_image_bytes(sprite.clone(), ImageOptions::new().at(40, 40).size(200, 200))?; // Normal
    base.draw_image_bytes(
        sprite.clone(),
        ImageOptions::new().at(280, 60).size(200, 200).circle(true).compose(ComposeMode::Overlay),
    )?;
    base.draw_image_bytes(
        sprite.clone(),
        ImageOptions::new().at(520, 60).size(200, 200).compose(ComposeMode::Blend),
    )?;
    base.draw_image_bytes(
        sprite.clone(),
        ImageOptions::new().at(740, 60).size(200, 200).circle(true).compose(ComposeMode::Blend),
    )?;
    base.draw_image_bytes(
        sprite,
        ImageOptions::new().at(820, 500).size(200, 200).compose(ComposeMode::Overlay),
    )?; // off-canvas → clipped

    base.save(&p("03_base_composite.png"), ExportOptions::png())?;
    base.save(&p("03_base_composite.webp"), ExportOptions::webp(85.0))?;

    println!("wrote 03_* files to {}", o.display());
    Ok(())
}
