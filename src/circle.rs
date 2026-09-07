
pub fn draw_circle(img: &mut image::RgbaImage, x: u32, y: u32, radius: u32, color: crate::utility::Rgba) {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || radius == 0 {
        return;
    }

    // Clamp the scan window to the canvas: anything outside is simply not drawn.
    let x0 = x.saturating_sub(radius);
    let x1 = x.saturating_add(radius).min(w - 1);
    let y0 = y.saturating_sub(radius);
    let y1 = y.saturating_add(radius).min(h - 1);

    let r2 = (radius as i64).pow(2);

    for draw_y in y0..=y1 {
        for draw_x in x0..=x1 {
            let dx = draw_x as i64 - x as i64;
            let dy = draw_y as i64 - y as i64;
            let distance = dx * dx + dy * dy;

            if distance <= r2 {
                let alpha = 1.0 - (distance as f64 / r2 as f64);
                let alpha = (alpha * 255.0) as u8;
                let pixel = img.get_pixel(draw_x, draw_y);
                let background = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                let blend = crate::utility::Rgba::blend_with_alpha(&color, background, alpha);
                img.put_pixel(draw_x, draw_y, blend);
            }
        }
    }
}
