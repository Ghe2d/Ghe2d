# ghe2d examples

Seven runnable examples that together drive every public entry point of the crate. Each
writes generated images into `target/example-output/` (created on run).

```bash
cargo run --example 01_shapes_and_gradients   # new / draw_rect / draw_circle / encode+save with ExportOptions / clipping
cargo run --example 02_text_multilang         # LoadFont / TextOptions builder: LTR, RTL, mixed, multiline, align, wrap, shrink-to-fit, LineLayout
cargo run --example 03_buffer_compositing     # encode -> Ghe2d::from_bytes / draw_image_bytes with every ComposeMode
cargo run --example 04_async_load_from_path   # Ghe2d::from_path / draw_image_path + source_format / detect_format  (needs the tokio dev-dependency)
cargo run --example 05_poster_pipeline        # full composition + multi-font text + WebP quality sweep
cargo run --example 06_edge_cases_and_errors  # every Ghe2dError variant + all hardened panic/abort paths
cargo run --example 07_image_formats          # encode one canvas as PNG / JPEG (+ background) / WebP, compare sizes, round-trip
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

These `.ttf` files are **not** packaged into the published crate (the `include` globs in
`Cargo.toml` only match `**/*.rs`).
