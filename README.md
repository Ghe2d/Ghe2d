# ghe2d

A small 2D canvas for Rust: draw shapes and text onto an RGBA image, load and composite
other images, and export to PNG / JPEG / WebP.

Every drawing call takes one **options builder** (`RectOptions`, `TextOptions`,
`ImageOptions`, `ExportOptions`, …). Coordinates are `f32`, everything is clipped to the
canvas, and every shape edge is anti-aliased.

```toml
[dependencies]
ghe2d = "0.2"
```

MSRV: Rust 1.80.

## Quick start

```rust
use ghe2d::utility::Rgba;
use ghe2d::{Ghe2d, RectOptions, CircleOptions, StarOptions, ExportOptions};

let mut c = Ghe2d::new(400, 300);

c.draw_rect(RectOptions::solid(0.0, 0.0, 400.0, 300.0, Rgba::rgb(18, 20, 28)));
c.draw_circle(CircleOptions::new(60.0).at(120.0, 150.0).fill(Rgba::rgb(90, 200, 255)));
c.draw_star(StarOptions::new(70.0).at(280.0, 150.0).points(6).fill(Rgba::WHITE).stroke(Rgba::rgb(240, 200, 110), 3.0));

c.save("out.png", ExportOptions::png())?;
```

## Canvas

```rust
Ghe2d::new(width, height)                    // blank RGBA canvas
Ghe2d::from_bytes(bytes, ImageOptions::new())?      // decode a PNG/JPEG/WebP/... (format auto-detected)
Ghe2d::from_path("pic.jpg", ImageOptions::new()).await?   // local file or http(s):// URL

c.width() / c.height()
c.source_format()                            // Option<ImageFormat> — what it was decoded from
c.image                                       // the underlying image::RgbaImage
```

## Shapes

All shapes: `.fill(Rgba)` (or `.no_fill()`), `.stroke(Rgba, width)`, and for polygons /
star / rhombus / rect a `.rotation(degrees)`. Edges are always anti-aliased.

```rust
c.draw_rect(RectOptions::new(80.0, 40.0).at(x, y).rotation(15.0).fill(col));
c.draw_circle(CircleOptions::new(radius).at(x, y).fill(col));
c.draw_polygon(PolygonOptions::new(radius).at(x, y).sides(6));   // sides default 3 = triangle
c.draw_star(StarOptions::new(radius).at(x, y).points(5).inner_ratio(0.45));
c.draw_rhombus(RhombusOptions::new(w, h).at(x, y).fill(col));
c.draw_line(LineOptions::new(x1, y1, x2, y2).color(col).width(3.0));
```

`RectOptions::solid(x, y, w, h, col)` and `CircleOptions::solid(x, y, r, col)` are one-shot
shortcuts. `Rgba` has `BLACK` / `WHITE` / `TRANSPARENT` and `Rgba::rgb(r, g, b)`.

More shapes (0.2.3):

```rust
c.draw_rect(RectOptions::solid(x, y, w, h, col).radius(16.0));          // rounded corners
c.draw_ellipse(EllipseOptions::in_box(x, y, w, h).fill(col));           // or ::new(rx, ry).at(cx, cy).rotation(20.0)
c.draw_path(PathOptions::triangle(x, y, w, h).fill(col));               // any polygon: PathOptions::new(points)
c.draw_path(PathOptions::new(points).closed(false).no_fill().stroke(col, 3.0));  // a polyline
```

## Effects: opacity, shadow, glow

Every drawing option (shapes, `TextOptions`, `FitImageOptions`) takes the same three setters:

```rust
use ghe2d::{Glow, Shadow};

c.draw_rect(
    RectOptions::solid(40.0, 40.0, 200.0, 100.0, Rgba::rgb(250, 190, 60))
        .radius(20.0)
        .shadow(Shadow::new(Rgba::new(0, 0, 0, 160)).offset(0.0, 8.0).blur(16.0))
        .opacity(0.9),
);
c.draw_text(TextOptions::new(font, "Glow").at(40.0, 200.0).size(48.0).color(Rgba::WHITE)
    .glow(Glow::new(Rgba::rgb(255, 120, 120)).blur(14.0).intensity(1.5)));
```

The shadow and the glow are made from the alpha of the drawn thing (the letters of a text, the opaque
part of a picture), the shadow is drawn first, then the glow, then the drawable. A blur of `R` px is a
Gaussian with a standard deviation of `R / 2` (the CSS convention). `.opacity()` fades the drawable
and its shadow and glow together.

## Groups (layers)

`draw_group` draws anything onto a separate transparent layer and puts it on the canvas as one piece:
clipped to a box, turned about its centre (or a `.pivot`), faded, with a shadow and a glow made from
the whole.

