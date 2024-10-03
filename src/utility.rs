#[derive(Clone)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

impl Rgba {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    pub fn to_rgba_image(self) -> image::Rgba<u8> {
        image::Rgba([self.r, self.g, self.b, self.a])
    }

    pub fn blend_with_alpha(foreground: &Self, background: Self, alpha: u8) -> image::Rgba<u8> {
        let fg = foreground;
        let bg = background;
        let alpha = alpha as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
    
        let r = (fg.r as f32 * alpha + bg.r as f32 * inv_alpha) as u8;
        let g = (fg.g as f32 * alpha + bg.g as f32 * inv_alpha) as u8;
        let b = (fg.b as f32 * alpha + bg.b as f32 * inv_alpha) as u8;
        let a = (fg.a as f32 * alpha + bg.a as f32 * inv_alpha) as u8;
    
        image::Rgba([r, g, b, a])
    }

    pub fn blend(foreground: Self, background: Self) -> image::Rgba<u8> {
        let alpha_fg = foreground.a as f32 / 255.0;
        let alpha_bg = background.a as f32 / 255.0;
        let combined_alpha = alpha_fg + alpha_bg * (1.0 - alpha_fg);
    
        let r = (foreground.r as f32 * alpha_fg + background.r as f32 * alpha_bg * (1.0 - alpha_fg)) / combined_alpha;
        let g = (foreground.g as f32 * alpha_fg + background.g as f32 * alpha_bg * (1.0 - alpha_fg)) / combined_alpha;
        let b = (foreground.b as f32 * alpha_fg + background.b as f32 * alpha_bg * (1.0 - alpha_fg)) / combined_alpha;
    
        image::Rgba([r as u8, g as u8, b as u8, (combined_alpha * 255.0) as u8])
    }

    pub fn enhance_brightness(pixel: Self, distance: u32, edge_width: u32) -> image::Rgba<u8> {
        let factor = 1.0 + (edge_width - distance) as f32 / edge_width as f32;
    
        let r = (pixel.r as f32 * factor).min(255.0) as u8;
        let g = (pixel.g as f32 * factor).min(255.0) as u8;
        let b = (pixel.b as f32 * factor).min(255.0) as u8;
    
        image::Rgba([r, g, b, pixel.a])
    }
}

