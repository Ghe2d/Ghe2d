//! 08 — shape gallery: polygons (3..8 sides), stars, rhombus, lines, rotation,
//! and fill / stroke / fill+stroke.
//!
//! Run: `cargo run --example 08_shapes`

use std::path::PathBuf;

use ghe2d::utility::Rgba;
use ghe2d::{
    CircleOptions, ExportOptions, Ghe2d, LineOptions, PolygonOptions, RectOptions, RhombusOptions,
    StarOptions,
};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut g = Ghe2d::new(1200, 900);
    g.draw_rect(RectOptions::solid(0.0, 0.0, 1200.0, 900.0, Rgba::rgb(20, 22, 30)));

    let ink = Rgba::rgb(120, 200, 255);
    let gold = Rgba::rgb(240, 200, 110);
    let white = Rgba::WHITE;

    // Row 1: regular polygons, 3..8 sides.
    for (i, sides) in (3u32..=8).enumerate() {
        let x = 120.0 + i as f32 * 180.0;
        g.draw_polygon(PolygonOptions::new(64.0).at(x, 130.0).sides(sides).fill(ink).stroke(white, 2.0));
    }

    // Row 2: stars with 5 / 6 / 8 points and varying inner ratio.
    for (i, (pts, ratio)) in [(5u32, 0.4f32), (6, 0.5), (8, 0.55)].iter().enumerate() {
        let x = 200.0 + i as f32 * 320.0;
        g.draw_star(StarOptions::new(72.0).at(x, 320.0).points(*pts).inner_ratio(*ratio).fill(gold));
    }

    // Row 3: a rhombus, a rotation sweep of one triangle, a circle with a ring.
    g.draw_rhombus(RhombusOptions::new(150.0, 90.0).at(160.0, 520.0).fill(Rgba::rgb(230, 130, 200)).stroke(white, 3.0));
    for k in 0..8 {
        let deg = k as f32 * 45.0;
        g.draw_polygon(PolygonOptions::new(46.0).at(460.0 + k as f32 * 78.0, 520.0).rotation(deg).fill(ink));
    }
    g.draw_circle(CircleOptions::new(60.0).at(1090.0, 520.0).fill(Rgba::new(120, 200, 255, 60)).stroke(white, 6.0));

    // Row 4: fill only / stroke only / fill+stroke of the same hexagon.
    g.draw_polygon(PolygonOptions::new(60.0).at(180.0, 730.0).sides(6).fill(gold));
    g.draw_polygon(PolygonOptions::new(60.0).at(400.0, 730.0).sides(6).no_fill().stroke(gold, 5.0));
    g.draw_polygon(PolygonOptions::new(60.0).at(620.0, 730.0).sides(6).fill(Rgba::rgb(60, 70, 90)).stroke(gold, 5.0));

    // Row 5: lines of increasing width, and a fan of angled lines.
    for (i, w) in [1.0f32, 2.0, 4.0, 8.0, 16.0].iter().enumerate() {
        let y = 830.0 + i as f32 * 12.0;
        g.draw_line(LineOptions::new(760.0, y, 1140.0, y).color(white).width(*w));
    }
    for k in 0..7 {
        let a = -60.0_f32 + k as f32 * 20.0;
        let (dx, dy) = (a.to_radians().cos() * 90.0, a.to_radians().sin() * 90.0);
        g.draw_line(LineOptions::new(180.0, 860.0, 180.0 + dx, 860.0 + dy).color(ink).width(3.0));
    }

    let o = out_dir();
    g.save(o.join("08_shapes.png").to_str().unwrap(), ExportOptions::png())?;
    g.save(o.join("08_shapes.webp").to_str().unwrap(), ExportOptions::webp(90.0))?;
    println!("wrote 08_shapes.{{png,webp}} to {}", o.display());
    Ok(())
}
