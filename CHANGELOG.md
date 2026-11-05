# Changelog

## 0.2.3

Additive release: no function, type or option of 0.2.2 was removed or changed. One thing that was
wrong is fixed (see *Fixed*).

### Added

- **Shadow and glow for every drawable.** Every drawing option (`RectOptions`, `CircleOptions`,
  `EllipseOptions`, `PolygonOptions`, `StarOptions`, `RhombusOptions`, `PathOptions`, `LineOptions`,
  `TextOptions`, `FitImageOptions`) takes `.opacity(f32)`, `.shadow(Shadow)`, `.glow(Glow)` and
  `.effects(Effects)`. The shadow and the glow are made from the alpha of what was drawn (the shape of
  the letters for a text, the opaque part of a picture), so they follow the drawable exactly. `Shadow`
  has an offset, a blur, a colour and an opacity; `Glow` has a blur, a colour and an intensity. Blur
  radii follow the CSS convention (a blur of `R` is a Gaussian with a standard deviation of `R / 2`).
  Opacity fades the drawable and its shadow and glow together.
- **Groups.** `Ghe2d::draw_group(GroupOptions, |layer, ox, oy| ...)` draws anything onto a separate
  transparent layer and puts it on the canvas as one piece: clipped to a box (`.clip(true)`), turned
  about the centre of the box or about a `.pivot(x, y)`, faded, with a shadow and a glow made from the
  combined shape. This is what the options above use inside, and it is how several drawings share one
  rotation, one clip or one shadow.
- **Rotation for text and images.** `TextOptions::rotation(degrees)` and
  `FitImageOptions::rotation(degrees)` turn the whole block about the centre of its box (clockwise).
- **Fitting an image into a box.** `Ghe2d::draw_image_fit` / `draw_image_fit_bytes` with
  `FitImageOptions` and `ImageFit::{Cover, Contain, Fill}` (like CSS `object-fit`) and `.position(x, y)`
  to choose which part shows or where a contained picture sits. The picture is resampled once
  (Lanczos when it shrinks, Catmull-Rom when it grows) with the colour weighted by alpha, so
  transparent edges get no dark fringe; only the part that can be seen is resampled.
- **Rounded rectangles, ellipses and free paths.** `RectOptions::radius`, `EllipseOptions` (with
  `in_box`) and `PathOptions` (a polygon or an open polyline through any points, with a `triangle`
  helper), plus `Ghe2d::draw_ellipse` and `Ghe2d::draw_path`.
- **`TextOptions::shrink_to_fit(false)`** keeps the size when a `width` is set (the width then only
  aligns the text).
- `Ghe2d::from_image(RgbaImage)`.
- Example `10_effects_and_layers` (it also prints the timing of a typical 1200 x 630 card of about
  twenty elements: about 150 ms in a release build on one thread, plus about 45 ms to encode it as
  WebP), and golden-image tests with a tolerance (`tests/golden.rs`, the pictures are in
  `tests/golden/`; run with `GHE2D_BLESS=1` to write them again).

### Fixed

- **Text on a transparent canvas.** The soft edge pixels of a glyph were mixed with the black of the
  empty canvas, so a light text got a dark fringe when the canvas was placed over something else. A
  text colour with its own alpha also made an opaque canvas thinner under the glyphs. The coverage of
  a glyph now scales the alpha of the colour and the result goes over the canvas the usual way. Text
  on an opaque canvas looks the same as before.
