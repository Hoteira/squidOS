use crate::types::Color;

pub mod primitives;

pub fn draw_pixel(buffer: &mut [u32], width: usize, x: usize, y: usize, mut color: Color) {
    if width != 0 && color.a != 0 {

        if color.a < 255 {
            let idx = y * width + x;
            if idx < buffer.len() {
                let previous_color = Color::from_u32(buffer[idx]);
                let alpha = color.a as f32 / 255.0;

                color.r = (alpha * color.r as f32 + (1.0 - alpha) * previous_color.r as f32) as u8;
                color.g = (alpha * color.g as f32 + (1.0 - alpha) * previous_color.g as f32) as u8;
                color.b = (alpha * color.b as f32 + (1.0 - alpha) * previous_color.b as f32) as u8;
                buffer[idx] = color.to_u32();
            }
        } else {
            let idx = y * width + x;
            if idx < buffer.len() {
                buffer[idx] = color.to_u32();
            }
        }
    }
}

pub fn draw_u32(buffer: &mut [u32], width: usize, x: usize, y: usize, color: u32) {
    let idx = y * width + x;
    if idx < buffer.len() {
        buffer[idx] = color;
    }
}
