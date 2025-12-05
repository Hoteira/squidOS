use libk::{print, println};
use crate::gui::{Align, Text, Color};
use crate::font_manager::{FontManager, render_text_to_buffer_optimized, measure_text_fast};
use crate::draw_2d::draw_pixel;

/// Draw text to a framebuffer - optimized version
pub fn draw_text(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    text: Text,
    x: usize,
    y: usize,
    max_width: Option<usize>,
    max_height: Option<usize>,
) {
    // Early exit for empty text
    if text.text.is_empty() {
        return;
    }

    // Use optimized font loading
    if FontManager::load_font(&text.font).is_err() {
        println!("404 FONT_NOT_FOUND {}", text.font);
        return;
    }

    let (text_width, text_height) = measure_text_fast(&text.text, &text.font, text.size, max_width);

    let start_x = match text.align {
        Align::Left => x,
        Align::Center => {
            if let Some(max_w) = max_width {
                x + (max_w.saturating_sub(text_width)) / 2
            } else {
                x
            }
        }
        Align::Right => {
            if let Some(max_w) = max_width {
                x + max_w.saturating_sub(text_width)
            } else {
                x
            }
        }
    };

    // Use optimized rendering
    render_text_to_buffer_optimized(
        buffer,
        buffer_width,
        &text.text,
        &text.font,
        text.size,
        text.color,
        start_x,
        y,
        max_width,
    );
}

/// Draw text within a specific rectangular region with clipping - optimized
pub fn draw_text_in_rect(
    buffer: &mut [u32],
    buffer_width: usize,
    buffer_height: usize,
    text: Text,
    rect_x: usize,
    rect_y: usize,
    rect_width: usize,
    rect_height: usize,
) {
    // Early exit for empty text or zero-sized rect
    if text.text.is_empty() || rect_width == 0 || rect_height == 0 {
        return;
    }

    let max_width = Some(rect_width);
    let max_height = Some(rect_height);

    let (_, text_height) = measure_text_fast(&text.text, &text.font, text.size, max_width);
    let start_y = rect_y + (rect_height.saturating_sub(text_height)) / 2;

    draw_text(
        buffer,
        buffer_width,
        buffer_height,
        text,
        rect_x,
        start_y,
        max_width,
        max_height,
    );
}

/// Optimized single character drawing with bounds checking
pub fn draw_char_optimized(
    buffer: &mut [u32],
    buffer_width: usize,
    character: char,
    font_path: &str,
    font_size: usize,
    color: Color,
    x: usize,
    y: usize,
) -> usize {
    // Use cached glyph lookup
    if let Some(glyph) = FontManager::get_char(font_path, character, font_size) {
        let glyph_y = y.saturating_sub(glyph.baseline_offset);

        // Early bounds check
        if glyph_y >= buffer.len() / buffer_width {
            return glyph.advance_width;
        }

        for (row_idx, row) in glyph.bitmap.iter().enumerate() {
            let pixel_y = glyph_y + row_idx;

            if pixel_y >= buffer.len() / buffer_width {
                break;
            }

            for (col_idx, &alpha) in row.iter().enumerate() {
                if alpha == 0 {
                    continue;
                }

                let pixel_x = x + col_idx;
                if pixel_x >= buffer_width {
                    break;
                }

                let text_color = Color::rgba(
                    color.r,
                    color.g,
                    color.b,
                    (color.a * alpha as usize) / 255
                );

                draw_pixel(buffer, buffer_width, pixel_x, pixel_y, text_color);
            }
        }

        glyph.advance_width
    } else {
        font_size / 2
    }
}

/// Clear a rectangular region - useful for text editing
pub fn clear_rect(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    background_color: Color,
) {
    let bg_color = background_color.to_u32();

    for row in 0..height {
        let pixel_y = y + row;
        if pixel_y >= buffer.len() / buffer_width {
            break;
        }

        let start_idx = pixel_y * buffer_width + x;
        let end_idx = (start_idx + width).min(buffer.len()).min((pixel_y + 1) * buffer_width);

        if start_idx < buffer.len() {
            for idx in start_idx..end_idx {
                buffer[idx] = bg_color;
            }
        }
    }
}

/// Incremental text rendering - only render new characters
pub fn append_text_incremental(
    buffer: &mut [u32],
    buffer_width: usize,
    new_text: &str,
    font_path: &str,
    font_size: usize,
    color: Color,
    start_x: usize,
    start_y: usize,
    current_cursor_x: usize,
    max_width: Option<usize>,
) -> usize {
    let mut cursor_x = current_cursor_x;
    let mut cursor_y = start_y;
    let line_height = font_size + (font_size / 4);

    for character in new_text.chars() {
        if character == '\n' {
            cursor_x = start_x;
            cursor_y += line_height;
            continue;
        }

        if character == '\r' {
            continue;
        }

        // Check for line wrapping
        if let Some(max_w) = max_width {
            if let Some(glyph) = FontManager::get_char(font_path, character, font_size) {
                if cursor_x + glyph.width > start_x + max_w && cursor_x > start_x {
                    cursor_x = start_x;
                    cursor_y += line_height;
                }
            }
        }

        let text_baseline_y = cursor_y + (font_size as f32 * 0.8) as usize;
        cursor_x += draw_char_optimized(
            buffer,
            buffer_width,
            character,
            font_path,
            font_size,
            color,
            cursor_x,
            text_baseline_y
        );
    }

    cursor_x
}