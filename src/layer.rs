//! Groups and effects: opacity, shadow and glow for every drawable, and rotation for text and
//! images.
//!
//! A *group* draws anything onto a separate transparent layer first, then puts the layer on the
//! canvas as one piece: clipped to a box (optional), with a glow and a drop shadow made from its
//! shape (the alpha of what was drawn), rotated about the centre of the box, and faded as a whole.
//! The same machinery sits behind the `.opacity()`, `.shadow()` and `.glow()` setters of every
//! drawing option (rectangles, circles, ellipses, polygons, stars, rhombi, lines, paths, text and
//! fitted images), and behind [`Ghe2d::draw_group`](crate::Ghe2d::draw_group) for anything
//! drawn by hand.
//!
//! Blur radii follow the CSS convention: a blur of `R` px is a Gaussian with a standard
//! deviation of `R / 2`.

use image::{Rgba as Px, RgbaImage};

use crate::utility::Rgba;

/// Bounding box `(x0, y0, x1, y1)` of a drawable.
pub(crate) type Bounds = (f32, f32, f32, f32);

/// The most pixels one layer may have (a layer is a few bytes per pixel while it is worked on).
const MAX_LAYER_PIXELS: u64 = 25_000_000;
/// The largest blur radius, and the largest shadow offset, that is taken as given.
const MAX_BLUR: f32 = 500.0;
const MAX_OFFSET: f32 = 5000.0;

fn finite_or(v: f32, default: f32) -> f32 {
    if v.is_finite() {
        v
    } else {
        default
    }
}

/// A drop shadow: the shape of the drawable, shifted by an offset, blurred and coloured.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset_x: f32,
    pub offset_y: f32,
    /// Blur radius in px (CSS convention, see the module docs). `0` is a hard shadow.
    pub blur: f32,
    pub color: Rgba,
    /// `0.0..=1.0`, multiplied with the alpha of `color`.
    pub opacity: f32,
}

impl Shadow {
    /// A hard shadow of `color` at no offset; chain [`offset`](Self::offset),
    /// [`blur`](Self::blur) and [`opacity`](Self::opacity).
    pub fn new(color: Rgba) -> Self {
        Shadow {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 0.0,
            color,
            opacity: 1.0,
        }
    }
    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = radius;
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}

/// A glow: the shape of the drawable, blurred and coloured, centred on the drawable (no offset).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glow {
    /// Blur radius in px (CSS convention). A glow with no blur is the drawable's own shape.
    pub blur: f32,
    pub color: Rgba,
    /// Multiplies the blurred alpha before it is capped at 1: `1.0` is a soft halo, larger
    /// values make it denser and wider-looking. `0` shows nothing.
    pub intensity: f32,
}

impl Glow {
    /// A glow of `color` with a blur of 8 px and an intensity of 1.
    pub fn new(color: Rgba) -> Self {
        Glow {
            blur: 8.0,
            color,
            intensity: 1.0,
        }
    }
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = radius;
        self
    }
    pub fn intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

/// What a drawable gets besides its own look: a fade for the whole piece, a shadow and a glow.
/// The shadow is drawn first, then the glow, then the drawable on top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    /// `0.0..=1.0` (clamped); the shadow and the glow fade with the rest.
    pub opacity: f32,
    pub shadow: Option<Shadow>,
    pub glow: Option<Glow>,
}

impl Default for Effects {
    fn default() -> Self {
        Effects {
            opacity: 1.0,
            shadow: None,
            glow: None,
        }
    }
}

impl Effects {
    /// True when nothing changes: full opacity, no shadow and no glow.
    pub fn is_plain(&self) -> bool {
        self.shadow.is_none() && self.glow.is_none() && (self.opacity >= 1.0 || self.opacity.is_nan())
    }

    /// How far, in px, the shadow and the glow reach past the shape.
    fn extent(&self) -> f32 {
        let mut m = 0.0_f32;
        if let Some(s) = &self.shadow {
            let off = finite_or(s.offset_x, 0.0)
                .abs()
                .min(MAX_OFFSET)
                .max(finite_or(s.offset_y, 0.0).abs().min(MAX_OFFSET));
            m = m.max(off + 1.5 * finite_or(s.blur, 0.0).clamp(0.0, MAX_BLUR));
        }
        if let Some(g) = &self.glow {
            m = m.max(1.5 * finite_or(g.blur, 0.0).clamp(0.0, MAX_BLUR));
        }
        m
    }
}

