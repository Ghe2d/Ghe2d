//! Golden images: whole scenes compared with stored pictures, with a tolerance (a few levels of
//! colour on average, and only a sliver of pixels may be off by more), so a change that moves an
//! edge or a shadow shows up while a rounding difference does not.
//!
//! The pictures are in `tests/golden/`. To write them again after a change that is meant:
//! `GHE2D_BLESS=1 cargo test --test golden`, then look at them.

use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{
    CircleOptions, EllipseOptions, ExportOptions, FitImageOptions, Ghe2d, Glow, ImageFit,
    LineOptions, PathOptions, RectOptions, Shadow, StarOptions, TextAlign, TextOptions,
};

fn font(name: &str) -> LoadFont {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/assets/fonts")
        .join(name);
    LoadFont::from_bytes(std::fs::read(p).expect("font")).expect("valid font")
}

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(format!("{name}.png"))
}

/// Premultiplied channels, so the colour under a transparent pixel does not count.
fn premultiplied(p: &image::Rgba<u8>) -> [f32; 4] {
    let a = p.0[3] as f32 / 255.0;
    [
        p.0[0] as f32 * a,
        p.0[1] as f32 * a,
        p.0[2] as f32 * a,
        p.0[3] as f32,
    ]
}

fn check(name: &str, g: &Ghe2d) {
    let path = golden_path(name);
    if std::env::var_os("GHE2D_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        g.save(path.to_str().unwrap(), ExportOptions::png())
            .unwrap();
        return;
    }
    let want = image::open(&path)
        .unwrap_or_else(|e| {
            panic!("{name}: no golden picture at {path:?} ({e}); run with GHE2D_BLESS=1")
        })
        .to_rgba8();
    assert_eq!(want.dimensions(), g.image.dimensions(), "{name}: size");
    let (mut sum, mut off, mut worst) = (0.0_f64, 0usize, 0.0_f32);
    for (a, b) in want.pixels().zip(g.image.pixels()) {
        let (pa, pb) = (premultiplied(a), premultiplied(b));
        let d = (0..4)
            .map(|i| (pa[i] - pb[i]).abs())
            .fold(0.0_f32, f32::max);
        sum += (0..4).map(|i| (pa[i] - pb[i]).abs() as f64).sum::<f64>() / 4.0;
        worst = worst.max(d);
        if d > 24.0 {
            off += 1;
        }
    }
    let n = want.pixels().len() as f64;
    let (mean, share) = (sum / n, off as f64 / n);
    assert!(
        mean <= 0.6,
        "{name}: mean difference {mean:.3} levels (worst pixel {worst})"
    );
    assert!(
        share <= 0.003,
        "{name}: {:.3}% of the pixels differ by more than 24 levels",
        share * 100.0
    );
}

fn ground(w: u32, h: u32) -> Ghe2d {
    let mut g = Ghe2d::new(w, h);
    g.draw_rect(RectOptions::solid(
        0.0,
        0.0,
        w as f32,
        h as f32,
        Rgba::rgb(236, 239, 246),
    ));
    g
}

#[test]
fn shapes_with_effects() {
    let mut g = ground(280, 180);
    let soft = Shadow::new(Rgba::new(10, 20, 60, 200))
        .offset(3.0, 6.0)
        .blur(10.0);
    g.draw_rect(
        RectOptions::solid(14.0, 14.0, 110.0, 70.0, Rgba::rgb(250, 190, 60))
            .radius(16.0)
            .shadow(soft),
    );
    g.draw_circle(
        CircleOptions::solid(190.0, 50.0, 26.0, Rgba::rgb(255, 120, 60))
            .glow(Glow::new(Rgba::rgb(255, 150, 60)).blur(16.0).intensity(1.8)),
    );
    g.draw_ellipse(
        EllipseOptions::new(46.0, 22.0)
            .at(70.0, 132.0)
            .rotation(-20.0)
            .fill(Rgba::rgb(60, 180, 140))
            .stroke(Rgba::WHITE, 3.0)
            .shadow(soft),
    );
    g.draw_path(
        PathOptions::triangle(150.0, 100.0, 56.0, 56.0)
            .fill(Rgba::rgb(120, 90, 240))
            .glow(Glow::new(Rgba::rgb(150, 120, 255)).blur(12.0)),
    );
    g.draw_star(
        StarOptions::new(26.0)
            .at(240.0, 126.0)
            .fill(Rgba::rgb(240, 70, 130))
            .rotation(15.0)
            .opacity(0.7)
            .shadow(soft),
    );
    g.draw_line(
        LineOptions::new(14.0, 168.0, 266.0, 168.0)
            .color(Rgba::rgb(90, 100, 130))
            .width(3.0)
            .shadow(
                Shadow::new(Rgba::new(0, 0, 0, 120))
                    .offset(0.0, 3.0)
                    .blur(3.0),
            ),
    );
    check("shapes_with_effects", &g);
}

#[test]
fn text_with_effects_and_rotation() {
    let mut g = ground(320, 200);
    let latin = font("Rubik-Regular.ttf");
    let arabic = font("Tajawal-Bold.ttf");
    g.draw_text(
        TextOptions::new(latin.clone(), "Shadowed")
            .at(16.0, 12.0)
            .size(latin.css_to_size(44.0))
            .color(Rgba::rgb(20, 30, 80))
            .shadow(
                Shadow::new(Rgba::new(40, 70, 220, 150))
                    .offset(3.0, 4.0)
                    .blur(5.0),
            ),
    );
    g.draw_text(
        TextOptions::new(arabic.clone(), "نص متوهج")
            .at(16.0, 74.0)
            .size(arabic.css_to_size(44.0))
            .color(Rgba::rgb(200, 40, 50))
            .width(288.0)
            .align(TextAlign::Right)
            .glow(
                Glow::new(Rgba::rgb(255, 130, 130))
                    .blur(10.0)
                    .intensity(1.5),
            ),
    );
    g.draw_text(
        TextOptions::new(latin.clone(), "Turned and faded")
            .at(40.0, 150.0)
            .size(latin.css_to_size(28.0))
            .color(Rgba::rgb(30, 90, 60))
            .rotation(-8.0)
            .opacity(0.8),
    );
    check("text_with_effects_and_rotation", &g);
}

#[test]
fn images_fitted_into_boxes() {
    let picture = image::RgbaImage::from_fn(160, 100, |x, y| {
        let t = (x + y) as f32 / 260.0;
        let (dx, dy) = (x as f32 - 80.0, y as f32 - 50.0);
        if (dx * dx + dy * dy).sqrt() < 26.0 {
            image::Rgba([255, 255, 255, 255])
        } else {
            image::Rgba([
                (50.0 + 170.0 * t) as u8,
                (100.0 + 40.0 * t) as u8,
                (230.0 - 150.0 * t) as u8,
                255,
            ])
        }
    });
    let mut g = ground(330, 130);
    let drop = Shadow::new(Rgba::new(0, 0, 0, 150))
        .offset(0.0, 5.0)
        .blur(8.0);
    for (i, fit) in [ImageFit::Cover, ImageFit::Contain, ImageFit::Fill]
        .into_iter()
        .enumerate()
    {
        g.draw_image_fit(
            &picture,
            FitImageOptions::new(12.0 + i as f32 * 106.0, 14.0, 90.0, 90.0)
                .fit(fit)
                .shadow(drop),
        );
    }
    g.draw_image_fit(
        &picture,
        FitImageOptions::new(240.0, 20.0, 80.0, 50.0)
            .rotation(14.0)
            .opacity(0.85),
    );
    check("images_fitted_into_boxes", &g);
}
