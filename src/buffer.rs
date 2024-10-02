use std::io::Cursor;
use image::{codecs::png::{CompressionType, FilterType, PngEncoder}, GenericImageView, ImageEncoder};

pub fn image_to_webp_buffer(img: &image::RgbaImage, quality: f32) -> Result<Vec<u8>,  libwebp_sys::WebPEncodingError> {
    let mut n_img = img.clone();
    for (x, y, pixel) in n_img.clone().enumerate_pixels() {
        n_img.put_pixel(x, y, image::Rgba([pixel.0[2], pixel.0[1], pixel.0[0], pixel.0[3]]));
    }
    unsafe {
        let mut out_buf = std::ptr::null_mut();
        let stride = (n_img.width() * 4) as i32;
        let len = libwebp_sys::WebPEncodeBGRA(
            n_img.as_ptr(),
            n_img.width() as i32,
            n_img.height() as i32,
            stride,
            quality,
            &mut out_buf
        );
        Ok(std::slice::from_raw_parts(out_buf, len).into())
    }
}

pub fn image_to_png_buffer(img: &image::RgbaImage, compression: CompressionType, filter: FilterType) -> Vec<u8> {
    let mut png: Vec<u8> = Vec::new();
    let encoder =PngEncoder::new_with_quality(Cursor::new(&mut png), compression, filter);
    encoder.write_image(&img.to_vec(), img.width(), img.height(), image::ExtendedColorType::Rgba8).unwrap();
    png
}

pub fn load_buffer_image(img: &mut image::RgbaImage, buffer: Vec<u8>, x: u32, y: u32, width: u32, height: u32, is_circle: bool) {
    let load_image = image::load_from_memory(&buffer).expect("Failed to decode image");
    let resize_image = 
        image::imageops::resize(&load_image, width as u32, height as u32, image::imageops::FilterType::Nearest);
    
    if is_circle {
        let cx = width / 2;
        let cy = height / 2;
        let radius = width.min(height) / 2;
        let mut circle_img: image::RgbaImage = image::RgbaImage::new(width as u32, height as u32);
        for x in 0..width as u32 {
            for y in 0..height as u32 {
                let dx = x - cx;
                let dy = y - cy;
                let dist = num_integer::Roots::sqrt(&(dx * dx + dy * dy)) as u32;

                let pixel = resize_image.get_pixel(x, y);

                if dist < radius as u32 {
                    circle_img.put_pixel(x, y, *pixel);
                }
            }
        }
        for (draw_x, draw_y, pixel,) in circle_img.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if pixel != &image::Rgba([0,0,0,0]) {
                super::rect::draw_rect(img, draw_x + x, draw_y + y, 1, 1, super::utility::Rgba::new(r, g, b, a));
            }
        }
    }
    else {
        for (draw_x, draw_y, pixel) in resize_image.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if pixel != &image::Rgba([0,0,0,0]) {
                super::rect::draw_rect(img, draw_x + x, draw_y + y, 1, 1, super::utility::Rgba::new(r, g, b, a));
            }
        }
    }
}
