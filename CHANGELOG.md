# Changelog

## 0.2.1

Additive release: nothing in the 0.2.0 public API changed.

### Added

- **`anim` module** (re-exported at the crate root):
  - `decode_animation(bytes, &DecodeLimits) -> Result<Animation, AnimError>` decodes
    PNG / JPEG (one frame), GIF and animated WebP (every frame with its delay, via
    `image::AnimationDecoder`). A 0 ms delay becomes 100 ms, like browsers do.
  - Safety limits chosen by the caller (`max_frames`, `max_total_pixels`, `max_duration_ms`).
    The canvas is checked before decoding and the rest as each frame arrives, so an
    over-limit file fails with a clear `AnimError` without decoding everything.
  - `Animation::encode_webp(quality)` / `encode_animated_webp` write an animated WebP with
    libwebp's `WebPAnimEncoder` (same `libwebp-sys` as before).
  - `crop_image` / `resize_image` (Lanczos3) for stills, and `Animation::crop` /
    `Animation::resize` applying the same operation to every frame.
  - `AnimError` is a separate error type, so `Ghe2dError` is unchanged.
  - Animated WebP encoding lets each frame choose lossy or lossless (`allow_mixed`) and forces a key frame at least every 6 frames, so flat backgrounds show no visible ghost traces of earlier frames (background error at most 1 of 255) and files get smaller.
  - `decode_animation` also accepts still (non-animated) WebP and BMP, returning a
    single-frame `Animation` with the same canvas-size limit check as PNG / JPEG.
- **Font chain (fallback).** `TextOptions::fallback(font)` / `fallbacks(iter)`: a character the
  primary font lacks is drawn from the first fallback that has it (scaled to the primary's
  pixels per em). A character no font has now draws the primary's `.notdef` box instead of
  vanishing (blank/control characters still draw nothing). Marks, joiners and spaces stay with
  the font of the preceding character. Arabic joining does not cross a font boundary.
- **Face selection.** `font::FontFamily` (`add`, `pick(weight, italic)`): nearest weight among
  the family's files, italic preferred when asked, a variable face covering the weight is an
  exact match. `LoadFont::{weight, is_italic, is_variable, weight_axis, has_glyph}`.
- **Variable fonts.** `LoadFont::with_weight(w)` and `TextOptions::weight(w)` set the `wght`
  axis (clamped to the axis range), applied to both shaping (`rustybuzz` `set_variations`) and
  rasterizing.
- **Measuring.** `measure_text(&TextOptions) -> TextMeasure { width, height, size, lines }` and
  `text_width(font, fallbacks, text, size)` use the same chain, shaping, wrapping and
  shrink-to-fit as `draw_text`, so a layout measured ahead of time breaks lines exactly like
  the final drawing.

### Changed (internal, API unchanged)

- **Rasterizer.** Glyphs are now drawn from `ttf-parser` outlines (the face rustybuzz already
  holds, so variation coordinates apply) filled by `ab_glyph_rasterizer`, the same coverage
  engine `rusttype` used. Reason: `rusttype` 0.9.3 reads outlines without variation support
  (no `gvar`/`HVAR`), so a variable font would shape at one weight and draw at the default.
  `rusttype` stays for `LoadFont::font` and line metrics. New dependency: `ab_glyph_rasterizer`.
