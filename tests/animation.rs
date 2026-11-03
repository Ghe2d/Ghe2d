use std::io::Cursor;

use ghe2d::image::codecs::gif::{GifEncoder, Repeat};
use ghe2d::image::{Delay, Frame, ImageFormat, Rgba, RgbaImage};
use ghe2d::{decode_animation, resize_image, crop_image, AnimError, DecodeLimits};

fn limits() -> DecodeLimits {
    DecodeLimits { max_frames: 50, max_total_pixels: 1_000_000, max_duration_ms: 60_000 }
}

fn frame_img(n: u8, w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| Rgba([(x * 10) as u8, (y * 10) as u8, n.wrapping_mul(40), 255]))
}

fn make_gif(frames: u8, w: u32, h: u32, delay_ms: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            let f = Frame::from_parts(frame_img(i, w, h), 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
            enc.encode_frame(f).unwrap();
        }
    }
    out
}

#[test]
fn gif_decodes_frames_and_delays() {
    let anim = decode_animation(&make_gif(4, 16, 12, 70), &limits()).unwrap();
    assert_eq!(anim.frames.len(), 4);
    assert!(anim.is_animated());
    assert_eq!((anim.width(), anim.height()), (16, 12));
    assert!(anim.frames.iter().all(|f| f.delay_ms == 70));
    assert_eq!(anim.total_duration_ms(), 280);
}

#[test]
fn still_png_is_one_frame() {
    let mut png = Vec::new();
    frame_img(1, 8, 8).write_to(&mut Cursor::new(&mut png), ImageFormat::Png).unwrap();
    let anim = decode_animation(&png, &limits()).unwrap();
    assert_eq!(anim.frames.len(), 1);
    assert!(!anim.is_animated());
}

#[test]
fn still_webp_is_one_frame() {
    let webp = ghe2d::export::encode(&frame_img(2, 8, 6), &ghe2d::ExportOptions::webp(80.0)).unwrap();
    let anim = decode_animation(&webp, &limits()).unwrap();
    assert_eq!(anim.frames.len(), 1);
    assert!(!anim.is_animated());
    assert_eq!((anim.width(), anim.height()), (8, 6));
    let mut l = limits();
    l.max_total_pixels = 10;
    assert!(matches!(decode_animation(&webp, &l), Err(AnimError::TooManyPixels { .. })));
}

#[test]
fn bmp_and_single_frame_gif_are_one_frame() {
    let mut bmp = Vec::new();
    frame_img(3, 7, 5).write_to(&mut Cursor::new(&mut bmp), ImageFormat::Bmp).unwrap();
    let anim = decode_animation(&bmp, &limits()).unwrap();
    assert_eq!(anim.frames.len(), 1);
    assert_eq!((anim.width(), anim.height()), (7, 5));
    let mut l = limits();
    l.max_total_pixels = 10;
    assert!(matches!(decode_animation(&bmp, &l), Err(AnimError::TooManyPixels { .. })));

    let anim = decode_animation(&make_gif(1, 6, 6, 50), &limits()).unwrap();
    assert_eq!(anim.frames.len(), 1);
    assert!(!anim.is_animated());
}

#[test]
fn limits_are_enforced() {
    let gif = make_gif(6, 16, 12, 70);
    let mut l = limits();
    l.max_frames = 3;
    assert_eq!(decode_animation(&gif, &l).unwrap_err(), AnimError::TooManyFrames { limit: 3 });
    let mut l = limits();
    l.max_total_pixels = 16 * 12 * 2;
    assert!(matches!(decode_animation(&gif, &l), Err(AnimError::TooManyPixels { .. })));
    // canvas alone over the limit fails before any frame is decoded
    l.max_total_pixels = 10;
    assert!(matches!(decode_animation(&gif, &l), Err(AnimError::TooManyPixels { .. })));
    let mut l = limits();
    l.max_duration_ms = 200;
    assert_eq!(decode_animation(&gif, &l).unwrap_err(), AnimError::TooLong { limit_ms: 200 });
    assert_eq!(decode_animation(&[], &limits()).unwrap_err(), AnimError::Decode);
    assert_eq!(decode_animation(b"not an image at all", &limits()).unwrap_err(), AnimError::UnsupportedFormat);
}

