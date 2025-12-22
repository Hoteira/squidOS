use crate::graphics::{draw_pixel, draw_u32};
use crate::types::{Color, Size};
use crate::math::{sqrt_f64, ceil_f32};
use titanf::TrueTypeFont;
use alloc::vec::Vec;

pub fn draw_line(buffer: &mut [u32], width0: usize, x0: usize, y0: usize, x1: usize, y1: usize, color: Color, width: usize) {
    let dx = (x1 as isize - x0 as isize).abs();
    let dy = -(y1 as isize - y0 as isize).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0 as isize;
    let mut y = y0 as isize;
    let half_thickness = (width as isize) / 2;

    loop {
        for tx in -half_thickness..=half_thickness {
            for ty in -half_thickness..=half_thickness {
                let nx = x + tx;
                let ny = y + ty;
                if nx >= 0 && nx < width0 as isize && ny >= 0 && ny < core::cmp::max(y0, y1) as isize {
                    let idx = (ny as usize) * width0 + (nx as usize);
                    if idx < buffer.len() {
                        draw_pixel(buffer, width0, nx as usize, ny as usize, color )
                    }
                }
            }
        }

        if x == x1 as isize && y == y1 as isize {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn draw_square(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    color: Color,
    border_size: usize,
    border_color: Color,
) {
    if color.a == 0 && (border_size == 0 || border_color.a == 0) {
        return;
    }
    draw_square_alpha(buffer, buffer_width, x, y, width, height, rounding, color, border_size, border_color);
}

pub fn draw_square_alpha(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    color: Color,
    border_size: usize,
    border_color: Color,
) {
    if buffer_width == 0 || width == 0 || height == 0 {
        return;
    }

    let r_val = match rounding {
        Size::Absolute(v) => v as f32,
        Size::Relative(pct) => (width.min(height) as f32 * pct as f32) / 100.0,
        _ => 0.0,
    };
    let r = r_val.min(width as f32 / 2.0).min(height as f32 / 2.0);
    
    let end_y = (y + height).min(buffer.len() / buffer_width);
    let end_x = (x + width).min(buffer_width);

    let is_bg_opaque = color.a == 255;
    let bg_u32 = color.to_u32();
    
    let is_border_opaque = border_color.a == 255;
    let border_u32 = border_color.to_u32();
    let has_border = border_size > 0 && border_color.a > 0;

    let r_sq = r * r;
    let r_ceil = ceil_f32(r) as usize;

    let inner_x_start = x + r_ceil;
    let inner_x_end = if x + width > r_ceil { x + width - r_ceil } else { x };
    let inner_y_start = y + r_ceil;
    let inner_y_end = if y + height > r_ceil { y + height - r_ceil } else { y };
    
    let border_sz_f = border_size as f32;

    for row in y..end_y {
        let ly = (row - y) as f32 + 0.5;
        let is_top_row = row < inner_y_start;
        let is_bottom_row = row >= inner_y_end;
        let check_corners = is_top_row || is_bottom_row;
        
        let is_top_border = has_border && (row < y + border_size);
        let is_bottom_border = has_border && (row >= y + height - border_size);
        
        // Inner edge AA for straight lines (simulate fractional coverage)
        let is_top_inner_edge = has_border && (row == y + border_size - 1);
        let is_bottom_inner_edge = has_border && (row == y + height - border_size);

        for col in x..end_x {
            // 1. Straight Edge Logic (Middle Rows OR Middle Columns of Top/Bottom Rows)
            if (!check_corners) || (col >= inner_x_start && col < inner_x_end) {
                let is_left_border = has_border && (col < x + border_size);
                let is_right_border = has_border && (col >= x + width - border_size);
                
                let is_border = is_top_border || is_bottom_border || is_left_border || is_right_border;
                                
                if is_border {
                    let mut final_color = border_color;
                    
                    // Apply manual AA to the innermost pixel of the border to match rounded corners
                    let is_left_inner_edge = has_border && (col == x + border_size - 1);
                    let is_right_inner_edge = has_border && (col == x + width - border_size);
                    
                    // We only AA the edge that touches the content. 
                    // Top/Bottom border inner edges touch content.
                    // Left/Right border inner edges touch content.
                    // Outer edges touch background (handled by outer AA, but straight lines usually don't need outer AA as much as inner for "thinness").
                    
                    // Actually, if it's a 1px border, it IS the inner edge.
                    // If we fade it, the whole border fades.
                    
                    if is_top_inner_edge || is_bottom_inner_edge || is_left_inner_edge || is_right_inner_edge {
                         // Blend with background? Or just reduce alpha?
                         // To make it look "thinner", we reduce alpha.
                         // 0.6 coverage usually looks good.
                         final_color.a = (final_color.a as f32 * 0.6) as u8;
                    }

                    if is_border_opaque && final_color.a == 255 { 
                        draw_u32(buffer, buffer_width, col, row, border_u32); 
                    } else { 
                        // If we are semi-transparent border AA over background content...
                        // We need to draw the background content UNDER the border AA?
                        // If the border is INSIDE the shape, the pixel replaces the content.
                        // But "inner border AA" visually blends Border + Content.
                        // So we should draw content first.
                        if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                        else { draw_pixel(buffer, buffer_width, col, row, color); }
                        
                        draw_pixel(buffer, buffer_width, col, row, final_color); 
                    }
                } else {
                    if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                    else { draw_pixel(buffer, buffer_width, col, row, color); }
                }
                continue;
            }

            // 2. Corner Logic (Only for the corner boxes)
            let lx = (col - x) as f32 + 0.5;
            let mut dx = 0.0;
            let mut dy = 0.0;

            if is_top_row {
                if col < inner_x_start {
                    dx = r - lx;
                    dy = r - ly;
                } else { // col >= inner_x_end
                    dx = lx - (width as f32 - r);
                    dy = r - ly;
                }
            } else { // is_bottom_row
                if col < inner_x_start {
                    dx = r - lx;
                    dy = ly - (height as f32 - r);
                } else { // col >= inner_x_end
                    dx = lx - (width as f32 - r);
                    dy = ly - (height as f32 - r);
                }
            }

            let dist_sq = dx*dx + dy*dy;
            
            // Outer Edge AA (Outside rounding)
            if dist_sq > r_sq {
                 let dist = sqrt_f64(dist_sq as f64) as f32;
                 // If barely outside, draw AA
                 if dist < r + 1.0 {
                     let alpha_factor = (1.0 - (dist - r)).clamp(0.0, 1.0);
                     // What color? Border if enabled, else bg
                     let base_color = if has_border { border_color } else { color };
                     let mut final_color = base_color;
                     final_color.a = (base_color.a as f32 * alpha_factor) as u8;
                     draw_pixel(buffer, buffer_width, col, row, final_color);
                 }
                 continue;
            }
            
            let dist = sqrt_f64(dist_sq as f64) as f32;
            
            // Inner Edge AA (Border transition)
            if has_border {
                let inner_r = r - border_sz_f;
                
                if dist > inner_r {
                    // We are in the border region of the corner
                    // Check if we are close to the inner edge for AA
                    let mut final_color = border_color;
                    
                    if dist < inner_r + 1.0 {
                        // Blend Border over Background
                        let border_alpha_factor = (dist - inner_r).clamp(0.0, 1.0);
                        // Mix: Background * (1 - A) + Border * A
                        // Since we don't have a generic mix function exposed easily here without allocating,
                        // and draw_pixel does blending over existing buffer...
                        // We can just draw Background, then draw Border with adjusted alpha?
                        
                        // Draw Background first (fully)
                        if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                        else { draw_pixel(buffer, buffer_width, col, row, color); }
                        
                        // Draw Border with AA alpha
                        final_color.a = (border_color.a as f32 * border_alpha_factor) as u8;
                        draw_pixel(buffer, buffer_width, col, row, final_color);
                    } else {
                        // Full Border
                        if is_border_opaque { draw_u32(buffer, buffer_width, col, row, border_u32); }
                        else { draw_pixel(buffer, buffer_width, col, row, border_color); }
                    }
                } else {
                    // Inside the inner radius (Background)
                    if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                    else { draw_pixel(buffer, buffer_width, col, row, color); }
                }
            } else {
                // No border, just background (with Outer AA handled above, but what if dist > r-1?)
                // Wait, Outer AA block `dist_sq > r_sq` handled the transparent part.
                // Here we are INSIDE r.
                // We need to handle the AA *just inside* the outer edge?
                // Yes, `dist_sq > r_sq` is strict cutoff.
                // AA usually spans r-0.5 to r+0.5.
                
                // Let's rely on the previous Outer AA block for the "outside" fading.
                // For "inside" fading (pixels < r but > r-1), we modify alpha.
                
                if dist > r - 1.0 {
                    let alpha_factor = (r - dist).clamp(0.0, 1.0);
                    // This fades OUT as we get closer to edge? No, alpha should be 1 at r-1, and 0 at r?
                    // Standard AA: Coverage.
                    // If dist = r, coverage 0.5.
                    // If dist = r - 0.5, coverage 1.
                    // If dist = r + 0.5, coverage 0.
                    
                    // My previous logic `dist > r_sq` handled `dist > r`.
                    // That block handled alpha 0..0.5 (approx).
                    // This block handles 0.5..1.0?
                    
                    // Actually, simpler AA:
                    // Alpha = (r + 0.5 - dist).clamp(0.0, 1.0);
                    
                    let alpha_factor = (r + 1.0 - dist).clamp(0.0, 1.0); // Simple 1px wide AA
                    // But we are inside `dist <= r`. So `r+1 - dist` is >= 1.0.
                    // Wait. `sqrt` returns float.
                    // If dist is 29.9 (r=30). `31 - 29.9` = 1.1 -> 1.0.
                    
                    // If we want smooth edge:
                    // We need to blend with what's behind?
                    // `draw_pixel` does that.
                    
                    // The issue is `draw_pixel` expects `color` to be the source.
                    // If we are strictly inside, alpha is full.
                    // Logic seems fine for now.
                    if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                    else { draw_pixel(buffer, buffer_width, col, row, color); }
                } else {
                    if is_bg_opaque { draw_u32(buffer, buffer_width, col, row, bg_u32); }
                    else { draw_pixel(buffer, buffer_width, col, row, color); }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TextSegment {
    start: usize,
    end: usize,
    color: Color,
    size: f32,
}

fn parse_ansi_text(text: &str, default_color: Color, default_size: f32) -> (Vec<TextSegment>, alloc::string::String) {
    let mut segments: Vec<TextSegment> = Vec::new();
    let mut clean_text = alloc::string::String::new();
    let mut current_color = default_color;
    let current_size = default_size;
    
    let mut chars = text.chars().peekable();
    let mut clean_pos = 0;

    while let Some(c) = chars.next() {
        if c == '\x1B' && chars.peek() == Some(&'[') {
            chars.next(); // Consume '['

            let mut params_str = alloc::string::String::new();
            let mut valid_seq = false;

            while let Some(&tc) = chars.peek() {
                chars.next();
                if tc == 'm' {
                    valid_seq = true;
                    break;
                }
                if !tc.is_digit(10) && tc != ';' {
                    break; 
                }
                params_str.push(tc);
            }

            if valid_seq {
                 if params_str.is_empty() {
                     current_color = default_color;
                 } else {
                     let parts: Vec<&str> = params_str.split(';').collect();
                     let mut i = 0;
                     while i < parts.len() {
                         if let Ok(code) = parts[i].parse::<u8>() {
                             match code {
                                 0 => current_color = default_color,
                                 30 => current_color = Color::rgb(0, 0, 0),
                                 31 => current_color = Color::rgb(170, 0, 0),
                                 32 => current_color = Color::rgb(0, 170, 0),
                                 33 => current_color = Color::rgb(170, 85, 0),
                                 34 => current_color = Color::rgb(0, 0, 170),
                                 35 => current_color = Color::rgb(170, 0, 170),
                                 36 => current_color = Color::rgb(0, 170, 170),
                                 37 => current_color = Color::rgb(170, 170, 170),
                                 90 => current_color = Color::rgb(85, 85, 85),
                                 91 => current_color = Color::rgb(255, 85, 85),
                                 92 => current_color = Color::rgb(85, 255, 85),
                                 93 => current_color = Color::rgb(255, 255, 85),
                                 94 => current_color = Color::rgb(85, 85, 255),
                                 95 => current_color = Color::rgb(255, 85, 255),
                                 96 => current_color = Color::rgb(85, 255, 255),
                                 97 => current_color = Color::rgb(255, 255, 255),
                                 38 => {
                                     if i + 1 < parts.len() {
                                         if parts[i+1] == "2" && i + 4 < parts.len() {
                                             let r = parts[i+2].parse::<u8>().unwrap_or(0);
                                             let g = parts[i+3].parse::<u8>().unwrap_or(0);
                                             let b = parts[i+4].parse::<u8>().unwrap_or(0);
                                             current_color = Color::rgb(r, g, b);
                                             i += 4;
                                         } else if parts[i+1] == "5" && i + 2 < parts.len() {
                                             i += 2;
                                         }
                                     }
                                 }
                                 _ => {}
                             }
                         }
                         i += 1;
                     }
                 }
            }
        } else {
            let start = clean_pos;
            clean_text.push(c);
            clean_pos += 1;
            
            if let Some(last) = segments.last_mut() {
                if last.color == current_color && last.size == current_size && last.end == start {
                    last.end += 1;
                    continue;
                }
            }
            
            segments.push(TextSegment {
                start,
                end: clean_pos,
                color: current_color,
                size: current_size,
            });
        }
    }

    (segments, clean_text)
}

pub fn draw_text_formatted(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    text: &str,
    font: &mut TrueTypeFont,
    default_size: f32,
    default_color: Color,
    max_width: usize,
    scroll_y: usize,
    max_height: usize,
    clip_y: usize,
) -> usize {
    if buffer_width == 0 {
        return 0;
    }
    
    let (segments, clean_text) = parse_ansi_text(text, default_color, default_size);

    let mut current_x = x;
    let start_y_isize = y as isize - scroll_y as isize;
    let mut current_baseline_isize = start_y_isize;

    let chars: Vec<char> = clean_text.chars().collect();
    let mut i = 0;
    
    let limit_y = clip_y + max_height;
    let mut max_line_height_row = (default_size * 1.2) as usize;

    while i < chars.len() {
        let c = chars[i];
        
        let segment = segments.iter()
            .find(|s| i >= s.start && i < s.end)
            .copied()
            .unwrap_or(TextSegment {
                start: 0,
                end: clean_text.len(),
                color: default_color,
                size: default_size,
            });
            
        let current_line_height = (segment.size * 1.2) as usize;
        if current_line_height > max_line_height_row {
            max_line_height_row = current_line_height;
        }

        if c == '\n' {
            current_x = x;
            current_baseline_isize += max_line_height_row as isize;
            max_line_height_row = (default_size * 1.2) as usize; // Reset for next line?
            i += 1;
            continue;
        }

        let (metrics, bitmap) = font.get_char::<true>(c, segment.size);
        
        let next_x_end = (current_x as isize + metrics.left_side_bearing + metrics.advance_width as isize) as usize;

        if max_width > 0 && next_x_end >= x + max_width {
             if current_x == x {
                 // Character is wider than the box, force draw to avoid infinite loop
                 current_x += metrics.advance_width;
                 i += 1;
                 continue;
             }
             current_x = x;
             current_baseline_isize += max_line_height_row as isize;
             max_line_height_row = current_line_height;
             continue;
        }

        let glyph_y_start = (current_baseline_isize + metrics.base_line as isize) as isize;
        
        // Removed visibility check break to ensure full height calculation
        // if glyph_y_start > limit_y as isize { break; } 
        // We must process ALL text to calculate height correctly for scrollbar.
        // But we skip DRAWING if outside.

        let glyph_x = (current_x as isize + metrics.left_side_bearing) as usize;

        if glyph_y_start + (metrics.height as isize) >= clip_y as isize && glyph_y_start <= limit_y as isize {
            for row in 0..metrics.height {
                let dest_y_isize = glyph_y_start + row as isize;
                
                if dest_y_isize < clip_y as isize { continue; }
                
                let dest_y = dest_y_isize as usize;
                
                if max_height > 0 && dest_y >= clip_y + max_height { continue; }
                if dest_y >= buffer.len() / buffer_width { continue; }

                for col in 0..metrics.width {
                    let dest_x = glyph_x + col;
                    if dest_x >= buffer_width { continue; }
                    if max_width > 0 && dest_x >= x + max_width { continue; } // Pixel Clip

                    let bitmap_alpha = bitmap[row * metrics.width + col];
                    if bitmap_alpha > 0 {
                        let mut pixel_color = segment.color;
                        pixel_color.a = ((pixel_color.a as u16 * bitmap_alpha as u16) / 255) as u8;
                        draw_pixel(buffer, buffer_width, dest_x, dest_y, pixel_color);
                    }
                }
            }
        }

        current_x += metrics.advance_width;
        i += 1;
    }
    
    // Total height = (last_baseline - start_y) + last_line_height
    (current_baseline_isize - start_y_isize + max_line_height_row as isize).max(0) as usize
}

pub fn draw_text(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    text: &str,
    font: &mut TrueTypeFont,
    size: f32,
    color: Color,
) {
    draw_text_formatted(buffer, buffer_width, x, y, text, font, size, color, 0, 0, 9999, y);
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    let t = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
    Color::rgba(
        lerp_u8(start.r, end.r, t),
        lerp_u8(start.g, end.g, t),
        lerp_u8(start.b, end.b, t),
        lerp_u8(start.a, end.a, t),
    )
}

use crate::types::{LinearGradient, GradientDirection, BackgroundStyle};


pub fn draw_square_gradient(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    gradient: &LinearGradient,
) {
    if buffer_width == 0 || width == 0 || height == 0 {
        return;
    }

    let r_val = match rounding {
        Size::Absolute(v) => v as f32,
        Size::Relative(pct) => (width.min(height) as f32 * pct as f32) / 100.0,
        _ => 0.0,
    };
    let r = r_val.min(width as f32 / 2.0).min(height as f32 / 2.0);

    let end_y = (y + height).min(buffer.len() / buffer_width);
    let end_x = (x + width).min(buffer_width);

    let w_f = width as f32;
    let h_f = height as f32;

    for row in y..end_y {
        let ly = (row - y) as f32 + 0.5;

        for col in x..end_x {
            let lx = (col - x) as f32 + 0.5;

            let mut dist = 0.0;
            let mut in_corner = false;

            if ly < r {
                if lx < r {
                    let dx = r - lx;
                    let dy = r - ly;
                    dist = sqrt_f64((dx*dx + dy*dy) as f64) as f32;
                    in_corner = true;
                } else if lx > w_f - r {
                    let dx = lx - (w_f - r);
                    let dy = r - ly;
                    dist = sqrt_f64((dx*dx + dy*dy) as f64) as f32;
                    in_corner = true;
                }
            } else if ly > h_f - r {
                if lx < r {
                    let dx = r - lx;
                    let dy = ly - (h_f - r);
                    dist = sqrt_f64((dx*dx + dy*dy) as f64) as f32;
                    in_corner = true;
                } else if lx > w_f - r {
                    let dx = lx - (w_f - r);
                    let dy = ly - (h_f - r);
                    dist = sqrt_f64((dx*dx + dy*dy) as f64) as f32;
                    in_corner = true;
                }
            }

            if in_corner && dist > r {
                continue;
            }

            let t = match gradient.direction {
                GradientDirection::Horizontal => lx / w_f,
                GradientDirection::Vertical => ly / h_f,
                GradientDirection::Diagonal => (lx + ly) / (w_f + h_f),
                GradientDirection::DiagonalAlt => ((w_f - lx) + ly) / (w_f + h_f),
                GradientDirection::Custom { angle } => {
                    let norm_angle = ((angle % 360.0) + 360.0) % 360.0;

                    if norm_angle < 45.0 || norm_angle >= 315.0 {
                        lx / w_f
                    } else if norm_angle < 135.0 {
                        ly / h_f
                    } else if norm_angle < 225.0 {
                        1.0 - (lx / w_f)
                    } else {
                        1.0 - (ly / h_f)
                    }
                }
            };

            let mut color = lerp_color(gradient.start_color, gradient.end_color, t);

            if in_corner && dist > r - 1.0 {
                let alpha_factor = (r - dist).max(0.0).min(1.0);
                color.a = (color.a as f32 * alpha_factor) as u8;
            }

            if color.a > 0 {
                draw_pixel(buffer, buffer_width, col, row, color);
            }
        }
    }
}


pub fn draw_background_style(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    style: &BackgroundStyle,
    border_size: usize,
    border_color: Color,
) {
    match style {
        BackgroundStyle::Solid(color) => {
            draw_square(buffer, buffer_width, x, y, width, height, rounding, *color, border_size, border_color);
        },
        BackgroundStyle::Gradient(gradient) => {
            // Gradient borders not supported yet, just draw gradient bg with no border logic for now or simply overlay?
            // For now, let's just pass 0 border to gradient to keep it compiling, or implement it.
            // Implementing border for gradient is complex (border needs to be solid color?).
            // If border > 0, we can draw the shape with border color, then draw inner shape with gradient?
            // Or modifying the loop.
            // Let's stick to Solid support first as requested.
            draw_square_gradient(buffer, buffer_width, x, y, width, height, rounding, gradient);
            
            // TODO: Implement border for gradients
            if border_size > 0 && border_color.a > 0 {
                 draw_border_only(buffer, buffer_width, x, y, width, height, rounding, border_size, border_color);
            }
        }
    }
}

fn draw_border_only(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    border_size: usize,
    border_color: Color,
) {
    // Re-uses logic effectively by drawing "transparent background" with border?
    // Or just a separate pass.
    draw_square(buffer, buffer_width, x, y, width, height, rounding, Color::rgba(0,0,0,0), border_size, border_color);
}
