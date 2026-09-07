use std::sync::LazyLock;
use regex::Regex;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"http(s)?://([/|.|\w|\s|%|-])*").expect("valid static regex"));

pub async fn add_image_blend_mut(img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage>  {
    let overlay = load_image(path, width, height, is_circle).await?;

    for (draw_x, draw_y, pixel) in overlay.enumerate_pixels() {
        if pixel == &image::Rgba([0, 0, 0, 0]) {
            continue;
        }
        // Destination pixel for this source pixel; skip anything off-canvas.
        let (Some(dst_x), Some(dst_y)) = (draw_x.checked_add(x), draw_y.checked_add(y)) else {
            continue;
        };
        let Some(b_pixel) = img.get_pixel_checked(dst_x, dst_y) else {
            continue;
        };
        let foreground = crate::utility::Rgba::new(pixel.0[0], pixel.0[1], pixel.0[2], pixel.0[3]);
        let background = crate::utility::Rgba::new(b_pixel.0[0], b_pixel.0[1], b_pixel.0[2], b_pixel.0[3]);
        let blend = crate::utility::Rgba::blend(foreground, background);
        super::rect::draw_rect(
            img,
            dst_x,
            dst_y,
            1,
            1,
            super::utility::Rgba::new(blend.0[0], blend.0[1], blend.0[2], blend.0[3]),
        );
    }
    Ok(())
}

pub async fn add_image_normal_mut( img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage>  {
    let overlay = load_image(path, width, height, is_circle).await?;
    image::imageops::replace(img, &overlay, x as i64, y as i64);
    Ok(())
}

pub async fn add_image_overlay_mut( img: &mut image::RgbaImage, path: &str, x: u32, y: u32, width: Option<u32>, height: Option<u32>, is_circle: bool) -> Result<(), crate::ErrorMessage> {
    let overlay = load_image(path, width, height, is_circle).await?;
    image::imageops::overlay(img, &overlay, x as i64, y as i64);
    Ok(())
}

pub async fn load_image(path: &str, w: Option<u32>, h: Option<u32>, is_circle: bool) -> Result<image::RgbaImage, crate::ErrorMessage> {

    let load_image: image::DynamicImage;

    if check_is_url_image(path) {
        let resp = reqwest::get(path)
            .await
            .map_err(|_| crate::ErrorMessage::FailedLoadImage)?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|_| crate::ErrorMessage::FailedLoadImage)?;
        load_image = image::load_from_memory(&bytes)
            .map_err(|_| crate::ErrorMessage::FailedDecodeImage)?;
    }
    else {
        load_image = image::open(path).map_err(|_| crate::ErrorMessage::FailedDecodeImage)?;
    }

    let width = w.unwrap_or_else(|| load_image.width());
    let height = h.unwrap_or_else(|| load_image.height());

    let r_img: image::RgbaImage = if width == load_image.width() && height == load_image.height() {
        load_image.to_rgba8()
    } else {
        image::imageops::resize(&load_image, width, height, image::imageops::FilterType::Nearest)
    };

    if is_circle {
        if width == 0 || height == 0 {
            return Ok(r_img);
        }

        let cx = width as i64 / 2;
        let cy = height as i64 / 2;
        let radius = (width.min(height) / 2) as i64;
        let r2 = radius * radius;
        let mut circle_img: image::RgbaImage = image::ImageBuffer::new(width, height);

        for x in 0..width {
            for y in 0..height {
                let dx = x as i64 - cx;
                let dy = y as i64 - cy;

                if dx * dx + dy * dy < r2 {
                    circle_img.put_pixel(x, y, *r_img.get_pixel(x, y));
                }
            }
        }
        Ok(circle_img)
    } else {
        Ok(r_img)
    }
}

pub fn check_is_url_image(path: &str) -> bool{
    URL_RE.is_match(path)
}
