//! Text rendering.
//!
//! Pipeline: box fitting (optional word-wrap or shrink-to-fit against a `width`/`height`)
//! → per line, Unicode BiDi reordering (`unicode-bidi`) + OpenType shaping (`rustybuzz`)
//! → glyph outlines (ttf-parser, variation-aware) rasterized with `ab_glyph_rasterizer` → clipped 1×1 alpha blend.

use image::RgbaImage;

use crate::font::LoadFont;
use crate::layer::{draw_drawable, effect_setters, Bounds, Drawable, Effects};
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
    fallbacks: Vec<LoadFont>,
    weight: Option<f32>,
    shrink: bool,
    rotation: f32,
    effects: Effects,
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
            fallbacks: Vec::new(),
            weight: None,
            shrink: true,
            rotation: 0.0,
            effects: Effects::default(),
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

    /// Requested size as a CSS `font-size` (the em size in pixels), the number a browser
    /// uses for the same text. Converts through the primary font's ascent and descent (see
    /// [`LoadFont::css_to_size`]) and replaces any earlier [`size`](Self::size). Call it
    /// after the primary font is set (it is the one given to [`TextOptions::new`]).
    pub fn css_size(mut self, css_px: f32) -> Self {
        self.size = self.font.css_to_size(css_px);
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

    /// Append a fallback font. Any character the primary font (and earlier fallbacks)
    /// lacks is drawn from the first font in the chain that has it, never dropped.
    pub fn fallback(mut self, font: LoadFont) -> Self {
        self.fallbacks.push(font);
        self
    }

    /// Replace the fallback list (tried in order after the primary font).
    pub fn fallbacks(mut self, fonts: impl IntoIterator<Item = LoadFont>) -> Self {
        self.fallbacks = fonts.into_iter().collect();
        self
    }

    /// Weight (100..=900) applied to every variable font in the chain, through the `wght`
    /// axis, in shaping and in rasterizing. Static fonts ignore it; pick a static file with
    /// [`FontFamily::pick`](crate::font::FontFamily::pick) instead.
    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = Some(weight);
        self
    }

    /// With `width` set and `wrap(false)`: `true` (default) shrinks the size until the widest
    /// line fits, `false` keeps the size and uses `width` for alignment only (a longer line runs
    /// out of the box). It also stops the height shrink.
    pub fn shrink_to_fit(mut self, shrink: bool) -> Self {
        self.shrink = shrink;
        self
    }

    /// Turn the whole text block by `degrees` (clockwise) about the centre of its box (the
    /// `width` by `height` box when set, the measured text otherwise).
    pub fn rotation(mut self, degrees: f32) -> Self {
        self.rotation = degrees;
        self
    }

    effect_setters!();

    /// Primary font then fallbacks, with the requested weight applied.
    pub(crate) fn chain_fonts(&self) -> Vec<LoadFont> {
        let mut v = Vec::with_capacity(1 + self.fallbacks.len());
        v.push(self.font.clone());
        v.extend(self.fallbacks.iter().cloned());
        if let Some(w) = self.weight {
            for f in v.iter_mut() {
                *f = f.with_weight(w);
            }
        }
        v
    }
}

/// Result of [`measure_text`]: what [`Ghe2d::draw_text`](crate::Ghe2d::draw_text) would lay out.
#[derive(Clone, Debug, PartialEq)]
pub struct TextMeasure {
    /// Width in pixels of the widest line, shaped with the same font chain as drawing.
    pub width: f32,
    /// Height in pixels of the line block (first line top to last line bottom).
    pub height: f32,
    /// Effective size after shrink-to-fit (equals the requested size when nothing shrinks).
    pub size: f32,
    /// The visual lines (after word wrap, when enabled).
    pub lines: Vec<String>,
}

/// Measure `opts` exactly the way `draw_text` lays it out (same chain, same shaping, same
/// wrapping and shrink-to-fit), without drawing, so a layout measured ahead of time breaks
/// lines exactly like the final drawing.
pub fn measure_text(opts: &TextOptions) -> TextMeasure {
    let empty = TextMeasure { width: 0.0, height: 0.0, size: opts.size, lines: Vec::new() };
    if opts.text.is_empty() {
        return empty;
    }
    let fonts = opts.chain_fonts();
    let chain = Chain::new(&fonts);
    let Some(em) = chain.primary_em() else {
        return empty;
    };
    let (size, lines) = fit(opts, &chain, em);
    if size < MIN_SIZE || lines.is_empty() {
        return empty;
    }
    let px_em = chain.px_per_em(size, em);
    let width = lines.iter().map(|l| measure_line(&chain, l, px_em)).fold(0.0_f32, f32::max);
    let height = block_height(opts, size, lines.len());
    TextMeasure { width, height, size, lines }
}

