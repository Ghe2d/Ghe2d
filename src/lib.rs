pub mod text;
pub mod image_io;
pub mod export;
pub mod utility;
pub mod font;
pub mod shapes;
pub mod anim;
pub mod layer;
pub mod fit;

pub use rusttype;
pub use rustybuzz;
pub use unicode_bidi;
pub use regex;
pub use image;
pub use libwebp_sys;
pub use png;
pub use reqwest;

pub use text::{measure_text, text_width, LineLayout, TextAlign, TextMeasure, TextOptions};
pub use font::{FontFamily, LoadFont};
pub use image_io::{ComposeMode, ImageOptions};
pub use export::{ExportFormat, ExportOptions};
pub use image::ImageFormat;
pub use anim::{
    crop_image, decode_animation, encode_animated_webp, resize_image, AnimError, AnimFrame, Animation,
    DecodeLimits,
};
pub use shapes::{
    CircleOptions, EllipseOptions, LineOptions, PathOptions, PolygonOptions, RectOptions,
    RhombusOptions, StarOptions, Stroke,
};
pub use layer::{Effects, Glow, GroupOptions, Shadow};
pub use fit::{FitImageOptions, ImageFit};

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
    /// The JPEG encoder rejected the image (e.g. a zero-sized canvas).
    JpegEncode,
    /// A font file was unreadable or not a valid font.
    FontLoad,
    /// A filesystem / IO error, e.g. from [`Ghe2d::save`].
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
            Ghe2dError::JpegEncode => write!(f, "JPEG encoding failed"),
            Ghe2dError::FontLoad => write!(f, "failed to load font"),
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
    /// Format the canvas was decoded from, if it came from [`Ghe2d::from_bytes`] /
    /// [`Ghe2d::from_path`].
    format: Option<image::ImageFormat>,
}

impl Ghe2d {
    pub fn new(width: u32, height: u32) -> Self {
        Ghe2d {
            image: image::RgbaImage::new(width, height),
            width,
            height,
            format: None,
        }
    }

    /// Build a canvas from encoded image bytes. The format is auto-detected from the
    /// bytes (PNG / JPEG / WebP / GIF / BMP / TIFF / …) and reported by
    /// [`Ghe2d::source_format`]. `ImageOptions::{at, compose}` are ignored here.
    pub fn from_bytes(bytes: Vec<u8>, opts: ImageOptions) -> Result<Self, Ghe2dError> {
        let (image, format) = image_io::decode_and_transform(&bytes, &opts)?;
        Ok(Ghe2d {
            width: image.width(),
            height: image.height(),
            image,
            format: Some(format),
        })
    }

    /// Build a canvas from a local file path or an `http(s)://` URL. Format is
    /// auto-detected from the fetched bytes.
    pub async fn from_path(path: &str, opts: ImageOptions) -> Result<Self, Ghe2dError> {
        let bytes = image_io::fetch_bytes(path).await?;
        Self::from_bytes(bytes, opts)
    }

    /// Format the canvas was decoded from, or `None` for [`Ghe2d::new`].
    pub fn source_format(&self) -> Option<image::ImageFormat> {
        self.format
    }

    /// `Some(format)` if `bytes` starts with a recognizable image signature.
    pub fn detect_format(bytes: &[u8]) -> Option<image::ImageFormat> {
        image_io::detect_format(bytes)
    }

