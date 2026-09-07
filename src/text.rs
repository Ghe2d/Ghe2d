//! Text rendering.
//!
//! Pipeline: box fitting (optional word-wrap or shrink-to-fit against a `width`/`height`)
//! → per line, Unicode BiDi reordering (`unicode-bidi`) + OpenType shaping (`rustybuzz`)
//! → glyph-by-glyph rasterization with `rusttype` → clipped 1×1 alpha blend.

use crate::font::LoadFont;
use crate::utility::Rgba;

const MIN_SIZE: f32 = 1.0;

/// Horizontal placement of each line inside the `[x, x + width]` box.
///
/// Only takes effect when [`TextOptions::width`] is set. Absolute to the box — it does not
/// change glyph order (that always follows the text's own direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
}

/// Vertical placement of the lines produced by
/// [`Ghe2d::draw_text`](crate::Ghe2d::draw_text).
///
/// The `f32` carried by `Gap` / `Top` / `Middle` / `Bottom` is **extra** pixels inserted
/// between consecutive lines, on top of the font's natural line height. `SpaceBetween` and
/// `SpaceEvenly` derive their spacing from the box height and ignore any gap.
///
/// Every variant except `Gap` needs [`TextOptions::height`]; without it they fall back to
/// `Gap` (carrying their own gap value) stacking downward from `y`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineLayout {
    /// Stack downward from `y`. Does not need a height.
    Gap(f32),
    /// Pack the lines against the top of the height box.
    Top(f32),
    /// Center the line block vertically in the height box.
    Middle(f32),
    /// Pack the lines against the bottom of the height box.
    Bottom(f32),
    /// First line at the top, last line at the bottom, remaining space split evenly
    /// between the lines.
    SpaceBetween,
    /// Equal space above, between and below every line.
    SpaceEvenly,
}

impl Default for LineLayout {
    fn default() -> Self {
        LineLayout::Gap(0.0)
    }
}

/// Everything [`Ghe2d::draw_text`](crate::Ghe2d::draw_text) needs, built fluently.
///
/// ```no_run
/// use ghe2d::{TextOptions, TextAlign};
/// use ghe2d::font::LoadFont;
/// use ghe2d::utility::Rgba;
///
/// # fn demo(font: LoadFont, g: &mut ghe2d::Ghe2d) {
/// g.draw_text(
///     TextOptions::new(font, "مرحبا بالعالم")
///         .at(20.0, 20.0)
///         .size(48.0)
///         .color(Rgba::new(255, 255, 255, 255))
///         .width(400.0)
///         .align(TextAlign::Center)
///         .wrap(true),
/// );
/// # }
/// ```
#[derive(Clone)]
pub struct TextOptions {
    font: LoadFont,
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: Rgba,
    width: Option<f32>,
    height: Option<f32>,
    align: Option<TextAlign>,
    wrap: bool,
    line_layout: LineLayout,
}

impl TextOptions {
    /// Sensible defaults: origin `(0, 0)`, `size` 16, opaque black, no box.
    pub fn new(font: LoadFont, text: impl Into<String>) -> Self {
        Self {
            font,
            text: text.into(),
            x: 0.0,
            y: 0.0,
            size: 16.0,
            color: Rgba::new(0, 0, 0, 255),
            width: None,
            height: None,
            align: None,
            wrap: false,
            line_layout: LineLayout::default(),
        }
    }

    /// Top-left origin of the text, or of the layout box when `width`/`height` are set.
    pub fn at(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }

    /// Requested size in pixels (`ascent - descent` spans this many pixels). May be reduced
    /// by shrink-to-fit when a `width`/`height` is set and `wrap` is off.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }

    /// Constrain horizontally. Enables alignment; with `wrap(false)` (default) the size
    /// shrinks until the widest line fits, with `wrap(true)` the text word-wraps instead.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Constrain vertically. Enables the [`LineLayout`] distribution modes and shrinks the
    /// size if the line block would be taller than this.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Horizontal alignment inside the width box. Ignored without `width`. Defaults to the
    /// text's base direction (RTL → right, LTR → left).
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// With `width` set: `true` word-wraps, `false` (default) shrinks the size to fit.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn line_layout(mut self, line_layout: LineLayout) -> Self {
        self.line_layout = line_layout;
        self
    }
}

