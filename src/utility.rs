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

    pub fn blend(foreground: &Self, background: Self, alpha: u8) -> image::Rgba<u8> {
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
}

