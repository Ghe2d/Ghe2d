use image::{ImageBuffer, Rgba, imageops::resize, DynamicImage};
use num_integer::Roots;
use reqwest::blocking::get;
use regex::Regex;

pub fn add_image_mut( img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: u32, height: u32, is_circle: bool) {

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
        let cx = width / 2;
        let cy = height / 2;
        let radius = width.min(height) / 2;
        let mut circle_img: ImageBuffer<Rgba<u8>, _> = ImageBuffer::new(width as u32, height as u32);
        for x in 0..width as u32 {
            for y in 0..height as u32 {
                let dx = x - cx;
                let dy = y - cy;
                let dist = (dx * dx + dy * dy).sqrt() as u32;

                let pixel = resize_image.get_pixel(x, y);

                if dist < radius as u32 {
                    circle_img.put_pixel(x, y, *pixel);
                }
            }
        }
        for (draw_x, draw_y, pixel,) in circle_img.enumerate_pixels() {
            if pixel != &Rgba([0,0,0,0]) {
                let foreground = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                let distance_to_edge = distance_to_nearest_edge(x, y, width, height);

                let color: super::utility::Rgba;
                let edge_width = 50;
                if distance_to_edge < edge_width {
                    let enhance_brightness = crate::utility::Rgba::enhance_brightness(foreground, distance_to_edge, edge_width);
                    color = super::utility::Rgba::new(enhance_brightness.0[0], enhance_brightness.0[1], enhance_brightness.0[2], enhance_brightness.0[3]);
                }
                else {
                    color = super::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                }
                super::rect::draw_rect(
                    img,
                    draw_x + x,
                    draw_y + y,
                    1,
                    1,
                    color
                );
            }
        }
    }
    else {
        for (draw_x, draw_y, pixel) in resize_image.enumerate_pixels() {
            if pixel != &Rgba([0,0,0,0]) {
                let foreground = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
                let distance_to_edge = distance_to_nearest_edge(x, y, width, height);
                
                let color: super::utility::Rgba;
                let edge_width = 50;
                if distance_to_edge < edge_width {
                    let enhance_brightness = crate::utility::Rgba::enhance_brightness(foreground, distance_to_edge, edge_width);
                    color = super::utility::Rgba::new(enhance_brightness.0[0], enhance_brightness.0[1], enhance_brightness.0[2], enhance_brightness.0[3]);
                }
                else {
                    color = foreground;
                }
                super::rect::draw_rect(
                    img,
                    draw_x + x,
                    draw_y + y,
                    1,
                    1,
                    color
                );
            }
        }
    }
}

pub fn check_is_url_image(path: &str) -> bool{
    let re = Regex::new(r"http(s)?://([/|.|\w|\s|-])*\.(?:jpg|gif|png|bmp|webp)").unwrap();
    re.is_match(path)
}

fn distance_to_nearest_edge(x: u32, y: u32, width: u32, height: u32) -> u32 {
    let left = x;
    let right = width - x - 1;
    let top = y;
    let bottom = height - y - 1;

    *[left, right, top, bottom].iter().min().unwrap()
}