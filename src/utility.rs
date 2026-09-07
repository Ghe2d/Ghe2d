#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

impl Rgba {
    pub const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };
    pub const TRANSPARENT: Rgba = Rgba { r: 0, g: 0, b: 0, a: 0 };

    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Rgba { r, g, b, a }
    }

    /// Opaque colour from RGB (`a = 255`).
    pub fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba { r, g, b, a: 255 }
    }

    pub fn to_rgba_image(self) -> image::Rgba<u8> {
        image::Rgba([self.r, self.g, self.b, self.a])
    }

    pub fn blend_with_alpha(foreground: &Self, background: Self, alpha: u8) -> image::Rgba<u8> {
        let fg = foreground;
        let bg = background;
        let alpha = alpha as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;

        let mix = |f: u8, b: u8| (f as f32 * alpha + b as f32 * inv_alpha).round().clamp(0.0, 255.0) as u8;
        image::Rgba([mix(fg.r, bg.r), mix(fg.g, bg.g), mix(fg.b, bg.b), mix(fg.a, bg.a)])
    }

    pub fn blend(foreground: Self, background: Self) -> image::Rgba<u8> {
        let alpha_fg = foreground.a as f32 / 255.0;
        let alpha_bg = background.a as f32 / 255.0;
        let combined_alpha = alpha_fg + alpha_bg * (1.0 - alpha_fg);
        if combined_alpha <= f32::EPSILON {
            return image::Rgba([0, 0, 0, 0]);
        }

        let mix = |f: u8, b: u8| {
            let v = (f as f32 * alpha_fg + b as f32 * alpha_bg * (1.0 - alpha_fg)) / combined_alpha;
            v.round().clamp(0.0, 255.0) as u8
        };
        image::Rgba([
            mix(foreground.r, background.r),
            mix(foreground.g, background.g),
            mix(foreground.b, background.b),
            (combined_alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        ])
    }

    pub fn enhance_brightness(pixel: Self, distance: u32, edge_width: u32) -> image::Rgba<u8> {
        let factor = 1.0 + (edge_width - distance) as f32 / edge_width as f32;
    
        let r = (pixel.r as f32 * factor).min(255.0) as u8;
        let g = (pixel.g as f32 * factor).min(255.0) as u8;
        let b = (pixel.b as f32 * factor).min(255.0) as u8;
    
        image::Rgba([r, g, b, pixel.a])
    }
}

