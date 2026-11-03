//! Animation support (added in 0.2.1):frame decoding for GIF / animated WebP, animated WebP
//! encoding, rectangular crop and quality (Lanczos3) resize for stills and for every frame.
//!
//! Everything here is additive and has its own error type, [`AnimError`], so the 0.2.0
//! [`crate::Ghe2dError`] enum is untouched.

use std::io::Cursor;

use image::codecs::gif::GifDecoder;
use image::codecs::webp::WebPDecoder;
use image::imageops::{self, FilterType};
use image::{AnimationDecoder, ImageDecoder, ImageFormat, RgbaImage};

/// Maximum side length WebP supports.
const WEBP_MAX_DIMENSION: u32 = 16383;
/// Delay used when a source frame declares 0 ms (browsers do the same for GIF).
const DEFAULT_DELAY_MS: u32 = 100;

/// Failures of the animation helpers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnimError {
    /// Input empty or unreadable.
    Decode,
    /// Input format is not PNG / JPEG / GIF / WebP.
    UnsupportedFormat,
    /// More frames than `DecodeLimits::max_frames`.
    TooManyFrames { limit: u32 },
    /// Decoded pixels (all frames) would exceed `DecodeLimits::max_total_pixels`.
    TooManyPixels { limit: u64 },
    /// Total duration exceeds `DecodeLimits::max_duration_ms`.
    TooLong { limit_ms: u64 },
    /// Crop rectangle is empty or outside the image.
    InvalidCrop,
    /// Target size is zero or above WebP's 16383 px limit.
    InvalidSize,
    /// No frames, or frames of different sizes.
    BadFrames,
    /// libwebp's animation encoder failed.
    Encode,
}

impl std::fmt::Display for AnimError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnimError::Decode => write!(f, "image could not be decoded"),
            AnimError::UnsupportedFormat => write!(f, "unsupported image format"),
            AnimError::TooManyFrames { limit } => write!(f, "more than {limit} frames"),
            AnimError::TooManyPixels { limit } => write!(f, "more than {limit} decoded pixels"),
            AnimError::TooLong { limit_ms } => write!(f, "animation longer than {limit_ms} ms"),
            AnimError::InvalidCrop => write!(f, "invalid crop rectangle"),
            AnimError::InvalidSize => write!(f, "invalid target size"),
            AnimError::BadFrames => write!(f, "no frames, or frames of different sizes"),
            AnimError::Encode => write!(f, "animated WebP encoding failed"),
        }
    }
}

impl std::error::Error for AnimError {}

/// Safety limits for [`decode_animation`]; the caller chooses them.
#[derive(Clone, Copy, Debug)]
pub struct DecodeLimits {
    pub max_frames: u32,
    /// Sum of `width * height` over all frames after decompression.
    pub max_total_pixels: u64,
    /// Sum of frame delays.
    pub max_duration_ms: u64,
}

/// One frame with the time it is shown.
#[derive(Clone, Debug)]
pub struct AnimFrame {
    pub image: RgbaImage,
    pub delay_ms: u32,
}

/// A decoded image: one frame for stills, several for GIF / animated WebP.
#[derive(Clone, Debug)]
pub struct Animation {
    pub frames: Vec<AnimFrame>,
    /// 0 = loop forever.
    pub loop_count: u32,
}

/// Rectangular crop of one image.
pub fn crop_image(img: &RgbaImage, x: u32, y: u32, w: u32, h: u32) -> Result<RgbaImage, AnimError> {
    let right = x.checked_add(w).ok_or(AnimError::InvalidCrop)?;
    let bottom = y.checked_add(h).ok_or(AnimError::InvalidCrop)?;
    if w == 0 || h == 0 || right > img.width() || bottom > img.height() {
        return Err(AnimError::InvalidCrop);
    }
    Ok(imageops::crop_imm(img, x, y, w, h).to_image())
}

/// High-quality (Lanczos3) resize of one image to exactly `w` x `h`.
pub fn resize_image(img: &RgbaImage, w: u32, h: u32) -> Result<RgbaImage, AnimError> {
    if w == 0 || h == 0 || w > WEBP_MAX_DIMENSION || h > WEBP_MAX_DIMENSION {
        return Err(AnimError::InvalidSize);
    }
    Ok(imageops::resize(img, w, h, FilterType::Lanczos3))
}

