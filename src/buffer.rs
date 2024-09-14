use std::io::Cursor;

use image::{codecs::png::{CompressionType, FilterType, PngEncoder}, ImageBuffer, ImageEncoder, Rgba, RgbaImage};
use raqote::DrawTarget;

pub fn image_to_webp_buffer(dt: &DrawTarget, quality: f32) -> Result<Vec<u8>,  libwebp_sys::WebPEncodingError> {
    let mut img: ImageBuffer<Rgba<u8>, Vec<_>> = image::ImageBuffer::new(dt.width() as u32, dt.height() as u32);
    raqote_to_image_for_webp(&dt, &mut img);
    unsafe {
        let mut out_buf = std::ptr::null_mut();
        let stride = (img.width() * 4) as i32;
        let len = libwebp_sys::WebPEncodeBGRA(
            img.as_ptr(),
            img.width() as i32,
            img.height() as i32,
            stride,
            quality,
            &mut out_buf
        );
        Ok(std::slice::from_raw_parts(out_buf, len).into())
    }
}

pub fn image_to_png_buffer(dt: &DrawTarget, compression: CompressionType, filter: FilterType) -> Vec<u8> {
    let mut img: ImageBuffer<Rgba<u8>, Vec<_>> = image::ImageBuffer::new(dt.width() as u32, dt.height() as u32);
    raqote_to_image(&dt, &mut img);
    let mut png: Vec<u8> = Vec::new();
    let encoder =PngEncoder::new_with_quality(Cursor::new(&mut png), compression, filter);
    encoder.write_image(&img.to_vec(), img.width(), img.height(), image::ExtendedColorType::Rgba8).unwrap();
    png
}

pub fn raqote_to_image(dt: &DrawTarget, img: &mut RgbaImage) {
    let mut i = 0;
    for pixel in dt.get_data() {
        let a = (pixel >> 24) & 0xffu32;
        let mut r = (pixel >> 16) & 0xffu32;
        let mut g = (pixel >> 8) & 0xffu32;
        let mut b = (pixel >> 0) & 0xffu32;

        if a > 0u32 {
            r = r * 255u32 / a;
            g = g * 255u32 / a;
            b = b * 255u32 / a;
        }

        let x = i % img.width();
        let y = i / img.width();

        img.put_pixel(x, y, Rgba([r as u8, g as u8, b as u8, a as u8]));
        i += 1;
    }
}

pub fn raqote_to_image_for_webp(dt: &DrawTarget, img: &mut RgbaImage) {
    let mut i = 0;
    for pixel in dt.get_data() {
        let a = (pixel >> 24) & 0xffu32;
        let mut r = (pixel >> 0) & 0xffu32;
        let mut g = (pixel >> 8) & 0xffu32;
        let mut b = (pixel >> 16) & 0xffu32;

        if a > 0u32 {
            r = r * 255u32 / a;
            g = g * 255u32 / a;
            b = b * 255u32 / a;
        }

        let x = i % img.width();
        let y = i / img.width();

        img.put_pixel(x, y, Rgba([r as u8, g as u8, b as u8, a as u8]));
        i += 1;
    }
}