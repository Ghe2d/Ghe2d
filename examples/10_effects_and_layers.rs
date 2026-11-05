//! 10 — effects and layers: shadow and glow on every kind of drawable, opacity, rotation of
//! text and images, rounded rectangles, ellipses, free paths, cover / contain / fill, clipping,
//! text that keeps its size inside a box, and a timing of a typical 1200 x 630 card.
//!
//! Run: `cargo run --release --example 10_effects_and_layers`
//! (add `-- bench` to print the timing of 50 draws of the card)

use std::path::PathBuf;
use std::time::Instant;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{
    CircleOptions, EllipseOptions, ExportOptions, FitImageOptions, Ghe2d, Glow, GroupOptions,
    ImageFit, LineOptions, PathOptions, PolygonOptions, RectOptions, Shadow, StarOptions,
    TextAlign, TextOptions,
};

fn out_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-output");
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn font(name: &str) -> LoadFont {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets/fonts")
        .join(name);
    LoadFont::from_bytes(std::fs::read(p).expect("font file")).expect("valid font")
}

/// A picture made in code: a diagonal gradient with a soft circle (so the fit modes show).
fn sample_picture(w: u32, h: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let t = (x as f32 / w as f32 + y as f32 / h as f32) / 2.0;
        let (cx, cy) = (x as f32 - w as f32 / 2.0, y as f32 - h as f32 / 2.0);
        let inside = (cx * cx + cy * cy).sqrt() < h as f32 * 0.3;
        *p = if inside {
            image::Rgba([255, 255, 255, 255])
        } else {
            image::Rgba([
                (40.0 + 180.0 * t) as u8,
                (90.0 + 60.0 * t) as u8,
                (220.0 - 150.0 * t) as u8,
                255,
            ])
        };
    }
    img
}

