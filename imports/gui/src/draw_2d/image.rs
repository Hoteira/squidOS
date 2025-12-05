use alloc::string::{String, ToString};
use libk::{print, println};
use tga::{TgaExtension, TgaFooter, TgaHeader};
use crate::draw_2d::draw_u32;

pub fn draw_image(buffer: &mut [u32], width0: usize, item_ptr: u32, x: usize, y: usize, width: usize, height: usize, file_size: usize) {
    let header = unsafe { &*(item_ptr as *const TgaHeader) };

    let version = detect_tga_version(item_ptr, file_size);
    println!("TGA v{}: {:#?}", version, header);

    println!("File size: {} bytes", file_size);

    if width == 0 || height == 0 {
        return;
    }

    if header.encoding != 2 {
        println!("ERROR: Unsupported TGA encoding: {}", header.encoding);
        return;
    }

    if header.width == 0 || header.height == 0 {
        println!("ERROR: Invalid dimensions: {}x{}", header.width + 0, header.height + 0);
        return;
    }

    // FIXED: Use struct fields directly
    let id_length = header.magic1 as usize;
    let color_map_size = if header.colormap != 0 {
        let bytes_per_entry = (header.cmapdepth as usize + 7) / 8;
        (header.cmaplen as usize) * bytes_per_entry  // Uses bytes 5-6 correctly
    } else {
        0
    };

    // Your original calculation was correct for 18-byte header
    let pixel_data_offset = 18 + id_length + color_map_size;
    println!("DEBUG: id={} cmap={} offset={}", id_length, color_map_size, pixel_data_offset);

    let src_bitmap_ptr = (item_ptr + pixel_data_offset as u32) as *const u8;

    match header.bpp {
        32 => r_n_n_32(
            buffer,
            width0,
            src_bitmap_ptr,
            header.width as u32,
            header.height as u32,
            width as u32,
            height as u32,
            x as u32,
            y as u32,
            header.pixeltype,
        ),
        24 => r_n_n_24(
            buffer,
            width0,
            src_bitmap_ptr,
            header.width as u32,
            header.height as u32,
            width as u32,
            height as u32,
            x as u32,
            y as u32,
            header.pixeltype,
        ),
        _ => {
            println!("ERROR: Unsupported BPP: {}", header.bpp);
        }
    }
}

pub fn detect_tga_version(item_ptr: u32, file_size: usize) -> u8 {
    // Need at least 26 bytes for a TGA 2.0 footer
    if file_size < 26 {
        return 1;
    }

    // Check last 26 bytes for TGA 2.0 footer
    let footer_ptr = (item_ptr + file_size as u32 - 26) as *const TgaFooter;
    let footer = unsafe { &*footer_ptr };

    // Check for TGA 2.0 signature "TRUEVISION-XFILE."
    let signature = b"TRUEVISION-XFILE";
    if footer.signature == *signature && footer.dot == b'.' && footer.null_term == 0 {
        2
    } else {
        1
    }
}

pub fn get_tga_extension(item_ptr: u32, file_size: usize) -> Option<TgaExtension> {
    if file_size < 26 {
        return None;
    }

    // The footer offsets are at the END of the file (bytes file_size-8 to file_size-1)
    let ext_area_offset_bytes = unsafe {
        let offset_ptr = (item_ptr + (file_size as u32 - 8)) as *const u32;
        u32::from_le(*offset_ptr)
    };

    // Check if extension area exists
    if ext_area_offset_bytes == 0 {
        return None;
    }

    // Verify the extension offset is valid (within file bounds)
    let ext_size = core::mem::size_of::<TgaExtension>();
    if (ext_area_offset_bytes as usize + ext_size) > file_size {
        println!("  WARNING: Extension area offset out of bounds: {} (file size: {})",
                 ext_area_offset_bytes, file_size);
        return None;
    }

    // Now verify the signature to confirm it's TGA 2.0
    // Signature is at file_size - 26 to file_size - 8
    let signature_start = (item_ptr + (file_size as u32 - 26)) as *const u8;
    let signature_bytes = unsafe { core::slice::from_raw_parts(signature_start, 18) };

    // Expected TGA 2.0 signature: "TRUEVISION-XFILE." (18 bytes)
    let expected_signature = b"TRUEVISION-XFILE.\0\0\0\0\0\0\0\0";

    if signature_bytes != expected_signature {
        println!("  WARNING: Invalid TGA 2.0 signature");
        // Still try to read extension anyway - some files might have partial footers
    }

    // Read the extension area
    let ext_ptr = (item_ptr + ext_area_offset_bytes) as *const TgaExtension;
    let extension = unsafe { &*ext_ptr };

    Some(*extension)
}