/// Width in pixels of `text` on one line at `size`, shaped through `font` then `fallbacks`.
/// The weight is whatever each font was loaded with (see [`LoadFont::with_weight`]).
/// With several `\n` lines it returns the widest.
pub fn text_width(font: &LoadFont, fallbacks: &[LoadFont], text: &str, size: f32) -> f32 {
    let mut fonts = Vec::with_capacity(1 + fallbacks.len());
    fonts.push(font.clone());
    fonts.extend(fallbacks.iter().cloned());
    let chain = Chain::new(&fonts);
    let Some(em) = chain.primary_em() else {
        return 0.0;
    };
    let px_em = chain.px_per_em(size, em);
    text.split('\n').map(|l| measure_line(&chain, l, px_em)).fold(0.0_f32, f32::max)
}

pub fn draw_text(img: &mut RgbaImage, opts: &TextOptions) {
    draw_drawable(img, opts);
}

impl Drawable for TextOptions {
    fn effects(&self) -> Effects {
        self.effects
    }
    fn group_rotation(&self) -> f32 {
        self.rotation
    }
    fn bounds(&self) -> Option<Bounds> {
        let m = measure_text(self);
        if m.lines.is_empty() || !self.x.is_finite() || !self.y.is_finite() {
            return None;
        }
        let w = self.width.unwrap_or(m.width).max(m.width).max(1.0);
        let h = self.height.unwrap_or(m.height).max(m.height).max(1.0);
        Some((self.x, self.y, self.x + w, self.y + h))
    }
    /// Glyphs reach past the line box (accents, slanted letters, long tails).
    fn extra_margin(&self) -> f32 {
        measure_text(self).size * 0.6
    }
    fn shifted(&self, dx: f32, dy: f32) -> Self {
        let mut t = self.clone();
        t.x += dx;
        t.y += dy;
        t
    }
    fn paint(&self, img: &mut RgbaImage) {
        paint_text(img, self);
    }
}

fn paint_text(img: &mut image::RgbaImage, opts: &TextOptions) {
    if opts.text.is_empty() {
        return;
    }

    let fonts = opts.chain_fonts();
    let chain = Chain::new(&fonts);
    let Some(em) = chain.primary_em() else {
        return;
    };

    let base_rtl = base_is_rtl(&opts.text);

    let (size, lines) = fit(opts, &chain, em);
    if size < MIN_SIZE || lines.is_empty() {
        return;
    }

    let scale = rusttype::Scale::uniform(size);
    let vm = opts.font.font.v_metrics(scale);
    let px_em = chain.px_per_em(size, em);

    let (first_baseline, advance) = baselines_for(opts, &vm, lines.len());

    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let baseline = first_baseline + i as f32 * advance;
        let glyphs = layout_line(&chain, line, px_em);
        let line_w: f32 = glyphs.iter().map(|g| g.advance).sum();
        let start_x = align_start_x(opts, base_rtl, line_w);
        render_glyphs(img, &chain, &glyphs, baseline, start_x, opts.color);
    }
}

fn base_is_rtl(text: &str) -> bool {
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    bidi.paragraphs.first().map(|p| p.level.is_rtl()).unwrap_or(false)
}

/// The font chain with a shaping face per font. Index 0 is the primary font.
struct Chain<'a> {
    faces: Vec<rustybuzz::Face<'a>>,
    upem: Vec<f32>,
}

impl<'a> Chain<'a> {
    fn new(fonts: &'a [LoadFont]) -> Self {
        let faces: Vec<_> = fonts.iter().map(|f| f.shaper_face()).collect();
        let upem = faces.iter().map(|f| f.units_per_em() as f32).collect();
        Chain { faces, upem }
    }

    /// Ascent minus descent of the primary font in design units: the basis `size` has
    /// always been relative to. `None` for a degenerate font.
    fn primary_em(&self) -> Option<f32> {
        let f = self.faces.first()?;
        let em = f.ascender() as f32 - f.descender() as f32;
        (em > 0.0).then_some(em)
    }