/// A typical card: a background, a rounded panel, a picture, a title with a shadow, a glowing
/// badge, a few shapes and a list of tags. About twenty elements.
fn card(latin: &LoadFont, arabic: &LoadFont, picture: &image::RgbaImage) -> Ghe2d {
    let mut g = Ghe2d::new(1200, 630);
    let ink = Rgba::rgb(16, 20, 34);
    let soft = Shadow::new(Rgba::new(0, 0, 0, 190))
        .offset(0.0, 8.0)
        .blur(22.0);

    g.draw_image_fit(
        picture,
        FitImageOptions::new(0.0, 0.0, 1200.0, 630.0).fit(ImageFit::Cover),
    );
    g.draw_rect(RectOptions::solid(
        0.0,
        0.0,
        1200.0,
        630.0,
        Rgba::new(10, 14, 28, 150),
    ));
    g.draw_rect(
        RectOptions::solid(60.0, 60.0, 1080.0, 510.0, Rgba::new(255, 255, 255, 235))
            .radius(36.0)
            .shadow(soft),
    );
    g.draw_image_fit(
        picture,
        FitImageOptions::new(90.0, 90.0, 420.0, 450.0)
            .fit(ImageFit::Cover)
            .shadow(
                Shadow::new(Rgba::new(0, 0, 0, 150))
                    .offset(0.0, 6.0)
                    .blur(14.0),
            ),
    );
    for i in 0..4 {
        let x = 540.0 + i as f32 * 138.0;
        g.draw_image_fit(
            picture,
            FitImageOptions::new(x, 90.0, 120.0, 120.0)
                .fit(if i % 2 == 0 {
                    ImageFit::Contain
                } else {
                    ImageFit::Fill
                })
                .rotation(i as f32 * 4.0 - 6.0)
                .opacity(0.95),
        );
    }
    g.draw_text(
        TextOptions::new(latin.clone(), "Raven Black Knight")
            .at(540.0, 240.0)
            .size(latin.css_to_size(64.0))
            .color(ink)
            .shadow(
                Shadow::new(Rgba::new(40, 60, 200, 120))
                    .offset(3.0, 4.0)
                    .blur(6.0),
            ),
    );
    g.draw_text(
        TextOptions::new(arabic.clone(), "فارس الغراب الأسود")
            .at(540.0, 330.0)
            .size(arabic.css_to_size(52.0))
            .color(Rgba::rgb(210, 60, 60))
            .width(560.0)
            .align(TextAlign::Right)
            .glow(
                Glow::new(Rgba::rgb(255, 120, 120))
                    .blur(14.0)
                    .intensity(1.6),
            ),
    );
    g.draw_circle(
        CircleOptions::new(34.0)
            .at(1040.0, 150.0)
            .fill(Rgba::rgb(255, 200, 60))
            .glow(Glow::new(Rgba::rgb(255, 190, 40)).blur(24.0).intensity(2.0)),
    );
    g.draw_ellipse(
        EllipseOptions::new(70.0, 34.0)
            .at(1000.0, 280.0)
            .rotation(-12.0)
            .fill(Rgba::rgb(60, 180, 140))
            .stroke(Rgba::WHITE, 4.0)
            .shadow(soft),
    );
    g.draw_path(
        PathOptions::triangle(930.0, 340.0, 120.0, 100.0)
            .fill(Rgba::rgb(120, 90, 240))
            .glow(Glow::new(Rgba::rgb(160, 130, 255)).blur(16.0)),
    );
    g.draw_star(
        StarOptions::new(46.0)
            .at(1090.0, 410.0)
            .points(5)
            .fill(Rgba::rgb(250, 90, 140))
            .rotation(12.0)
            .opacity(0.8),
    );
    g.draw_polygon(
        PolygonOptions::new(30.0)
            .at(860.0, 150.0)
            .sides(6)
            .fill(Rgba::rgb(60, 120, 240))
            .opacity(0.7),
    );
    g.draw_line(
        LineOptions::new(540.0, 420.0, 1100.0, 420.0)
            .color(Rgba::rgb(200, 205, 220))
            .width(4.0),
    );
    for (i, tag) in ["Epic", "Outfit", "Rare"].iter().enumerate() {
        let x = 540.0 + i as f32 * 190.0;
        g.draw_rect(
            RectOptions::solid(x, 450.0, 170.0, 64.0, Rgba::rgb(30, 40, 90))
                .radius(32.0)
                .shadow(
                    Shadow::new(Rgba::new(30, 40, 90, 120))
                        .offset(0.0, 4.0)
                        .blur(8.0),
                ),
        );
        g.draw_text(
            TextOptions::new(latin.clone(), *tag)
                .at(x, 450.0)
                .size(latin.css_to_size(30.0))
                .color(Rgba::WHITE)
                .width(170.0)
                .height(64.0)
                .align(TextAlign::Center)
                .line_layout(ghe2d::LineLayout::Middle(0.0)),
        );
    }
    g
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let latin = font("Rubik-Regular.ttf");
    let arabic = font("Tajawal-Bold.ttf");
    let picture = sample_picture(900, 600);

    // The card, and how long it takes.
    let started = Instant::now();
    let g = card(&latin, &arabic, &picture);
    println!(
        "card (about 20 elements, 1200x630): {:.1} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    g.save(
        out_dir().join("10_card.png").to_str().unwrap(),
        ExportOptions::png(),
    )?;
    let webp = Instant::now();
    let bytes = g.encode(ExportOptions::webp(90.0))?;
    println!(
        "webp encode: {:.1} ms, {} bytes",
        webp.elapsed().as_secs_f64() * 1000.0,
        bytes.len()
    );
    if std::env::args().any(|a| a == "bench") {
        let t = Instant::now();
        for _ in 0..50 {
            let _ = card(&latin, &arabic, &picture);
        }
        println!(
            "50 cards: {:.1} ms each",
            t.elapsed().as_secs_f64() * 1000.0 / 50.0
        );
    }

    // A group by hand: a clipped, rotated piece with a shadow, made of several drawings.
    let mut c = Ghe2d::new(520, 300);
    c.draw_rect(RectOptions::solid(
        0.0,
        0.0,
        520.0,
        300.0,
        Rgba::rgb(235, 238, 245),
    ));
    c.draw_group(
        GroupOptions::new(110.0, 80.0, 300.0, 140.0)
            .rotation(-8.0)
            .clip(true)
            .opacity(0.95)
            .shadow(
                Shadow::new(Rgba::new(0, 0, 0, 170))
                    .offset(6.0, 8.0)
                    .blur(16.0),
            ),
        |layer, ox, oy| {
            layer.draw_rect(RectOptions::solid(
                ox,
                oy,
                300.0,
                140.0,
                Rgba::rgb(250, 190, 60),
            ));
            layer.draw_circle(CircleOptions::solid(
                ox + 280.0,
                oy + 20.0,
                90.0,
                Rgba::rgb(230, 80, 90),
            ));
            layer.draw_text(
                TextOptions::new(latin.clone(), "Clipped at the box edge")
                    .at(ox + 14.0, oy + 50.0)
                    .size(latin.css_to_size(40.0))
                    .color(Rgba::rgb(20, 24, 40))
                    .shrink_to_fit(false),
            );
        },
    );
    c.save(
        out_dir().join("10_group.png").to_str().unwrap(),
        ExportOptions::png(),
    )?;
    println!("wrote {}", out_dir().display());
    Ok(())
}
