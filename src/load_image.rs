use num_integer::Roots;

pub async fn add_image_blend_mut(img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), String>  {

    let load_image = load_image(path, width, height, is_circle).await;
    if load_image.is_err() {
        return Err(load_image.err().unwrap());
    }

    for (draw_x, draw_y, pixel,) in load_image.unwrap().enumerate_pixels() {
        if pixel != &image::Rgba([0,0,0,0]) {
            let b_pixel = img.get_pixel(x, y);
            let foreground = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
            let background = crate::utility::Rgba::new(b_pixel.0[0], b_pixel.0[1], b_pixel.0[2], b_pixel.0[3]);
            let blend = crate::utility::Rgba::blend(foreground, background);
            super::rect::draw_rect(img,
                draw_x + x,
                draw_y + y,
                1,
                1,
                super::utility::Rgba::new(blend.0[0], blend.0[1], blend.0[2], blend.0[3])
            );
        }
    }
    Ok(())
}

pub async fn add_image_normal_mut( img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), String>  {

    let load_image = load_image(path, width, height, is_circle).await;
    if load_image.is_err() {
        return Err(load_image.err().unwrap());
    }

    image::imageops::replace(img, &load_image.unwrap(), x as i64, y as i64);
    Ok(())
}

pub async fn add_image_overlay_mut( img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), String> {
    let load_image = load_image(path, width, height, is_circle).await;
    if load_image.is_err() {
        return Err(load_image.err().unwrap());
    }

    image::imageops::overlay(img, &load_image.unwrap(), x as i64, y as i64);
    Ok(())
}

pub async fn load_image(path: &str, w: Option<u32>, h: Option<u32>, is_circle: bool) -> Result<image::RgbaImage, String> {
    
    let load_image: image::DynamicImage;
    
    if check_is_url_image(path) {
        let resp = reqwest::get(path).await;
        if resp.is_err() {
            return Err("Failed to load image from URL".to_string());
        }
        let bytes = resp.unwrap().bytes().await;
        if bytes.is_err() {
            return Err("Failed to load image from URL".to_string());
        }
        let _load_image = image::load_from_memory(&bytes.unwrap());
        if _load_image.is_err() {
            return Err("Failed to decode image".to_string());
        }
        load_image = _load_image.unwrap();
    }
    else {
        let _load_image = image::open(path);
        if _load_image.is_err() {
            return Err("Failed to decode image".to_string());
        }
        load_image = _load_image.unwrap();
    }

    let r_img: image::RgbaImage;
    let width = match w {
        Some(w) => w,
        None => load_image.width(),
    };
    let height = match h {
        Some(h) => h,
        None => load_image.height(),
    };

    if width == load_image.width() && height == load_image.height() {
        r_img = load_image.to_rgba8();
    } else {
        r_img = image::imageops::resize(&load_image, width as u32, height as u32, image::imageops::FilterType::Nearest);
    }

    if is_circle {
        let cx = width / 2;
        let cy = height / 2;
        let radius = width.min(height) / 2;
        let mut circle_img: image::ImageBuffer<image::Rgba<u8>, _> = image::ImageBuffer::new(width as u32, height as u32);
        for x in 0..width as u32 {
            for y in 0..height as u32 {
                let dx = x - cx;
                let dy = y - cy;
                let dist = (dx * dx + dy * dy).sqrt() as u32;

                let pixel = r_img.get_pixel(x, y);

                if dist < radius as u32 {
                    circle_img.put_pixel(x, y, *pixel);
                }
            }
        }
        Ok(circle_img)
    } else {
        Ok(r_img)
    }
}

pub fn check_is_url_image(path: &str) -> bool{
    // let re = regex::Regex::new(r"http(s)?://([/|.|\w|\s|%|-])*\.(?:jpg|gif|png|bmp|webp)").unwrap();
    let re = regex::Regex::new(r"http(s)?://([/|.|\w|\s|%|-])*").unwrap();
    re.is_match(path)
}