#[test]
fn crop_and_resize_every_frame() {
    let anim = decode_animation(&make_gif(3, 20, 10, 50), &limits()).unwrap();
    let c = anim.crop(2, 1, 8, 6).unwrap();
    assert_eq!(c.frames.len(), 3);
    assert!(c.frames.iter().all(|f| f.image.dimensions() == (8, 6)));
    // crop keeps the right pixels
    assert_eq!(c.frames[0].image.get_pixel(0, 0), anim.frames[0].image.get_pixel(2, 1));
    let r = c.resize(16, 12).unwrap();
    assert!(r.frames.iter().all(|f| f.image.dimensions() == (16, 12)));
    assert!(r.frames.iter().all(|f| f.delay_ms == 50));
    assert_eq!(anim.crop(15, 0, 10, 5).unwrap_err(), AnimError::InvalidCrop);
    assert_eq!(anim.crop(0, 0, 0, 5).unwrap_err(), AnimError::InvalidCrop);
    assert_eq!(anim.crop(u32::MAX, 0, 5, 5).unwrap_err(), AnimError::InvalidCrop);
    assert_eq!(anim.resize(0, 5).unwrap_err(), AnimError::InvalidSize);
}

#[test]
fn still_crop_resize() {
    let img = frame_img(2, 30, 20);
    let c = crop_image(&img, 5, 5, 10, 10).unwrap();
    assert_eq!(c.dimensions(), (10, 10));
    assert_eq!(resize_image(&c, 4, 4).unwrap().dimensions(), (4, 4));
}

#[test]
fn animated_webp_round_trip() {
    let anim = decode_animation(&make_gif(4, 16, 16, 80), &limits()).unwrap();
    let webp = anim.encode_webp(80.0).unwrap();
    assert_eq!(&webp[0..4], b"RIFF");
    assert_eq!(&webp[8..12], b"WEBP");
    assert!(webp.windows(4).any(|w| w == b"ANIM"));
    let back = decode_animation(&webp, &limits()).unwrap();
    assert_eq!(back.frames.len(), 4);
    assert_eq!((back.width(), back.height()), (16, 16));
    assert!(back.frames.iter().all(|f| f.delay_ms == 80));
    // crop + resize + re-encode also works
    let small = back.crop(0, 0, 8, 8).unwrap().resize(12, 12).unwrap().encode_webp(70.0).unwrap();
    assert_eq!(decode_animation(&small, &limits()).unwrap().frames.len(), 4);
}

#[test]
fn encode_rejects_bad_frames() {
    let mut anim = decode_animation(&make_gif(2, 8, 8, 50), &limits()).unwrap();
    anim.frames[1].image = frame_img(0, 9, 8);
    assert_eq!(anim.encode_webp(80.0).unwrap_err(), AnimError::BadFrames);
    anim.frames.clear();
    assert_eq!(anim.encode_webp(80.0).unwrap_err(), AnimError::BadFrames);
}

const BG: Rgba<u8> = Rgba([20, 24, 40, 255]);

/// A gold disc circling the centre of a solid dark canvas (like a loading spinner).
fn orbit_frames(n: u32, w: u32, h: u32) -> Vec<RgbaImage> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let (cx, cy) = (w as f32 / 2.0 + 30.0 * a.cos(), h as f32 / 2.0 + 30.0 * a.sin());
            RgbaImage::from_fn(w, h, |x, y| {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                if d < 9.0 {
                    Rgba([240, 200, 60, 255])
                } else {
                    BG
                }
            })
        })
        .collect()
}

/// (max, mean) absolute RGBA difference over the whole frame.
fn diff_all(a: &RgbaImage, b: &RgbaImage) -> (u8, f64) {
    let mut max = 0u8;
    let mut sum = 0u64;
    for (p, q) in a.as_raw().iter().zip(b.as_raw().iter()) {
        let d = p.abs_diff(*q);
        max = max.max(d);
        sum += d as u64;
    }
    (max, sum as f64 / a.as_raw().len() as f64)
}

/// Max absolute difference over background pixels that are at least 6 px away from the shape
/// in the original frame. A lossy edge error stays near the shape; a ghost of an earlier
/// frame shows up here.
fn bg_far_max(orig: &RgbaImage, dec: &RgbaImage) -> u8 {
    let (w, h) = orig.dimensions();
    let mut max = 0u8;
    for y in 0..h {
        for x in 0..w {
            let near = (y.saturating_sub(6)..=(y + 6).min(h - 1)).any(|yy| {
                (x.saturating_sub(6)..=(x + 6).min(w - 1)).any(|xx| *orig.get_pixel(xx, yy) != BG)
            });
            if !near {
                let (p, q) = (orig.get_pixel(x, y), dec.get_pixel(x, y));
                max = max.max(p.0.iter().zip(q.0.iter()).map(|(a, b)| a.abs_diff(*b)).max().unwrap());
            }
        }
    }
    max
}