    /// Pixels per em of the primary font at `size`. Fallback fonts are scaled to the same
    /// pixels per em, so mixed text keeps one visual size.
    fn px_per_em(&self, size: f32, em: f32) -> f32 {
        size / em * self.upem[0]
    }

    /// Index of the first font that maps `c`; 0 (primary, drawing .notdef) when none does.
    fn font_for(&self, c: char) -> usize {
        self.faces
            .iter()
            .position(|f| f.glyph_index(c).map(|g| g.0 != 0).unwrap_or(false))
            .unwrap_or(0)
    }
}

/// Characters that never pick a font of their own: they stay with the preceding character
/// so marks, joiners and spaces are not split away from their base.
fn inherits_font(c: char) -> bool {
    let u = c as u32;
    c.is_control()
        || c.is_whitespace()
        || matches!(u,
            0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5 | 0x05C7 | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670
            | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7..=0x06E8 | 0x06EA..=0x06ED
            | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F | 0xE0100..=0xE01EF)
}

/// One positioned glyph in visual order. All lengths in pixels.
struct PlacedGlyph {
    font: usize,
    gid: u32,
    /// Pixels per design unit of `font` at the drawn size.
    scale: f32,
    advance: f32,
    x_offset: f32,
    y_offset: f32,
    /// False for a missing glyph of a blank character (nothing to draw).
    paint: bool,
}

/// Split a bidi run into `(font index, byte range)` pieces in logical order.
fn split_by_font(chain: &Chain<'_>, run_text: &str) -> Vec<(usize, std::ops::Range<usize>)> {
    let mut out: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
    let mut prev: Option<usize> = None;
    for (i, c) in run_text.char_indices() {
        let fi = if inherits_font(c) {
            prev.unwrap_or_else(|| chain.font_for(c))
        } else {
            chain.font_for(c)
        };
        prev = Some(fi);
        let end = i + c.len_utf8();
        match out.last_mut() {
            Some((f, r)) if *f == fi => r.end = end,
            _ => out.push((fi, i..end)),
        }
    }
    out
}

/// The real script of `c`, or `None` for Common / Inherited / unknown characters (spaces,
/// punctuation, digits, marks), which take the script of their surroundings.
fn strong_script(c: char) -> Option<rustybuzz::Script> {
    if c.is_ascii() {
        return c.is_ascii_alphabetic().then_some(rustybuzz::script::LATIN);
    }
    let mut buf = rustybuzz::UnicodeBuffer::new();
    buf.push_str(c.encode_utf8(&mut [0u8; 4]));
    buf.guess_segment_properties();
    let s = buf.script();
    (s != rustybuzz::script::COMMON
        && s != rustybuzz::script::INHERITED
        && s != rustybuzz::script::UNKNOWN)
        .then_some(s)
}

/// Effective script per byte offset of `line` (valid at every char start). A character
/// without a script of its own takes the script of the nearest preceding character that has
/// one, like browsers itemize runs; leading ones take the first script found after them.
/// This matters for fonts whose `locl` / `calt` rules depend on the script, e.g. a space after
/// Arabic text gets the Arabic space even when it lands in a left-to-right bidi run.
fn effective_scripts(line: &str) -> Vec<Option<rustybuzz::Script>> {
    let mut out = vec![None; line.len()];
    let mut cur: Option<rustybuzz::Script> = None;
    for (i, c) in line.char_indices() {
        if let Some(s) = strong_script(c) {
            cur = Some(s);
        }
        out[i] = cur;
    }
    // Leading characters before the first strong one.
    if let Some(first) = out.iter().flatten().next().copied() {
        for (i, _) in line.char_indices() {
            if out[i].is_some() {
                break;
            }
            out[i] = Some(first);
        }
    }
    out
}

