
pub fn draw_circle(img: &mut image::RgbaImage, x: u32, y: u32, radius: u32, color: crate::utility::Rgba) {
    for draw_y in (if y > radius { y - radius } else { 0 })..=(if y < radius { y + radius } else { img.height() - 1 }) {
        for draw_x in (if x > radius { x - radius } else { 0 })..=(if x < radius { x + radius } else { img.width() - 1 }) {
            let dx = draw_x as f64 - x as f64;
            let dy = draw_y as f64 - y as f64;
            if dx * dx + dy * dy <= (radius * radius).into() {
                img.put_pixel(draw_x as u32, draw_y as u32, image::Rgba([color.r, color.g, color.b, color.a]));
            }
        }
    }
}