#[test]
fn animated_webp_has_no_ghosts_at_q85() {
    use ghe2d::{AnimFrame, Animation};
    let src = orbit_frames(12, 120, 90);
    let anim = Animation {
        frames: src.iter().map(|i| AnimFrame { image: i.clone(), delay_ms: 80 }).collect(),
        loop_count: 0,
    };
    let bytes = anim.encode_webp(85.0).unwrap();
    let dec = decode_animation(&bytes, &limits()).unwrap();
    assert_eq!(dec.frames.len(), src.len());
    for (i, (o, d)) in src.iter().zip(dec.frames.iter()).enumerate() {
        let (max, mean) = diff_all(o, &d.image);
        let far = bg_far_max(o, &d.image);
        eprintln!("frame {i}: max {max} mean {mean:.3} bg_far_max {far} (file {} bytes)", bytes.len());
        assert!(far <= 3, "frame {i}: ghost, background far from the shape is off by {far}");
        assert!(mean < 3.0, "frame {i}: mean diff {mean}");
    }
}

/// Same as `bg_far_max` but the background is whatever the frame's own corner pixel is,
/// and "near the shape" means different from that colour.
fn bg_far_stats(orig: &RgbaImage, dec: &RgbaImage) -> (u8, f64) {
    let (w, h) = orig.dimensions();
    let bg = *orig.get_pixel(0, 0);
    let (mut max, mut sum, mut n) = (0u8, 0u64, 0u64);
    for y in 0..h {
        for x in 0..w {
            let near = (y.saturating_sub(6)..=(y + 6).min(h - 1)).any(|yy| {
                (x.saturating_sub(6)..=(x + 6).min(w - 1)).any(|xx| *orig.get_pixel(xx, yy) != bg)
            });
            if !near {
                let d = orig.get_pixel(x, y).0.iter().zip(dec.get_pixel(x, y).0.iter()).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
                max = max.max(d);
                sum += d as u64;
                n += 1;
            }
        }
    }
    (max, sum as f64 / n.max(1) as f64)
}

/// 240x240, 12 frames: a dot circling a centre with a translucent hub, GIF -> decode -> WebP q85.
#[test]
fn gif_to_webp_240_has_no_ghosts() {
    use ghe2d::{CircleOptions, Ghe2d, RectOptions};
    let bg = ghe2d::utility::Rgba::rgb(18, 20, 28);
    let mut gif = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut gif);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..12 {
            let mut c = Ghe2d::new(240, 240);
            c.draw_rect(RectOptions::solid(0.0, 0.0, 240.0, 240.0, bg));
            let a = i as f32 / 12.0 * std::f32::consts::TAU;
            c.draw_circle(CircleOptions::solid(120.0 + 70.0 * a.cos(), 120.0 + 70.0 * a.sin(), 22.0, ghe2d::utility::Rgba::new(220, 170, 70, 255)));
            c.draw_circle(CircleOptions::solid(120.0, 120.0, 12.0, ghe2d::utility::Rgba::new(120, 200, 255, 220)));
            enc.encode_frame(Frame::from_parts(c.image.clone(), 0, 0, Delay::from_numer_denom_ms(80, 1))).unwrap();
        }
    }
    let lim = DecodeLimits { max_frames: 50, max_total_pixels: 5_000_000, max_duration_ms: 60_000 };
    let from_gif = decode_animation(&gif, &lim).unwrap();
    let webp = from_gif.encode_webp(85.0).unwrap();
    let back = decode_animation(&webp, &lim).unwrap();
    assert_eq!(back.frames.len(), 12);
    eprintln!("webp size {}", webp.len());
    for (i, (o, d)) in from_gif.frames.iter().zip(back.frames.iter()).enumerate() {
        let (max, mean) = bg_far_stats(&o.image, &d.image);
        eprintln!("frame {i}: bg_far max {max} mean {mean:.4}");
        assert!(max <= 2, "frame {i}: ghost, background off by {max}");
    }
}
