
pub fn draw_circle(img: &mut image::RgbaImage, x: u32, y: u32, radius: u32, color: crate::utility::Rgba) {
    for draw_y in (if y > radius { y - radius } else { 0 })..=(if y < radius { y + radius } else { img.height() - 1 }) {
        for draw_x in (if x > radius { x - radius } else { 0 })..=(if x < radius { x + radius } else { img.width() - 1 }) {
            let dx = draw_x as f64 - x as f64;
            let dy = draw_y as f64 - y as f64;
            let distance = dx * dx + dy * dy;
            if dx * dx + dy * dy <= (radius * radius).into() {
                let alpha = 1.0 - (distance / (radius * radius) as f64);
                let alpha = (alpha * 255.0) as u8;
                let pixel = img.get_pixel(draw_x, draw_y);
                let background = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                let blend = crate::utility::Rgba::blend_with_alpha(&color, background, alpha);
                img.put_pixel(draw_x as u32, draw_y as u32, blend);
            }
        }
    }
}
