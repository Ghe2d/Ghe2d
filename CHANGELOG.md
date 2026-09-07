# Changelog

## 0.2.0

**BREAKING.** This release removes every known panic/abort path in the crate and makes the
public API consistently fallible.

### Fixed

- **`image_to_webp_buffer` / `to_webp_buffer` no longer abort the process.** The old code
  ignored `WebPEncodeBGRA`'s return value and called `slice::from_raw_parts` on the NULL
  pointer libwebp leaves on failure — undefined behaviour that fired a non-unwinding
  `abort` (`exit code 0xc0000409`), uncatchable by `catch_unwind` or `.map_err`. It now
  validates dimensions up front and checks the encoder result, returning
  `Err(Ghe2dError::ImageTooLarge)` for a side `> 16383` px and `Err(Ghe2dError::WebpEncode)`
  otherwise.
- **libwebp output buffer is now freed** with `WebPFree` on every path (was leaked on every
  successful encode).
- **Dropped a redundant full-image clone** in the RGBA→BGRA swizzle.
- **`image_to_png_buffer` / `to_png_buffer`** no longer panic on a zero-sized canvas; they
  return `Err(Ghe2dError::PngEncode)`.
- **`LoadFont::new`** no longer panics on a missing file or invalid font data.
- **Shapes and text clip instead of panicking.** `draw_rect`, `draw_circle` and glyph
  rasterisation in `draw_text` now draw the in-bounds portion and silently skip anything
  outside the canvas. Coordinate math is overflow-safe.
- **Circle-mask underflow fixed.** The `is_circle` resize path in `load_image` /
  `load_buffer` computed `x - cx` on `u32`, panicking in debug and producing a corrupted
  mask in release for the left/top half of the image. Distances are now computed with
  signed arithmetic and squared comparison (no `sqrt`).
- **`add_image_blend_mut`** blended every source pixel against the overlay's origin pixel
  `(x, y)` instead of the matching destination pixel; it now reads the correct destination
  and clips out-of-bounds reads.
- The per-call `Regex::new(...).unwrap()` compilation in `check_is_url_image` is now a
  `LazyLock` static compiled once.

### Text engine

- **`draw_text` now uses real OpenType shaping.** The hand-rolled reshaper
  (`ar-reshaper` → Unicode Arabic Presentation Forms → manual RTL reversal in
  `fix_arabic_text`) is replaced by `rustybuzz` (a pure-Rust HarfBuzz port) for shaping and
  `unicode-bidi` for the Unicode Bidirectional Algorithm; `rusttype` stays only as the
  glyph rasterizer.
- Any font that contains the base glyphs now renders correctly — fonts that omit the legacy
  Arabic presentation-form glyphs (common for OpenType-first faces) no longer produce
  `.notdef` boxes.
- Mixed LTR/RTL lines are reordered correctly per bidi run.
- `.notdef` glyphs are skipped (nothing drawn) rather than rendered as tofu.
- `fix_arabic_text` / `is_arabic` are removed.

### Text layout — `TextOptions` builder

- **`Ghe2d::draw_text` now takes a single [`TextOptions`] builder** instead of seven
  positional args (`font`, `text`, `x`, `y`, `size`, `color` become `.new(font, text)` +
  `.at(x, y).size(..).color(..)`). Coordinates and size are `f32`. **BREAKING.**
- `.width(w)` constrains the text horizontally:
  - `.wrap(false)` (default) shrinks the effective size until the widest line fits;
  - `.wrap(true)` greedily word-wraps (an over-long word gets its own line and overflows —
    glyphs are never split).
- `.align(TextAlign::{Left,Right,Center})` positions each line inside the width box; without
  an explicit align it follows the text's base direction (RTL → right, LTR → left). Needs
  `.width`.
- `.height(h)` enables the vertical distribution modes and shrinks the size if the line
  block would be taller than `h`.
- `.line_layout(LineLayout::…)` — `Gap(px)` (extra px between lines, default `Gap(0.0)` ==
  previous behavior), `Top(px)` / `Middle(px)` / `Bottom(px)` (need `height`), and
  `SpaceBetween` / `SpaceEvenly` (need `height`, derive their own spacing).
- `utility::Rgba` now derives `Copy` (and `Debug`, `PartialEq`, `Eq`).

### Image I/O — option structs, format detection, JPEG

**BREAKING.** The 12 positional-argument load/encode/save methods are replaced by 8
methods driven by builders.