/// Adds `.opacity()`, `.shadow()`, `.glow()` and `.effects()` to an options struct that has an
/// `effects: Effects` field.
macro_rules! effect_setters {
    () => {
        /// Fade the whole drawable, its shadow and its glow (`0.0..=1.0`).
        pub fn opacity(mut self, opacity: f32) -> Self {
            self.effects.opacity = opacity;
            self
        }
        /// Add a drop shadow made from the shape of the drawable.
        pub fn shadow(mut self, shadow: $crate::layer::Shadow) -> Self {
            self.effects.shadow = Some(shadow);
            self
        }
        /// Add a glow made from the shape of the drawable.
        pub fn glow(mut self, glow: $crate::layer::Glow) -> Self {
            self.effects.glow = Some(glow);
            self
        }
        /// Set opacity, shadow and glow at once.
        pub fn effects(mut self, effects: $crate::layer::Effects) -> Self {
            self.effects = effects;
            self
        }
    };
}
pub(crate) use effect_setters;

/// A box to draw into as one piece. `(x, y, width, height)` is the box on the canvas: it is the
/// pivot of the rotation (its centre) and the clip area (when [`clip`](Self::clip) is on).
#[derive(Clone, Copy, Debug)]
pub struct GroupOptions {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rotation: f32,
    effects: Effects,
    clip: bool,
    margin: f32,
    pivot: Option<(f32, f32)>,
}

impl GroupOptions {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        GroupOptions {
            x,
            y,
            width,
            height,
            rotation: 0.0,
            effects: Effects::default(),
            clip: false,
            margin: 0.0,
            pivot: None,
        }
    }
    /// Degrees, clockwise, about the centre of the box (or the [`pivot`](Self::pivot)).
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }
    /// The point, on the canvas, the rotation turns about, when it is not the centre of the box
    /// (for a clipped box that is only a part of a bigger element).
    pub fn pivot(mut self, x: f32, y: f32) -> Self {
        self.pivot = Some((x, y));
        self
    }
    /// Cut everything drawn outside the box before the effects are made.
    pub fn clip(mut self, on: bool) -> Self {
        self.clip = on;
        self
    }
    /// Extra room around the box, in px, for what is drawn past it (what lies farther is not
    /// kept; use [`clip`](Self::clip) to cut on purpose). The room the shadow and the glow need,
    /// and 2 px more, are added by themselves.
    pub fn margin(mut self, px: f32) -> Self {
        self.margin = px;
        self
    }
    effect_setters!();
}

