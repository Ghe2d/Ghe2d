use std::io::Cursor;
use image::{codecs::png::{CompressionType, FilterType, PngEncoder}, ImageEncoder};

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

pub fn load_buffer_image_overlay(img: &mut image::RgbaImage, buffer: Vec<u8>, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage>  {
    let load_image = load_buffer(buffer, width, height, is_circle);
    if load_image.is_err() {
        return Err(load_image.err().unwrap());
    }
    image::imageops::overlay(img, &load_image.unwrap(), x as i64, y as i64);
    Ok(())
}

pub fn load_buffer_image_normal(img: &mut image::RgbaImage, buffer: Vec<u8>, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage> {
    let load_image = load_buffer(buffer, width, height, is_circle);
    if load_image.is_err() {
        return Err(load_image.err().unwrap());
    }
    image::imageops::replace(img, &load_image.unwrap(), x as i64, y as i64);
    Ok(())
}

pub fn load_buffer(buffer: Vec<u8>, w: Option<u32>, h: Option<u32>, is_circle: bool) -> Result<image::RgbaImage, crate::ErrorMessage> {
    if buffer.is_empty() {
        return Err(crate::ErrorMessage::BufferEmpty);
    }
    let load_image = image::load_from_memory(&buffer);
    if load_image.is_err() {
        return Err(crate::ErrorMessage::FailedDecodeImage);
    }
    let load_image = load_image.unwrap();

    let r_img: image::RgbaImage;

    let width = match w {
        Some(w) => w,
        None => load_image.width(),
    };

    let height = match h {
        Some(h) => h,
        None => load_image.height(),
    };

    if width == load_image.width() && height == load_image.height() {
        r_img = load_image.to_rgba8();
    } else {
        r_img = image::imageops::resize(&load_image, width as u32, height as u32, image::imageops::FilterType::Nearest);
    }

    if is_circle {
        let cx = width / 2;
        let cy = height / 2;
        let radius = width.min(height) / 2;
        let mut circle_img = image::RgbaImage::new(width as u32, height as u32);
        
        for x in 0..width as u32 {
            for y in 0..height as u32 {
                let dx = x - cx;
                let dy = y - cy;
                let dist = num_integer::Roots::sqrt(&(dx * dx + dy * dy)) as u32;

                let pixel = r_img.get_pixel(x, y);

                if dist < radius as u32 {
                    circle_img.put_pixel(x, y, *pixel);
                }
            }
        }
        Ok(circle_img)
    } else {
        Ok(r_img)
    }
}
