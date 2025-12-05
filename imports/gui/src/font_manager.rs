
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use libk::{print, println};
use ttf::{Font, RenderedGlyph};
use crate::gui::Color;
use crate::draw_2d::draw_pixel;

// Cache for rendered glyphs to avoid re-rendering
static mut GLYPH_CACHE: Option<BTreeMap<String, RenderedGlyph>> = None;
static mut FONT_CACHE: Option<BTreeMap<String, Font>> = None;
static mut FONT_CACHE_INITIALIZED: bool = false;

pub struct FontManager;

impl FontManager {
    /// Initialize the font cache - should be called once at startup
    pub fn init() {
        unsafe {
            if !FONT_CACHE_INITIALIZED {
                FONT_CACHE = Some(BTreeMap::new());
                GLYPH_CACHE = Some(BTreeMap::new());
                FONT_CACHE_INITIALIZED = true;
            }
        }
    }

    /// Load a font from the given path, or return cached version if already loaded
    pub fn load_font(path: &str) -> Result<(), &'static str> {
        unsafe {
            // Ensure cache is initialized
            if !FONT_CACHE_INITIALIZED {
                Self::init();
            }

            let cache = (*(&raw mut FONT_CACHE)).as_mut().unwrap();


            // Check if font is already loaded - EARLY RETURN
            if cache.contains_key(path) {
                return Ok(());
            }

            // Only load from disk if not cached
            match ttf::load_font(path) {
                Some(font) => {
                    cache.insert(String::from(path), font);
                    Ok(())
                }
                None => Err("Failed to load font")
            }
        }
    }

    /// Get a rendered character from cache with glyph-level caching
    pub fn get_char(path: &str, character: char, size: usize) -> Option<RenderedGlyph> {
        unsafe {
            if !FONT_CACHE_INITIALIZED {
                Self::init();
            }


            // Create cache key for this specific glyph
            let cache_key = format!("{}:{}:{}", path, character as u32, size);

            let glyph_cache = (*(&raw mut GLYPH_CACHE)).as_mut().unwrap();

            // Return cached glyph if available
            if let Some(cached_glyph) = glyph_cache.get(&cache_key) {
                return Some(cached_glyph.clone());
            }

            // Only load font if we need to render new glyph
            if Self::load_font(path).is_err() {
                return None;
            }

            // Render the glyph and cache it
            if let Some(glyph) = ttf::get_char(character, path, size) {
                glyph_cache.insert(cache_key, glyph.clone());
                Some(glyph)
            } else {
                None
            }
        }
    }

    /// Clean up all loaded fonts and glyphs - call this in exit_fn
    pub fn cleanup() {
        unsafe {
            if let Some(cache) = (*(&raw mut FONT_CACHE)).take() {
                for (name, font) in cache {
                    libk::syscall::free(font.font_file_ptr)
                }
            }

            if let Some(mut cache) = (*(&raw mut FONT_CACHE)).take() {
                cache.clear();
            }

            if let Some(mut glyph_cache) = (*(&raw mut GLYPH_CACHE)).take() {
                glyph_cache.clear();
            }

            FONT_CACHE_INITIALIZED = false;
        }
    }
}

// Optimized text measurement with early termination
pub fn measure_text_fast(
    text: &str,
    font_path: &str,
    font_size: usize,
    max_width: Option<usize>,
) -> (usize, usize) {

    let mut width = 0;
    let mut height = font_size;
    let mut current_line_width = 0;
    let line_height = font_size + (font_size / 4);

    for character in text.chars() {
        if character == '\n' {
            width = width.max(current_line_width);
            current_line_width = 0;
            height += line_height;
            continue;
        }

        if character == '\r' {
            continue;
        }

        // Use cached glyph lookup
        if let Some(glyph) = FontManager::get_char(font_path, character, font_size) {

            if let Some(max_w) = max_width {
                if current_line_width + glyph.advance_width > max_w && current_line_width > 0 {
                    width = width.max(current_line_width);
                    current_line_width = glyph.advance_width;
                    height += line_height;
                } else {
                    current_line_width += glyph.advance_width;
                }
            } else {
                current_line_width += glyph.advance_width;
            }
        } else {
            current_line_width += font_size / 2;
        }
    }

    width = width.max(current_line_width);
    (width, height)
}

// Optimized rendering function with better batching
pub fn render_text_to_buffer_optimized(
    buffer: &mut [u32],
    buffer_width: usize,
    text: &str,
    font_path: &str,
    font_size: usize,
    color: Color,
    x: usize,
    y: usize,
    max_width: Option<usize>,
) {
    // Early exit for empty text
    if text.is_empty() {
        return;
    }

    // Pre-load font to avoid repeated loading
    if FontManager::load_font(font_path).is_err() {
        return;
    }

    let mut cursor_x = x;
    let mut cursor_y = y;
    let line_height = font_size + (font_size / 4);

    // Batch character processing
    for character in text.chars() {
        if character == '\n' {
            cursor_x = x;
            cursor_y += line_height;
            continue;
        }

        if character == '\r' {
            continue;
        }

        // Use optimized glyph cache
        if let Some(glyph) = FontManager::get_char(font_path, character, font_size) {
            if let Some(max_w) = max_width {
                if cursor_x + glyph.width > x + max_w && cursor_x > x {
                    cursor_x = x;
                    cursor_y += line_height;
                }
            }

            let text_baseline_y = cursor_y + (font_size as f32 * 0.8) as usize;
            let glyph_y = text_baseline_y.saturating_sub(glyph.baseline_offset);

            // Bounds checking before pixel operations
            if glyph_y >= buffer.len() / buffer_width {
                break;
            }

            // Render glyph bitmap with bounds checking
            for (row_idx, row) in glyph.bitmap.iter().enumerate() {
                let pixel_y = glyph_y + row_idx;
                if pixel_y >= buffer.len() / buffer_width {
                    break;
                }

                for (col_idx, &alpha) in row.iter().enumerate() {
                    if alpha == 0 {
                        continue;
                    }

                    let pixel_x = cursor_x + col_idx;
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

            cursor_x += glyph.advance_width;
        } else {
            cursor_x += font_size / 2;
        }
    }
}