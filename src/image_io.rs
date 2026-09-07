//! Image input: decode (format auto-detected from the bytes), optionally resize and
//! circle-crop, and composite onto a canvas.

use std::sync::LazyLock;

use image::{ImageFormat, RgbaImage};
use regex::Regex;

use crate::utility::Rgba;
use crate::ErrorMessage;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"http(s)?://([/|.|\w|\s|%|-])*").expect("valid static regex"));

/// How a loaded image is combined with the destination canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ComposeMode {
    /// Replace destination pixels outright (`image::imageops::replace`).
    #[default]
    Normal,
    /// Alpha-composite the source over the destination (`image::imageops::overlay`).
    Overlay,
    /// Per-pixel [`Rgba::blend`], skipping fully transparent source pixels.
    Blend,
}

/// Options for every image-input entry point on [`Ghe2d`](crate::Ghe2d).
///
/// `at` / `compose` are ignored by the constructors
/// ([`Ghe2d::from_bytes`](crate::Ghe2d::from_bytes) /
/// [`Ghe2d::from_path`](crate::Ghe2d::from_path)) — they only matter when compositing onto
/// an existing canvas.
#[derive(Clone)]
pub struct ImageOptions {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) is_circle: bool,
    pub(crate) compose: ComposeMode,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageOptions {
    pub fn new() -> Self {
        Self {
            x: 0,
            y: 0,
            width: None,
            height: None,
            is_circle: false,
            compose: ComposeMode::Normal,
        }
    }

    /// Destination top-left when compositing.
    pub fn at(mut self, x: u32, y: u32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Resize the loaded image to exactly `w x h`.
    pub fn size(mut self, w: u32, h: u32) -> Self {
        self.width = Some(w);
        self.height = Some(h);
        self
    }

    pub fn width(mut self, w: u32) -> Self {
        self.width = Some(w);
        self
    }

    pub fn height(mut self, h: u32) -> Self {
        self.height = Some(h);
        self
    }

    /// Crop the loaded image to a centred circle.
    pub fn circle(mut self, on: bool) -> Self {
        self.is_circle = on;
        self
    }

    pub fn compose(mut self, mode: ComposeMode) -> Self {
        self.compose = mode;
        self
    }
}

pub fn check_is_url_image(path: &str) -> bool {
    URL_RE.is_match(path)
}

/// `Some(format)` if `bytes` starts with a recognizable image signature.
pub fn detect_format(bytes: &[u8]) -> Option<ImageFormat> {
    image::guess_format(bytes).ok()
}

/// Fetch the raw bytes behind `path` — an `http(s)://` URL or a local file.
pub async fn fetch_bytes(path: &str) -> Result<Vec<u8>, ErrorMessage> {
    if check_is_url_image(path) {
        let resp = reqwest::get(path)
            .await
            .map_err(|_| ErrorMessage::FailedLoadImage)?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|_| ErrorMessage::FailedLoadImage)?;
        Ok(bytes.to_vec())
    } else {
        std::fs::read(path).map_err(|_| ErrorMessage::FailedLoadImage)
    }
}

/// Decode `bytes`, then resize / circle-crop per `opts`. Returns the pixels and the
/// detected source format.
pub fn decode_and_transform(
    bytes: &[u8],
    opts: &ImageOptions,
) -> Result<(RgbaImage, ImageFormat), ErrorMessage> {
    if bytes.is_empty() {
        return Err(ErrorMessage::BufferEmpty);
    }
    let format = image::guess_format(bytes).map_err(|_| ErrorMessage::FailedDecodeImage)?;
    let decoded = image::load_from_memory(bytes).map_err(|_| ErrorMessage::FailedDecodeImage)?;
    Ok((transform(&decoded, opts), format))
}

/// Resize to `opts` dimensions (nearest) and, if requested, mask to a centred circle.
fn transform(img: &image::DynamicImage, opts: &ImageOptions) -> RgbaImage {
    let width = opts.width.unwrap_or_else(|| img.width());
    let height = opts.height.unwrap_or_else(|| img.height());

    let resized: RgbaImage = if width == img.width() && height == img.height() {
        img.to_rgba8()
    } else {
        image::imageops::resize(img, width, height, image::imageops::FilterType::Nearest)
    };

    if !opts.is_circle || width == 0 || height == 0 {
        return resized;
    }

    let cx = width as i64 / 2;
    let cy = height as i64 / 2;
    let radius = (width.min(height) / 2) as i64;
    let r2 = radius * radius;
    let mut circle = RgbaImage::new(width, height);
    for x in 0..width {
        for y in 0..height {
            let dx = x as i64 - cx;
            let dy = y as i64 - cy;
            if dx * dx + dy * dy < r2 {
                circle.put_pixel(x, y, *resized.get_pixel(x, y));
            }
        }
    }
    circle
}

/// Combine `src` into `dst` at `(x, y)` using `mode`. Everything outside `dst` is clipped.
pub fn composite(dst: &mut RgbaImage, src: &RgbaImage, x: u32, y: u32, mode: ComposeMode) {
    match mode {
        ComposeMode::Normal => image::imageops::replace(dst, src, x as i64, y as i64),
        ComposeMode::Overlay => image::imageops::overlay(dst, src, x as i64, y as i64),
        ComposeMode::Blend => {
            for (sx, sy, px) in src.enumerate_pixels() {
                if px.0 == [0, 0, 0, 0] {
                    continue;
                }
                let (Some(dx), Some(dy)) = (sx.checked_add(x), sy.checked_add(y)) else {
                    continue;
                };
                let Some(bp) = dst.get_pixel_checked(dx, dy) else {
                    continue;
                };
                let fg = Rgba::new(px.0[0], px.0[1], px.0[2], px.0[3]);
                let bg = Rgba::new(bp.0[0], bp.0[1], bp.0[2], bp.0[3]);
                let blended = Rgba::blend(fg, bg);
                dst.put_pixel(
                    dx,
                    dy,
                    image::Rgba([blended.0[0], blended.0[1], blended.0[2], blended.0[3]]),
                );
            }
        }
    }
}
