use std::sync::Arc;
use std::{fs, io::{self, Read}, path::Path};
use rusttype::Font;

use crate::Ghe2dError;

#[derive(Debug, Clone)]
pub struct LoadFont {
    /// Parsed face used for glyph rasterization.
    pub font: Font<'static>,
    /// Raw font bytes, re-parsed per `draw_text` call for OpenType shaping.
    data: Arc<Vec<u8>>,
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
        Ok(LoadFont { font, data: Arc::new(font_data) })
    }

    /// A shaping face borrowing the stored bytes. Cheap (sub-millisecond) table parse.
    pub(crate) fn shaper_face(&self) -> rustybuzz::Face<'_> {
        rustybuzz::Face::from_slice(&self.data, 0).expect("validated in LoadFont::from_bytes")
    }
}

fn get_font_file_to_buffer<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}
