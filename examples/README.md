# ghe2d examples

Ten runnable examples that together drive every public entry point of the crate. Each
writes generated images into `target/example-output/` (created on run).

```bash
cargo run --example 01_shapes_and_gradients   # new / shape builders / encode+save with ExportOptions / clipping
cargo run --example 02_text_multilang         # LoadFont / TextOptions builder: LTR, RTL, mixed, multiline, align, wrap, shrink-to-fit, LineLayout
cargo run --example 03_buffer_compositing     # encode -> Ghe2d::from_bytes / draw_image_bytes with every ComposeMode
cargo run --example 04_async_load_from_path   # Ghe2d::from_path / draw_image_path + source_format / detect_format  (needs the tokio dev-dependency)
cargo run --example 05_poster_pipeline        # full composition + multi-font text + WebP quality sweep
cargo run --example 06_edge_cases_and_errors  # every Ghe2dError variant + all hardened panic/abort paths
cargo run --example 07_image_formats          # encode one canvas as PNG / JPEG (+ background) / WebP, compare sizes, round-trip
cargo run --example 08_shapes                 # polygon (3..8 sides) / star / rhombus / line, rotation, fill + stroke
cargo run --example 09_animation_and_fonts    # 0.2.1: font fallback chain, variable weights, FontFamily::pick, measure_text, make an animated GIF -> read it -> convert to animated WebP (+ crop/resize), DecodeLimits
cargo run --release --example 10_effects_and_layers   # 0.2.3: shadow / glow / opacity on every drawable, rotated text and images, rounded rect / ellipse / path, cover / contain / fill, groups with a clip, and the timing of a typical 1200x630 card (add `-- bench`)
```

`06_edge_cases_and_errors` prints an `OK / FAIL` line per case and must finish without a
panic — it is the "everything is theoretically sound" check.

## Assets

`assets/fonts/` holds four faces used by examples 02 and 05:

| file | use |
|---|---|
| `Cairo-Regular.ttf` | Arabic + Latin body text |
| `Tajawal-Bold.ttf` | Arabic display |
| `Rubik-Regular.ttf` | Latin |
| `Ruwudu-Regular.ttf` | decorative Arabic (OpenType-shaped, ships no presentation forms) |

Text is shaped with `rustybuzz` (OpenType) + `unicode-bidi`, so any font that has the base
glyphs renders correctly — no Arabic Presentation Forms required.

Example 09 also uses the variable font `tests/fixtures/Cairo-Variable.ttf` (SIL Open Font
License 1.1).

These `.ttf` files are **not** packaged into the published crate (the `include` globs in
`Cargo.toml` only match the `.rs` files under `src/`, `tests/` and `examples/`).
