
pub fn draw_rect(img: &mut image::RgbaImage, x: u32, y: u32, width: u32, height: u32, color: super::utility::Rgba) {
    let (iw, ih) = img.dimensions();
    // Origin past the canvas: nothing to draw.
    if x >= iw || y >= ih {
        return;
    }

    // Clip the rect to the canvas instead of bounds-checking every pixel.
    let xw = width.min(iw - x);
    let yh = height.min(ih - y);

    for mx in 0..xw {
        for my in 0..yh {
            img.put_pixel(x + mx, y + my, image::Rgba([color.r, color.g, color.b, color.a]));
        }
    }
}
