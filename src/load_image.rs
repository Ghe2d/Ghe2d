use image::{ImageBuffer, Rgba, imageops::resize, DynamicImage};
use num_integer::Roots;
use raqote::{DrawOptions, DrawTarget, SolidSource, Source};
use reqwest::blocking::get;
use regex::Regex;

pub fn add_image_mut( dt: &mut DrawTarget, path: &str, x: f32, y: f32, width: f32, height: f32, is_circle: bool) {

    let load_image: DynamicImage;
    
    if check_is_url_image(path) {
        let resp = get(path).expect("Failed to load image from URL");
        let bytes = resp.bytes().expect("Failed to read image bytes");
        load_image = image::load_from_memory(&bytes).expect("Failed to decode image");
    }
    else {
        load_image = image::open(path).expect("Failed to load image");
    }
    let resize_image = resize(&load_image, width as u32, height as u32, image::imageops::FilterType::Nearest);

    if is_circle {
        let cx = width / 2.;
        let cy = height / 2.;
        let radius = width.min(height) / 2.;
        let mut circle_img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::new(width as u32, height as u32);
        for x in 0..width as u32 {
            for y in 0..height as u32 {
                let dx = x as i32 - cx as i32;
                let dy = y as i32 - cy as i32;
                let dist = (dx * dx + dy * dy).sqrt() as u32;

                let pixel = resize_image.get_pixel(x, y);

                if dist < radius as u32 {
                    circle_img.put_pixel(x, y, *pixel);
                }
            }
        }
        for (_x, _y, pixel,) in circle_img.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if pixel != &Rgba([0,0,0,0]) {
                dt.fill_rect(_x as f32 + x, _y as f32 + y, 1., 1., 
                    &Source::Solid(SolidSource::from_unpremultiplied_argb(a, r, g, b)),
                    &DrawOptions::default()
                );
            }
        }
    }
    else {
        for (_x, _y, pixel,) in resize_image.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if pixel != &Rgba([0,0,0,0]) {
                dt.fill_rect(_x as f32 + x, _y as f32 + y, 1., 1., 
                    &Source::Solid(SolidSource::from_unpremultiplied_argb(a, r, g, b)),
                    &DrawOptions::default()
                );
            }
        }
    }
}

pub fn check_is_url_image(path: &str) -> bool{
    let re = Regex::new(r"http(s)?://([/|.|\w|\s|-])*\.(?:jpg|gif|png|bmp|webp)").unwrap();
    re.is_match(path)
}