- **Input** — `ImageOptions` builder + `ComposeMode { Normal (default), Overlay, Blend }`:
  - `Ghe2d::from_bytes(bytes, ImageOptions)` (sync) and
    `Ghe2d::from_path(path, ImageOptions).await` (async; local file **or** `http(s)://`)
    replace `new_by_load_from_buffer` / `new_by_load_from_path`.
  - `g.draw_image_bytes(bytes, ImageOptions)` (sync) and
    `g.draw_image_path(path, ImageOptions).await` (async) replace
    `from_buffer_to_normal` / `from_buffer_to_overlay` /
    `load_normal_image` / `load_blend_image` / `load_overlay_image` — the `_normal` /
    `_overlay` / `_blend` suffix is now `ImageOptions::compose(ComposeMode::…)`.
  - `ImageOptions`: `.at(x, y)` `.size(w, h)` `.width(w)` `.height(h)` `.circle(bool)`
    `.compose(mode)`. `at` / `compose` are ignored by the constructors.
- **Format detection**: decoding auto-detects the container from the bytes (PNG / JPEG /
  WebP / GIF / BMP / TIFF / …). `Ghe2d::detect_format(&[u8]) -> Option<ImageFormat>` and
  `g.source_format() -> Option<ImageFormat>` (the format a canvas was loaded from; `None`
  for `Ghe2d::new`). `image::ImageFormat` is re-exported as `ghe2d::ImageFormat`.
- **Output** — `ExportOptions` + `ExportFormat`:
  - `g.encode(ExportOptions) -> Result<Vec<u8>, Ghe2dError>` replaces `to_png_buffer` /
    `to_webp_buffer`.
  - `g.save(path, ExportOptions) -> Result<(), Ghe2dError>` replaces the old 1-arg `save`
    and `save_with_png` / `save_with_webp`.
  - `ExportOptions::png()` / `jpeg(quality)` / `webp(quality)`, plus `.compression(c)` /
    `.filter(f)` (PNG) and `.background(Rgba)` (JPEG).
- **JPEG** is now supported for both decode and encode. It has no alpha channel, so
  `ExportFormat::Jpeg { quality, background }` flattens transparency over `background`
  (opaque white by default). New `Ghe2dError::JpegEncode`.
- `Ghe2dError::InvalidText` removed (was unused).
- Dependency `num-integer` dropped (unused since the circle-mask rewrite); the
  `pub use num_integer` re-export is gone. Modules `ghe2d::buffer` and `ghe2d::load_image`
  are replaced by `ghe2d::image_io` and `ghe2d::export`.

### Changed

- New crate-wide error type **`Ghe2dError`** (`Debug + Display + std::error::Error`, **not
  `Clone`** — it can carry `std::io::Error`). `ErrorMessage` is unchanged and is wrapped by
  `Ghe2dError::Image`.
- Every fallible public method returns `Result<_, Ghe2dError>` (the image-input methods
  return `Result<&mut Self, _>` instead of swallowing errors into a hidden buffer).
- `LoadFont::new` returns `Result<Self, Ghe2dError>`; added `LoadFont::from_bytes`.
- Removed the unreachable `Ghe2d::errors` buffer and the `Error` / `ErrorKind` types.
- Dependencies: removed `ar-reshaper`; added `rustybuzz` and `unicode-bidi`. Crate
  re-exports change accordingly (`pub use ar_reshaper` → `pub use rustybuzz` +
  `pub use unicode_bidi`).
- MSRV raised to Rust **1.80** (`std::sync::LazyLock`).

### Migration

```rust
// before
let g = Ghe2d::new_by_load_from_buffer(bytes, None, None, false).unwrap();
g.from_buffer_to_overlay(logo, 10, 10, Some(64), Some(64), true)?;
let webp = g.to_webp_buffer(70.0).map_err(|_| MyErr)?;
g.save_with_png("out.png", CompressionType::Best, FilterType::Paeth)?;
let font = LoadFont::new(path);
g.draw_text(font, "hi".into(), 10, 20, 32, color);

// after
use ghe2d::{ImageOptions, ComposeMode, ExportOptions, TextOptions};
let g = Ghe2d::from_bytes(bytes, ImageOptions::new())?;
g.draw_image_bytes(logo, ImageOptions::new().at(10, 10).size(64, 64).circle(true)
                                            .compose(ComposeMode::Overlay))?;
let webp = g.encode(ExportOptions::webp(70.0))?;
g.save("out.png", ExportOptions::png().compression(CompressionType::Best)
                                      .filter(FilterType::Paeth))?;
let font = LoadFont::new(path)?;
g.draw_text(TextOptions::new(font, "hi").at(10.0, 20.0).size(32.0).color(color));
```
