
pub fn draw_rect(img: &mut image::RgbaImage, x: u32, y: u32, width: u32, height: u32, color: super::utility::Rgba) {
    for mx in 0..width {
        for my in 0..height {
            if mx + x < img.width() && my + y < img.height() {
                img.put_pixel(mx + x, my + y, image::Rgba([color.r, color.g, color.b, color.a]));
            }
        }
    }
}