```rust
use ghe2d::GroupOptions;

c.draw_group(
    GroupOptions::new(40.0, 30.0, 300.0, 120.0).rotation(-8.0).clip(true).opacity(0.95)
        .shadow(Shadow::new(Rgba::new(0, 0, 0, 170)).offset(6.0, 8.0).blur(16.0)),
    |layer, ox, oy| {
        // (ox, oy) is where the box starts inside the layer: draw at ox + x, oy + y.
        layer.draw_rect(RectOptions::solid(ox, oy, 300.0, 120.0, Rgba::rgb(250, 190, 60)));
        layer.draw_text(TextOptions::new(font, "Cut at the box").at(ox + 12.0, oy + 40.0).size(40.0));
    },
);
```

What is drawn farther than `.margin(px)` outside the box (2 px by default, plus the reach of the shadow
and the glow) is not kept; `.clip(true)` cuts at the box on purpose.

## Fitting an image into a box

```rust
use ghe2d::{FitImageOptions, ImageFit};

c.draw_image_fit_bytes(&bytes, FitImageOptions::new(x, y, w, h).fit(ImageFit::Cover))?;   // Cover (default) / Contain / Fill
c.draw_image_fit(&decoded_rgba, FitImageOptions::new(x, y, w, h).fit(ImageFit::Contain).position(0.5, 0.0)
    .rotation(10.0).shadow(Shadow::new(Rgba::BLACK).blur(10.0)));
```

`Cover` fills the box and crops the overflow, `Contain` keeps the whole picture, `Fill` stretches it;
`.position(x, y)` (0 to 1) says which part shows or where a contained picture sits.

## Text

Real OpenType shaping (`rustybuzz`) + the Unicode bidi algorithm — Arabic, mixed
directions and any font with the base glyphs render correctly.

```rust
use ghe2d::{TextOptions, TextAlign, LineLayout, font::LoadFont};

let font = LoadFont::new("Cairo-Regular.ttf".into())?;   // or LoadFont::from_bytes(bytes)?

c.draw_text(
    TextOptions::new(font, "مرحبا بالعالم")
        .at(20.0, 20.0)
        .size(48.0)
        .color(Rgba::WHITE)
        .width(360.0)                 // constrain horizontally
        .align(TextAlign::Center)     // Left / Right / Center (default follows text direction)
        .wrap(true)                   // word-wrap; without it the size shrinks to fit
        .height(200.0)                // constrain vertically
        .line_layout(LineLayout::SpaceBetween),
);
```

`size` is the pixel height of ascent minus descent. To match a browser's CSS `font-size` use
`.css_size(px)` (or `font.css_to_size(px)`), which converts through the font's own metrics.

With a `width` the size shrinks until the widest line fits (or the text wraps, with `.wrap(true)`);
`.shrink_to_fit(false)` keeps the size and uses the width only for alignment. `.rotation(degrees)`
turns the whole block about the centre of its box.

`LineLayout`: `Gap(px)` (default), `Top(px)` / `Middle(px)` / `Bottom(px)` and
`SpaceBetween` / `SpaceEvenly` (the last four need `height`).

## Compositing other images

```rust
use ghe2d::{ImageOptions, ComposeMode};

c.draw_image_bytes(png_bytes, ImageOptions::new().at(10, 10).size(64, 64).circle(true))?;
c.draw_image_path("logo.png", ImageOptions::new().at(10, 10).compose(ComposeMode::Blend)).await?;
```

`ComposeMode`: `Normal` (replace, default), `Overlay` (alpha over), `Blend` (per-pixel,
skips transparent source pixels). `.circle(true)` crops the source to a centred circle.

## Export

```rust
c.encode(ExportOptions::png())?;                                   // -> Vec<u8>
c.encode(ExportOptions::webp(80.0))?;
c.encode(ExportOptions::jpeg(85).background(Rgba::WHITE))?;         // JPEG has no alpha -> flattened
c.save("out.jpg", ExportOptions::jpeg(85))?;

ghe2d::Ghe2d::detect_format(&bytes)   // Option<ImageFormat>, from the magic bytes (never the extension)
```

PNG options: `.compression(..)` / `.filter(..)`. Decoding accepts PNG / JPEG / WebP / GIF /
BMP / TIFF / …; encoding supports PNG / JPEG / WebP.

## Errors

Every fallible call returns `Result<_, ghe2d::Ghe2dError>` (`Debug + Display + Error`).
Nothing panics on bad input — oversized WebP, a zero-sized canvas, an unreadable font, a
corrupt buffer, and shapes drawn far off-canvas all return an error or are simply clipped.

## Examples

`cargo run --example 08_shapes` (and `01`…`07`, `09`, `10`) — see [`examples/`](examples/). Output
lands in `target/example-output/`.

## Status

`0.2.x` is a breaking rewrite of `0.1.x` (options builders everywhere, real text shaping,
JPEG support, hardened error handling), with animation, font fallback and variable fonts
added since. See [`CHANGELOG.md`](CHANGELOG.md) for the latest release.

License: [LICENSE-MIT](LICENSE).
