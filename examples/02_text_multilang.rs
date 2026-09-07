//! 02 — multi-language text rendering.
//!
//! Exercises: `LoadFont::new` (from path), `LoadFont::from_bytes`, and `draw_text` with
//! LTR English, RTL Arabic, mixed Arabic/Latin/digits, multi-line (`\n`), a line that
//! overflows the right/bottom edge (clipped, not panicking), an empty string, and an
//! emoji-containing string. Several fonts, sizes and colours.
//!
//! Run: `cargo run --example 02_text_multilang`

use std::fs;
use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::image::codecs::png::{CompressionType, FilterType};
use ghe2d::utility::Rgba;
use ghe2d::Ghe2d;

fn asset(p: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(p)
}
fn out_dir() -> PathBuf {
    let d = asset("target/example-output");
    fs::create_dir_all(&d).unwrap();
    d
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fp = |n: &str| asset(&format!("examples/assets/fonts/{n}")).to_string_lossy().into_owned();

    let cairo = LoadFont::new(fp("Cairo-Regular.ttf"))?;
    let rubik = LoadFont::from_bytes(fs::read(fp("Rubik-Regular.ttf"))?)?;
    let tajawal = LoadFont::new(fp("Tajawal-Bold.ttf"))?;
    let ruwudu = LoadFont::new(fp("Ruwudu-Regular.ttf"))?;

    let mut g = Ghe2d::new(1000, 1400);
    g.draw_rect(0, 0, 1000, 1400, Rgba::new(18, 20, 28, 255));

    let white = Rgba::new(240, 240, 245, 255);
    let gold = Rgba::new(220, 170, 70, 255);
    let faint = Rgba::new(120, 200, 255, 140);

    g.draw_text(rubik.clone(), "The quick brown fox 0123456789".to_string(), 40, 40, 34, white.clone());
    g.draw_text(cairo.clone(), "مرحبا بالعالم من مكتبة ghe2d".to_string(), 40, 130, 44, gold.clone());
    g.draw_text(cairo.clone(), "الإصدار v0.2.0 — تجربة 2026 مع أرقام 42".to_string(), 40, 230, 36, white.clone());
    // explicit mixed-direction line: RTL words + LTR token + digits, must read in order
    g.draw_text(cairo.clone(), "سعر المنتج 250 ريال — order #42".to_string(), 40, 300, 36, faint.clone());
    g.draw_text(tajawal.clone(), "عنوان عريض".to_string(), 40, 370, 90, gold.clone());
    g.draw_text(ruwudu.clone(), "خط زخرفي".to_string(), 40, 500, 70, faint.clone());

    // multi-line via '\n'
    g.draw_text(cairo.clone(), "سطر أول\nسطر ثاني\nthird line mixed".to_string(), 40, 600, 40, white.clone());

    // deliberately overflow the right + bottom edges (must clip, never panic)
    g.draw_text(
        rubik.clone(),
        "This very long line keeps going past the right edge of the canvas and should just be clipped".to_string(),
        700,
        900,
        60,
        Rgba::new(255, 120, 120, 255),
    );
    g.draw_text(tajawal.clone(), "نص عند الحافة السفلية".to_string(), 60, 1380, 80, gold.clone());

    // empty string + emoji — must not panic
    g.draw_text(cairo.clone(), String::new(), 40, 1000, 40, white.clone());
    g.draw_text(cairo.clone(), "🙂 اختبار مع رمز تعبيري 👍".to_string(), 40, 1060, 40, white.clone());

    // tiny + huge sizes
    g.draw_text(rubik.clone(), "tiny 18px".to_string(), 40, 1160, 18, faint.clone());
    g.draw_text(rubik.clone(), "BIG".to_string(), 40, 1210, 120, gold.clone());

    let o = out_dir();
    g.save_with_png(o.join("02_text.png").to_str().unwrap(), CompressionType::Default, FilterType::Adaptive)?;
    g.save_with_webp(o.join("02_text.webp").to_str().unwrap(), 92.0)?;
    println!("wrote 02_text.{{png,webp}} to {}", o.display());
    Ok(())
}
