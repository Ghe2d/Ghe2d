//! 09 — what's new in 0.2.1: font chains, variable weights, measuring, and animation.
//!
//! Exercises: `TextOptions::fallback` (a character the main font lacks comes from the next
//! font, and one no font has draws a visible `.notdef` box); `LoadFont::{is_variable,
//! weight_axis, with_weight, has_glyph}` and `TextOptions::weight` on a variable font;
//! `FontFamily::pick`; `measure_text` (a box drawn from the measurement fits the drawn text);
//! the `anim` module: make an animated GIF, read it with `decode_animation` + `DecodeLimits`,
//! convert it to an animated WebP with `encode_webp` and read that back, `Animation::crop` /
//! `resize` to square and wide outputs, a still PNG as one frame, and a limit that refuses a
//! file.
//!
//! Run: `cargo run --example 09_animation_and_fonts`

use std::fs;
use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{
    decode_animation, measure_text, CircleOptions, DecodeLimits,
    ExportOptions, FontFamily, Ghe2d, RectOptions, TextOptions,
};

fn asset(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
}
fn out_dir() -> PathBuf {
    let d = asset("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}
fn font(p: &str) -> Result<LoadFont, Box<dyn std::error::Error>> {
    Ok(LoadFont::new(asset(p).to_string_lossy().into_owned())?)
}

/// A 1px outline from four thin rectangles.
fn outline(g: &mut Ghe2d, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
    g.draw_rect(RectOptions::solid(x, y, w, 1.0, c));
    g.draw_rect(RectOptions::solid(x, y + h - 1.0, w, 1.0, c));
    g.draw_rect(RectOptions::solid(x, y, 1.0, h, c));
    g.draw_rect(RectOptions::solid(x + w - 1.0, y, 1.0, h, c));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let o = out_dir();
    let cairo = font("examples/assets/fonts/Cairo-Regular.ttf")?;
    let rubik = font("examples/assets/fonts/Rubik-Regular.ttf")?;
    let tajawal = font("examples/assets/fonts/Tajawal-Bold.ttf")?;
    // Cairo variable font (wght 200..1000), SIL Open Font License 1.1.
    let cairo_var = font("tests/fixtures/Cairo-Variable.ttf")?;

    let bg = Rgba::rgb(18, 20, 28);
    let white = Rgba::new(240, 240, 245, 255);
    let gold = Rgba::new(220, 170, 70, 255);
    let faint = Rgba::new(120, 200, 255, 220);
    let dim = Rgba::new(150, 156, 175, 255);

    // ------------------------------------------------------------------ fonts
    let mut g = Ghe2d::new(1200, 800);
    g.draw_rect(RectOptions::solid(0.0, 0.0, 1200.0, 800.0, bg));

    // 1) Font chain. Cairo has Arabic and Latin but no Cyrillic or Hebrew; Rubik has them.
    let mixed = "عربي Latin Кириллица עברית";
    for ch in ['Ж', 'א'] {
        println!("'{ch}': Cairo {}, Rubik {}", cairo.has_glyph(ch), rubik.has_glyph(ch));
    }
    g.draw_text(TextOptions::new(rubik.clone(), "1) font chain").at(40.0, 30.0).size(22.0).color(dim));
    g.draw_text(TextOptions::new(cairo.clone(), mixed).at(40.0, 70.0).size(40.0).color(white));
    g.draw_text(TextOptions::new(rubik.clone(), "no fallback: missing letters show a box").at(700.0, 82.0).size(18.0).color(dim));
    g.draw_text(TextOptions::new(cairo.clone(), mixed).fallback(rubik.clone()).at(40.0, 130.0).size(40.0).color(gold));
    g.draw_text(TextOptions::new(rubik.clone(), "with .fallback(rubik)").at(700.0, 142.0).size(18.0).color(dim));

    // 2) Variable weight ladder on one font file.
    let (min, def, max) = cairo_var.weight_axis().unwrap_or((400.0, 400.0, 400.0));
    println!("Cairo variable: is_variable={}, wght {min}..{max} (default {def})", cairo_var.is_variable());
    g.draw_text(TextOptions::new(rubik.clone(), "2) one variable font, many weights").at(40.0, 210.0).size(22.0).color(dim));
    for (i, w) in [200.0, 300.0, 400.0, 600.0, 800.0, 1000.0].into_iter().enumerate() {
        let y = 250.0 + i as f32 * 52.0;
        g.draw_text(TextOptions::new(rubik.clone(), format!("{w:>4}")).at(40.0, y + 8.0).size(20.0).color(dim));
        g.draw_text(TextOptions::new(cairo_var.clone(), "خط متغيّر Variable").weight(w).at(120.0, y).size(38.0).color(white));
    }

    // 3) Pick the closest face in a family.
    let mut family = FontFamily::new();
    family.add(cairo.clone()).add(cairo_var.with_weight(def));
    for (w, italic) in [(300.0, false), (400.0, false), (700.0, false), (900.0, true)] {
        let picked = family.pick(w, italic).map(|f| f.weight());
        println!("FontFamily::pick({w}, italic={italic}) -> weight {picked:?}");
    }

    // 4) Measure first, then draw: the outline is the measured block.
    g.draw_text(TextOptions::new(rubik.clone(), "3) measure_text: the box comes from the measurement").at(40.0, 580.0).size(22.0).color(dim));
    let para = "هذه فقرة تُقاس قبل رسمها، فيُرسم الإطار حولها بالضبط ويلتفّ النص داخله.";
    let opts = TextOptions::new(tajawal.clone(), para).at(60.0, 630.0).size(34.0).color(gold).width(520.0).wrap(true);
    let m = measure_text(&opts);
    println!("measure_text: {:.0}x{:.0} at size {:.0}, {} lines", m.width, m.height, m.size, m.lines.len());
    outline(&mut g, 60.0 + 520.0 - m.width - 6.0, 630.0 - 6.0, m.width + 12.0, m.height + 12.0, faint);
    g.draw_text(opts);

    let shrink = TextOptions::new(rubik.clone(), "Shrink to fit a 420px box").at(640.0, 640.0).size(80.0).color(white).width(420.0);
    let ms = measure_text(&shrink);
    println!("shrink-to-fit: asked 80, measured size {:.1}", ms.size);
    outline(&mut g, 640.0 - 6.0, 640.0 - 6.0, ms.width + 12.0, ms.height + 12.0, faint);
    g.draw_text(shrink);

    g.save(o.join("09_fonts.png").to_str().unwrap(), ExportOptions::png())?;

    // -------------------------------------------------------------- animation
    // Step 1: make a GIF. 12 frames of a dot circling a centre, 80 ms each, looping forever.
    // ghe2d has no GIF writer of its own; the `image` crate it re-exports writes one.
    let mut gif = Vec::new();
    {
        use ghe2d::image::codecs::gif::{GifEncoder, Repeat};
        use ghe2d::image::{Delay, Frame};
        let mut enc = GifEncoder::new(&mut gif);
        enc.set_repeat(Repeat::Infinite)?;
        for i in 0..12 {
            let mut c = Ghe2d::new(240, 240);
            c.draw_rect(RectOptions::solid(0.0, 0.0, 240.0, 240.0, bg));
            let a = i as f32 / 12.0 * std::f32::consts::TAU;
            c.draw_circle(CircleOptions::solid(120.0 + 70.0 * a.cos(), 120.0 + 70.0 * a.sin(), 22.0, gold));
            c.draw_circle(CircleOptions::solid(120.0, 120.0, 12.0, faint));
            enc.encode_frame(Frame::from_parts(c.image.clone(), 0, 0, Delay::from_numer_denom_ms(80, 1)))?;
        }
    }
    fs::write(o.join("09_orbit.gif"), &gif)?;

    // Step 2: read the GIF back (every frame and its delay), with safety limits.
    let limits = DecodeLimits { max_frames: 300, max_total_pixels: 50_000_000, max_duration_ms: 30_000 };
    let from_gif = decode_animation(&gif, &limits)?;
    println!(
        "GIF  : {:>6} bytes, {} frames, {}x{}, {} ms, animated={}",
        gif.len(),
        from_gif.frames.len(),
        from_gif.width(),
        from_gif.height(),
        from_gif.total_duration_ms(),
        from_gif.is_animated()
    );

    // Step 3: convert it to an animated WebP (delays and looping are kept).
    let webp = from_gif.encode_webp(85.0)?;
    fs::write(o.join("09_orbit.webp"), &webp)?;

    // Step 4: read the WebP back to check the conversion kept the animation.
    let back = decode_animation(&webp, &limits)?;
    println!(
        "WebP : {:>6} bytes, {} frames, {}x{}, {} ms, animated={}",
        webp.len(),
        back.frames.len(),
        back.width(),
        back.height(),
        back.total_duration_ms(),
        back.is_animated()
    );
    assert_eq!(back.frames.len(), from_gif.frames.len(), "same frame count after GIF -> WebP");
    assert_eq!(back.total_duration_ms(), from_gif.total_duration_ms(), "same duration after GIF -> WebP");

    // How close is the WebP to the GIF? Compare every pixel that is plain background in the
    // GIF frame (where an earlier frame's dot could leave a trace) and report the worst channel.
    let bg_px = from_gif.frames[0].image.get_pixel(0, 0).0;
    let mut worst = 0u8;
    for (g_frame, w_frame) in from_gif.frames.iter().zip(&back.frames) {
        for (gp, wp) in g_frame.image.pixels().zip(w_frame.image.pixels()) {
            if gp.0 == bg_px {
                for ch in 0..4 {
                    worst = worst.max(gp.0[ch].abs_diff(wp.0[ch]));
                }
            }
        }
    }
    println!("GIF -> WebP: worst background difference {worst} of 255 (0 = identical)");

    // Crop and resize the GIF's frames straight to WebP: a 512x512 square from the centre,
    // and a wide 4:1 strip (1600x400). The same operation runs on every frame.
    fs::write(o.join("09_orbit_square_512.webp"), from_gif.crop(40, 40, 160, 160)?.resize(512, 512)?.encode_webp(85.0)?)?;
    fs::write(o.join("09_orbit_wide_1600x400.webp"), from_gif.crop(0, 90, 240, 60)?.resize(1600, 400)?.encode_webp(85.0)?)?;

    // A contact sheet: the first six GIF frames on top, the same frames after GIF -> WebP below.
    let mut sheet = Ghe2d::new(6 * 250, 2 * 250);
    sheet.draw_rect(RectOptions::solid(0.0, 0.0, 1500.0, 500.0, Rgba::rgb(8, 9, 12)));
    for (row, anim) in [&from_gif, &back].into_iter().enumerate() {
        for (i, f) in anim.frames.iter().take(6).enumerate() {
            ghe2d::image::imageops::overlay(&mut sheet.image, &f.image, (i * 250 + 5) as i64, (row * 250 + 5) as i64);
        }
    }
    sheet.save(o.join("09_gif_vs_webp_frames.png").to_str().unwrap(), ExportOptions::png())?;

    // A still PNG goes through the same call and comes back as one frame.
    let still = g.encode(ExportOptions::png())?;
    let one = decode_animation(&still, &limits)?;
    println!("still PNG: {} frame, animated={}", one.frames.len(), one.is_animated());

    // A limit refuses the file instead of decoding it all.
    let tight = DecodeLimits { max_frames: 5, ..limits };
    match decode_animation(&webp, &tight) {
        Err(e) => println!("max_frames=5 -> refused: {e}"),
        Ok(a) => println!("max_frames=5 -> unexpectedly decoded {} frames", a.frames.len()),
    }

    println!("wrote 09_* files to {}", o.display());
    Ok(())
}
