
pub fn draw_rect(img: &mut image::RgbaImage, x: u32, y: u32, width: u32, height: u32, color: crate::utility::Rgba) {
    let max = width * height;
    let mut count = 0;
    loop {
        if count < max {
            let draw_x = (count % width) + x;
            let draw_y = (count / height) + y;
            if draw_x < img.width() && draw_y < img.height() {
                img.put_pixel(draw_x, draw_y, image::Rgba([color.r, color.g, color.b, color.a]));
            }
        }
        else {
            break;
        }
        count += 1;
    }
}