/// Decode PNG / JPEG / GIF / WebP (still or animated) into frames, enforcing `limits`
/// while decoding: the canvas size is checked before any frame is decoded, and the frame
/// count, pixel total and duration are checked as each frame arrives (at most one frame
/// beyond a limit is ever held in memory).
pub fn decode_animation(bytes: &[u8], limits: &DecodeLimits) -> Result<Animation, AnimError> {
    if bytes.is_empty() {
        return Err(AnimError::Decode);
    }
    let fmt = image::guess_format(bytes).map_err(|_| AnimError::UnsupportedFormat)?;
    match fmt {
        ImageFormat::Gif => {
            let dec = GifDecoder::new(Cursor::new(bytes)).map_err(|_| AnimError::Decode)?;
            collect(dec, limits)
        }
        ImageFormat::WebP if webp_is_animated(bytes) => {
            let dec = WebPDecoder::new(Cursor::new(bytes)).map_err(|_| AnimError::Decode)?;
            collect(dec, limits)
        }
        // Still WebP and the other still formats: canvas check, then one frame.
        ImageFormat::WebP | ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp => {
            let reader = image::ImageReader::with_format(Cursor::new(bytes), fmt);
            let (w, h) = reader.into_dimensions().map_err(|_| AnimError::Decode)?;
            check_canvas(w, h, limits)?;
            let img = image::load_from_memory_with_format(bytes, fmt)
                .map_err(|_| AnimError::Decode)?
                .to_rgba8();
            Ok(Animation {
                frames: vec![AnimFrame { image: img, delay_ms: 0 }],
                loop_count: 0,
            })
        }
        _ => Err(AnimError::UnsupportedFormat),
    }
}

/// True when the RIFF/WebP container is extended (`VP8X`) with the animation flag set.
fn webp_is_animated(bytes: &[u8]) -> bool {
    bytes.len() > 20 && &bytes[12..16] == b"VP8X" && bytes[20] & 0x02 != 0
}

fn check_canvas(w: u32, h: u32, limits: &DecodeLimits) -> Result<(), AnimError> {
    if limits.max_frames == 0 {
        return Err(AnimError::TooManyFrames { limit: 0 });
    }
    if (w as u64) * (h as u64) > limits.max_total_pixels {
        return Err(AnimError::TooManyPixels { limit: limits.max_total_pixels });
    }
    Ok(())
}

fn collect<'a, D>(dec: D, limits: &DecodeLimits) -> Result<Animation, AnimError>
where
    D: ImageDecoder + AnimationDecoder<'a>,
{
    let (w, h) = dec.dimensions();
    check_canvas(w, h, limits)?;
    let per_frame = (w as u64) * (h as u64);

    let mut frames: Vec<AnimFrame> = Vec::new();
    let mut pixels: u64 = 0;
    let mut duration: u64 = 0;
    for item in dec.into_frames() {
        let frame = item.map_err(|_| AnimError::Decode)?;
        if frames.len() as u32 >= limits.max_frames {
            return Err(AnimError::TooManyFrames { limit: limits.max_frames });
        }
        pixels = pixels.saturating_add(per_frame);
        if pixels > limits.max_total_pixels {
            return Err(AnimError::TooManyPixels { limit: limits.max_total_pixels });
        }
        let (n, d) = frame.delay().numer_denom_ms();
        let mut delay = if d == 0 { 0 } else { ((n as u64 + d as u64 / 2) / d as u64) as u32 };
        if delay == 0 {
            delay = DEFAULT_DELAY_MS;
        }
        duration = duration.saturating_add(delay as u64);
        if duration > limits.max_duration_ms {
            return Err(AnimError::TooLong { limit_ms: limits.max_duration_ms });
        }
        frames.push(AnimFrame { image: frame.into_buffer(), delay_ms: delay });
    }
    if frames.is_empty() {
        return Err(AnimError::Decode);
    }
    Ok(Animation { frames, loop_count: 0 })
}