/// Draws a group: `paint` gets a transparent layer and the position of the box's top-left corner
/// inside it (draw at `origin + your coordinates`), then the layer is put on `canvas`.
pub(crate) fn run_group(
    canvas: &mut RgbaImage,
    g: &GroupOptions,
    paint: impl FnOnce(&mut RgbaImage, f32, f32),
) {
    let (cw, ch) = canvas.dimensions();
    if cw == 0 || ch == 0 {
        return;
    }
    if !(g.x.is_finite()
        && g.y.is_finite()
        && g.width.is_finite()
        && g.height.is_finite()
        && g.width > 0.0
        && g.height > 0.0)
    {
        return;
    }
    let opacity = if g.effects.opacity.is_nan() {
        1.0
    } else {
        g.effects.opacity.clamp(0.0, 1.0)
    };
    if opacity <= 0.0 {
        return;
    }
    let rotation = finite_or(g.rotation, 0.0).rem_euclid(360.0);
    let rotated = rotation.abs() > 1e-4 && (rotation - 360.0).abs() > 1e-4;

    let m = ((g.effects.extent() + finite_or(g.margin, 0.0).clamp(0.0, 16384.0)).ceil() as i64) + 2;
    let mut rx0 = g.x.floor() as i64 - m;
    let mut ry0 = g.y.floor() as i64 - m;
    let mut rx1 = (g.x + g.width).ceil() as i64 + m;
    let mut ry1 = (g.y + g.height).ceil() as i64 + m;
    if !rotated {
        // What lies farther than the reach of the effects past the canvas cannot touch it.
        rx0 = rx0.max(-m);
        ry0 = ry0.max(-m);
        rx1 = rx1.min(cw as i64 + m);
        ry1 = ry1.min(ch as i64 + m);
    }
    if rx1 <= rx0 || ry1 <= ry0 {
        return;
    }
    let (lw, lh) = ((rx1 - rx0) as u64, (ry1 - ry0) as u64);
    if lw * lh > MAX_LAYER_PIXELS || lw > 32_768 || lh > 32_768 {
        return;
    }
    let mut layer = RgbaImage::new(lw as u32, lh as u32);
    let ox = g.x - rx0 as f32;
    let oy = g.y - ry0 as f32;
    paint(&mut layer, ox, oy);
    if g.clip {
        clip_to_box(&mut layer, ox, oy, g.width, g.height);
    }
    let layer = apply_effects(layer, &g.effects);
    let pivot = match g.pivot {
        Some((px, py)) if px.is_finite() && py.is_finite() => (px - rx0 as f32, py - ry0 as f32),
        _ => (ox + g.width / 2.0, oy + g.height / 2.0),
    };
    if rotated {
        composite_rotated(canvas, &layer, (rx0, ry0), pivot, rotation, opacity);
    } else {
        composite_plain(canvas, &layer, (rx0, ry0), opacity);
    }
}

/// Multiplies the alpha by how much of each pixel lies inside the box.
fn clip_to_box(layer: &mut RgbaImage, ox: f32, oy: f32, w: f32, h: f32) {
    let (lw, lh) = layer.dimensions();
    let cover = |i: u32, from: f32, len: f32| -> f32 {
        ((i as f32 + 1.0).min(from + len) - (i as f32).max(from)).clamp(0.0, 1.0)
    };
    let cx: Vec<f32> = (0..lw).map(|x| cover(x, ox, w)).collect();
    for y in 0..lh {
        let cy = cover(y, oy, h);
        for x in 0..lw {
            let k = cx[x as usize] * cy;
            if k >= 1.0 {
                continue;
            }
            let p = layer.get_pixel_mut(x, y);
            p.0[3] = (p.0[3] as f32 * k).round() as u8;
        }
    }
}

/// Straight-alpha "over" of the colour `(r, g, b)` at alpha `a` (0..1) onto `dst`.
#[inline]
fn over(dst: &mut Px<u8>, r: f32, g: f32, b: f32, a: f32) {
    if a <= 0.0 {
        return;
    }
    let a = a.min(1.0);
    let da = dst.0[3] as f32 / 255.0;
    let oa = a + da * (1.0 - a);
    if oa <= 0.0 {
        return;
    }
    let k = da * (1.0 - a);
    let mix = |s: f32, d: u8| ((s * a + d as f32 * k) / oa).round().clamp(0.0, 255.0) as u8;
    let out = [
        mix(r, dst.0[0]),
        mix(g, dst.0[1]),
        mix(b, dst.0[2]),
        (oa * 255.0).round().clamp(0.0, 255.0) as u8,
    ];
    dst.0 = out;
}

