use std::{fs, io::{self, Read}, path::Path};
use rusttype::Font;

#[derive(Debug, Clone)]
pub struct LoadFont{
    pub font: Font<'static>
}

impl LoadFont{
    pub fn new(font_path: String) -> Self {
        let font_data = get_font_file_to_buffer(font_path).unwrap();
        LoadFont{
            font: Font::try_from_vec(font_data).unwrap()
        }
    }
}

fn get_font_file_to_buffer<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}