pub fn r_n_n_32(
    buffer: &mut [u32],
    width0: usize,
    src_bitmap_ptr: *const u8,
    src_width: u32,
    src_height: u32,
    dest_width: u32,
    dest_height: u32,
    rx: u32,
    ry: u32,
    pixeltype: u8,
) {
    if src_width == 0 || src_height == 0 || dest_width == 0 || dest_height == 0 {
        return;
    }

    let x_ratio = src_width as f32 / dest_width as f32;
    let y_ratio = src_height as f32 / dest_height as f32;

    // Check if image is bottom-up (bit 5 of pixeltype = 0)
    let is_bottom_up = (pixeltype & 0x20) == 0;

    println!("DEBUG 32bpp: src={}x{}, dest={}x{}, ratios=({:.2}, {:.2}), orientation={}",
             src_width, src_height, dest_width, dest_height, x_ratio, y_ratio,
             if is_bottom_up { "bottom-up" } else { "top-down" });

    let mut pixels_drawn = 0;
    let mut pixels_skipped_alpha = 0;
    let mut pixels_out_of_bounds = 0;

    // FIXED: Check if alpha should be used based on pixeltype
    let use_alpha = (pixeltype & 0x0F) != 0;  // Any alpha bits in lower 4 bits?

    for y in 0..dest_height {
        for x in 0..dest_width {
            // Calculate destination coordinates
            let dest_x = rx + x;
            let dest_y = ry + y;

            // Bounds check for destination buffer
            if dest_x as usize >= width0 || (dest_y as usize * width0 + dest_x as usize) >= buffer.len() {
                pixels_out_of_bounds += 1;
                continue;
            }

            // Calculate source coordinates
            let src_x_f = x as f32 * x_ratio;
            let src_y_f = y as f32 * y_ratio;

            // Apply orientation correction for bottom-up images
            let src_y_f_corrected = if is_bottom_up {
                (src_height - 1) as f32 - src_y_f
            } else {
                src_y_f
            };

            // FIXED: Use your min_f32/max_f32 helpers for clamping
            let src_x = min_f32(max_f32(src_x_f, 0.0), (src_width - 1) as f32) as u32;
            let src_y = min_f32(max_f32(src_y_f_corrected, 0.0), (src_height - 1) as f32) as u32;

            // Calculate byte offset with proper stride (4 bytes per pixel)
            let row_stride = src_width * 4;
            let byte_offset = (src_y * row_stride + src_x * 4) as usize;

            // Bounds check for source data
            let max_offset = (src_height * row_stride) as usize;
            if byte_offset + 3 >= max_offset {
                continue;
            }

            // Read BGRA pixel safely
            let (b_val, g_val, r_val, a_val) = unsafe {
                let byte_ptr = src_bitmap_ptr.wrapping_add(byte_offset);
                (
                    *byte_ptr,
                    *byte_ptr.wrapping_add(1),
                    *byte_ptr.wrapping_add(2),
                    *byte_ptr.wrapping_add(3),
                )
            };

            // Convert to ARGB format
            let color = ((a_val as u32) << 24) |
                ((r_val as u32) << 16) |
                ((g_val as u32) << 8) |
                (b_val as u32);

            // FIXED: Only skip alpha if alpha is actually used
            if !use_alpha || a_val != 0 {
                draw_u32(buffer, width0, dest_x as usize, dest_y as usize, color);
                pixels_drawn += 1;
            } else {
                pixels_skipped_alpha += 1;
            }
        }
    }

    println!("DEBUG 32bpp: Drew {} pixels, skipped {} (alpha=0), {} out of bounds, use_alpha={}",
             pixels_drawn, pixels_skipped_alpha, pixels_out_of_bounds, use_alpha);
}
pub fn r_n_n_24(
    buffer: &mut [u32],
    width0: usize,
    src_bitmap_ptr: *const u8,
    src_width: u32,
    src_height: u32,
    dest_width: u32,
    dest_height: u32,
    rx: u32,
    ry: u32,
    pixeltype: u8,
) {
    if src_width == 0 || src_height == 0 || dest_width == 0 || dest_height == 0 {
        return;
    }

    let x_ratio = src_width as f32 / dest_width as f32;
    let y_ratio = src_height as f32 / dest_height as f32;
    let is_bottom_up = (pixeltype & 0x20) == 0;

    println!("DEBUG 24bpp: orientation={}", if is_bottom_up { "bottom-up" } else { "top-down" });

    let mut pixels_drawn = 0;
    let mut pixels_out_of_bounds = 0;

    for y in 0..dest_height {
        for x in 0..dest_width {
            let dest_x = rx + x;
            let dest_y = ry + y;

            // Bounds check for destination buffer
            if dest_x as usize >= width0 || (dest_y as usize * width0 + dest_x as usize) >= buffer.len() {
                pixels_out_of_bounds += 1;
                continue;
            }

            // Calculate source coordinates
            let src_x_f = x as f32 * x_ratio;
            let src_y_f = y as f32 * y_ratio;

            // Apply orientation correction
            let src_y_f_corrected = if is_bottom_up {
                (src_height - 1) as f32 - src_y_f
            } else {
                src_y_f
            };

            // FIXED: Use min_f32/max_f32 for clamping
            let src_x = min_f32(max_f32(src_x_f, 0.0), (src_width - 1) as f32) as u32;
            let src_y = min_f32(max_f32(src_y_f_corrected, 0.0), (src_height - 1) as f32) as u32;

            // Calculate byte offset (no padding for 24bpp TGA)
            let row_bytes = src_width * 3;
            let byte_offset = (src_y as usize * row_bytes as usize) + (src_x as usize * 3);

            // Bounds check
            let max_offset = src_height as usize * row_bytes as usize;
            if byte_offset + 2 >= max_offset {
                continue;
            }

            // Read BGR pixel
            let (b, g, r) = unsafe {
                let byte_ptr = src_bitmap_ptr.wrapping_add(byte_offset);
                (
                    *byte_ptr,
                    *byte_ptr.wrapping_add(1),
                    *byte_ptr.wrapping_add(2),
                )
            };

            // Convert to ARGB with full alpha
            let color = (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            draw_u32(buffer, width0, dest_x as usize, dest_y as usize, color);
            pixels_drawn += 1;
        }
    }

    println!("DEBUG 24bpp: Drew {} pixels, {} out of bounds", pixels_drawn, pixels_out_of_bounds);
}

// Helper function to convert null-terminated byte arrays to strings
pub fn bytes_to_string(bytes: &[u8]) -> String {
    // Find the first null byte or use entire array
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());

    // Convert to string, replacing invalid UTF-8 with replacement chars
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

// Helper functions for safe coordinate clamping
#[inline]
pub fn clamp_f32_to_u32(value: f32, min_val: f32, max_val: f32) -> u32 {
    if value < min_val {
        min_val as u32
    } else if value > max_val {
        max_val as u32
    } else {
        value as u32
    }
}

#[inline]
pub fn min_f32(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

#[inline]
pub fn max_f32(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}