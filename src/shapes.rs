//! Filled / outlined shapes: rect (square or rounded corners), circle, ellipse, regular polygon,
//! star, rhombus, free path, line.
//!
//! Geometry is `f32` and may fall outside the canvas — everything is clipped at raster
//! time. Every shape takes a `fill: Option<Rgba>` and a `stroke: Option<Stroke>`, and
//! every edge is anti-aliased (rect/circle/line analytically, polygons via 4x4
//! supersampling).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use image::RgbaImage;

use crate::layer::{draw_drawable, effect_setters, Bounds, Drawable, Effects};
use crate::utility::Rgba;

/// An outline drawn along a shape's edge.
#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    pub color: Rgba,
    pub width: f32,
}

// ---------------------------------------------------------------------------
// low-level raster primitives
// ---------------------------------------------------------------------------

/// Alpha-composite `color` over `img[x, y]`. Out-of-bounds / fully transparent is a no-op.
pub(crate) fn blend_pixel(img: &mut RgbaImage, x: i64, y: i64, color: Rgba) {
    if color.a == 0 || x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as u32, y as u32);
    let Some(dst) = img.get_pixel_checked(x, y) else {
        return;
    };
    if color.a == 255 {
        img.put_pixel(x, y, image::Rgba([color.r, color.g, color.b, 255]));
        return;
    }
    let bg = Rgba::new(dst.0[0], dst.0[1], dst.0[2], dst.0[3]);
    img.put_pixel(x, y, Rgba::blend(color, bg));
}

/// Sub-samples per axis for supersampled edge anti-aliasing (`AA x AA` grid per pixel).
const AA: u32 = 4;

/// Even-odd point-in-polygon test.
fn point_in_polygon(verts: &[(f32, f32)], px: f32, py: f32) -> bool {
    let n = verts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = verts[i];
        let (xj, yj) = verts[j];
        if (yi > py) != (yj > py) {
            let x_cross = xi + (py - yi) / (yj - yi) * (xj - xi);
            if px < x_cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn bbox_clamped(verts: &[(f32, f32)], iw: u32, ih: u32) -> (i64, i64, i64, i64) {
    let mut lo = (f32::INFINITY, f32::INFINITY);
    let mut hi = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for &(x, y) in verts {
        lo.0 = lo.0.min(x);
        lo.1 = lo.1.min(y);
        hi.0 = hi.0.max(x);
        hi.1 = hi.1.max(y);
    }
    (
        lo.0.floor().max(0.0) as i64,
        lo.1.floor().max(0.0) as i64,
        hi.0.ceil().min((iw - 1) as f32) as i64,
        hi.1.ceil().min((ih - 1) as f32) as i64,
    )
}

/// Filled polygon with supersampled anti-aliased edges (even-odd rule).
pub fn fill_polygon(img: &mut RgbaImage, verts: &[(f32, f32)], color: Rgba) {
    if verts.len() < 3 || color.a == 0 {
        return;
    }
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return;
    }

    let (min_x, min_y, max_x, max_y) = bbox_clamped(verts, iw, ih);
    let step = 1.0 / AA as f32;
    let inv = 1.0 / (AA * AA) as f32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let mut hits = 0u32;
            for sy in 0..AA {
                let py = y as f32 + (sy as f32 + 0.5) * step;
                for sx in 0..AA {
                    let px = x as f32 + (sx as f32 + 0.5) * step;
                    if point_in_polygon(verts, px, py) {
                        hits += 1;
                    }
                }
            }
            if hits > 0 {
                blend_pixel(img, x, y, scale_alpha(color, hits as f32 * inv));
            }
        }
    }
}

/// Thick line segment with round caps and a 1px anti-aliased edge.
pub fn draw_segment(img: &mut RgbaImage, a: (f32, f32), b: (f32, f32), width: f32, color: Rgba) {
    if color.a == 0 {
        return;
    }
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return;
    }

    let hw = width.max(1.0) * 0.5;
    let pad = hw + 1.0;
    let min_x = (a.0.min(b.0) - pad).floor().max(0.0) as i64;
    let max_x = (a.0.max(b.0) + pad).ceil().min((iw - 1) as f32) as i64;
    let min_y = (a.1.min(b.1) - pad).floor().max(0.0) as i64;
    let max_y = (a.1.max(b.1) + pad).ceil().min((ih - 1) as f32) as i64;

    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let dist = if len2 <= f32::EPSILON {
                ((px - a.0).powi(2) + (py - a.1).powi(2)).sqrt()
            } else {
                let t = (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0);
                let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
                ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
            };
            // 1px anti-aliased edge: full coverage inside, ramps to 0 across the last pixel
            let cov = (hw + 0.5 - dist).clamp(0.0, 1.0);
            if cov > 0.0 {
                blend_pixel(img, x, y, scale_alpha(color, cov));
            }
        }
    }
}

