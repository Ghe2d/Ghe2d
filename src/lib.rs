pub mod text;
pub mod load_image;
pub mod utility;
pub mod font;
pub mod buffer;
pub mod rect;
pub mod circle;

use std::fs::File;
use std::io::copy;
use std::io::Cursor;

use image::codecs::png::{CompressionType, FilterType};

pub use rusttype;
pub use rustybuzz;
pub use unicode_bidi;
pub use regex;
pub use image;
pub use libwebp_sys;
pub use png;
pub use reqwest;
pub use num_integer;

pub use text::{LineLayout, TextAlign, TextOptions};

/// Load/decode failures produced by the internal image helpers.
#[derive(Clone, Debug)]
pub enum ErrorMessage {
    BufferEmpty,
    FailedLoadImage,
    FailedDecodeImage,
}

/// Crate-wide error type. Every fallible public entry point returns this.
///
/// Not `Clone`: it can carry a [`std::io::Error`].
#[derive(Debug)]
pub enum Ghe2dError {
    /// An image could not be loaded or decoded.
    Image(ErrorMessage),
    /// libwebp's encoder returned 0 / a null buffer.
    WebpEncode,
    /// A side length exceeds WebP's hard limit (16383 px).
    ImageTooLarge,
    /// The PNG encoder rejected the image (e.g. a zero-sized canvas).
    PngEncode,
    /// A font file was unreadable or not a valid font.
    FontLoad,
    /// Text could not be shaped into valid output (reserved; shaping is lossy).
    InvalidText,
    /// A filesystem / IO error, e.g. from `save_with_*`.
    Io(std::io::Error),
}

impl std::fmt::Display for Ghe2dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ghe2dError::Image(e) => write!(f, "image load/decode error: {e:?}"),
            Ghe2dError::WebpEncode => write!(f, "WebP encoding failed"),
            Ghe2dError::ImageTooLarge => {
                write!(f, "image dimension exceeds WebP's 16383 px limit")
            }
            Ghe2dError::PngEncode => write!(f, "PNG encoding failed"),
            Ghe2dError::FontLoad => write!(f, "failed to load font"),
            Ghe2dError::InvalidText => write!(f, "text could not be encoded"),
            Ghe2dError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for Ghe2dError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Ghe2dError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ErrorMessage> for Ghe2dError {
    fn from(e: ErrorMessage) -> Self {
        Ghe2dError::Image(e)
    }
}

impl From<std::io::Error> for Ghe2dError {
    fn from(e: std::io::Error) -> Self {
        Ghe2dError::Io(e)
    }
}

#[derive(Clone)]
pub struct Ghe2d {
    pub image: image::RgbaImage,
    width: u32,
    height: u32,
}

impl Ghe2d {
    pub fn new(width: u32, height: u32) -> Self {
        Ghe2d {
            image: image::RgbaImage::new(width, height),
            width,
            height,
        }
    }

    pub async fn new_by_load_from_path(
        path: &str,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<Self, Ghe2dError> {
        let image = load_image::load_image(path, width, height, is_circle).await?;

        Ok(Ghe2d {
            width: width.unwrap_or_else(|| image.width()),
            height: height.unwrap_or_else(|| image.height()),
            image,
        })
    }

    pub fn new_by_load_from_buffer(
        buffer: Vec<u8>,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<Self, Ghe2dError> {
        let image = buffer::load_buffer(buffer, width, height, is_circle)?;

        Ok(Ghe2d {
            width: width.unwrap_or_else(|| image.width()),
            height: height.unwrap_or_else(|| image.height()),
            image,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn save(&self, path: &str) -> Result<(), image::ImageError> {
        self.image.save(path)
    }

    pub fn save_with_png(
        &self,
        path: &str,
        compression: CompressionType,
        filter: FilterType,
    ) -> Result<(), Ghe2dError> {
        let mut dest = File::create(path)?;
        let mut content = Cursor::new(self.to_png_buffer(compression, filter)?);
        copy(&mut content, &mut dest)?;
        Ok(())
    }

    pub fn save_with_webp(&self, path: &str, quality: f32) -> Result<(), Ghe2dError> {
        let mut dest = File::create(path)?;
        let mut content = Cursor::new(self.to_webp_buffer(quality)?);
        copy(&mut content, &mut dest)?;
        Ok(())
    }

    pub fn to_png_buffer(
        &self,
        compression: CompressionType,
        filter: FilterType,
    ) -> Result<Vec<u8>, Ghe2dError> {
        buffer::image_to_png_buffer(&self.image, compression, filter)
    }

    pub fn to_webp_buffer(&self, quality: f32) -> Result<Vec<u8>, Ghe2dError> {
        buffer::image_to_webp_buffer(&self.image, quality)
    }

    pub fn draw_text(&mut self, opts: text::TextOptions) -> &Self {
        text::draw_text(&mut self.image, &opts);
        self
    }

    pub async fn load_normal_image(
        &mut self,
        path: &str,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<&mut Self, Ghe2dError> {
        load_image::add_image_normal_mut(&mut self.image, path, x, y, width, height, is_circle)
            .await?;
        Ok(self)
    }

    pub async fn load_blend_image(
        &mut self,
        path: &str,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<&mut Self, Ghe2dError> {
        load_image::add_image_blend_mut(&mut self.image, path, x, y, width, height, is_circle)
            .await?;
        Ok(self)
    }

    pub async fn load_overlay_image(
        &mut self,
        path: &str,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<&mut Self, Ghe2dError> {
        load_image::add_image_overlay_mut(&mut self.image, path, x, y, width, height, is_circle)
            .await?;
        Ok(self)
    }

    pub fn draw_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: utility::Rgba,
    ) -> &Ghe2d {
        rect::draw_rect(&mut self.image, x, y, width, height, color);
        self
    }

    pub fn draw_circle(&mut self, x: u32, y: u32, raduis: u32, color: utility::Rgba) -> &Ghe2d {
        circle::draw_circle(&mut self.image, x, y, raduis, color);
        self
    }

    pub fn from_buffer_to_overlay(
        &mut self,
        buffer: Vec<u8>,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<&mut Self, Ghe2dError> {
        buffer::load_buffer_image_overlay(&mut self.image, buffer, x, y, width, height, is_circle)?;
        Ok(self)
    }

    pub fn from_buffer_to_normal(
        &mut self,
        buffer: Vec<u8>,
        x: u32,
        y: u32,
        width: Option<u32>,
        height: Option<u32>,
        is_circle: bool,
    ) -> Result<&mut Self, Ghe2dError> {
        buffer::load_buffer_image_normal(&mut self.image, buffer, x, y, width, height, is_circle)?;
        Ok(self)
    }
}
