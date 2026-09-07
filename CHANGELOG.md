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
- `utility::Rgba` now derives `Copy`.

### Changed

- New crate-wide error type **`Ghe2dError`** (`Debug + Display + std::error::Error`, **not
  `Clone`** — it can carry `std::io::Error`). `ErrorMessage` is unchanged and is wrapped by
  `Ghe2dError::Image`.
- Public API now returns `Result<_, Ghe2dError>`:
  `Ghe2d::new_by_load_from_path`, `new_by_load_from_buffer`, `to_png_buffer`,
  `to_webp_buffer`, `save_with_png`, `save_with_webp`; and
  `load_normal_image` / `load_blend_image` / `load_overlay_image` /
  `from_buffer_to_overlay` / `from_buffer_to_normal` now return
  `Result<&mut Self, Ghe2dError>` instead of swallowing errors.
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
let webp = g.to_webp_buffer(70.0).map_err(|_| MyErr)?;
let font = LoadFont::new(path);

// after
let g = Ghe2d::new_by_load_from_buffer(bytes, None, None, false)?; // Ghe2dError
let webp = g.to_webp_buffer(70.0)?;                                // Ghe2dError
let font = LoadFont::new(path)?;                                   // Ghe2dError
```
