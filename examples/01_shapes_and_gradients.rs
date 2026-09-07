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
use ghe2d::{CircleOptions, ExportOptions, Ghe2d, RectOptions};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const W: u32 = 1200;
    const H: u32 = 800;
    let mut g = Ghe2d::new(W, H);

    let wf = W as f32;
    let hf = H as f32;

    // Vertical gradient background: one 1px-tall rect per row.
    for y in 0..H {
        let t = y as f32 / H as f32;
        let r = (20.0 + t * 60.0) as u8;
        let gg = (30.0 + t * 120.0) as u8;
        let b = (60.0 + t * 180.0) as u8;
        g.draw_rect(RectOptions::solid(0.0, y as f32, wf, 1.0, Rgba::rgb(r, gg, b)));
    }

    // Concentric translucent circles — stacks alpha blending.
    for i in (0..14u32).rev() {
        g.draw_circle(CircleOptions::solid(wf / 2.0, hf / 2.0, (60 + i * 34) as f32, Rgba::new(255, 255, 255, 22)));
    }

    // A grid of solid squares.
    for gx in 0..10u32 {
        for gy in 0..6u32 {
            let c = Rgba::new((gx * 25) as u8, (gy * 40) as u8, 200u8.saturating_sub((gx * 15) as u8), 235);
            g.draw_rect(RectOptions::solid((60 + gx * 110) as f32, (60 + gy * 110) as f32, 70.0, 70.0, c));
        }
    }

    // A row of the new shapes: triangle, pentagon, hexagon, star, rhombus, rotated square.
    g.draw_polygon(ghe2d::PolygonOptions::new(48.0).at(120.0, 700.0).fill(Rgba::rgb(90, 200, 255)).stroke(Rgba::WHITE, 3.0));
    g.draw_polygon(ghe2d::PolygonOptions::new(48.0).at(280.0, 700.0).sides(5).fill(Rgba::rgb(255, 170, 90)));
    g.draw_polygon(ghe2d::PolygonOptions::new(48.0).at(440.0, 700.0).sides(6).rotation(15.0).fill(Rgba::rgb(160, 255, 160)));
    g.draw_star(ghe2d::StarOptions::new(50.0).at(600.0, 700.0).points(6).fill(Rgba::rgb(255, 220, 120)));
    g.draw_rhombus(ghe2d::RhombusOptions::new(90.0, 60.0).at(760.0, 700.0).fill(Rgba::rgb(230, 130, 200)));
    g.draw_rect(RectOptions::new(70.0, 70.0).at(885.0, 665.0).rotation(30.0).fill(Rgba::rgb(120, 160, 255)));
    g.draw_line(ghe2d::LineOptions::new(60.0, 760.0, 1000.0, 760.0).color(Rgba::WHITE).width(2.0));

    // --- clipping: every shape below is partly or fully outside the canvas ---
    g.draw_rect(RectOptions::solid(wf - 40.0, hf - 40.0, 400.0, 400.0, Rgba::new(255, 80, 80, 255)));
    g.draw_rect(RectOptions::solid(1_000_000.0, 10.0, 50.0, 50.0, Rgba::rgb(0, 255, 0)));
    g.draw_circle(CircleOptions::solid(0.0, 0.0, 180.0, Rgba::new(255, 220, 0, 160)));
    g.draw_circle(CircleOptions::solid(wf, hf, 220.0, Rgba::new(0, 255, 255, 160)));
    g.draw_circle(CircleOptions::solid(wf / 2.0, 40.0, 5000.0, Rgba::new(255, 0, 255, 10)));
    g.draw_circle(CircleOptions::solid(300.0, 300.0, 0.0, Rgba::WHITE)); // radius 0 -> no-op

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
