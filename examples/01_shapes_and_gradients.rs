//! 01 — procedural shapes, gradients, alpha blending, and clipping.
//!
//! Exercises: `Ghe2d::new`, `draw_rect`, `draw_circle` (incl. `radius == 0` and shapes
//! pushed past every edge/corner), `utility::Rgba` with alpha, `width`/`height`,
//! `encode` / `save` with `ExportOptions` (PNG / WebP).
//!
//! Run: `cargo run --example 01_shapes_and_gradients`

use std::fs;
use std::path::PathBuf;

use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::{ExportOptions, Ghe2d};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const W: u32 = 1200;
    const H: u32 = 800;
    let mut g = Ghe2d::new(W, H);

    // Vertical gradient background: one 1px-tall rect per row.
    for y in 0..H {
        let t = y as f32 / H as f32;
        let r = (20.0 + t * 60.0) as u8;
        let gg = (30.0 + t * 120.0) as u8;
        let b = (60.0 + t * 180.0) as u8;
        g.draw_rect(0, y, W, 1, Rgba::new(r, gg, b, 255));
    }

    // Concentric translucent circles — stacks alpha blending.
    for i in (0..14u32).rev() {
        g.draw_circle(W / 2, H / 2, 60 + i * 34, Rgba::new(255, 255, 255, 22));
    }

    // A grid of solid squares.
    for gx in 0..10u32 {
        for gy in 0..6u32 {
            let c = Rgba::new(
                (gx * 25) as u8,
                (gy * 40) as u8,
                200u8.saturating_sub((gx * 15) as u8),
                235,
            );
            g.draw_rect(60 + gx * 110, 60 + gy * 110, 70, 70, c);
        }
    }

    // --- clipping: every shape below is partly or fully outside the canvas ---
    g.draw_rect(W - 40, H - 40, 400, 400, Rgba::new(255, 80, 80, 255)); // bleeds off corner
    g.draw_rect(1_000_000, 10, 50, 50, Rgba::new(0, 255, 0, 255)); // origin fully outside
    g.draw_circle(0, 0, 180, Rgba::new(255, 220, 0, 160)); // centred on top-left corner
    g.draw_circle(W, H, 220, Rgba::new(0, 255, 255, 160)); // centre outside bottom-right
    g.draw_circle(W / 2, 40, 5000, Rgba::new(255, 0, 255, 10)); // radius >> canvas (faint wash)
    g.draw_circle(300, 300, 0, Rgba::new(255, 255, 255, 255)); // radius 0 -> no-op

    println!("canvas: {}x{}", g.width(), g.height());

    let o = out_dir();
    let p = |name: &str| o.join(name).to_str().unwrap().to_owned();

    g.save(&p("01_shapes.png"), ExportOptions::png())?;
    g.save(
        &p("01_shapes.best_paeth.png"),
        ExportOptions::png().compression(CompressionType::Best).filter(FilterType::Paeth),
    )?;
    g.save(&p("01_shapes.q90.webp"), ExportOptions::webp(90.0))?;

    fs::write(
        o.join("01_shapes.frombuffer.png"),
        g.encode(ExportOptions::png().compression(CompressionType::Fast).filter(FilterType::NoFilter))?,
    )?;
    fs::write(o.join("01_shapes.frombuffer.q75.webp"), g.encode(ExportOptions::webp(75.0))?)?;

    println!("wrote 5 files to {}", o.display());
    Ok(())
}
