pub mod text;
pub mod load_image;
pub mod utility;
pub mod font;
pub mod buffer;

use std::fs::File;
use std::io::copy;
use std::io::Cursor;


use raqote::DrawOptions;
use raqote::DrawTarget;
use image::{codecs::png::{CompressionType, FilterType, PngEncoder}, ImageBuffer, ImageEncoder, Rgba, RgbaImage};

pub use raqote;
pub use image;
use raqote::PathBuilder;
use raqote::Source;

// pub trait Shapes {
//     fn new(width: i32, height: i32) -> Self;
//     fn save(&self, path: &str) -> Result<(), png::EncodingError>;
//     fn save_with_buffer(&self, path: &str, buffer: Vec<u8>) -> Result<(), png::EncodingError>;
//     fn get_png_buffer(&self, compression: CompressionType, filter: FilterType) -> Vec<u8>;
//     fn draw_text(self, font_path: String, text: String, x: f32, y: f32, size: f32, color: utility::Rgba) -> Self;
// }

pub struct Ghe2d {
    pub draw_target : DrawTarget
}

impl Ghe2d {
    pub fn new(width: i32, height: i32) -> Ghe2d {
        // DrawTarget::new(width, height)
        Ghe2d {
            draw_target: DrawTarget::new(width, height)
        }
    }

    pub fn save(&self, path: &str) -> Result<(), png::EncodingError> {
        self.draw_target.write_png(path)
    }

    pub fn save_with_png(&self, path: &str, compression: CompressionType, filter: FilterType) -> Result<(), png::EncodingError> {
        let mut dest = File::create(path)?;
        let mut content = Cursor::new(self.get_png_buffer(compression, filter));
        copy(&mut content, &mut dest)?;
        Ok(())
    }

    pub fn save_with_webp(&self, path: &str, quality: f32) -> Result<(), libwebp_sys::WebPEncodingError> {
        let mut dest = File::create(path).unwrap();
        let mut content = Cursor::new(self.get_webp_buffer(quality)?);
        copy(&mut content, &mut dest).unwrap();
        Ok(())
    }
    
    pub fn get_png_buffer(&self, compression: CompressionType, filter: FilterType) -> Vec<u8> {
        buffer::image_to_png_buffer(&self.draw_target, compression, filter)
    }

    pub fn get_webp_buffer(&self, quality: f32) -> Result<Vec<u8>, libwebp_sys::WebPEncodingError> {
        buffer::image_to_webp_buffer(&self.draw_target, quality)
    }

    pub fn draw_text(&mut self, load_font: font::LoadFont, text: String, x: f32, y: f32, size: f32, color: utility::Rgba) -> &Ghe2d {
        text::draw_text(&mut self.draw_target, load_font, text, x, y, size, color);
        self
    }

    pub fn load_image(&mut self, path: &str, x: f32, y: f32, width: f32, height: f32, is_circle: bool) -> &Ghe2d {
        load_image::add_image_mut(&mut self.draw_target, path, x, y, width, height, is_circle);
        self
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, width: f32, height: f32, source: Source) -> &Ghe2d {
        self.draw_target.fill_rect(x, y, width, height, &source, &DrawOptions::default());
        self
    }

    pub fn draw_circle(&mut self, x: f32, y: f32, raduis: f32, start: f32, end: f32, source: Source) -> &Ghe2d {
        let mut path_builder = PathBuilder::new();
        path_builder.arc(x, y, raduis, start, end);
        self.draw_target.fill( &path_builder.finish(), &source, &DrawOptions::default());
        self
    }
}
