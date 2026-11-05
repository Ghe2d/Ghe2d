//! Fitting an image into a box: `Cover`, `Contain` or `Fill`, like CSS `object-fit`.
//!
//! The image is resampled once (Lanczos when it shrinks, Catmull-Rom when it grows; colour is
//! weighted by alpha so transparent edges stay clean) and only the part that can be seen is
//! resampled. Rotation, opacity, shadow and glow come with the options, like every other drawing.

use image::imageops::FilterType;
use image::RgbaImage;

use crate::layer::{effect_setters, Bounds, Drawable, Effects};

/// How an image fills its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageFit {
    /// Scale up or down until the box is covered; what overflows is cropped.
    #[default]
    Cover,
    /// Scale until the whole image is inside the box; the rest of the box stays empty.
    Contain,
    /// Stretch to the box, ignoring the proportions.
    Fill,
}

/// An image drawn into the box `(x, y, width, height)`.
#[derive(Clone, Copy, Debug)]
pub struct FitImageOptions {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    fit: ImageFit,
    position: (f32, f32),
    rotation: f32,
    effects: Effects,
}

impl FitImageOptions {
    /// The box on the canvas. Defaults: `Cover`, centred, no rotation, no effects.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        FitImageOptions {
            x,
            y,
            width,
            height,
            fit: ImageFit::Cover,
            position: (0.5, 0.5),
            rotation: 0.0,
            effects: Effects::default(),
        }
    }
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    /// Where the image sits when it does not fill the box (`Contain`) or what part of it shows
    /// (`Cover`): `0.0` start, `0.5` centre (default), `1.0` end, on each axis.
    pub fn position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }
    /// Degrees, clockwise, about the centre of the box.
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }
    effect_setters!();
}

/// An image and the options it is drawn with.
#[derive(Clone, Copy)]
pub(crate) struct FitDraw<'a> {
    pub src: &'a RgbaImage,
    pub opts: FitImageOptions,
}

impl Drawable for FitDraw<'_> {
    fn effects(&self) -> Effects {
        self.opts.effects
    }
    fn group_rotation(&self) -> f32 {
        self.opts.rotation
    }
    fn bounds(&self) -> Option<Bounds> {
        let o = &self.opts;
        let ok = o.x.is_finite() && o.y.is_finite() && o.width > 0.0 && o.height > 0.0;
        (ok && o.width.is_finite() && o.height.is_finite()).then_some((
            o.x,
            o.y,
            o.x + o.width,
            o.y + o.height,
        ))
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        FitDraw {
            src: self.src,
            opts: FitImageOptions {
                x: self.opts.x + dx,
                y: self.opts.y + dy,
                ..self.opts
            },
        }
    }
    fn paint(&self, img: &mut RgbaImage) {
        paint_fit(img, self.src, &self.opts);
    }
}

/// Most pixels one resampled piece may have.
const MAX_PIECE_PIXELS: u64 = 100_000_000;

fn paint_fit(dst: &mut RgbaImage, src: &RgbaImage, o: &FitImageOptions) {
    let (sw, sh) = (src.width() as f32, src.height() as f32);
    if sw < 1.0 || sh < 1.0 {
        return;
    }
    if !(o.x.is_finite() && o.y.is_finite() && o.width.is_finite() && o.height.is_finite())
        || o.width <= 0.0
        || o.height <= 0.0
    {
        return;
    }
    let px = if o.position.0.is_nan() {
        0.5
    } else {
        o.position.0.clamp(0.0, 1.0)
    };
    let py = if o.position.1.is_nan() {
        0.5
    } else {
        o.position.1.clamp(0.0, 1.0)
    };

    // The box the image lands in, and the part of the source it shows.
    let (dx, dy, dw, dh, sx, sy, vw, vh) = match o.fit {
        ImageFit::Fill => (o.x, o.y, o.width, o.height, 0.0, 0.0, sw, sh),
        ImageFit::Contain => {
            let k = (o.width / sw).min(o.height / sh);
            let (w, h) = (sw * k, sh * k);
            (
                o.x + (o.width - w) * px,
                o.y + (o.height - h) * py,
                w,
                h,
                0.0,
                0.0,
                sw,
                sh,
            )
        }
        ImageFit::Cover => {
            let k = (o.width / sw).max(o.height / sh);
            let (vw, vh) = (o.width / k, o.height / k);
            (
                o.x,
                o.y,
                o.width,
                o.height,
                (sw - vw) * px,
                (sh - vh) * py,
                vw,
                vh,
            )
        }
    };

    // Snap to whole pixels, then keep the part that is on the image.
    let (iw, ih) = dst.dimensions();
    let (fx0, fx1) = (dx.round(), (dx + dw).round());
    let (fy0, fy1) = (dy.round(), (dy + dh).round());
    if fx1 <= fx0 || fy1 <= fy0 {
        return;
    }
    let (cx0, cx1) = (fx0.max(0.0), fx1.min(iw as f32));
    let (cy0, cy1) = (fy0.max(0.0), fy1.min(ih as f32));
    if cx1 <= cx0 || cy1 <= cy0 {
        return;
    }
    let (out_w, out_h) = ((cx1 - cx0) as u32, (cy1 - cy0) as u32);
    if out_w as u64 * out_h as u64 > MAX_PIECE_PIXELS {
        return;
    }

    // The same range in the source.
    let kx = vw / (fx1 - fx0);
    let ky = vh / (fy1 - fy0);
    let s_x0 = sx + (cx0 - fx0) * kx;
    let s_x1 = sx + (cx1 - fx0) * kx;
    let s_y0 = sy + (cy0 - fy0) * ky;
    let s_y1 = sy + (cy1 - fy0) * ky;
    let a0 = s_x0.floor().max(0.0) as u32;
    let a1 = (s_x1.ceil().max(0.0) as u32).min(src.width());
    let b0 = s_y0.floor().max(0.0) as u32;
    let b1 = (s_y1.ceil().max(0.0) as u32).min(src.height());
    if a1 <= a0 || b1 <= b0 {
        return;
    }
    let piece = if (a0, b0, a1, b1) == (0, 0, src.width(), src.height()) {
        resample(src, out_w, out_h)
    } else {
        let cropped = image::imageops::crop_imm(src, a0, b0, a1 - a0, b1 - b0).to_image();
        resample(&cropped, out_w, out_h)
    };
    image::imageops::overlay(dst, &piece, cx0 as i64, cy0 as i64);
}

/// `img` at `w x h`. Colour is weighted by alpha while it is resampled.
pub(crate) fn resample(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if img.dimensions() == (w, h) {
        return img.clone();
    }
    let grows = (w as u64 * h as u64) > (img.width() as u64 * img.height() as u64);
    let filter = if grows {
        FilterType::CatmullRom
    } else {
        FilterType::Lanczos3
    };
    if img.pixels().all(|p| p.0[3] == 255) {
        return image::imageops::resize(img, w, h, filter);
    }
    let mut pre = img.clone();
    for p in pre.pixels_mut() {
        let a = p.0[3] as u32;
        for c in 0..3 {
            p.0[c] = ((p.0[c] as u32 * a + 127) / 255) as u8;
        }
    }
    let mut out = image::imageops::resize(&pre, w, h, filter);
    for p in out.pixels_mut() {
        let a = p.0[3] as u32;
        if a == 0 {
            p.0 = [0, 0, 0, 0];
            continue;
        }
        for c in 0..3 {
            p.0[c] = ((p.0[c] as u32 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    out
}
