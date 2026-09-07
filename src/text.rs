//! Text rendering: Unicode BiDi reordering (`unicode-bidi`) + OpenType shaping
//! (`rustybuzz`), rasterized glyph-by-glyph with `rusttype`.

pub fn draw_text(
    img: &mut image::RgbaImage,
    load_font: crate::font::LoadFont,
    text: String,
    x: f32,
    y: f32,
    size: f32,
    color: crate::utility::Rgba,
) {
    if size <= 0.0 || text.is_empty() {
        return;
    }

    let scale = rusttype::Scale::uniform(size);
    let vm = load_font.font.v_metrics(scale);
    let line_step = vm.ascent - vm.descent + vm.line_gap;
    let glyph_count = load_font.font.glyph_count() as u32;

    // `rusttype::Scale::uniform(size)` rasterizes so that `ascent - descent` spans `size`
    // pixels, i.e. its design-unit -> pixel factor is `size / (ascent - descent)` (NOT
    // `size / units_per_em`). `rustybuzz` advances/offsets are in the same design units, so
    // they must be scaled by the same factor to line up with the rasterized glyphs.
    let vmu = load_font.font.v_metrics_unscaled();
    let em = vmu.ascent - vmu.descent;
    if em <= 0.0 {
        return;
    }
    let sf = size / em;

    let face = load_font.shaper_face();

    for (line_idx, line) in text.split('\n').enumerate() {
        let baseline = y + vm.ascent + line_idx as f32 * line_step;
        if line.is_empty() {
            continue;
        }

        // Reorder the line into visual runs (left -> right), each a single bidi level.
        let bidi = unicode_bidi::BidiInfo::new(line, None);
        let Some(para) = bidi.paragraphs.first() else {
            continue;
        };
        let (levels, runs) = bidi.visual_runs(para, para.range.clone());

        let mut pen_x = x;
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
            // Fills script + language from the run content; leaves our direction untouched.
            buf.guess_segment_properties();

            let shaped = rustybuzz::shape(&face, &[], buf);
            let infos = shaped.glyph_infos();
            let positions = shaped.glyph_positions();

            for (info, pos) in infos.iter().zip(positions.iter()) {
                let gid = info.glyph_id;
                let gx = pen_x + pos.x_offset as f32 * sf;
                let gy = baseline - pos.y_offset as f32 * sf;

                // Skip .notdef (render nothing rather than tofu) and any out-of-range id.
                if gid != 0 && gid < glyph_count {
                    let glyph = load_font
                        .font
                        .glyph(rusttype::GlyphId(gid as u16))
                        .scaled(scale)
                        .positioned(rusttype::point(gx, gy));

                    if let Some(bb) = glyph.pixel_bounding_box() {
                        glyph.draw(|dx, dy, v| {
                            let px = bb.min.x + dx as i32;
                            let py = bb.min.y + dy as i32;
                            if px < 0 || py < 0 {
                                return; // clip: outside the canvas
                            }
                            let (px, py) = (px as u32, py as u32);
                            let Some(pixel) = img.get_pixel_checked(px, py) else {
                                return;
                            };
                            let alpha = (v * 255.0) as u8;
                            let background = crate::utility::Rgba::new(
                                pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3],
                            );
                            let blend =
                                crate::utility::Rgba::blend_with_alpha(&color, background, alpha);
                            crate::rect::draw_rect(
                                img,
                                px,
                                py,
                                1,
                                1,
                                crate::utility::Rgba::new(
                                    blend.0[0], blend.0[1], blend.0[2], blend.0[3],
                                ),
                            );
                        });
                    }
                }

                pen_x += pos.x_advance as f32 * sf;
            }
        }
    }
}