/// Bidi-reorder, split by font, shape: the glyphs of one line in visual order. Shared by
/// measuring and drawing so both always agree.
fn layout_line(chain: &Chain<'_>, line: &str, px_em: f32) -> Vec<PlacedGlyph> {
    let mut out = Vec::new();
    if line.is_empty() {
        return out;
    }
    let bidi = unicode_bidi::BidiInfo::new(line, None);
    let Some(para) = bidi.paragraphs.first() else {
        return out;
    };
    let (levels, runs) = bidi.visual_runs(para, para.range.clone());
    let scripts = effective_scripts(line);

    for run in runs {
        let run_text = &line[run.clone()];
        if run_text.is_empty() {
            continue;
        }
        let rtl = levels[run.start].is_rtl();
        // Split by font, then by effective script (both in logical order).
        let mut pieces: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
        for (fi, range) in split_by_font(chain, run_text) {
            let mut start = range.start;
            let mut cur = scripts[run.start + start];
            for (off, _) in run_text[range.clone()].char_indices().skip(1) {
                let at = range.start + off;
                let sc = scripts[run.start + at];
                if sc != cur {
                    pieces.push((fi, start..at));
                    start = at;
                    cur = sc;
                }
            }
            pieces.push((fi, start..range.end));
        }
        if rtl {
            // Visual order of an RTL run is the reverse of its logical pieces.
            pieces.reverse();
        }
        for (fi, range) in pieces {
            let script = scripts[run.start + range.start];
            let piece = &run_text[range];
            let sf = px_em / chain.upem[fi];
            let mut buf = rustybuzz::UnicodeBuffer::new();
            buf.push_str(piece);
            buf.set_direction(if rtl {
                rustybuzz::Direction::RightToLeft
            } else {
                rustybuzz::Direction::LeftToRight
            });
            if let Some(sc) = script {
                buf.set_script(sc);
            }
            buf.guess_segment_properties();
            let shaped = rustybuzz::shape(&chain.faces[fi], &[], buf);
            for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                // A glyph id of 0 is .notdef: draw it (visible box) unless the character
                // is blank, so a missing character never vanishes silently.
                let paint = info.glyph_id != 0
                    || piece
                        .get(info.cluster as usize..)
                        .and_then(|s| s.chars().next())
                        .map(|c| !inherits_font(c))
                        .unwrap_or(false);
                out.push(PlacedGlyph {
                    font: fi,
                    gid: info.glyph_id,
                    scale: sf,
                    advance: pos.x_advance as f32 * sf,
                    x_offset: pos.x_offset as f32 * sf,
                    y_offset: pos.y_offset as f32 * sf,
                    paint,
                });
            }
        }
    }
    out
}

/// Total shaped width of one line, in pixels.
fn measure_line(chain: &Chain<'_>, line: &str, px_em: f32) -> f32 {
    layout_line(chain, line, px_em).iter().map(|g| g.advance).sum()
}