pub fn draw_text(img: &mut image::RgbaImage, opts: &TextOptions) {
    if opts.text.is_empty() {
        return;
    }

    let vmu = opts.font.font.v_metrics_unscaled();
    let em = vmu.ascent - vmu.descent;
    if em <= 0.0 {
        return;
    }

    let face = opts.font.shaper_face();
    let base_rtl = base_is_rtl(&opts.text);

    let (size, lines) = fit(opts, &face, em);
    if size < MIN_SIZE || lines.is_empty() {
        return;
    }

    let scale = rusttype::Scale::uniform(size);
    let vm = opts.font.font.v_metrics(scale);
    let sf = size / em;
    let glyph_count = opts.font.font.glyph_count() as u32;

    let (first_baseline, advance) = baselines_for(opts, &vm, lines.len());

    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let baseline = first_baseline + i as f32 * advance;
        let line_w = measure_line(&face, line, sf);
        let start_x = align_start_x(opts, base_rtl, line_w);
        render_line(img, &face, &opts.font.font, line, baseline, start_x, scale, sf, glyph_count, opts.color);
    }
}

fn base_is_rtl(text: &str) -> bool {
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    bidi.paragraphs.first().map(|p| p.level.is_rtl()).unwrap_or(false)
}

/// Total shaped width of one line, in pixels, at the given design-unit -> pixel factor.
fn measure_line(face: &rustybuzz::Face<'_>, line: &str, sf: f32) -> f32 {
    if line.is_empty() {
        return 0.0;
    }
    let bidi = unicode_bidi::BidiInfo::new(line, None);
    let Some(para) = bidi.paragraphs.first() else {
        return 0.0;
    };
    let (levels, runs) = bidi.visual_runs(para, para.range.clone());

    let mut width = 0.0f32;
    for run in runs {
        let run_text = &line[run.clone()];
        if run_text.is_empty() {
            continue;
        }
        let rtl = levels[run.start].is_rtl();
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(run_text);
        buf.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        buf.guess_segment_properties();
        let shaped = rustybuzz::shape(face, &[], buf);
        for pos in shaped.glyph_positions() {
            width += pos.x_advance as f32 * sf;
        }
    }
    width
}

