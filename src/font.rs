use std::sync::Arc;
use std::{fs, io::{self, Read}, path::Path};
use rusttype::Font;

use crate::Ghe2dError;

const WGHT: [u8; 4] = *b"wght";

#[derive(Debug, Clone)]
pub struct LoadFont {
    /// Parsed face used for glyph rasterization.
    pub font: Font<'static>,
    /// Raw font bytes, re-parsed per `draw_text` call for OpenType shaping.
    data: Arc<Vec<u8>>,
    /// Requested `wght` axis value for variable fonts (already clamped to the axis range).
    weight: Option<f32>,
}

impl LoadFont {
    pub fn new(font_path: String) -> Result<Self, Ghe2dError> {
        let font_data = get_font_file_to_buffer(font_path).map_err(Ghe2dError::Io)?;
        Self::from_bytes(font_data)
    }

    pub fn from_bytes(font_data: Vec<u8>) -> Result<Self, Ghe2dError> {
        let font = Font::try_from_vec(font_data.clone()).ok_or(Ghe2dError::FontLoad)?;
        // Reject anything the shaper cannot parse either, so `draw_text` never has to.
        if rustybuzz::Face::from_slice(&font_data, 0).is_none() {
            return Err(Ghe2dError::FontLoad);
        }
        Ok(LoadFont { font, data: Arc::new(font_data), weight: None })
    }

    /// A shaping face borrowing the stored bytes, with the requested variation applied.
    /// Cheap (sub-millisecond) table parse. The same face is used for shaping and for
    /// reading glyph outlines, so both always agree.
    pub(crate) fn shaper_face(&self) -> rustybuzz::Face<'_> {
        let mut face =
            rustybuzz::Face::from_slice(&self.data, 0).expect("validated in LoadFont::from_bytes");
        if let Some(w) = self.weight {
            face.set_variations(&[rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag::from_bytes(&WGHT),
                value: w,
            }]);
        }
        face
    }

    /// `(min, default, max)` of the `wght` axis when this is a variable font with one.
    pub fn weight_axis(&self) -> Option<(f32, f32, f32)> {
        let face = rustybuzz::Face::from_slice(&self.data, 0)?;
        let tag = rustybuzz::ttf_parser::Tag::from_bytes(&WGHT);
        face.variation_axes()
            .into_iter()
            .find(|a| a.tag == tag)
            .map(|a| (a.min_value, a.def_value, a.max_value))
    }

    /// True when the font has a `wght` variation axis.
    pub fn is_variable(&self) -> bool {
        self.weight_axis().is_some()
    }

    /// The weight this font renders at: the requested axis value for a variable font,
    /// otherwise the OS/2 weight class (400 when absent).
    pub fn weight(&self) -> f32 {
        if let Some(w) = self.weight {
            return w;
        }
        if let Some((_, def, _)) = self.weight_axis() {
            return def;
        }
        rustybuzz::Face::from_slice(&self.data, 0)
            .map(|f| f.weight().to_number() as f32)
            .unwrap_or(400.0)
    }

    /// Italic / oblique flag from the font's own tables.
    pub fn is_italic(&self) -> bool {
        rustybuzz::Face::from_slice(&self.data, 0)
            .map(|f| f.is_italic() || f.is_oblique())
            .unwrap_or(false)
    }

    /// A copy rendering at `weight` (clamped to the `wght` axis). A font without a `wght`
    /// axis is returned unchanged. Shaping and rasterizing both honor it.
    pub fn with_weight(&self, weight: f32) -> LoadFont {
        let mut out = self.clone();
        if let Some((min, _, max)) = self.weight_axis() {
            out.weight = Some(weight.clamp(min, max));
        }
        out
    }

    /// Whether the font maps `c` to a real glyph (not `.notdef`).
    pub fn has_glyph(&self, c: char) -> bool {
        rustybuzz::Face::from_slice(&self.data, 0)
            .and_then(|f| f.glyph_index(c))
            .map(|g| g.0 != 0)
            .unwrap_or(false)
    }
}

/// The files of one font family, with weight/italic face selection.
///
/// Files are added with their own weight (OS/2 class, or the `wght` axis range for a
/// variable font) and italic flag read from the font; [`FontFamily::pick`] returns the
/// closest face, using the variable axis when one covers the requested weight.
#[derive(Debug, Clone, Default)]
pub struct FontFamily {
    faces: Vec<LoadFont>,
}

impl FontFamily {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one file of the family.
    pub fn add(&mut self, font: LoadFont) -> &mut Self {
        self.faces.push(font);
        self
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    /// The best face for `weight` (100..=900) and `italic`.
    ///
    /// 1. Faces with the wanted italic flag win over the others (a missing italic falls back
    ///    to upright, never the reverse silently preferred).
    /// 2. A variable face whose `wght` range contains `weight` is an exact match.
    /// 3. Otherwise the nearest static weight; on a tie the heavier one when `weight >= 500`,
    ///    the lighter one below that.
    ///
    /// `None` only for an empty family.
    pub fn pick(&self, weight: f32, italic: bool) -> Option<LoadFont> {
        let same: Vec<&LoadFont> = self.faces.iter().filter(|f| f.is_italic() == italic).collect();
        let pool: Vec<&LoadFont> = if same.is_empty() { self.faces.iter().collect() } else { same };

        let mut best: Option<(f32, f32, &LoadFont)> = None; // (distance, tie key, face)
        for f in pool {
            let (dist, tie) = match f.weight_axis() {
                Some((min, _, max)) => {
                    let w = weight.clamp(min, max);
                    ((weight - w).abs(), 0.0)
                }
                None => {
                    let fw = f.weight();
                    let heavier = if weight >= 500.0 { -fw } else { fw };
                    ((weight - fw).abs(), heavier)
                }
            };
            let better = match &best {
                None => true,
                Some((bd, bt, _)) => dist < *bd - 0.001 || ((dist - *bd).abs() <= 0.001 && tie < *bt),
            };
            if better {
                best = Some((dist, tie, f));
            }
        }
        best.map(|(_, _, f)| if f.is_variable() { f.with_weight(weight) } else { f.clone() })
    }
}

fn get_font_file_to_buffer<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
