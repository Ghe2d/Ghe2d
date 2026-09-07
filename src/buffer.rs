use std::io::Cursor;
use image::{codecs::png::{CompressionType, FilterType, PngEncoder}, ImageEncoder};

use crate::Ghe2dError;

/// Maximum side length WebP supports.
const WEBP_MAX_DIMENSION: u32 = 16383;

pub fn image_to_webp_buffer(img: &image::RgbaImage, quality: f32) -> Result<Vec<u8>, Ghe2dError> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(Ghe2dError::WebpEncode);
    }
    if w > WEBP_MAX_DIMENSION || h > WEBP_MAX_DIMENSION {
        return Err(Ghe2dError::ImageTooLarge);
    }

    // RGBA -> BGRA in a single pass (no second full-image clone).
    let mut bgra = img.clone();
    for px in bgra.pixels_mut() {
        px.0.swap(0, 2);
    }

    unsafe {
        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let stride = (w * 4) as i32;
        let len = libwebp_sys::WebPEncodeBGRA(
            bgra.as_ptr(),
            w as i32,
            h as i32,
            stride,
            quality,
            &mut out_buf,
        );

        // `WebPEncodeBGRA` returns 0 and leaves `out_buf` null on failure
        // (dimension 0, dimension > 16383, bad stride, internal OOM).
        if len == 0 || out_buf.is_null() {
            if !out_buf.is_null() {
                libwebp_sys::WebPFree(out_buf.cast());
            }
            return Err(Ghe2dError::WebpEncode);
        }

        let bytes = std::slice::from_raw_parts(out_buf, len).to_vec();
        libwebp_sys::WebPFree(out_buf.cast());
        Ok(bytes)
    }
}

pub fn image_to_png_buffer(
    img: &image::RgbaImage,
    compression: CompressionType,
    filter: FilterType,
) -> Result<Vec<u8>, Ghe2dError> {
    if img.width() == 0 || img.height() == 0 {
        return Err(Ghe2dError::PngEncode);
    }
    let mut png: Vec<u8> = Vec::new();
    let encoder = PngEncoder::new_with_quality(Cursor::new(&mut png), compression, filter);
    encoder
        .write_image(
            img,
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| Ghe2dError::PngEncode)?;
    Ok(png)
}

pub fn load_buffer_image_overlay(img: &mut image::RgbaImage, buffer: Vec<u8>, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage>  {
    let load_image = load_buffer(buffer, width, height, is_circle)?;
    image::imageops::overlay(img, &load_image, x as i64, y as i64);
    Ok(())
}

pub fn load_buffer_image_normal(img: &mut image::RgbaImage, buffer: Vec<u8>, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage> {
    let load_image = load_buffer(buffer, width, height, is_circle)?;
    image::imageops::replace(img, &load_image, x as i64, y as i64);
    Ok(())
}

pub fn load_buffer(buffer: Vec<u8>, w: Option<u32>, h: Option<u32>, is_circle: bool) -> Result<image::RgbaImage, crate::ErrorMessage> {
    if buffer.is_empty() {
        return Err(crate::ErrorMessage::BufferEmpty);
    }
    let load_image = image::load_from_memory(&buffer)
        .map_err(|_| crate::ErrorMessage::FailedDecodeImage)?;

    let width = w.unwrap_or_else(|| load_image.width());
    let height = h.unwrap_or_else(|| load_image.height());

    let r_img: image::RgbaImage = if width == load_image.width() && height == load_image.height() {
        load_image.to_rgba8()
    } else {
        image::imageops::resize(&load_image, width, height, image::imageops::FilterType::Nearest)
    };

    if is_circle {
        if width == 0 || height == 0 {
            return Ok(r_img);
        }

        let cx = width as i64 / 2;
        let cy = height as i64 / 2;
        let radius = (width.min(height) / 2) as i64;
        let r2 = radius * radius;
        let mut circle_img = image::RgbaImage::new(width, height);

        for x in 0..width {
            for y in 0..height {
                let dx = x as i64 - cx;
                let dy = y as i64 - cy;

                if dx * dx + dy * dy < r2 {
                    circle_img.put_pixel(x, y, *r_img.get_pixel(x, y));
                }
            }
        }
        Ok(circle_img)
    } else {
        Ok(r_img)
    }
}