    /// Wraps an existing RGBA image as a canvas (no format is recorded).
    pub fn from_image(image: image::RgbaImage) -> Self {
        Ghe2d { width: image.width(), height: image.height(), image, format: None }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Encode the canvas to bytes in the format described by `opts`.
    pub fn encode(&self, opts: ExportOptions) -> Result<Vec<u8>, Ghe2dError> {
        export::encode(&self.image, &opts)
    }

    /// Encode the canvas and write it to `path`.
    pub fn save(&self, path: &str, opts: ExportOptions) -> Result<(), Ghe2dError> {
        let bytes = self.encode(opts)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn draw_text(&mut self, opts: text::TextOptions) -> &Self {
        text::draw_text(&mut self.image, &opts);
        self
    }

    /// Decode `bytes` and composite the result onto this canvas per `opts`
    /// (`at` = destination, `compose` = blend mode).
    pub fn draw_image_bytes(
        &mut self,
        bytes: Vec<u8>,
        opts: ImageOptions,
    ) -> Result<&mut Self, Ghe2dError> {
        let (src, _format) = image_io::decode_and_transform(&bytes, &opts)?;
        image_io::composite(&mut self.image, &src, opts.x, opts.y, opts.compose);
        Ok(self)
    }

    /// Fetch a local file / `http(s)://` URL and composite it onto this canvas per `opts`.
    pub async fn draw_image_path(
        &mut self,
        path: &str,
        opts: ImageOptions,
    ) -> Result<&mut Self, Ghe2dError> {
        let bytes = image_io::fetch_bytes(path).await?;
        self.draw_image_bytes(bytes, opts)
    }

    pub fn draw_rect(&mut self, opts: shapes::RectOptions) -> &mut Self {
        shapes::draw_rect(&mut self.image, &opts);
        self
    }

    pub fn draw_circle(&mut self, opts: shapes::CircleOptions) -> &mut Self {
        shapes::draw_circle(&mut self.image, &opts);
        self
    }

    /// Regular N-gon (default 3 sides = an equilateral triangle).
    pub fn draw_polygon(&mut self, opts: shapes::PolygonOptions) -> &mut Self {
        shapes::draw_polygon(&mut self.image, &opts);
        self
    }

    pub fn draw_star(&mut self, opts: shapes::StarOptions) -> &mut Self {
        shapes::draw_star(&mut self.image, &opts);
        self
    }

    pub fn draw_rhombus(&mut self, opts: shapes::RhombusOptions) -> &mut Self {
        shapes::draw_rhombus(&mut self.image, &opts);
        self
    }

    pub fn draw_line(&mut self, opts: shapes::LineOptions) -> &mut Self {
        shapes::draw_line(&mut self.image, &opts);
        self
    }

    pub fn draw_ellipse(&mut self, opts: shapes::EllipseOptions) -> &mut Self {
        shapes::draw_ellipse(&mut self.image, &opts);
        self
    }

    /// A polygon or polyline through any list of points.
    pub fn draw_path(&mut self, opts: shapes::PathOptions) -> &mut Self {
        shapes::draw_path(&mut self.image, &opts);
        self
    }

    /// Draw an already decoded image into a box (cover / contain / fill), with rotation,
    /// opacity, shadow and glow from `opts`.
    pub fn draw_image_fit(&mut self, src: &image::RgbaImage, opts: fit::FitImageOptions) -> &mut Self {
        layer::draw_drawable(&mut self.image, &fit::FitDraw { src, opts });
        self
    }

    /// Decode `bytes` (any supported format) and draw it into a box, like
    /// [`draw_image_fit`](Self::draw_image_fit).
    pub fn draw_image_fit_bytes(
        &mut self,
        bytes: &[u8],
        opts: fit::FitImageOptions,
    ) -> Result<&mut Self, Ghe2dError> {
        if bytes.is_empty() {
            return Err(Ghe2dError::Image(ErrorMessage::BufferEmpty));
        }
        let src = image::load_from_memory(bytes)
            .map_err(|_| Ghe2dError::Image(ErrorMessage::FailedDecodeImage))?
            .to_rgba8();
        Ok(self.draw_image_fit(&src, opts))
    }

    /// Draw by hand into a box as one piece. `paint` gets a transparent canvas and the position
    /// `(ox, oy)` of the box's top-left corner on it (draw at `ox + x`, `oy + y`); what it
    /// draws is then clipped to the box (when `opts` says so), given the glow and the shadow
    /// from its own shape, turned about the centre of the box, faded, and put on this canvas.
    ///
    /// ```
    /// use ghe2d::{Ghe2d, GroupOptions, RectOptions, Shadow};
    /// use ghe2d::utility::Rgba;
    ///
    /// let mut c = Ghe2d::new(200, 120);
    /// c.draw_group(
    ///     GroupOptions::new(40.0, 30.0, 120.0, 60.0)
    ///         .rotation(10.0)
    ///         .opacity(0.9)
    ///         .shadow(Shadow::new(Rgba::new(0, 0, 0, 160)).offset(4.0, 6.0).blur(10.0)),
    ///     |layer, ox, oy| {
    ///         layer.draw_rect(RectOptions::solid(ox, oy, 120.0, 60.0, Rgba::rgb(240, 90, 60)));
    ///     },
    /// );
    /// ```
    pub fn draw_group(
        &mut self,
        opts: layer::GroupOptions,
        paint: impl FnOnce(&mut Ghe2d, f32, f32),
    ) -> &mut Self {
        layer::run_group(&mut self.image, &opts, |layer_img, ox, oy| {
            let taken = std::mem::replace(layer_img, image::RgbaImage::new(0, 0));
            let mut g = Ghe2d::from_image(taken);
            paint(&mut g, ox, oy);
            *layer_img = g.image;
        });
        self
    }
}
