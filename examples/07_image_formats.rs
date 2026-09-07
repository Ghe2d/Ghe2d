//! 07 — image formats: encode the same canvas as PNG / JPEG / WebP, compare sizes, show
//! JPEG's `background` flattening, then load a `.jpg` back and report the detected format.
//!
//! Run: `cargo run --example 07_image_formats`

use std::path::PathBuf;

use ghe2d::utility::Rgba;
use ghe2d::{CircleOptions, ExportOptions, Ghe2d, ImageOptions, RectOptions};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn scene() -> Ghe2d {
    let mut g = Ghe2d::new(640, 400);
    // photo-ish gradient on the left, transparent hole on the right
    for y in 0..400u32 {
        let t = y as f32 / 400.0;
        let c = Rgba::rgb((30.0 + t * 200.0) as u8, (80.0 + t * 120.0) as u8, (200.0 - t * 150.0) as u8);
        g.draw_rect(RectOptions::solid(0.0, y as f32, 400.0, 1.0, c));
    }
    g.draw_circle(CircleOptions::solid(300.0, 200.0, 120.0, Rgba::rgb(255, 210, 90)));
    g.draw_circle(CircleOptions::solid(180.0, 120.0, 60.0, Rgba::rgb(240, 90, 120)));
    g
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = out_dir();
    let g = scene();

    let variants: [(&str, ExportOptions); 5] = [
        ("07_scene.png", ExportOptions::png()),
        ("07_scene.q80.jpg", ExportOptions::jpeg(80)),
        ("07_scene.q80.black.jpg", ExportOptions::jpeg(80).background(Rgba::new(0, 0, 0, 255))),
        ("07_scene.q40.webp", ExportOptions::webp(40.0)),
        ("07_scene.q95.webp", ExportOptions::webp(95.0)),
    ];

    println!("{:<26} {:>9}", "file", "bytes");
    for (name, opts) in variants {
        let bytes = g.encode(opts)?;
        std::fs::write(o.join(name), &bytes)?;
        println!("{name:<26} {:>9}  detect={:?}", bytes.len(), Ghe2d::detect_format(&bytes));
    }

    // round-trip a JPEG through from_bytes
    let jpg = g.encode(ExportOptions::jpeg(85))?;
    let back = Ghe2d::from_bytes(jpg, ImageOptions::new().size(200, 125))?;
    println!(
        "\nround-trip JPEG -> from_bytes: source_format = {:?}, {}x{}",
        back.source_format(),
        back.width(),
        back.height()
    );
    back.save(o.join("07_roundtrip.png").to_str().unwrap(), ExportOptions::png())?;

    println!("wrote 07_* files to {}", o.display());
    Ok(())
}