/// Shadow, glow, then the layer itself on top.
fn apply_effects(layer: RgbaImage, e: &Effects) -> RgbaImage {
    if e.shadow.is_none() && e.glow.is_none() {
        return layer;
    }
    let (w, h) = layer.dimensions();
    let (wu, hu) = (w as usize, h as usize);
    let alpha: Vec<f32> = layer.pixels().map(|p| p.0[3] as f32 / 255.0).collect();
    let mut out = RgbaImage::new(w, h);
    if let Some(s) = &e.shadow {
        let blur = finite_or(s.blur, 0.0).clamp(0.0, MAX_BLUR);
        let blurred = gaussian_blur(&alpha, wu, hu, blur / 2.0);
        let dx = finite_or(s.offset_x, 0.0)
            .clamp(-MAX_OFFSET, MAX_OFFSET)
            .round() as i64;
        let dy = finite_or(s.offset_y, 0.0)
            .clamp(-MAX_OFFSET, MAX_OFFSET)
            .round() as i64;
        let k = finite_or(s.opacity, 1.0).clamp(0.0, 1.0) * s.color.a as f32 / 255.0;
        let (r, g, b) = (s.color.r as f32, s.color.g as f32, s.color.b as f32);
        if k > 0.0 {
            for y in 0..hu as i64 {
                let sy = y - dy;
                if sy < 0 || sy >= hu as i64 {
                    continue;
                }
                for x in 0..wu as i64 {
                    let sx = x - dx;
                    if sx < 0 || sx >= wu as i64 {
                        continue;
                    }
                    let a = blurred[sy as usize * wu + sx as usize] * k;
                    if a > 0.0 {
                        over(out.get_pixel_mut(x as u32, y as u32), r, g, b, a);
                    }
                }
            }
        }
    }
    if let Some(gl) = &e.glow {
        let blur = finite_or(gl.blur, 0.0).clamp(0.0, MAX_BLUR);
        let blurred = gaussian_blur(&alpha, wu, hu, blur / 2.0);
        let intensity = finite_or(gl.intensity, 1.0).clamp(0.0, 64.0);
        let k = gl.color.a as f32 / 255.0;
        let (r, g, b) = (gl.color.r as f32, gl.color.g as f32, gl.color.b as f32);
        if intensity > 0.0 && k > 0.0 {
            for (i, px) in out.pixels_mut().enumerate() {
                let a = (blurred[i] * intensity).min(1.0) * k;
                if a > 0.0 {
                    over(px, r, g, b, a);
                }
            }
        }
    }
    for (src, dst) in layer.pixels().zip(out.pixels_mut()) {
        let a = src.0[3] as f32 / 255.0;
        if a > 0.0 {
            over(dst, src.0[0] as f32, src.0[1] as f32, src.0[2] as f32, a);
        }
    }
    out
}

/// Puts `layer` at `off` on the canvas, faded by `opacity`.
fn composite_plain(canvas: &mut RgbaImage, layer: &RgbaImage, off: (i64, i64), opacity: f32) {
    let (cw, ch) = canvas.dimensions();
    let (lw, lh) = layer.dimensions();
    let y0 = (-off.1).max(0);
    let y1 = (lh as i64).min(ch as i64 - off.1);
    let x0 = (-off.0).max(0);
    let x1 = (lw as i64).min(cw as i64 - off.0);
    for ly in y0..y1 {
        for lx in x0..x1 {
            let s = layer.get_pixel(lx as u32, ly as u32).0;
            if s[3] == 0 {
                continue;
            }
            let d = canvas.get_pixel_mut((lx + off.0) as u32, (ly + off.1) as u32);
            over(
                d,
                s[0] as f32,
                s[1] as f32,
                s[2] as f32,
                s[3] as f32 / 255.0 * opacity,
            );
        }
    }
}