fn stroke_path(img: &mut RgbaImage, verts: &[(f32, f32)], closed: bool, s: Stroke) {
    let n = verts.len();
    if n < 2 {
        return;
    }
    let edges = if closed { n } else { n - 1 };
    for i in 0..edges {
        draw_segment(img, verts[i], verts[(i + 1) % n], s.width, s.color);
    }
}

// ---------------------------------------------------------------------------
// vertex generators
// ---------------------------------------------------------------------------

fn regular_polygon(cx: f32, cy: f32, r: f32, sides: u32, rot_deg: f32) -> Vec<(f32, f32)> {
    let sides = sides.max(3);
    let rot = rot_deg.to_radians() - FRAC_PI_2; // first vertex points up
    (0..sides)
        .map(|i| {
            let a = rot + i as f32 * TAU / sides as f32;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

fn star_verts(cx: f32, cy: f32, r: f32, points: u32, inner_ratio: f32, rot_deg: f32) -> Vec<(f32, f32)> {
    let points = points.max(2);
    let ri = r * inner_ratio.clamp(0.0, 1.0);
    let rot = rot_deg.to_radians() - FRAC_PI_2;
    let step = PI / points as f32;
    (0..points * 2)
        .map(|i| {
            let a = rot + i as f32 * step;
            let rad = if i % 2 == 0 { r } else { ri };
            (cx + rad * a.cos(), cy + rad * a.sin())
        })
        .collect()
}

fn rhombus_verts(cx: f32, cy: f32, w: f32, h: f32, rot_deg: f32) -> [(f32, f32); 4] {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let pts = [(cx, cy - hh), (cx + hw, cy), (cx, cy + hh), (cx - hw, cy)];
    rotate_about(pts, cx, cy, rot_deg)
}

fn rect_quad(x: f32, y: f32, w: f32, h: f32, rot_deg: f32) -> [(f32, f32); 4] {
    let pts = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    rotate_about(pts, x + w * 0.5, y + h * 0.5, rot_deg)
}

fn rotate_about(pts: [(f32, f32); 4], cx: f32, cy: f32, rot_deg: f32) -> [(f32, f32); 4] {
    if rot_deg == 0.0 {
        return pts;
    }
    let (s, c) = rot_deg.to_radians().sin_cos();
    pts.map(|(px, py)| {
        let (dx, dy) = (px - cx, py - cy);
        (cx + dx * c - dy * s, cy + dx * s + dy * c)
    })
}


// ---------------------------------------------------------------------------
// distance-field painting (rounded rectangles, ellipses)
// ---------------------------------------------------------------------------

/// A size that is above zero (and not NaN).
fn pos(v: f32) -> bool {
    v > 0.0
}

/// Bounds of `verts` grown by `pad`.
fn pad_bounds(verts: &[(f32, f32)], pad: f32) -> Option<Bounds> {
    if verts.is_empty() || verts.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }
    let mut lo = (f32::INFINITY, f32::INFINITY);
    let mut hi = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for &(x, y) in verts {
        lo.0 = lo.0.min(x);
        lo.1 = lo.1.min(y);
        hi.0 = hi.0.max(x);
        hi.1 = hi.1.max(y);
    }
    Some((lo.0 - pad, lo.1 - pad, hi.0 + pad, hi.1 + pad))
}

fn stroke_pad(stroke: Option<Stroke>) -> f32 {
    stroke.map_or(0.0, |s| s.width.max(0.0) * 0.5) + 1.0
}

/// Paints the pixels of `area` from a signed distance (negative inside, in px): a 1px
/// anti-aliased fill, and a stroke centred on the edge.
fn paint_sdf(
    img: &mut RgbaImage,
    area: Bounds,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    dist: impl Fn(f32, f32) -> f32,
) {
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return;
    }
    let x0 = area.0.floor().max(0.0) as i64;
    let y0 = area.1.floor().max(0.0) as i64;
    let x1 = (area.2.ceil() as i64).min(iw as i64 - 1);
    let y1 = (area.3.ceil() as i64).min(ih as i64 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = dist(x as f32 + 0.5, y as f32 + 0.5);
            if let Some(c) = fill {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                if cov > 0.0 {
                    blend_pixel(img, x, y, scale_alpha(c, cov));
                }
            }
            if let Some(s) = stroke {
                let cov = (s.width.max(0.0) * 0.5 + 0.5 - d.abs()).clamp(0.0, 1.0);
                if cov > 0.0 {
                    blend_pixel(img, x, y, scale_alpha(s.color, cov));
                }
            }
        }
    }
}

