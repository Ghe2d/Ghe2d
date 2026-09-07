//! 04 — async image loading.
//!
//! Exercises: `Ghe2d::from_path` (plain / circle+resize) + `source_format`,
//! `Ghe2d::detect_format`, and `draw_image_path` chained on `&mut self` with every
//! `ComposeMode`, `circle`, and an off-canvas (clipped) placement.
//!
//! `from_path` also accepts `http(s)://` URLs; this example generates local files so it
//! stays deterministic and offline.
//!
//! Run: `cargo run --example 04_async_load_from_path`

use std::path::PathBuf;

use ghe2d::utility::Rgba;
use ghe2d::{ComposeMode, ExportOptions, Ghe2d, ImageOptions};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    std::fs::create_dir_all(&d).unwrap();
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

    // one PNG source, one JPEG source — decoding auto-detects the format
    let src_png = p("04_src_a.png");
    let src_jpg = p("04_src_b.jpg");
    make_source(360, 360, [200, 60, 60, 255]).save(&src_png, ExportOptions::png())?;
    make_source(360, 360, [60, 90, 200, 255]).save(&src_jpg, ExportOptions::jpeg(90))?;

    println!("detect_format(src_a.png) = {:?}", Ghe2d::detect_format(&std::fs::read(&src_png)?));
    println!("detect_format(src_b.jpg) = {:?}", Ghe2d::detect_format(&std::fs::read(&src_jpg)?));

    let plain = Ghe2d::from_path(&src_png, ImageOptions::new()).await?;
    println!("from_path(png) source_format = {:?}", plain.source_format());
    plain.save(&p("04_from_path_plain.png"), ExportOptions::png())?;

    let circ = Ghe2d::from_path(&src_jpg, ImageOptions::new().size(500, 500).circle(true)).await?;
    println!("from_path(jpg) source_format = {:?}", circ.source_format());
    circ.save(&p("04_from_path_circle_500.png"), ExportOptions::png())?;

    // compose: chain draw_image_path with each ComposeMode
    let mut c = Ghe2d::new(1000, 700);
    for y in 0..700u32 {
        c.draw_rect(0, y, 1000, 1, Rgba::new(22, 24, 30, 255));
    }
    c.draw_image_path(&src_png, ImageOptions::new().at(20, 20).size(300, 300)).await?; // Normal
    c.draw_image_path(
        &src_jpg,
        ImageOptions::new().at(350, 60).size(320, 320).compose(ComposeMode::Blend),
    )
    .await?;
    c.draw_image_path(
        &src_png,
        ImageOptions::new().at(700, 360).size(280, 280).circle(true).compose(ComposeMode::Overlay),
    )
    .await?;
    c.draw_image_path(
        &src_jpg,
        ImageOptions::new().at(900, 620).size(240, 240).compose(ComposeMode::Overlay),
    )
    .await?; // mostly off-canvas → clipped

    c.save(&p("04_async_composite.png"), ExportOptions::png())?;
    c.save(&p("04_async_composite.webp"), ExportOptions::webp(85.0))?;

    println!("wrote 04_* files to {}", o.display());
    Ok(())
}
