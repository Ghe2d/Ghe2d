use regex::Regex;
use rusttype::{Scale, point, PositionedGlyph};

use ar_reshaper::{config::LigaturesFlags, ReshaperConfig};

use crate::font::LoadFont;

pub fn draw_text(img: &mut image::RgbaImage, load_font: LoadFont, text: String, x: f32, y: f32, size: f32, color: crate::utility::Rgba) {
    let scale = Scale::uniform(size);
    let offset = point(x, load_font.font.v_metrics(scale).ascent + y);
    let a = ar_reshaper::ArabicReshaper::new(ReshaperConfig::new(ar_reshaper::Language::ArabicV2, LigaturesFlags::none()));
    let _text = fix_arabic_text(&a.reshape(text.clone()));
    let lines:Vec<&str> = _text.split("\n").collect();
    let mut large = 0;
    let sy = y;

    for (i, line) in lines.iter().enumerate() {
        let glyphs: Vec<PositionedGlyph> = load_font.font.layout(line, scale, offset).collect();
        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let av = (color.a as f32 * v) as u8;
                    if av != 0 {
                        let x = x as i32 + bounding_box.min.x;
                        let y = y as i32 + bounding_box.min.y;
                        if i == 0 && large < y {
                            large = y;
                        }
                        super::rect::draw_rect(
                            img,
                            (x + i as i32 * 1) as u32,
                            (y + i as i32 * (large as i32 - sy as i32))  as u32, 
                            1,
                            1,
                            super::utility::Rgba::new(color.r, color.g, color.b, av)
                        );
                    }
                });
            }
        }
    }
}



fn fix_arabic_text(text: &str) -> String {
    let mut text_to_vet_u16: Vec<u16> = str::encode_utf16(text).collect();

    let mut is_right = false;
    if is_arabic(text_to_vet_u16[0]) {
        is_right = true;
    }

    if is_right {
        text_to_vet_u16.reverse();
    }

    let mut index = 0;
    let mut end = 0;
    let mut sub_vec_u16: Vec<u16> = vec![];
    
    for i in 0..text_to_vet_u16.len() {
        let char_u16 = text_to_vet_u16[i];
        if (is_arabic(char_u16) && !is_right) || (!is_arabic(char_u16) && is_right) || (char_u16 == 32 && (index != 0 && end != 0)) {
            if index == 0 {
                index = i + 1;
            }
            else {
                end += 1;
            }
        }
        else if index != 0 && end != 0 {
            let mut is_space = false;
            for j in (0..end + 1).rev() {
                if text_to_vet_u16[index - 1 + j] == 32 && j == (0..end).len() && !is_right {
                    is_space = true;
                }
                else {
                    sub_vec_u16.push(text_to_vet_u16[index - 1 + j]);
                }
            }
            if is_space {
                sub_vec_u16.push(32);
            }
            sub_vec_u16.push(char_u16);
            index = 0;
            end = 0;
        }
        else {
            if index != 0 {
                sub_vec_u16.push(text_to_vet_u16[index - 1]);
            }
            index = 0;
            end = 0;
            sub_vec_u16.push(char_u16);
        }
    }

    if index != 0 && end != 0 {
        for j in (0..end + 1).rev() {
            sub_vec_u16.push(text_to_vet_u16[index - 1 + j]);
        }
    }
    else if index != 0 {
        sub_vec_u16.push(text_to_vet_u16[index - 1]);
    }

    let mut sub_fix_vec_u16: Vec<u16> = vec![];

    for i in 0..sub_vec_u16.len() {
        let mut is_push = true;
        let mut char_u16 = sub_vec_u16[i];
        if char_u16 == 65152 {
            char_u16 = 1569;
        }
        else if char_u16 == 65248 || char_u16 == 65247 {
            if sub_vec_u16[i - 1] == 65154 {
                is_push = false;
            }
        }
        else if char_u16 == 65154 {
            if text_to_vet_u16[i + 1] == 65248 {
                char_u16 = 65270;
            }
            else {
                char_u16 = 65269;
            }
        }
        else if char_u16 == 65185 {
            char_u16 = 1581;
        }
        if is_push {
            sub_fix_vec_u16.push(char_u16);
        }
    }
    
    String::from_utf16(&sub_fix_vec_u16).unwrap()
}


fn is_arabic(c: u16) -> bool {
    let ar = Regex::new(r"\p{Arabic}").unwrap();
    ar.is_match(&std::char::from_u32(c as u32).unwrap().to_string())
}
