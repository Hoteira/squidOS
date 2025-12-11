use crate::graphics::{draw_pixel, draw_u32};
use crate::types::{Color, Size};
use crate::math::{ceil_f64, floor_f64, min_f32, sqrt_f64};

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
) {
    if color.a == 0 {
        return
    } else if color.a == 255 {
        draw_square_monochrome(buffer, buffer_width, x, y, width, height, rounding, color);
    } else {
        draw_square_alpha(buffer, buffer_width, x, y, width, height, rounding, color);
    }
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
) {
    if buffer_width == 0 || width == 0 || height == 0 {
        return;
    }

    let radius_px = match rounding {
        Size::Absolute(r) => r,
        Size::Relative(pct) => {
            let min_dim = usize::min(width, height);
            (min_dim * pct) / 100
        }

        _ =>  0
    };

    let r = radius_px as isize;
    let r2 = r * r;

    for i in 0..height {
        let iy = i as isize;
        let mut x_start = 0isize;
        let mut x_end = width as isize;

        if iy < r {
            let dy = r - iy;
            let dx = floor_f64(sqrt_f64((r2 - (dy * dy)) as f64)) as isize;
            x_start = r - dx;
            x_end = (width as isize) - (r - dx);
        } else if iy >= (height as isize - r) {
            let dy = iy - ((height as isize) - r - 1);
            let dx = floor_f64(sqrt_f64((r2 - (dy * dy)) as f64)) as isize;
            x_start = r - dx;
            x_end = (width as isize) - (r - dx);
        }

        let xs = x_start.max(0).min(width as isize) as usize;
        let xe = x_end.max(0).min(width as isize) as usize;

        let row = y + i;
        if row >= buffer.len() / buffer_width {
            continue;
        }
        // let base = row * buffer_width;

        for col in xs..xe {
            let col_idx = x + col;
            if col_idx >= buffer_width {
                continue;
            }

            draw_pixel(buffer, buffer_width, col_idx, row, color );
        }
    }
}

