//! 04 — async loaders.
//!
//! Exercises: `Ghe2d::new_by_load_from_path` (plain / circle+resize) and
//! `load_normal_image` / `load_blend_image` / `load_overlay_image` chained on `&mut self`,
//! with `is_circle` and off-canvas (clipped) placements.
//!
//! `load_image` also accepts `http(s)://` URLs; this example uses local files it generates
//! itself so it stays deterministic and offline.
//!
//! Run: `cargo run --example 04_async_load_from_path`

use std::fs;
use std::path::PathBuf;

use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::Ghe2d;

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn make_source(w: u32, h: u32, rgba: [u8; 4]) -> Ghe2d {
    let mut g = Ghe2d::new(w, h);
    g.draw_rect(0, 0, w, h, Rgba::new(rgba[0], rgba[1], rgba[2], rgba[3]));
    g.draw_circle(w / 2, h / 2, w.min(h) / 2 - 4, Rgba::new(255, 255, 255, 90));
    g
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = out_dir();
    let p = |name: &str| o.join(name).to_str().unwrap().to_owned();

    let src_a = p("04_src_a.png");
    let src_b = p("04_src_b.png");
    make_source(360, 360, [200, 60, 60, 255]).save(&src_a)?;
    make_source(360, 360, [60, 90, 200, 255]).save(&src_b)?;

    // new_by_load_from_path: plain, and circle + resize
    Ghe2d::new_by_load_from_path(&src_a, None, None, false)
        .await?
        .save(&p("04_from_path_plain.png"))?;
    Ghe2d::new_by_load_from_path(&src_b, Some(500), Some(500), true)
        .await?
        .save(&p("04_from_path_circle_500.png"))?;

    // compose: chain the three async loaders on one canvas
    let mut c = Ghe2d::new(1000, 700);
    for y in 0..700u32 {
        c.draw_rect(0, y, 1000, 1, Rgba::new(22, 24, 30, 255));
    }
    c.load_normal_image(&src_a, 20, 20, Some(300), Some(300), false).await?;
    c.load_blend_image(&src_b, 350, 60, Some(320), Some(320), false).await?;
    c.load_overlay_image(&src_a, 700, 360, Some(280), Some(280), true).await?; // circle
    c.load_overlay_image(&src_b, 900, 620, Some(240), Some(240), false).await?; // mostly off-canvas → clip

    c.save_with_png(&p("04_async_composite.png"), CompressionType::Default, FilterType::Adaptive)?;
    c.save_with_webp(&p("04_async_composite.webp"), 85.0)?;

    println!("wrote 04_* files to {}", o.display());
    Ok(())
}