/// Pixel centre to the local frame of a shape centred on `(cx, cy)` and turned by `rot` degrees.
fn to_local(cx: f32, cy: f32, rot: f32) -> impl Fn(f32, f32) -> (f32, f32) {
    let (st, ct) = rot.to_radians().sin_cos();
    move |px, py| {
        let (dx, dy) = (px - cx, py - cy);
        (ct * dx + st * dy, -st * dx + ct * dy)
    }
}

// ---------------------------------------------------------------------------
// option builders
// ---------------------------------------------------------------------------

macro_rules! fill_stroke_setters {
    () => {
        /// Set the interior colour.
        pub fn fill(mut self, color: Rgba) -> Self {
            self.fill = Some(color);
            self
        }
        /// Draw no interior (outline only — pair with `stroke`).
        pub fn no_fill(mut self) -> Self {
            self.fill = None;
            self
        }
        /// Add an outline of `color` at `width` px.
        pub fn stroke(mut self, color: Rgba, width: f32) -> Self {
            self.stroke = Some(Stroke { color, width });
            self
        }
    };
}

/// Axis-aligned (or rotated) rectangle. `at` is the top-left before rotation.
#[derive(Clone, Copy, Debug)]
pub struct RectOptions {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rotation: f32,
    radius: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl RectOptions {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            rotation: 0.0,
            radius: 0.0,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    /// Filled rectangle at `(x, y)` in one call.
    pub fn solid(x: f32, y: f32, width: f32, height: f32, color: Rgba) -> Self {
        Self::new(width, height).at(x, y).fill(color)
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation = deg;
        self
    }
    /// Rounded corners of this radius in px (limited to half the shorter side).
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// Circle centred on `at`.
#[derive(Clone, Copy, Debug)]
pub struct CircleOptions {
    x: f32,
    y: f32,
    radius: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl CircleOptions {
    pub fn new(radius: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            radius,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    pub fn solid(x: f32, y: f32, radius: f32, color: Rgba) -> Self {
        Self::new(radius).at(x, y).fill(color)
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// Ellipse centred on `at`, with a half-width `rx` and a half-height `ry` (before rotation).
/// The stroke is centred on the edge.
#[derive(Clone, Copy, Debug)]
pub struct EllipseOptions {
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
    rotation: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl EllipseOptions {
    pub fn new(rx: f32, ry: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rx,
            ry,
            rotation: 0.0,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    /// The ellipse that fills the box `(x, y, width, height)`.
    pub fn in_box(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(width / 2.0, height / 2.0).at(x + width / 2.0, y + height / 2.0)
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation = deg;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// Regular N-gon centred on `at`. `sides` defaults to 3 (an equilateral triangle).
#[derive(Clone, Copy, Debug)]
pub struct PolygonOptions {
    x: f32,
    y: f32,
    radius: f32,
    sides: u32,
    rotation: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl PolygonOptions {
    pub fn new(radius: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            radius,
            sides: 3,
            rotation: 0.0,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    /// Number of sides (clamped to `>= 3`).
    pub fn sides(mut self, sides: u32) -> Self {
        self.sides = sides;
        self
    }
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation = deg;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// N-pointed star centred on `at`. `points` defaults to 5, `inner_ratio` to 0.45.
#[derive(Clone, Copy, Debug)]
pub struct StarOptions {
    x: f32,
    y: f32,
    radius: f32,
    points: u32,
    inner_ratio: f32,
    rotation: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl StarOptions {
    pub fn new(radius: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            radius,
            points: 5,
            inner_ratio: 0.45,
            rotation: 0.0,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    /// Number of star points (clamped to `>= 2`).
    pub fn points(mut self, points: u32) -> Self {
        self.points = points;
        self
    }
    /// Inner-vertex radius as a fraction of `radius` (0..1). Smaller ⇒ spikier.
    pub fn inner_ratio(mut self, ratio: f32) -> Self {
        self.inner_ratio = ratio;
        self
    }
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation = deg;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// Rhombus / diamond centred on `at`, `width` and `height` are the full diagonals.
#[derive(Clone, Copy, Debug)]
pub struct RhombusOptions {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    rotation: f32,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl RhombusOptions {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            rotation: 0.0,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn rotation(mut self, deg: f32) -> Self {
        self.rotation = deg;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// A polygon (or an open polyline) from any list of points. Even-odd fill.
#[derive(Clone, Debug)]
pub struct PathOptions {
    points: Vec<(f32, f32)>,
    closed: bool,
    fill: Option<Rgba>,
    stroke: Option<Stroke>,
    effects: Effects,
}

impl PathOptions {
    /// A closed path through `points`, filled black.
    pub fn new(points: impl IntoIterator<Item = (f32, f32)>) -> Self {
        Self {
            points: points.into_iter().collect(),
            closed: true,
            fill: Some(Rgba::BLACK),
            stroke: None,
            effects: Effects::default(),
        }
    }
    /// An isosceles triangle in the box `(x, y, width, height)`: the apex at the top middle, the
    /// base along the bottom.
    pub fn triangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new([(x + width / 2.0, y), (x + width, y + height), (x, y + height)])
    }
    /// `false` leaves the last point unjoined to the first and draws no fill (a polyline).
    pub fn closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
    }
    fill_stroke_setters!();
    effect_setters!();
}

/// Straight line from `(x1, y1)` to `(x2, y2)`. `width` defaults to 1.
#[derive(Clone, Copy, Debug)]
pub struct LineOptions {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    color: Rgba,
    width: f32,
    effects: Effects,
}

impl LineOptions {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2, color: Rgba::BLACK, width: 1.0, effects: Effects::default() }
    }
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    effect_setters!();
}

// ---------------------------------------------------------------------------
// draw entry points
// ---------------------------------------------------------------------------

pub fn draw_rect(img: &mut RgbaImage, o: &RectOptions) {
    draw_drawable(img, o);
}

fn paint_rect(img: &mut RgbaImage, o: &RectOptions) {
    if !(pos(o.width) && pos(o.height)) {
        return;
    }
    if o.radius > 0.0 {
        return paint_round_rect(img, o);
    }
    let quad = rect_quad(o.x, o.y, o.width, o.height, o.rotation);
    if let Some(c) = o.fill {
        if o.rotation == 0.0 {
            fill_axis_rect(img, o.x, o.y, o.width, o.height, c);
        } else {
            fill_polygon(img, &quad, c);
        }
    }
    if let Some(s) = o.stroke {
        stroke_path(img, &quad, true, s);
    }
}

fn paint_round_rect(img: &mut RgbaImage, o: &RectOptions) {
    let (hw, hh) = (o.width / 2.0, o.height / 2.0);
    let r = o.radius.min(hw).min(hh).max(0.0);
    let (cx, cy) = (o.x + hw, o.y + hh);
    let quad = rect_quad(o.x, o.y, o.width, o.height, o.rotation);
    let Some(area) = pad_bounds(&quad, stroke_pad(o.stroke)) else {
        return;
    };
    let local = to_local(cx, cy, o.rotation);
    paint_sdf(img, area, o.fill, o.stroke, |px, py| {
        let (lx, ly) = local(px, py);
        let (qx, qy) = (lx.abs() - (hw - r), ly.abs() - (hh - r));
        let outside = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
        outside + qx.max(qy).min(0.0) - r
    });
}

fn fill_axis_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
    let (iw, ih) = img.dimensions();
    let (rx0, ry0, rx1, ry1) = (x, y, x + w, y + h);
    let x0 = rx0.floor().max(0.0) as i64;
    let y0 = ry0.floor().max(0.0) as i64;
    let x1 = rx1.ceil().min(iw as f32) as i64;
    let y1 = ry1.ceil().min(ih as f32) as i64;

    // exact per-pixel coverage = overlap of the pixel cell with the rect on each axis
    for py in y0..y1 {
        let cov_y = ((py as f32 + 1.0).min(ry1) - (py as f32).max(ry0)).clamp(0.0, 1.0);
        if cov_y <= 0.0 {
            continue;
        }
        for px in x0..x1 {
            let cov_x = ((px as f32 + 1.0).min(rx1) - (px as f32).max(rx0)).clamp(0.0, 1.0);
            if cov_x <= 0.0 {
                continue;
            }
            blend_pixel(img, px, py, scale_alpha(c, cov_x * cov_y));
        }
    }
}

pub fn draw_circle(img: &mut RgbaImage, o: &CircleOptions) {
    draw_drawable(img, o);
}

fn paint_circle(img: &mut RgbaImage, o: &CircleOptions) {
    if !pos(o.radius) {
        return;
    }
    let (iw, ih) = img.dimensions();
    if iw == 0 || ih == 0 {
        return;
    }
    let r = o.radius;
    let x0 = (o.x - r - 1.0).floor().max(0.0) as i64;
    let x1 = (o.x + r + 1.0).ceil().min((iw - 1) as f32) as i64;
    let y0 = (o.y - r - 1.0).floor().max(0.0) as i64;
    let y1 = (o.y + r + 1.0).ceil().min((ih - 1) as f32) as i64;
    let ring_inner = o.stroke.map(|s| (r - s.width).max(0.0));

    for py in y0..=y1 {
        for px in x0..=x1 {
            let d = (((px as f32 + 0.5) - o.x).powi(2) + ((py as f32 + 0.5) - o.y).powi(2)).sqrt();

            if let Some(fc) = o.fill {
                let cov = (r + 0.5 - d).clamp(0.0, 1.0); // solid inside, 1px AA rim
                if cov > 0.0 {
                    blend_pixel(img, px, py, scale_alpha(fc, cov));
                }
            }
            if let (Some(s), Some(inner)) = (o.stroke, ring_inner) {
                let cov = (r + 0.5 - d).clamp(0.0, 1.0).min((d - (inner - 0.5)).clamp(0.0, 1.0));
                if cov > 0.0 {
                    blend_pixel(img, px, py, scale_alpha(s.color, cov));
                }
            }
        }
    }
}

pub fn draw_ellipse(img: &mut RgbaImage, o: &EllipseOptions) {
    draw_drawable(img, o);
}

/// Half extents of the box that holds an ellipse turned by `rot` degrees.
fn ellipse_half_extents(rx: f32, ry: f32, rot: f32) -> (f32, f32) {
    let (s, c) = rot.to_radians().sin_cos();
    (
        ((rx * c).powi(2) + (ry * s).powi(2)).sqrt(),
        ((rx * s).powi(2) + (ry * c).powi(2)).sqrt(),
    )
}

fn paint_ellipse(img: &mut RgbaImage, o: &EllipseOptions) {
    if !(pos(o.rx) && pos(o.ry)) {
        return;
    }
    let (ex, ey) = ellipse_half_extents(o.rx, o.ry, o.rotation);
    let pad = stroke_pad(o.stroke);
    let area = (o.x - ex - pad, o.y - ey - pad, o.x + ex + pad, o.y + ey + pad);
    let local = to_local(o.x, o.y, o.rotation);
    let (rx, ry) = (o.rx, o.ry);
    paint_sdf(img, area, o.fill, o.stroke, |px, py| {
        let (lx, ly) = local(px, py);
        let k0 = ((lx / rx).powi(2) + (ly / ry).powi(2)).sqrt();
        let k1 = ((lx / (rx * rx)).powi(2) + (ly / (ry * ry)).powi(2)).sqrt();
        if k1 < 1e-9 {
            -rx.min(ry)
        } else {
            k0 * (k0 - 1.0) / k1
        }
    });
}

fn scale_alpha(c: Rgba, cov: f32) -> Rgba {
    Rgba { a: (c.a as f32 * cov).round().clamp(0.0, 255.0) as u8, ..c }
}

fn fill_and_stroke(img: &mut RgbaImage, verts: &[(f32, f32)], fill: Option<Rgba>, stroke: Option<Stroke>) {
    if let Some(c) = fill {
        fill_polygon(img, verts, c);
    }
    if let Some(s) = stroke {
        stroke_path(img, verts, true, s);
    }
}

pub fn draw_polygon(img: &mut RgbaImage, o: &PolygonOptions) {
    draw_drawable(img, o);
}

pub fn draw_star(img: &mut RgbaImage, o: &StarOptions) {
    draw_drawable(img, o);
}

pub fn draw_rhombus(img: &mut RgbaImage, o: &RhombusOptions) {
    draw_drawable(img, o);
}

pub fn draw_path(img: &mut RgbaImage, o: &PathOptions) {
    draw_drawable(img, o);
}

pub fn draw_line(img: &mut RgbaImage, o: &LineOptions) {
    draw_drawable(img, o);
}

// ---------------------------------------------------------------------------
// drawables (bounds, shifting, plain painting)
// ---------------------------------------------------------------------------

impl Drawable for RectOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !(pos(self.width) && pos(self.height)) {
            return None;
        }
        pad_bounds(
            &rect_quad(self.x, self.y, self.width, self.height, self.rotation),
            stroke_pad(self.stroke),
        )
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        paint_rect(img, self);
    }
}

impl Drawable for CircleOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !pos(self.radius) || !self.x.is_finite() || !self.y.is_finite() {
            return None;
        }
        let r = self.radius + 1.0;
        Some((self.x - r, self.y - r, self.x + r, self.y + r))
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        paint_circle(img, self);
    }
}

impl Drawable for EllipseOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !(pos(self.rx) && pos(self.ry) && self.x.is_finite() && self.y.is_finite()) {
            return None;
        }
        let (ex, ey) = ellipse_half_extents(self.rx, self.ry, self.rotation);
        let pad = stroke_pad(self.stroke);
        Some((self.x - ex - pad, self.y - ey - pad, self.x + ex + pad, self.y + ey + pad))
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        paint_ellipse(img, self);
    }
}

impl Drawable for PolygonOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !pos(self.radius) {
            return None;
        }
        pad_bounds(
            &regular_polygon(self.x, self.y, self.radius, self.sides, self.rotation),
            stroke_pad(self.stroke),
        )
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        if self.radius > 0.0 {
            let v = regular_polygon(self.x, self.y, self.radius, self.sides, self.rotation);
            fill_and_stroke(img, &v, self.fill, self.stroke);
        }
    }
}

impl Drawable for StarOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !pos(self.radius) {
            return None;
        }
        pad_bounds(
            &star_verts(self.x, self.y, self.radius, self.points, self.inner_ratio, self.rotation),
            stroke_pad(self.stroke),
        )
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        if self.radius > 0.0 {
            let v = star_verts(self.x, self.y, self.radius, self.points, self.inner_ratio, self.rotation);
            fill_and_stroke(img, &v, self.fill, self.stroke);
        }
    }
}

impl Drawable for RhombusOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        if !(pos(self.width) && pos(self.height)) {
            return None;
        }
        pad_bounds(
            &rhombus_verts(self.x, self.y, self.width, self.height, self.rotation),
            stroke_pad(self.stroke),
        )
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    fn paint(&self, img: &mut RgbaImage) {
        if self.width > 0.0 && self.height > 0.0 {
            let v = rhombus_verts(self.x, self.y, self.width, self.height, self.rotation);
            fill_and_stroke(img, &v, self.fill, self.stroke);
        }
    }
}

impl Drawable for PathOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        pad_bounds(&self.points, stroke_pad(self.stroke))
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self {
            points: self.points.iter().map(|&(x, y)| (x + dx, y + dy)).collect(),
            ..self.clone()
        }
    }
    fn paint(&self, img: &mut RgbaImage) {
        if self.closed {
            if let Some(c) = self.fill {
                fill_polygon(img, &self.points, c);
            }
        }
        if let Some(s) = self.stroke {
            stroke_path(img, &self.points, self.closed, s);
        }
    }
}

impl Drawable for LineOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn bounds(&self) -> Option<Bounds> {
        pad_bounds(
            &[(self.x1, self.y1), (self.x2, self.y2)],
            self.width.max(1.0) * 0.5 + 1.0,
        )
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        Self {
            x1: self.x1 + dx,
            y1: self.y1 + dy,
            x2: self.x2 + dx,
            y2: self.y2 + dy,
            ..*self
        }
    }
    fn paint(&self, img: &mut RgbaImage) {
        draw_segment(img, (self.x1, self.y1), (self.x2, self.y2), self.width, self.color);
    }
}