pub fn draw_square_monochrome(
    buffer: &mut [u32],
    buffer_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    rounding: Size,
    color: Color,
) {
    let color_u32 = color.to_u32();
    let alpha = ((color_u32 >> 24) & 0xFF) as u8;
    let r_base = ((color_u32 >> 16) & 0xFF) as f32;
    let g_base = ((color_u32 >> 8) & 0xFF) as f32;
    let b_base = (color_u32 & 0xFF) as f32;

    if buffer_width == 0 || width == 0 || height == 0 {
        return;
    }

    let radius_px = match rounding {
        Size::Absolute(r) => r,
        Size::Relative(pct) => {
            let min_dim = usize::min(width, height);
            (min_dim * pct) / 100
        }
        _ => 0,
    };

    let r = radius_px as f64;
    let r2 = r * r;

    for i in 0..height {
        let iy = i as f64;
        let mut x_start = 0.0;
        let mut x_end = width as f64;

        let mut is_corner_row = false;
        if iy < r {
            is_corner_row = true;
            let dy = r - iy;
            let dx = sqrt_f64(r2 - dy * dy);
            x_start = r - dx;
            x_end = (width as f64) - (r - dx);
        } else if iy >= (height as f64 - r) {
            is_corner_row = true;
            let dy = iy - (height as f64 - r - 1.0);
            let dx = sqrt_f64(r2 - dy * dy);
            x_start = r - dx;
            x_end = (width as f64) - (r - dx);
        }

        let xs = floor_f64(x_start) as usize;
        let xe = ceil_f64(x_end) as usize;

        let row = y + i;
        if row >= buffer.len() / buffer_width {
            continue;
        }
        let base = row * buffer_width;

        // Draw the main square pixels
        for col in xs..xe {
            let col_idx = x + col;
            if col_idx >= buffer_width {
                continue;
            }

            let idx = base + col_idx;
            if idx < buffer.len() {
                draw_u32(buffer, buffer_width, col_idx, row, color_u32);
            }
        }

        // Anti-aliasing for corner rows
        if is_corner_row {
            let (center_x_left, center_x_right, center_y) = if iy < r {
                // Top corners
                (r, width as f64 - r, r)
            } else {
                // Bottom corners
                (r, width as f64 - r, height as f64 - r - 1.0)
            };

            // Anti-alias left edge
            let x_left = floor_f64(x_start) as usize;
            if x_left > 0 {
                let col_idx = x + x_left - 1;
                if col_idx < buffer_width {
                    // let idx = base + col_idx;
                    // if idx < buffer.len() 
                    {
                        let px = (x_left as f64 - center_x_left + 0.5).abs();
                        let py = (iy - center_y).abs();
                        let dist = sqrt_f64(px * px + py * py);
                        let t = ((dist - r + 0.5).clamp(0.0, 1.0) * 0.3) as f32; // More subtle
                        
                        let r_new = min_f32(r_base * (1.0 - t) + 100.0 * t, 255.0) as u8;
                        let g_new = min_f32(g_base * (1.0 - t) + 150.0 * t, 255.0) as u8;
                        let b_new = min_f32(b_base * (1.0 - t) + 255.0 * t, 255.0) as u8;
                        let aa_color = (alpha as u32) << 24 | (r_new as u32) << 16 | (g_new as u32) << 8 | (b_new as u32);
                        draw_u32(buffer, buffer_width, col_idx, row, aa_color);
                    }
                }
            }

            // Anti-alias right edge
            let x_right = ceil_f64(x_end) as usize;
            if x_right < width {
                let col_idx = x + x_right;
                if col_idx < buffer_width {
                    {
                        let px = (x_right as f64 - center_x_right + 0.5).abs();
                        let py = (iy - center_y).abs();
                        let dist = sqrt_f64(px * px + py * py);
                        let t = ((dist - r + 0.5).clamp(0.0, 1.0) * 0.3) as f32; // More subtle
                        
                        let r_new = min_f32(r_base * (1.0 - t) + 100.0 * t, 255.0) as u8;
                        let g_new = min_f32(g_base * (1.0 - t) + 150.0 * t, 255.0) as u8;
                        let b_new = min_f32(b_base * (1.0 - t) + 255.0 * t, 255.0) as u8;
                        let aa_color = (alpha as u32) << 24 | (r_new as u32) << 16 | (g_new as u32) << 8 | (b_new as u32);
                        draw_u32(buffer, buffer_width, col_idx, row, aa_color);
                    }
                }
            }
        }
    }

    // Anti-alias rows just outside the corner regions
    for i in [r as usize, height - r as usize].iter() {
        if *i >= height {
            continue;
        }
        let iy = *i as f64;
        let row = y + *i;
        if row >= buffer.len() / buffer_width {
            continue;
        }
        let base = row * buffer_width;

        let (center_y, center_x_left, center_x_right) = if *i == r as usize {
            (r, r, width as f64 - r)
        } else {
            (height as f64 - r - 1.0, r, width as f64 - r)
        };

        // Anti-alias left corner
        for col in 0..r as usize {
            let col_idx = x + col;
            if col_idx >= buffer_width {
                continue;
            }
            let idx = base + col_idx;
            if idx < buffer.len() {
                let px = (col as f64 - center_x_left + 0.5).abs();
                let py = (iy - center_y).abs();
                let dist = sqrt_f64(px * px + py * py);
                let t = ((dist - r + 0.5).clamp(0.0, 1.0) * 0.3) as f32; // More subtle
                
                let r_new = min_f32(r_base * (1.0 - t) + 100.0 * t, 255.0) as u8;
                let g_new = min_f32(g_base * (1.0 - t) + 150.0 * t, 255.0) as u8;
                let b_new = min_f32(b_base * (1.0 - t) + 255.0 * t, 255.0) as u8;
                let aa_color = (alpha as u32) << 24 | (r_new as u32) << 16 | (g_new as u32) << 8 | (b_new as u32);
                draw_u32(buffer, buffer_width, col_idx, row, aa_color);
            }
        }

        // Anti-alias right corner
        for col in (width - r as usize)..width {
            let col_idx = x + col;
            if col_idx >= buffer_width {
                continue;
            }
            let idx = base + col_idx;
            if idx < buffer.len() {
                let px = (col as f64 - center_x_right + 0.5).abs();
                let py = (iy - center_y).abs();
                let dist = sqrt_f64(px * px + py * py);
                let t = ((dist - r + 0.5).clamp(0.0, 1.0) * 0.3) as f32; 
                
                let r_new = min_f32(r_base * (1.0 - t) + 100.0 * t, 255.0) as u8;
                let g_new = min_f32(g_base * (1.0 - t) + 150.0 * t, 255.0) as u8;
                let b_new = min_f32(b_base * (1.0 - t) + 255.0 * t, 255.0) as u8;
                let aa_color = (alpha as u32) << 24 | (r_new as u32) << 16 | (g_new as u32) << 8 | (b_new as u32);
                draw_u32(buffer, buffer_width, col_idx, row, aa_color);
            }
        }
    }
}
