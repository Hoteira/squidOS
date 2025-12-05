use crate::gui::Color;

pub mod square;
pub mod line;
pub mod circle;
pub mod text;
pub(crate) mod image;

pub fn draw_pixel(buffer: &mut [u32], width: usize, x: usize, y: usize, mut color: Color) {
    if width != 0 && color.a != 0 {

        if color.a < 255 {
            let previous_color = Color::from_u32(buffer[y * width + x]);
            let alpha = color.a as f32 / 255.0;

            color.r = (alpha * color.r as f32 + (1.0 - alpha) * previous_color.r as f32) as u8 as usize;
            color.g = (alpha * color.g as f32 + (1.0 - alpha) * previous_color.g as f32) as u8 as usize;
            color.b = (alpha * color.b as f32 + (1.0 - alpha) * previous_color.b as f32) as u8 as usize;
        }

        buffer[y * width + x] = color.to_u32();
    }
}

pub fn draw_u32(buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
    buffer[y * width + x] = color;
}