/// Greedy word wrap. A single word wider than `width` gets its own line and overflows
/// (glyphs are never split). An empty logical line stays as one empty visual line.
fn wrap_line(face: &rustybuzz::Face<'_>, line: &str, sf: f32, width: f32) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for w in words {
        let candidate = if cur.is_empty() {
            w.to_string()
        } else {
            format!("{cur} {w}")
        };
        if !cur.is_empty() && measure_line(face, &candidate, sf) > width {
            out.push(std::mem::take(&mut cur));
            cur = w.to_string();
        } else {
            cur = candidate;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Height of the line block (first line top to last line bottom) at `size`, ignoring any
/// derived spacing (used to decide whether to shrink).
fn block_height(opts: &TextOptions, size: f32, n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    let vm = opts.font.font.v_metrics(rusttype::Scale::uniform(size));
    let line_box = vm.ascent - vm.descent;
    let natural = line_box + vm.line_gap;
    let gap = match opts.line_layout {
        LineLayout::Gap(g) | LineLayout::Top(g) | LineLayout::Middle(g) | LineLayout::Bottom(g) => g,
        LineLayout::SpaceBetween | LineLayout::SpaceEvenly => 0.0,
    };
    (n as f32 - 1.0) * (natural + gap) + line_box
}

/// Resolve the effective size and the visual lines to draw.
fn fit(opts: &TextOptions, face: &rustybuzz::Face<'_>, em: f32) -> (f32, Vec<String>) {
    let logical: Vec<&str> = opts.text.split('\n').collect();

    let Some(width) = opts.width else {
        return (opts.size, logical.into_iter().map(String::from).collect());
    };

    let build = |size: f32| -> Vec<String> {
        let sf = size / em;
        if opts.wrap {
            logical.iter().flat_map(|l| wrap_line(face, l, sf, width)).collect()
        } else {
            logical.iter().map(|l| (*l).to_string()).collect()
        }
    };

    let fits = |size: f32| -> bool {
        let lines = build(size);
        let sf = size / em;
        let w_ok = opts.wrap
            || lines
                .iter()
                .map(|l| measure_line(face, l, sf))
                .fold(0.0_f32, f32::max)
                <= width + 0.5;
        let h_ok = match opts.height {
            Some(h) => block_height(opts, size, lines.len()) <= h + 0.5,
            None => true,
        };
        w_ok && h_ok
    };

    if fits(opts.size) {
        return (opts.size, build(opts.size));
    }

    let (mut lo, mut hi) = (MIN_SIZE, opts.size);
    for _ in 0..14 {
        let mid = 0.5 * (lo + hi);
        if fits(mid) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo, build(lo))
}

/// `(baseline of the first line, distance between consecutive baselines)`.
fn baselines_for(opts: &TextOptions, vm: &rusttype::VMetrics, n: usize) -> (f32, f32) {
    let ascent = vm.ascent;
    let line_box = vm.ascent - vm.descent;
    let natural = line_box + vm.line_gap;
    let n_f = n.max(1) as f32;

    let carried_gap = match opts.line_layout {
        LineLayout::Gap(g) | LineLayout::Top(g) | LineLayout::Middle(g) | LineLayout::Bottom(g) => g,
        LineLayout::SpaceBetween | LineLayout::SpaceEvenly => 0.0,
    };

    // Gap mode, or any height-mode with no height set: stack from y.
    let h = match opts.height {
        Some(h) if !matches!(opts.line_layout, LineLayout::Gap(_)) => h,
        _ => return (opts.y + ascent, natural + carried_gap),
    };

    match opts.line_layout {
        LineLayout::Gap(_) => (opts.y + ascent, natural + carried_gap),
        LineLayout::Top(g) => (opts.y + ascent, natural + g),
        LineLayout::Bottom(g) => {
            let adv = natural + g;
            let block = (n_f - 1.0) * adv + line_box;
            (opts.y + h - block + ascent, adv)
        }
        LineLayout::Middle(g) => {
            let adv = natural + g;
            let block = (n_f - 1.0) * adv + line_box;
            (opts.y + (h - block) / 2.0 + ascent, adv)
        }
        LineLayout::SpaceBetween => {
            if n < 2 {
                (opts.y + ascent, natural)
            } else {
                let gap = ((h - n_f * line_box) / (n_f - 1.0)).max(0.0);
                (opts.y + ascent, line_box + gap)
            }
        }
        LineLayout::SpaceEvenly => {
            let space = ((h - n_f * line_box) / (n_f + 1.0)).max(0.0);
            (opts.y + space + ascent, line_box + space)
        }
    }
}

fn align_start_x(opts: &TextOptions, base_rtl: bool, line_w: f32) -> f32 {
    let Some(width) = opts.width else {
        return opts.x;
    };
    let align = opts
        .align
        .unwrap_or(if base_rtl { TextAlign::Right } else { TextAlign::Left });
    let start = match align {
        TextAlign::Left => opts.x,
        TextAlign::Right => opts.x + width - line_w,
        TextAlign::Center => opts.x + (width - line_w) / 2.0,
    };
    start.max(opts.x)
}

#[allow(clippy::too_many_arguments)]
fn render_line(
    img: &mut image::RgbaImage,
    face: &rustybuzz::Face<'_>,
    font: &rusttype::Font<'_>,
    line: &str,
    baseline: f32,
    start_x: f32,
    scale: rusttype::Scale,
    sf: f32,
    glyph_count: u32,
    color: Rgba,
) {
    let bidi = unicode_bidi::BidiInfo::new(line, None);
    let Some(para) = bidi.paragraphs.first() else {
        return;
    };
    let (levels, runs) = bidi.visual_runs(para, para.range.clone());

    let mut pen_x = start_x;
    for run in runs {
        let run_text = &line[run.clone()];
        if run_text.is_empty() {
            continue;
        }
        let rtl = levels[run.start].is_rtl();
        let mut buf = rustybuzz::UnicodeBuffer::new();
        buf.push_str(run_text);
        buf.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        buf.guess_segment_properties();
        let shaped = rustybuzz::shape(face, &[], buf);

        for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
            let gid = info.glyph_id;
            let gx = pen_x + pos.x_offset as f32 * sf;
            let gy = baseline - pos.y_offset as f32 * sf;

            if gid != 0 && gid < glyph_count {
                let glyph = font
                    .glyph(rusttype::GlyphId(gid as u16))
                    .scaled(scale)
                    .positioned(rusttype::point(gx, gy));

                if let Some(bb) = glyph.pixel_bounding_box() {
                    glyph.draw(|dx, dy, v| {
                        let px = bb.min.x + dx as i32;
                        let py = bb.min.y + dy as i32;
                        if px < 0 || py < 0 {
                            return;
                        }
                        let (px, py) = (px as u32, py as u32);
                        let Some(pixel) = img.get_pixel_checked(px, py) else {
                            return;
                        };
                        let alpha = (v * 255.0) as u8;
                        let background =
                            Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                        let blend = Rgba::blend_with_alpha(&color, background, alpha);
                        crate::rect::draw_rect(
                            img,
                            px,
                            py,
                            1,
                            1,
                            Rgba::new(blend.0[0], blend.0[1], blend.0[2], blend.0[3]),
                        );
                    });
                }
            }

            pen_x += pos.x_advance as f32 * sf;
        }
    }
}
