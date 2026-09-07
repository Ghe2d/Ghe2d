//! 02 — multi-language text + box layout.
//!
//! Exercises: `LoadFont::new` / `LoadFont::from_bytes`; the `TextOptions` builder;
//! `draw_text` with LTR / RTL / mixed / multi-line / overflow / empty / emoji text;
//! `width` + `TextAlign` (Left/Right/Center); `width` + `wrap`; `width` shrink-to-fit;
//! `height` + `LineLayout` (Gap / Middle / SpaceBetween).
//!
//! Run: `cargo run --example 02_text_multilang`

use std::fs;
use std::path::PathBuf;

use ghe2d::font::LoadFont;
use ghe2d::utility::Rgba;
use ghe2d::{ExportOptions, Ghe2d, LineLayout, RectOptions, TextAlign, TextOptions};

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

    let mut g = Ghe2d::new(1000, 1600);
    g.draw_rect(RectOptions::solid(0.0, 0.0, 1000.0, 1600.0, Rgba::rgb(18, 20, 28)));

    let white = Rgba::new(240, 240, 245, 255);
    let gold = Rgba::new(220, 170, 70, 255);
    let faint = Rgba::new(120, 200, 255, 200);
    let red = Rgba::new(255, 120, 120, 255);

    // --- basic lines (no box) ---
    g.draw_text(TextOptions::new(rubik.clone(), "The quick brown fox 0123456789").at(40.0, 40.0).size(34.0).color(white));
    g.draw_text(TextOptions::new(cairo.clone(), "مرحبا بالعالم من مكتبة ghe2d").at(40.0, 120.0).size(44.0).color(gold));
    g.draw_text(TextOptions::new(cairo.clone(), "الإصدار v0.2.0 — تجربة 2026 مع أرقام 42").at(40.0, 210.0).size(36.0).color(white));
    // mixed-direction line: RTL words + LTR token + digits, must read in order
    g.draw_text(TextOptions::new(cairo.clone(), "سعر المنتج 250 ريال — order #42").at(40.0, 280.0).size(36.0).color(faint));
    g.draw_text(TextOptions::new(tajawal.clone(), "عنوان عريض").at(40.0, 350.0).size(90.0).color(gold));
    g.draw_text(TextOptions::new(ruwudu.clone(), "خط زخرفي").at(40.0, 480.0).size(70.0).color(faint));
    // multi-line via '\n'
    g.draw_text(TextOptions::new(cairo.clone(), "سطر أول\nسطر ثاني\nthird line mixed").at(40.0, 560.0).size(40.0).color(white));
    // empty string + emoji — must not panic
    g.draw_text(TextOptions::new(cairo.clone(), "").at(40.0, 720.0).size(40.0).color(white));
    g.draw_text(TextOptions::new(cairo.clone(), "🙂 اختبار مع رمز تعبيري 👍").at(40.0, 720.0).size(40.0).color(white));

    // --- width + alignment (box is x=40..960) ---
    let box_w = 920.0;
    g.draw_rect(RectOptions::solid(40.0, 800.0, box_w, 2.0, Rgba::rgb(60, 66, 84)));
    g.draw_text(TextOptions::new(rubik.clone(), "left aligned").at(40.0, 810.0).size(34.0).color(white).width(box_w).align(TextAlign::Left));
    g.draw_text(TextOptions::new(rubik.clone(), "centered").at(40.0, 860.0).size(34.0).color(gold).width(box_w).align(TextAlign::Center));
    g.draw_text(TextOptions::new(rubik.clone(), "right aligned").at(40.0, 910.0).size(34.0).color(faint).width(box_w).align(TextAlign::Right));
    // Arabic with a width and NO explicit align -> defaults to the right
    g.draw_text(TextOptions::new(cairo.clone(), "محاذاة افتراضية للعربية").at(40.0, 960.0).size(36.0).color(white).width(box_w));

    // --- width + word wrap ---
    g.draw_text(
        TextOptions::new(cairo.clone(), "هذه فقرة طويلة يتم لفّها تلقائياً داخل عرض محدد بدون أي انهيار، والكلمات تنتقل كاملةً إلى السطر التالي عند الحاجة.")
            .at(40.0, 1030.0)
            .size(30.0)
            .color(white)
            .width(500.0)
            .wrap(true)
            .align(TextAlign::Right),
    );
    g.draw_text(
        TextOptions::new(rubik.clone(), "A long English paragraph that wraps to the given width one whole word at a time.")
            .at(560.0, 1030.0)
            .size(26.0)
            .color(faint)
            .width(400.0)
            .wrap(true),
    );

    // --- width shrink-to-fit (no wrap): 120px requested, forced into 400px ---
    g.draw_text(TextOptions::new(tajawal.clone(), "يتقلّص الحجم ليتناسب").at(40.0, 1260.0).size(120.0).color(gold).width(400.0));

    // --- height + LineLayout, three variants side by side in 300px-tall boxes ---
    let block = "سطر\nثاني\nثالث";
    for (i, (label, layout)) in [
        ("Gap(20)", LineLayout::Gap(20.0)),
        ("Middle", LineLayout::Middle(0.0)),
        ("SpaceBetween", LineLayout::SpaceBetween),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 40.0 + i as f32 * 320.0;
        g.draw_rect(RectOptions::solid(x, 1360.0, 300.0, 300.0, Rgba::rgb(28, 32, 44)));
        g.draw_text(TextOptions::new(rubik.clone(), label).at(x + 8.0, 1364.0).size(20.0).color(faint));
        g.draw_text(
            TextOptions::new(cairo.clone(), block)
                .at(x + 8.0, 1400.0)
                .size(40.0)
                .color(white)
                .height(250.0)
                .line_layout(layout),
        );
    }

    // --- deliberately overflow the right edge (clip, never panic) ---
    g.draw_text(TextOptions::new(rubik.clone(), "This very long line keeps going past the right edge and is simply clipped").at(760.0, 560.0).size(48.0).color(red));

    let o = out_dir();
    g.save(o.join("02_text.png").to_str().unwrap(), ExportOptions::png())?;
    g.save(o.join("02_text.webp").to_str().unwrap(), ExportOptions::webp(92.0))?;
    println!("wrote 02_text.{{png,webp}} to {}", o.display());
    Ok(())
}