impl Animation {
    pub fn width(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.image.width())
    }

    pub fn height(&self) -> u32 {
        self.frames.first().map_or(0, |f| f.image.height())
    }

    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.frames.iter().map(|f| f.delay_ms as u64).sum()
    }

    /// Crop every frame to the same rectangle.
    pub fn crop(&self, x: u32, y: u32, w: u32, h: u32) -> Result<Animation, AnimError> {
        let mut frames = Vec::with_capacity(self.frames.len());
        for f in &self.frames {
            frames.push(AnimFrame { image: crop_image(&f.image, x, y, w, h)?, delay_ms: f.delay_ms });
        }
        Ok(Animation { frames, loop_count: self.loop_count })
    }

    /// Lanczos3-resize every frame to `w` x `h`.
    pub fn resize(&self, w: u32, h: u32) -> Result<Animation, AnimError> {
        let mut frames = Vec::with_capacity(self.frames.len());
        for f in &self.frames {
            frames.push(AnimFrame { image: resize_image(&f.image, w, h)?, delay_ms: f.delay_ms });
        }
        Ok(Animation { frames, loop_count: self.loop_count })
    }

    /// Encode as an animated WebP (lossy, `quality` 0.0..=100.0). Frames must be non-empty
    /// and all the same size. A one-frame animation yields a one-frame animated WebP.
    pub fn encode_webp(&self, quality: f32) -> Result<Vec<u8>, AnimError> {
        encode_animated_webp(self, quality)
    }
}

/// Encode `anim` as an animated WebP with `WebPAnimEncoder`.
pub fn encode_animated_webp(anim: &Animation, quality: f32) -> Result<Vec<u8>, AnimError> {
    let (w, h) = (anim.width(), anim.height());
    if anim.frames.is_empty() || anim.frames.iter().any(|f| f.image.dimensions() != (w, h)) {
        return Err(AnimError::BadFrames);
    }
    if w == 0 || h == 0 || w > WEBP_MAX_DIMENSION || h > WEBP_MAX_DIMENSION {
        return Err(AnimError::InvalidSize);
    }
    use libwebp_sys as ws;

    unsafe {
        let mut config = ws::WebPConfig::new().map_err(|_| AnimError::Encode)?;
        config.quality = quality.clamp(0.0, 100.0);
        if ws::WebPValidateConfig(&config) == 0 {
            return Err(AnimError::Encode);
        }

        let mut opts: ws::WebPAnimEncoderOptions = std::mem::zeroed();
        if ws::WebPAnimEncoderOptionsInitInternal(&mut opts, ws::WEBP_MUX_ABI_VERSION as _) == 0 {
            return Err(AnimError::Encode);
        }
        // Let each frame (sub-frame) pick lossy or lossless. All-lossy leaves ghosts of earlier
        // frames on flat backgrounds because lossy sub-frame errors do not cancel on blending.
        opts.allow_mixed = 1;
        // A key frame every 6 frames stops the small lossy/lossless drift from piling up.
        opts.kmin = 5;
        opts.kmax = 6;
        opts.anim_params.loop_count = anim.loop_count.min(65535) as _;

        let enc = ws::WebPAnimEncoderNewInternal(w as _, h as _, &opts, ws::WEBP_MUX_ABI_VERSION as _);
        if enc.is_null() {
            return Err(AnimError::Encode);
        }

        let result = (|| {
            let mut ts: u64 = 0;
            for f in &anim.frames {
                let mut pic = ws::WebPPicture::new().map_err(|_| AnimError::Encode)?;
                pic.use_argb = 1;
                pic.width = w as _;
                pic.height = h as _;
                if ws::WebPPictureImportRGBA(&mut pic, f.image.as_ptr(), (w * 4) as _) == 0 {
                    ws::WebPPictureFree(&mut pic);
                    return Err(AnimError::Encode);
                }
                let ok = ws::WebPAnimEncoderAdd(enc, &mut pic, ts.min(i32::MAX as u64) as _, &config);
                ws::WebPPictureFree(&mut pic);
                if ok == 0 {
                    return Err(AnimError::Encode);
                }
                ts += f.delay_ms.max(1) as u64;
            }
            // A NULL picture marks the end time of the last frame.
            let end = ts.min(i32::MAX as u64) as _;
            if ws::WebPAnimEncoderAdd(enc, std::ptr::null_mut(), end, std::ptr::null()) == 0 {
                return Err(AnimError::Encode);
            }
            let mut data = ws::WebPData::default();
            if ws::WebPAnimEncoderAssemble(enc, &mut data) == 0 || data.bytes.is_null() {
                return Err(AnimError::Encode);
            }
            let out = std::slice::from_raw_parts(data.bytes, data.size).to_vec();
            ws::WebPDataClear(&mut data);
            Ok(out)
        })();

        ws::WebPAnimEncoderDelete(enc);
        result
    }
}