/// Puts `layer` on the canvas turned by `deg` (clockwise) about `pivot` (layer coordinates),
/// sampling bilinearly in premultiplied colour so the edges stay clean.
fn composite_rotated(
    canvas: &mut RgbaImage,
    layer: &RgbaImage,
    off: (i64, i64),
    pivot: (f32, f32),
    deg: f32,
    opacity: f32,
) {
    let (cw, ch) = canvas.dimensions();
    let (lw, lh) = layer.dimensions();
    let (s, c) = deg.to_radians().sin_cos();
    let px = off.0 as f32 + pivot.0;
    let py = off.1 as f32 + pivot.1;
    let (mut minx, mut miny) = (f32::INFINITY, f32::INFINITY);
    let (mut maxx, mut maxy) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for (cx, cy) in [
        (0.0, 0.0),
        (lw as f32, 0.0),
        (lw as f32, lh as f32),
        (0.0, lh as f32),
    ] {
        let dx = off.0 as f32 + cx - px;
        let dy = off.1 as f32 + cy - py;
        let rx = px + dx * c - dy * s;
        let ry = py + dx * s + dy * c;
        minx = minx.min(rx);
        miny = miny.min(ry);
        maxx = maxx.max(rx);
        maxy = maxy.max(ry);
    }
    let x0 = (minx.floor().max(0.0)) as i64;
    let y0 = (miny.floor().max(0.0)) as i64;
    let x1 = (maxx.ceil().min(cw as f32)) as i64;
    let y1 = (maxy.ceil().min(ch as f32)) as i64;
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let tap = |ix: i64, iy: i64| -> Option<[f32; 4]> {
        if ix < 0 || iy < 0 || ix >= lw as i64 || iy >= lh as i64 {
            return None;
        }
        let p = layer.get_pixel(ix as u32, iy as u32).0;
        Some([p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32 / 255.0])
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as f32 + 0.5 - px;
            let dy = y as f32 + 0.5 - py;
            let u = pivot.0 + dx * c + dy * s;
            let v = pivot.1 - dx * s + dy * c;
            let fu = u - 0.5;
            let fv = v - 0.5;
            let (iu, iv) = (fu.floor(), fv.floor());
            let (tu, tv) = (fu - iu, fv - iv);
            let (iu, iv) = (iu as i64, iv as i64);
            let (mut sr, mut sg, mut sb, mut sa) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
            for (ddx, ddy, wgt) in [
                (0, 0, (1.0 - tu) * (1.0 - tv)),
                (1, 0, tu * (1.0 - tv)),
                (0, 1, (1.0 - tu) * tv),
                (1, 1, tu * tv),
            ] {
                if wgt <= 0.0 {
                    continue;
                }
                if let Some(p) = tap(iu + ddx, iv + ddy) {
                    let wa = wgt * p[3];
                    sr += p[0] * wa;
                    sg += p[1] * wa;
                    sb += p[2] * wa;
                    sa += wa;
                }
            }
            if sa > 0.0 {
                over(
                    canvas.get_pixel_mut(x as u32, y as u32),
                    sr / sa,
                    sg / sa,
                    sb / sa,
                    sa * opacity,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// blur
// ---------------------------------------------------------------------------

/// Box widths whose three passes approximate a Gaussian of `sigma`.
fn box_sizes(sigma: f32) -> [usize; 3] {
    let n = 3.0_f32;
    let ideal = (12.0 * sigma * sigma / n + 1.0).sqrt();
    let mut wl = ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    wl = wl.max(1);
    let wu = wl + 2;
    let m = ((12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n)
        / (-4.0 * wl as f32 - 4.0))
        .round() as i64;
    let mut out = [0usize; 3];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (if (i as i64) < m { wl } else { wu }) as usize;
    }
    out
}

/// Gaussian blur of a single channel (what is outside counts as 0).
pub(crate) fn gaussian_blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if w == 0 || h == 0 || sigma.is_nan() || sigma < 0.3 {
        return src.to_vec();
    }
    let mut cur = src.to_vec();
    let mut tmp = vec![0.0_f32; src.len()];
    for size in box_sizes(sigma) {
        let r = (size - 1) / 2;
        if r == 0 {
            continue;
        }
        box_horizontal(&cur, &mut tmp, w, h, r);
        box_vertical(&tmp, &mut cur, w, h, r);
    }
    cur
}

fn box_horizontal(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut dst[y * w..(y + 1) * w];
        let mut acc: f32 = row.iter().take(r + 1).sum();
        for x in 0..w {
            out[x] = acc * norm;
            if x + r + 1 < w {
                acc += row[x + r + 1];
            }
            if x >= r {
                acc -= row[x - r];
            }
        }
    }
}

fn box_vertical(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    let mut acc = vec![0.0_f32; w];
    for y in 0..=r.min(h - 1) {
        for (a, v) in acc.iter_mut().zip(&src[y * w..(y + 1) * w]) {
            *a += *v;
        }
    }
    for y in 0..h {
        for (o, a) in dst[y * w..(y + 1) * w].iter_mut().zip(&acc) {
            *o = a * norm;
        }
        if y + r + 1 < h {
            for (a, v) in acc.iter_mut().zip(&src[(y + r + 1) * w..(y + r + 2) * w]) {
                *a += *v;
            }
        }
        if y >= r {
            for (a, v) in acc.iter_mut().zip(&src[(y - r) * w..(y - r + 1) * w]) {
                *a -= *v;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// drawables
// ---------------------------------------------------------------------------

/// A drawing option that can be drawn plain, or as a group when it has effects.
pub(crate) trait Drawable: Sized {
    fn effects(&self) -> Effects;
    /// Degrees applied to the whole piece about the centre of its bounds. Shapes that turn their
    /// own geometry return 0.
    fn group_rotation(&self) -> f32 {
        0.0
    }
    fn bounds(&self) -> Option<Bounds>;
    /// Room the drawing may need past `bounds` (the overhang of glyphs).
    fn extra_margin(&self) -> f32 {
        0.0
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self;
    fn paint(&self, img: &mut RgbaImage);
}

/// Draws `d` on `img` with its effects and group rotation.
pub(crate) fn draw_effected<D: Drawable>(img: &mut RgbaImage, d: &D) {
    let Some((x0, y0, x1, y1)) = d.bounds() else {
        return;
    };
    let g = GroupOptions::new(x0, y0, x1 - x0, y1 - y0)
        .effects(d.effects())
        .rotation(d.group_rotation())
        .margin(d.extra_margin());
    run_group(img, &g, |layer, ox, oy| {
        d.shifted(ox - x0, oy - y0).paint(layer)
    });
}

/// Plain when the option has no effect and no group rotation, a group otherwise.
pub(crate) fn draw_drawable<D: Drawable>(img: &mut RgbaImage, d: &D) {
    if d.effects().is_plain() && d.group_rotation() == 0.0 {
        d.paint(img);
    } else {
        draw_effected(img, d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blur_keeps_the_total_and_spreads_symmetrically() {
        let (w, h) = (101usize, 101usize);
        let mut a = vec![0.0_f32; w * h];
        a[50 * w + 50] = 1.0;
        let b = gaussian_blur(&a, w, h, 5.0);
        let total: f32 = b.iter().sum();
        assert!((total - 1.0).abs() < 1e-3, "{total}");
        assert!(
            (b[50 * w + 45] - b[50 * w + 55]).abs() < 1e-6,
            "left and right"
        );
        assert!(
            (b[45 * w + 50] - b[55 * w + 50]).abs() < 1e-6,
            "up and down"
        );
        assert!(b[50 * w + 50] > b[50 * w + 52] && b[50 * w + 52] > b[50 * w + 58]);
        // The spread is close to a Gaussian of sigma 5: variance along a row.
        let row: f32 = (0..w).map(|x| b[50 * w + x]).sum();
        let var: f32 = (0..w)
            .map(|x| b[50 * w + x] * (x as f32 - 50.0).powi(2))
            .sum::<f32>()
            / row;
        assert!((var.sqrt() - 5.0).abs() < 0.5, "sigma {}", var.sqrt());
    }

    #[test]
    fn a_tiny_sigma_is_no_blur_and_empty_input_is_fine() {
        let a = vec![0.0, 1.0, 0.0, 0.0];
        assert_eq!(gaussian_blur(&a, 2, 2, 0.0), a);
        assert!(gaussian_blur(&[], 0, 0, 3.0).is_empty());
        assert_eq!(gaussian_blur(&a, 4, 1, f32::NAN), a);
    }

    #[test]
    fn the_box_sizes_are_odd_and_grow_with_sigma() {
        let mut last = 0;
        for s in [1.0_f32, 2.0, 4.0, 8.0, 16.0, 64.0, 250.0] {
            let sizes = box_sizes(s);
            assert!(sizes.iter().all(|z| z % 2 == 1), "{s}: {sizes:?}");
            let total: usize = sizes.iter().sum();
            assert!(total > last);
            last = total;
        }
    }

    #[test]
    fn effects_know_when_they_are_plain() {
        assert!(Effects::default().is_plain());
        assert!(!Effects {
            opacity: 0.9,
            ..Effects::default()
        }
        .is_plain());
        assert!(!Effects {
            glow: Some(Glow::new(Rgba::WHITE)),
            ..Effects::default()
        }
        .is_plain());
        assert!(Effects::default().extent() == 0.0);
        let e = Effects {
            shadow: Some(Shadow::new(Rgba::BLACK).offset(10.0, -20.0).blur(8.0)),
            ..Effects::default()
        };
        assert_eq!(e.extent(), 20.0 + 12.0);
    }
}
