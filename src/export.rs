//! Image output: encode the canvas to PNG, JPEG or WebP bytes.

use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use crate::utility::Rgba;
use crate::Ghe2dError;

/// Maximum side length WebP supports.
const WEBP_MAX_DIMENSION: u32 = 16383;

/// Target format and its per-format parameters.
#[derive(Clone, Copy, Debug)]
pub enum ExportFormat {
    Png {
        compression: CompressionType,
        filter: FilterType,
    },
    /// JPEG has no alpha channel — transparency is flattened over `background`.
    Jpeg {
        /// 1..=100 (clamped).
        quality: u8,
        background: Rgba,
    },
    WebP {
        /// 0.0..=100.0.
        quality: f32,
    },
}

/// What [`Ghe2d::encode`](crate::Ghe2d::encode) / [`Ghe2d::save`](crate::Ghe2d::save) write.
#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    format: ExportFormat,
}

impl ExportOptions {
    pub fn new(format: ExportFormat) -> Self {
        Self { format }
    }

    /// PNG, `CompressionType::Default` + `FilterType::Adaptive`.
    pub fn png() -> Self {
        Self::new(ExportFormat::Png {
            compression: CompressionType::Default,
            filter: FilterType::Adaptive,
        })
    }

    /// JPEG at `quality` (1..=100), transparency flattened over opaque white.
    pub fn jpeg(quality: u8) -> Self {
        Self::new(ExportFormat::Jpeg {
            quality,
            background: Rgba::new(255, 255, 255, 255),
        })
    }

    /// WebP at `quality` (0.0..=100.0).
    pub fn webp(quality: f32) -> Self {
        Self::new(ExportFormat::WebP { quality })
    }

    /// Set the JPEG flatten colour. No-op for other formats.
    pub fn background(mut self, bg: Rgba) -> Self {
        if let ExportFormat::Jpeg { background, .. } = &mut self.format {
            *background = bg;
        }
        self
    }

    /// Set the PNG compression. No-op for other formats.
    pub fn compression(mut self, c: CompressionType) -> Self {
        if let ExportFormat::Png { compression, .. } = &mut self.format {
            *compression = c;
        }
        self
    }

    /// Set the PNG filter. No-op for other formats.
    pub fn filter(mut self, f: FilterType) -> Self {
        if let ExportFormat::Png { filter, .. } = &mut self.format {
            *filter = f;
        }
        self
    }
}

pub fn encode(img: &RgbaImage, opts: &ExportOptions) -> Result<Vec<u8>, Ghe2dError> {
    match opts.format {
        ExportFormat::Png { compression, filter } => encode_png(img, compression, filter),
        ExportFormat::Jpeg { quality, background } => encode_jpeg(img, quality, background),
        ExportFormat::WebP { quality } => encode_webp(img, quality),
    }
}

fn encode_png(
    img: &RgbaImage,
    compression: CompressionType,
    filter: FilterType,
) -> Result<Vec<u8>, Ghe2dError> {
    if img.width() == 0 || img.height() == 0 {
        return Err(Ghe2dError::PngEncode);
    }
    let mut out: Vec<u8> = Vec::new();
    PngEncoder::new_with_quality(Cursor::new(&mut out), compression, filter)
        .write_image(img, img.width(), img.height(), ExtendedColorType::Rgba8)
        .map_err(|_| Ghe2dError::PngEncode)?;
    Ok(out)
}

fn encode_jpeg(img: &RgbaImage, quality: u8, background: Rgba) -> Result<Vec<u8>, Ghe2dError> {
    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err(Ghe2dError::JpegEncode);
    }

    // JPEG is RGB only: composite every pixel over `background`.
    let mut rgb: Vec<u8> = Vec::with_capacity(w as usize * h as usize * 3);
    for px in img.pixels() {
        let a = px.0[3] as f32 / 255.0;
        let ia = 1.0 - a;
        let mix = |ch: u8, bg: u8| (ch as f32 * a + bg as f32 * ia).round().clamp(0.0, 255.0) as u8;
        rgb.push(mix(px.0[0], background.r));
        rgb.push(mix(px.0[1], background.g));
        rgb.push(mix(px.0[2], background.b));
    }

    let mut out: Vec<u8> = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(
        Cursor::new(&mut out),
        quality.clamp(1, 100),
    );
    enc.encode(&rgb, w, h, ExtendedColorType::Rgb8)
        .map_err(|_| Ghe2dError::JpegEncode)?;
    Ok(out)
}

fn encode_webp(img: &RgbaImage, quality: f32) -> Result<Vec<u8>, Ghe2dError> {
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

        // `WebPEncodeBGRA` returns 0 and leaves `out_buf` null on failure.
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