/// Greedy word wrap. A single word wider than `width` gets its own line and overflows
/// (glyphs are never split). An empty logical line stays as one empty visual line.
fn wrap_line(chain: &Chain<'_>, line: &str, px_em: f32, width: f32) -> Vec<String> {
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
        if !cur.is_empty() && measure_line(chain, &candidate, px_em) > width {
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
fn fit(opts: &TextOptions, chain: &Chain<'_>, em: f32) -> (f32, Vec<String>) {
    let logical: Vec<&str> = opts.text.split('\n').collect();

    let Some(width) = opts.width else {
        return (opts.size, logical.into_iter().map(String::from).collect());
    };

    let build = |size: f32| -> Vec<String> {
        let px_em = chain.px_per_em(size, em);
        if opts.wrap {
            logical.iter().flat_map(|l| wrap_line(chain, l, px_em, width)).collect()
        } else {
            logical.iter().map(|l| (*l).to_string()).collect()
        }
    };

    let fits = |size: f32| -> bool {
        let lines = build(size);
        let px_em = chain.px_per_em(size, em);
        let w_ok = opts.wrap
            || lines
                .iter()
                .map(|l| measure_line(chain, l, px_em))
                .fold(0.0_f32, f32::max)
                <= width + 0.5;
        let h_ok = match opts.height {
            Some(h) => block_height(opts, size, lines.len()) <= h + 0.5,
            None => true,
        };
        w_ok && h_ok
    };

    if !opts.shrink || fits(opts.size) {
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


/// Collects a glyph outline in pixel space (y down) for the path rasterizer.
struct OutlineCollector {
    scale: f32,
    segs: Vec<Seg>,
    last: (f32, f32),
    start: (f32, f32),
    min: (f32, f32),
    max: (f32, f32),
}

enum Seg {
    Line((f32, f32), (f32, f32)),
    Quad((f32, f32), (f32, f32), (f32, f32)),
    Cubic((f32, f32), (f32, f32), (f32, f32), (f32, f32)),
}

impl OutlineCollector {
    fn p(&mut self, x: f32, y: f32) -> (f32, f32) {
        let p = (x * self.scale, -y * self.scale);
        self.min = (self.min.0.min(p.0), self.min.1.min(p.1));
        self.max = (self.max.0.max(p.0), self.max.1.max(p.1));
        p
    }
}

impl rustybuzz::ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.last = p;
        self.start = p;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.p(x, y);
        self.segs.push(Seg::Line(self.last, p));
        self.last = p;
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let c = self.p(x1, y1);
        let p = self.p(x, y);
        self.segs.push(Seg::Quad(self.last, c, p));
        self.last = p;
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c1 = self.p(x1, y1);
        let c2 = self.p(x2, y2);
        let p = self.p(x, y);
        self.segs.push(Seg::Cubic(self.last, c1, c2, p));
        self.last = p;
    }
    fn close(&mut self) {
        if self.last != self.start {
            self.segs.push(Seg::Line(self.last, self.start));
        }
        self.last = self.start;
    }
}

/// Largest glyph bitmap side we rasterize (guards absurd sizes).
const MAX_GLYPH_PX: f32 = 8192.0;

fn render_glyphs(
    img: &mut image::RgbaImage,
    chain: &Chain<'_>,
    glyphs: &[PlacedGlyph],
    baseline: f32,
    start_x: f32,
    color: Rgba,
) {
    let mut pen_x = start_x;
    for g in glyphs {
        if g.paint {
            paint_glyph(
                img,
                &chain.faces[g.font],
                g.gid,
                g.scale,
                pen_x + g.x_offset,
                baseline - g.y_offset,
                color,
            );
        }
        pen_x += g.advance;
    }
}

/// Rasterize one glyph outline (variations already applied on `face`) and blend it.
fn paint_glyph(
    img: &mut image::RgbaImage,
    face: &rustybuzz::Face<'_>,
    gid: u32,
    sf: f32,
    gx: f32,
    gy: f32,
    color: Rgba,
) {
    use ab_glyph_rasterizer::{point, Rasterizer};

    if gid > u16::MAX as u32 {
        return;
    }
    let mut oc = OutlineCollector {
        scale: sf,
        segs: Vec::new(),
        last: (0.0, 0.0),
        start: (0.0, 0.0),
        min: (f32::MAX, f32::MAX),
        max: (f32::MIN, f32::MIN),
    };
    if face
        .outline_glyph(rustybuzz::ttf_parser::GlyphId(gid as u16), &mut oc)
        .is_none()
        || oc.segs.is_empty()
    {
        return;
    }
    let (min, max) = (oc.min, oc.max);
    if !(max.0 - min.0 < MAX_GLYPH_PX && max.1 - min.1 < MAX_GLYPH_PX) {
        return;
    }
    let bx = (gx + min.0).floor() as i32;
    let by = (gy + min.1).floor() as i32;
    let w = ((gx + max.0).ceil() as i32 - bx + 1).max(1) as usize;
    let h = ((gy + max.1).ceil() as i32 - by + 1).max(1) as usize;

    let ox = gx - bx as f32;
    let oy = gy - by as f32;
    let pt = |p: (f32, f32)| point(p.0 + ox, p.1 + oy);
    let mut r = Rasterizer::new(w, h);
    for s in &oc.segs {
        match *s {
            Seg::Line(a, b) => r.draw_line(pt(a), pt(b)),
            Seg::Quad(a, b, c) => r.draw_quad(pt(a), pt(b), pt(c)),
            Seg::Cubic(a, b, c, d) => r.draw_cubic(pt(a), pt(b), pt(c), pt(d)),
        }
    }
    r.for_each_pixel_2d(|dx, dy, v| {
        let px = bx + dx as i32;
        let py = by + dy as i32;
        if px < 0 || py < 0 {
            return;
        }
        let (px, py) = (px as u32, py as u32);
        let Some(pixel) = img.get_pixel_checked(px, py) else {
            return;
        };
        let alpha = (v * 255.0) as u8;
        if alpha == 0 {
            return;
        }
        let background = Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
        // The coverage scales the colour's own alpha, then it goes over the background: on a
        // transparent canvas the edge pixels keep the colour of the text (no dark fringe).
        let covered = Rgba { a: (color.a as u32 * alpha as u32 / 255) as u8, ..color };
        img.put_pixel(px, py, Rgba::blend(covered, background));
    });
}
