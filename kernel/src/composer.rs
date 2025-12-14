use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::display::{Color, DisplayServer, Mouse, State};
use core::sync::atomic::{AtomicU16, Ordering};
use log::debug;
use std::os::print;
use crate::{debugln, println};
use crate::drivers::video::virtio;

pub static mut VIRTIO_ACTIVE: bool = false;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct MouseEvent {
    pub wid: u32,
    pub x: usize,
    pub y: usize,
    pub buttons: [bool; 3],
    pub scroll: i8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct KeyboardEvent {
    pub wid: u32,
    pub char: char,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct ResizeEvent {
    pub wid: u32,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct RedrawEvent {
    pub wid: u32,
    pub to_fb: bool,
    pub to_db: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum Event {
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
    Resize(ResizeEvent),
    Redraw(RedrawEvent),
    None
}

impl Event {
    pub fn get_window_id(&self) -> u32 {
        match self {
            Event::Mouse(event) => event.wid,
            Event::Keyboard(event) => event.wid,
            Event::Resize(event) => event.wid,
            Event::Redraw(event) => event.wid,
            Event::None => 0,
        }
    }
}

pub static mut DISPLAY_SERVER: DisplayServer = DisplayServer {
    width: 0,
    height: 0,
    pitch: 0,
    depth: 32,
    framebuffer: 0,
    double_buffer: 0,
};

pub static mut MOUSE: Mouse = Mouse {
    x: 0,
    y: 0,
    left: false,
    center: false,
    right: false,
    state: State::Point,
};

pub static mut GLOBAL_EVENT_QUEUE: EventQueue = EventQueue { queue: Vec::new() };

pub struct EventQueue {
    pub queue: Vec<Event>,
}

impl EventQueue {
    pub fn get_and_remove_events(&mut self, window_id: u32, max_events: usize) -> Vec<Event> {
        let mut result = Vec::with_capacity(max_events.min(self.queue.len()));
        let mut write_idx = 0;
        let mut read_idx = 0;

        while read_idx < self.queue.len() && result.len() < max_events {
            if self.queue[read_idx].get_window_id() == window_id {
                result.push(self.queue[read_idx]);
                read_idx += 1;
            } else {
                if write_idx != read_idx {
                    self.queue[write_idx] = self.queue[read_idx];
                }
                write_idx += 1;
                read_idx += 1;
            }
        }

        while read_idx < self.queue.len() {
            if write_idx != read_idx {
                self.queue[write_idx] = self.queue[read_idx];
            }
            write_idx += 1;
            read_idx += 1;
        }

        self.queue.truncate(write_idx);
        result
    }

    pub fn add_event(&mut self, event: Event) {
        if self.queue.len() >= 1000 {
            self.reset_queue();
        }
        self.queue.push(event);
    }

    pub fn reset_queue(&mut self) {
        self.queue.clear();
    }
}

impl DisplayServer {
    pub fn init(&mut self) {
        let boot_info = unsafe { crate::boot::BOOT_INFO };
        let vbe = boot_info.mode;

        // Try VirtIO GPU first
        unsafe {
            virtio::init();
            if virtio::queue::VIRT_QUEUES[0].is_some() {
                // VirtIO is available - use requested resolution
                self.width = 1280;
                self.height = 720;
                self.pitch = self.width * 4; // VirtIO always uses width * 4
                self.depth = 32;

                crate::display::DEPTH = 32;

                let size_bytes = (self.pitch * self.height) as usize;
                let pages = (size_bytes + 4095) / 4096;

                let db = crate::memory::pmm::allocate_frames(pages, 0).expect("Failed to allocate double buffer");
                let fb = crate::memory::pmm::allocate_frames(pages, 0).expect("Failed to allocate framebuffer");

                core::ptr::write_bytes(db as *mut u8, 0, size_bytes);
                core::ptr::write_bytes(fb as *mut u8, 0, size_bytes);


                self.double_buffer = db;
                self.framebuffer = fb;

                virtio::start_gpu(self.width as u32, self.height as u32, self.framebuffer);
                VIRTIO_ACTIVE = true;

                println!("DisplayServer: VirtIO GPU active at {}x{}", self.width, self.height);
                return;
            } else {
                println!("DisplayServer: VirtIO GPU active at {}x{}", self.width, self.height);
                return;
            }
        }

        // Fallback to VBE
        println!("DisplayServer: Using VBE fallback");
        self.width = vbe.width as u64;
        self.pitch = vbe.pitch as u64;
        self.height = vbe.height as u64;
        self.depth = 32;

        unsafe {
            crate::display::DEPTH = 32;
        }

        // Map VBE framebuffer with Write-Combining
        let fb_phys = vbe.framebuffer as u64;
        let fb_size = (self.pitch * self.height) as usize;
        let fb_pages = (fb_size + 4095) / 4096;
        let fb_virt_base = 0xFFFF_8000_FD00_0000;

        for i in 0..fb_pages {
            let offset = (i * 4096) as u64;
            let phys = fb_phys + offset;
            let virt = fb_virt_base + offset;

            let flags = crate::memory::paging::PAGE_PRESENT |
                crate::memory::paging::PAGE_WRITABLE |
                crate::memory::paging::PAGE_PAT;

            crate::memory::vmm::map_page(virt, phys, flags, None);
        }

        self.framebuffer = fb_virt_base;

        let size_bytes = self.pitch as usize * self.height as usize;
        let pages = (size_bytes + 4095) / 4096;

        unsafe {
            if let Some(buffer) = crate::memory::pmm::allocate_frames(pages, 0) {
                self.double_buffer = buffer;
                core::ptr::write_bytes(buffer as *mut u8, 0, size_bytes);
            } else {
                panic!("[DisplayServer] Failed to allocate double buffer!");
            }
        }
    }

    pub fn copy(&self) {
        unsafe {
            let buffer_size = self.pitch as u64 * self.height as u64;
            core::ptr::copy(
                self.double_buffer as *const u8,
                self.framebuffer as *mut u8,
                buffer_size as usize,
            );

            if VIRTIO_ACTIVE {
                virtio::flush(0, 0, self.width as u32, self.height as u32, self.width as u32);
            }
        }
    }

    pub fn copy_to_fb(&self, x: u32, y: u32, width: u32, height: u32) {
        let bytes_per_pixel = 4;

        let max_x = (self.width as u32).min(x + width);
        let max_y = (self.height as u32).min(y + height);

        if x >= self.width as u32 || y >= self.height as u32 {
            return;
        }

        let copy_width = (max_x - x) as usize;
        let copy_height = (max_y - y) as usize;

        let src = self.double_buffer as *const u8;
        let dst = self.framebuffer as *mut u8;
        let pitch = self.pitch as usize;

        unsafe {
            for row in 0..copy_height {
                let src_offset = ((y as usize + row) * pitch + x as usize * bytes_per_pixel) as usize;
                let dst_offset = ((y as usize + row) * pitch + x as usize * bytes_per_pixel) as usize;

                core::ptr::copy_nonoverlapping(
                    src.add(src_offset),
                    dst.add(dst_offset),
                    copy_width * bytes_per_pixel
                );
            }

            if VIRTIO_ACTIVE {
                virtio::flush(x, y, width, height, self.width as u32);
            }
        }
    }

    pub fn copy_to_db(&self, width: u32, height: u32, buffer: usize, x: u32, y: u32) {
        let dst_pitch = self.pitch as usize;
        let src_pitch = (width as usize) * 4;

        let max_x = (self.width as u32).min(x + width);
        let max_y = (self.height as u32).min(y + height);

        if x >= self.width as u32 || y >= self.height as u32 {
            return;
        }

        let copy_width = (max_x - x) as usize;
        let copy_height = (max_y - y) as usize;

        unsafe {
            let src_base = buffer as *const u8;
            let dst_base = self.double_buffer as *mut u8;

            for row in 0..copy_height {
                let src_row_ptr = src_base.add(row * src_pitch);
                let dst_row_ptr = dst_base.add((y as usize + row) * dst_pitch + (x as usize) * 4);

                for col in 0..copy_width {
                    let offset = col * 4;
                    let src_ptr = src_row_ptr.add(offset);
                    let dst_ptr = dst_row_ptr.add(offset);

                    let src_a = *src_ptr.add(3);

                    if src_a == 255 {
                        *(dst_ptr as *mut u32) = *(src_ptr as *const u32);
                    } else if src_a == 0 {
                        continue;
                    } else {
                        let alpha = src_a as u32;
                        let inv_alpha = 255 - alpha;

                        let src_r = *src_ptr as u32;
                        let src_g = *src_ptr.add(1) as u32;
                        let src_b = *src_ptr.add(2) as u32;

                        let dst_r = *dst_ptr as u32;
                        let dst_g = *dst_ptr.add(1) as u32;
                        let dst_b = *dst_ptr.add(2) as u32;

                        *dst_ptr = ((src_r * alpha + dst_r * inv_alpha) / 255) as u8;
                        *dst_ptr.add(1) = ((src_g * alpha + dst_g * inv_alpha) / 255) as u8;
                        *dst_ptr.add(2) = ((src_b * alpha + dst_b * inv_alpha) / 255) as u8;
                        *dst_ptr.add(3) = 255;
                    }
                }
            }
        }
    }

    pub fn copy_to_fb_a(&self, width: u32, height: u32, buffer: usize, x: u32, y: u32) {
        const BYTES_PER_PIXEL: usize = 4;

        let dst_pitch = self.pitch as usize;
        let src_pitch = (width as usize) * BYTES_PER_PIXEL;

        let max_x = (self.width as u32).min(x + width);
        let max_y = (self.height as u32).min(y + height);

        if x >= self.width as u32 || y >= self.height as u32 {
            return;
        }

        let copy_width = (max_x - x) as usize;
        let copy_height = (max_y - y) as usize;

        unsafe {
            let src_base = buffer as *const u8;
            let dst_base = self.framebuffer as *mut u8;

            for row in 0..copy_height {
                let src_row_ptr = src_base.add(row * src_pitch);
                let dst_row_ptr = dst_base.add((y as usize + row) * dst_pitch + (x as usize) * 4);

                for col in 0..copy_width {
                    let offset = col * 4;
                    let src_ptr = src_row_ptr.add(offset);
                    let dst_ptr = dst_row_ptr.add(offset);

                    let src_a = *src_ptr.add(3);

                    if src_a == 255 {
                        *(dst_ptr as *mut u32) = *(src_ptr as *const u32);
                    } else if src_a == 0 {
                        continue;
                    } else {
                        let alpha = src_a as u32;
                        let inv_alpha = 255 - alpha;

                        let src_r = *src_ptr as u32;
                        let src_g = *src_ptr.add(1) as u32;
                        let src_b = *src_ptr.add(2) as u32;

                        let dst_r = *dst_ptr as u32;
                        let dst_g = *dst_ptr.add(1) as u32;
                        let dst_b = *dst_ptr.add(2) as u32;

                        *dst_ptr = ((src_r * alpha + dst_r * inv_alpha) / 255) as u8;
                        *dst_ptr.add(1) = ((src_g * alpha + dst_g * inv_alpha) / 255) as u8;
                        *dst_ptr.add(2) = ((src_b * alpha + dst_b * inv_alpha) / 255) as u8;
                        *dst_ptr.add(3) = 255;
                    }
                }
            }
        }
    }

    pub fn write_pixel(&self, row: u32, col: u32, color: Color) {
        if col < self.width as u32 && row < self.height as u32 {
            unsafe {
                let offset = (row as u64 * self.pitch + col as u64 * 4) as usize;
                *((self.framebuffer as *mut u8).add(offset) as *mut u32) = color.to_u32();
            }
        }
    }

    pub fn draw_mouse(&self, x: u16, y: u16) {
        use crate::drivers::periferics::mouse::{CURSOR_BUFFER, CURSOR_WIDTH, CURSOR_HEIGHT};

        let mut temp_buf: [u32; 1024] = [0; 1024];
        let pitch_bytes = self.pitch as usize;
        let fb_ptr = self.framebuffer as *mut u8;
        let db_ptr = self.double_buffer as *const u8;
        let screen_w = self.width as usize;
        let screen_h = self.height as usize;
        let mx = x as usize;
        let my = y as usize;

        // If dragging, read from FB (contains window). Else read from DB (clean).
        let is_dragging = unsafe { (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) != 0 };
        let bg_src = if is_dragging { fb_ptr as *const u8 } else { db_ptr };

        unsafe {
            for row in 0..CURSOR_HEIGHT {
                let screen_y = my + row;
                if screen_y >= screen_h { break; }

                let row_byte_offset = screen_y * pitch_bytes;

                for col in 0..CURSOR_WIDTH {
                    let screen_x = mx + col;
                    if screen_x >= screen_w { break; }

                    let pixel_offset = row_byte_offset + screen_x * 4;

                    let bg_color = *(bg_src.add(pixel_offset) as *const u32);
                    let cursor_color = CURSOR_BUFFER[row * CURSOR_WIDTH + col];

                    if cursor_color != 0 {
                        temp_buf[row * CURSOR_WIDTH + col] = cursor_color;
                    } else {
                        temp_buf[row * CURSOR_WIDTH + col] = bg_color;
                    }
                }
            }

            for row in 0..CURSOR_HEIGHT {
                let screen_y = my + row;
                if screen_y >= screen_h { break; }

                let fb_offset = screen_y * pitch_bytes + mx * 4;

                let copy_w = if mx + CURSOR_WIDTH > screen_w {
                    screen_w - mx
                } else {
                    CURSOR_WIDTH
                };

                core::ptr::copy_nonoverlapping(
                    temp_buf.as_ptr().add(row * CURSOR_WIDTH) as *const u8,
                    fb_ptr.add(fb_offset),
                    copy_w * 4
                );
            }

            if VIRTIO_ACTIVE {
                virtio::flush(x as u32, y as u32, 32, 32, self.width as u32);
            }
        }
    }
}

pub static mut LAST_INPUT: u8 = 0;
pub static mut DRAGS: u8 = 0;
pub static mut DRAG: bool = false;
pub static mut DRAGGING_WINDOW: AtomicU16 = AtomicU16::new(0);
pub static mut RESIZING_WINDOW: AtomicU16 = AtomicU16::new(0);
pub static mut CLICK_STARTED_IN_TITLEBAR: bool = false;
pub static mut W_WIDTH: usize = 0;
pub static mut W_HEIGHT: usize = 0;
pub static mut MOUSE_PENDING: bool = false;

pub fn handle_mouse_update() {
    unsafe {
        use crate::drivers::periferics::mouse::MOUSE_PACKET;
        (*(&raw mut MOUSE)).cursor(MOUSE_PACKET);
    }
}

impl Mouse {
    pub fn cursor(&mut self, data: [u8; 4]) {
        // Store old position
        let old_x = self.x;
        let old_y = self.y;

        debugln!(".");

        // Calculate relative movement
        let mut x_rel = data[1] as i16;
        let mut y_rel = data[2] as i16;

        if (data[0] & 0x10) != 0 {
            x_rel |= 0xFF00u16 as i16;
        }

        if (data[0] & 0x20) != 0 {
            y_rel |= 0xFF00u16 as i16;
        }

        // Update mouse position
        self.x = self.clamp_mx(x_rel);
        self.y = self.clamp_my(-y_rel);

        // Store previous button state
        let prev_left = self.left;

        // Update button states
        self.left = (data[0] & 0b00000001) != 0;
        self.right = (data[0] & 0b00000010) != 0;
        self.center = (data[0] & 0b00000100) != 0;

        unsafe {
            LAST_INPUT = data[0];
        }

        let scroll_val = data[3] as i8;

        // Check for click start in titlebar
        if self.left && !prev_left {
            let w = unsafe { (*(&raw mut COMPOSER)).find_window(self.x as usize, self.y as usize) };
            if let Some(ws) = w {
                if ws.can_move && self.y as usize >= ws.y && self.y as usize <= ws.y + 25 {
                    unsafe { CLICK_STARTED_IN_TITLEBAR = true; }
                } else {
                    unsafe { CLICK_STARTED_IN_TITLEBAR = false; }
                }
            } else {
                unsafe { CLICK_STARTED_IN_TITLEBAR = false; }
            }
        } else if !self.left {
            unsafe { CLICK_STARTED_IN_TITLEBAR = false; }
        }

        // Handle drag state
        unsafe {
            if self.left {
                DRAGS = DRAGS.wrapping_add(1);
                if DRAGS > 2 {
                    DRAG = true;
                }
            } else {
                DRAGS = 0;
                DRAG = false;

                // Handle resize completion
                if (*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) != 0 {
                    let w = (*(&raw mut COMPOSER))
                        .find_window_id((*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) as usize)
                        .unwrap();

                    (*(&raw mut DRAGGING_WINDOW)).store(0, Ordering::Relaxed);
                    (*(&raw mut RESIZING_WINDOW)).store(0, Ordering::Relaxed);

                    if w.event_handler != 0 {
                        (*(&raw mut GLOBAL_EVENT_QUEUE)).add_event(Event::Resize(ResizeEvent {
                            wid: w.id as u32,
                            width: W_WIDTH,
                            height: W_HEIGHT,
                        }));
                    }
                    W_WIDTH = 0;
                    W_HEIGHT = 0;

                // Handle drag completion - window released
                } else if (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) != 0 {
                    let wid = (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) as usize;
                    let composer = &mut *(&raw mut COMPOSER);
                    let display_server = &mut *(&raw mut DISPLAY_SERVER);

                    // Get window info
                    let w = composer.find_window_id(wid).unwrap();
                    let win_x = w.x;
                    let win_y = w.y;
                    let win_width = w.width;
                    let win_height = w.height;

                    // Copy window to double buffer at its new position
                    composer.copy_window(wid);

                    // Clear and restore the old cursor area from DB
                    display_server.copy_to_fb(old_x as u32, old_y as u32, 32, 32);

                    // Copy window from DB to FB
                    display_server.copy_to_fb(win_x as u32, win_y as u32, win_width as u32, win_height as u32);

                    // FLUSH THE WINDOW AREA TO VIRTIO
                    if VIRTIO_ACTIVE {
                        virtio::flush(win_x as u32, win_y as u32, win_width as u32, win_height as u32, display_server.width as u32);
                    }

                    // Draw cursor at new position
                    display_server.draw_mouse(self.x, self.y);

                    (*(&raw mut DRAGGING_WINDOW)).store(0, Ordering::Relaxed);
                    (*(&raw mut RESIZING_WINDOW)).store(0, Ordering::Relaxed);
                    W_WIDTH = 0;
                    W_HEIGHT = 0;

                    return;
                }
            }
        }

        let w = unsafe { (*(&raw mut COMPOSER)).find_window(self.x as usize, self.y as usize) };

        // Handle active resize
        if unsafe { (*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) != 0 } {
            let x_vec = x_rel;
            let y_vec = y_rel;
            let dx = x_vec;
            let dy = y_vec * -1;

            let w = unsafe {
                (*(&raw mut COMPOSER))
                    .find_window_id((*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) as usize)
                    .unwrap()
            };

            let final_width = self.rem_sign(unsafe { W_WIDTH } as i16 + dx) as usize;
            let final_height = self.rem_sign(unsafe { W_HEIGHT } as i16 + dy) as usize;

            unsafe {
                W_WIDTH = cap(final_width, ((*(&raw mut DISPLAY_SERVER)).width as usize).saturating_sub(w.x));
                W_HEIGHT = cap(final_height, ((*(&raw mut DISPLAY_SERVER)).height as usize).saturating_sub(w.y));
            }

            self.draw_square_outline(
                w.y as u16,
                w.x as u16,
                unsafe { W_HEIGHT as u16 },
                unsafe { W_WIDTH as u16 },
                Color::rgb(245, 245, 247)
            );

            unsafe { (*(&raw mut DISPLAY_SERVER)).draw_mouse(self.x, self.y) };
            return;

        // Handle active drag - window being moved
        } else if unsafe { (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) != 0 } {
            let composer = unsafe { &mut *(&raw mut COMPOSER) };
            let display_server = unsafe { &mut *(&raw mut DISPLAY_SERVER) };
            let wid = unsafe { (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) as usize };

            let x_vec = x_rel;
            let y_vec = y_rel;

            // Get window info and calculate new position
            let window_opt = composer.find_window_id(wid);
            let w = match window_opt {
                Some(w) => w,
                None => return,
            };

            let old_win_x = w.x;
            let old_win_y = w.y;
            let width = w.width;
            let height = w.height;
            let id = w.id;
            let buffer = w.buffer;

            // Calculate new window position based on mouse delta
            let mut new_x = old_win_x as i32 + x_vec as i32;
            let mut new_y = old_win_y as i32 - y_vec as i32; // Note: y_vec is inverted

            // Only clamp the TOP edge to keep titlebar grabbable (keep 3px visible)
            let screen_height = display_server.height as i32;
            let min_y = -(height as i32) + 3;

            if new_y < min_y { new_y = min_y; }

            // No other clamping - let window go anywhere else!

            // Update window position
            let new_x = new_x.abs() as usize;
            let new_y = new_y.abs() as usize;

            // Update the window coordinates in the composer
            for i in 0..composer.windows.len() {
                if composer.windows[i].id == id {
                    composer.windows[i].x = new_x;
                    composer.windows[i].y = new_y;
                    break;
                }
            }

            // Erase old window position by copying clean background from DB to FB
            let src = display_server.double_buffer as *const u8;
            let dst = display_server.framebuffer as *mut u8;
            let pitch = display_server.pitch as usize;

            // Fast path: window fully onscreen
            if old_win_x + width <= display_server.width as usize &&
                old_win_y + height <= display_server.height as usize {
                unsafe {
                    for row in 0..height {
                        let offset = ((old_win_y + row) * pitch + old_win_x * 4) as usize;
                        core::ptr::copy_nonoverlapping(
                            src.add(offset),
                            dst.add(offset),
                            width * 4
                        );
                    }
                }
            } else if old_win_x < display_server.width as usize && old_win_y < display_server.height as usize {
                // Slow path: window partially offscreen
                let visible_width = width.min(display_server.width as usize - old_win_x);
                let visible_height = height.min(display_server.height as usize - old_win_y);

                unsafe {
                    for row in 0..visible_height {
                        let offset = ((old_win_y + row) * pitch + old_win_x * 4) as usize;
                        core::ptr::copy_nonoverlapping(
                            src.add(offset),
                            dst.add(offset),
                            visible_width * 4
                        );
                    }
                }
            }

            // Draw window from its buffer at new position directly to FB
            let src_buffer = buffer as *const u8;
            let dst_fb = display_server.framebuffer as *mut u8;
            let src_pitch = width * 4;
            let dst_pitch = display_server.pitch as usize;

            // Fast path: window fully onscreen at new position
            if  new_x + width <= display_server.width as usize &&
                new_y + height <= display_server.height as usize {
                let new_x = new_x as usize;
                let new_y = new_y as usize;
                unsafe {
                    for row in 0..height {
                        let src_offset = row * src_pitch;
                        let dst_offset = (new_y + row) * dst_pitch + new_x * 4;
                        core::ptr::copy_nonoverlapping(
                            src_buffer.add(src_offset),
                            dst_fb.add(dst_offset),
                            width * 4
                        );
                    }
                }
            } else {
                // Slow path: window partially offscreen
                let draw_x = new_x.max(0) as usize;
                let draw_y = new_y.max(0) as usize;

                if draw_x < display_server.width as usize && draw_y < display_server.height as usize {
                    let src_x_offset = if new_x < 0 { (-(new_x as i32)) as usize } else { 0 };
                    let src_y_offset = if new_y < 0 { (-(new_y as i32)) as usize } else { 0 };

                    let visible_width = (width - src_x_offset).min(display_server.width as usize - draw_x);
                    let visible_height = (height - src_y_offset).min(display_server.height as usize - draw_y);

                    unsafe {
                        for row in 0..visible_height {
                            let src_offset = (src_y_offset + row) * src_pitch + src_x_offset * 4;
                            let dst_offset = (draw_y + row) * dst_pitch + draw_x * 4;
                            core::ptr::copy_nonoverlapping(
                                src_buffer.add(src_offset),
                                dst_fb.add(dst_offset),
                                visible_width * 4
                            );
                        }
                    }
                }
            }

            // Calculate union rect of old and new positions for efficient flush
            // Clamp to screen bounds for VirtIO flush
            let old_x_clamped = (old_win_x as i32).max(0) as u32;
            let old_y_clamped = (old_win_y as i32).max(0) as u32;
            let new_x_clamped = new_x.max(0) as u32;
            let new_y_clamped = new_y.max(0) as u32;
            let screen_w = unsafe { (*(&raw const DISPLAY_SERVER)).width as usize };
            let screen_h = unsafe { (*(&raw const DISPLAY_SERVER)).height as usize };

            let old_x_end = (old_win_x + width).max(0).min(screen_w) as u32;
            let old_y_end = (old_win_y + height).max(0).min(screen_h) as u32;
            let new_x_end = (new_x + width).max(0).min(screen_w) as u32;
            let new_y_end = (new_y + height).max(0).min(screen_h) as u32;

            let min_x = old_x_clamped.min(new_x_clamped);
            let min_y = old_y_clamped.min(new_y_clamped);
            let max_x = old_x_end.max(new_x_end);
            let max_y = old_y_end.max(new_y_end);

            let flush_x = min_x;
            let flush_y = min_y;
            let flush_w = max_x.saturating_sub(min_x);
            let flush_h = max_y.saturating_sub(min_y);

            // SINGLE FLUSH TO VIRTIO for both old and new positions
            unsafe {
                if VIRTIO_ACTIVE {
                    virtio::flush(flush_x, flush_y, flush_w, flush_h, display_server.width as u32);
                }
            }

            // Draw cursor at new position
            display_server.draw_mouse(self.x, self.y);
            return;
        }

        // Normal case (not dragging) - erase old cursor by copying from DB to FB
        unsafe {
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            display_server.copy_to_fb(old_x as u32, old_y as u32, 32, 32);
        };

        // Handle window interaction
        if let Some(ws) = w {
            // Bring window to front on click
            if ws.w_type == Items::Window && ws.z != 0 && self.left && !prev_left {
                let x = ws.x;
                let y = ws.y;
                let width = ws.width;
                let height = ws.height;
                let id = ws.id;

                unsafe {
                    for i in (*(&raw mut COMPOSER)).windows.iter_mut() {
                        if i.id != id {
                            i.z = i.z.wrapping_add(1);
                        } else {
                            i.z = 0;
                        }
                    }
                    (*(&raw mut COMPOSER)).windows.sort_by_key(|w| w.z);
                    (*(&raw mut COMPOSER)).copy_window(id);

                    (*(&raw mut DISPLAY_SERVER)).copy_to_fb(
                        x as u32,
                        y as u32,
                        width as u32,
                        height as u32,
                    );
                }
            }

            let mut handled_drag = false;
            if self.left {
                // Check if starting drag from titlebar
                if ws.can_move && self.y as usize >= ws.y && self.y as usize <= ws.y + 25 {
                    if unsafe { DRAG == true && CLICK_STARTED_IN_TITLEBAR } {
                        if unsafe { (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) == 0 } {
                            unsafe {
                                (*(&raw mut DRAGGING_WINDOW)).store(ws.id as u16, Ordering::Relaxed);
                                (*(&raw mut COMPOSER)).recompose_except(ws.id);
                            }
                        }
                        handled_drag = true;
                    }
                // Check if starting resize from bottom-right corner
                } else if ws.can_move && self.is_bottom_right(
                    ws.x as u16,
                    ws.y as u16,
                    ws.width as u16,
                    ws.height as u16,
                    self.x,
                    self.y
                ) {
                    if unsafe { DRAG == true } {
                        if unsafe { (*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) == 0 } {
                            unsafe {
                                W_WIDTH = ws.width;
                                W_HEIGHT = ws.height;
                                (*(&raw mut RESIZING_WINDOW)).store(ws.id as u16, Ordering::Relaxed);
                            }
                        }
                        handled_drag = true;
                    }
                }
            }

            if handled_drag {
                unsafe { (*(&raw mut DISPLAY_SERVER)).draw_mouse(self.x, self.y) };
                return;
            }

            // Send mouse event to window
            if ws.event_handler != 0 && unsafe { DRAG == false } {
                let xc = ws.x;
                let yc = ws.y;
                let id = ws.id;
                unsafe {
                    (*(&raw mut GLOBAL_EVENT_QUEUE)).add_event(Event::Mouse(MouseEvent {
                        wid: id as u32,
                        x: self.x as usize - xc,
                        y: self.y as usize - yc,
                        buttons: [self.left, self.center, self.right],
                        scroll: scroll_val,
                    }));
                }
            }
        }

        // Draw mouse cursor at new position
        unsafe { (*(&raw mut DISPLAY_SERVER)).draw_mouse(self.x, self.y) };
    }

    // Helper function to copy a square from FB to FB (for cursor erasure during drag)
    fn copy_fb_to_fb(&self, display_server: &DisplayServer, x: u16, y: u16, width: u32, height: u32) {
        let x = x as usize;
        let y = y as usize;
        let width = width as usize;
        let height = height as usize;

        let screen_w = display_server.width as usize;
        let screen_h = display_server.height as usize;
        let pitch = display_server.pitch as usize;

        if x >= screen_w || y >= screen_h {
            return;
        }

        let copy_w = width.min(screen_w - x);
        let copy_h = height.min(screen_h - y);

        unsafe {
            let fb_ptr = display_server.framebuffer as *mut u8;

            // Copy each row (we need to use a temp buffer since source and dest overlap)
            let mut temp_row: [u8; 128] = [0; 128]; // 32 pixels * 4 bytes

            for row in 0..copy_h {
                let offset = (y + row) * pitch + x * 4;
                let src_ptr = fb_ptr.add(offset);
                let dst_ptr = fb_ptr.add(offset);

                // Copy to temp, then back (effectively a no-op but clears cursor pixels)
                core::ptr::copy_nonoverlapping(src_ptr, temp_row.as_mut_ptr(), copy_w * 4);
                core::ptr::copy_nonoverlapping(temp_row.as_ptr(), dst_ptr, copy_w * 4);
            }

            if VIRTIO_ACTIVE {
                virtio::flush(x as u32, y as u32, width as u32, height as u32, screen_w as u32);
            }
        }
    }

    fn rem_sign(&self, n: i16) -> u16 {
        if n < 0 { (n * -1) as u16 } else { n as u16 }
    }

    pub fn union_rect(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        x2: u32,
        y2: u32,
    ) -> (u32, u32, u32, u32) {
        let min_x = x.min(x2);
        let max_x = (x + width).max(x2 + width);
        let min_y = y.min(y2);
        let max_y = (y + height).max(y2 + height);

        let width = max_x - min_x;
        let height = max_y - min_y;

        (min_x, min_y, width, height)
    }

    fn is_bottom_right(
        &self,
        w_x: u16,
        w_y: u16,
        w_width: u16,
        w_height: u16,
        mouse_x: u16,
        mouse_y: u16,
    ) -> bool {
        let x_min = w_x.wrapping_add(w_width.wrapping_sub(8));
        let x_max = w_x.wrapping_add(w_width.wrapping_sub(0));
        let y_min = w_y.wrapping_add(w_height.wrapping_sub(8));
        let y_max = w_y.wrapping_add(w_height.wrapping_sub(0));

        (mouse_x >= x_min && mouse_x <= x_max) && (mouse_y >= y_min && mouse_y <= y_max)
    }

    pub fn draw_square_outline(&self, x: u16, y: u16, width: u16, height: u16, color: Color) {
        let max_x = x + width - 1;
        let max_y = y + height - 1;
        unsafe {
            for i in x..=max_x {
                (*(&raw mut DISPLAY_SERVER)).write_pixel(i as u32, y as u32, color);
                (*(&raw mut DISPLAY_SERVER)).write_pixel(i as u32, max_y as u32, color);
            }
            for i in y..=max_y {
                (*(&raw mut DISPLAY_SERVER)).write_pixel(x as u32, i as u32, color);
                (*(&raw mut DISPLAY_SERVER)).write_pixel(max_x as u32, i as u32, color);
            }
        }
    }

    fn clamp_mx(&self, n: i16) -> u16 {
        let mx_0 = self.x as i16;
        let sx = unsafe { (*(&raw mut DISPLAY_SERVER)).width } as u16;

        if n + mx_0 >= (sx as i16 - 2) {
            sx.wrapping_sub(2)
        } else if n + mx_0 <= 0 {
            0
        } else {
            (n + mx_0) as u16
        }
    }

    fn clamp_my(&self, n: i16) -> u16 {
        let my_0 = self.y as i16;
        let sy = unsafe { (*(&raw mut DISPLAY_SERVER)).height } as u16;

        if n + my_0 >= (sy as i16 - 2) {
            sy.wrapping_sub(2)
        } else if n + my_0 <= 0 {
            return 0;
        } else {
            (n + my_0) as u16
        }
    }
}

fn cap(n: usize, value: usize) -> usize {
    if n > value { value } else { n }
}

pub fn add_delta(n: u16, m: i16) -> u16 {
    if (n as i16 + m) < 0 {
        0
    } else {
        (n as i16 + m) as u16
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum Items {
    Wallpaper,
    Bar,
    Popup,
    Window,
    Null,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Window {
    pub id: usize,
    pub buffer: usize,
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub width: usize,
    pub height: usize,
    pub can_move: bool,
    pub can_resize: bool,
    pub min_width: usize,
    pub min_height: usize,
    pub event_handler: usize,
    pub w_type: Items,
}

pub static NULL_WINDOW: Window = Window {
    id: 0,
    buffer: 0,
    x: 0,
    y: 0,
    z: 0,
    width: 0,
    height: 0,
    can_move: false,
    can_resize: false,
    min_width: 0,
    min_height: 0,
    event_handler: 0,
    w_type: Items::Null,
};

#[derive(Debug, Clone)]
pub struct Composer {
    pub windows: [Window; 16],
}

pub static mut COMPOSER: Composer = Composer {
    windows: [NULL_WINDOW; 16],
};

impl Composer {
    pub fn copy_window(&mut self, id: usize) {
        for i in 0..self.windows.len() {
            if id == self.windows[i].id {
                match self.windows[i].w_type {
                    Items::Null => {}
                    _ => unsafe {
                        (*(&raw mut crate::composer::DISPLAY_SERVER)).copy_to_db(
                            self.windows[i].width as u32,
                            self.windows[i].height as u32,
                            self.windows[i].buffer,
                            self.windows[i].x as u32,
                            self.windows[i].y as u32,
                        )
                    },
                }
            }
        }
    }

    pub fn copy_window_fb(&mut self, id: usize) {
        for i in 0..self.windows.len() {
            if id == self.windows[i].id {
                match self.windows[i].w_type {
                    Items::Null => {}
                    _ => unsafe {
                        (*(&raw mut DISPLAY_SERVER)).copy_to_fb_a(
                            self.windows[i].width as u32,
                            self.windows[i].height as u32,
                            self.windows[i].buffer,
                            self.windows[i].x as u32,
                            self.windows[i].y as u32,
                        )
                    },
                }
            }
        }
    }

    pub fn find_window(&mut self, x: usize, y: usize) -> Option<&mut Window> {
        for i in 0..self.windows.len() {
            if x >= self.windows[i].x
                && x <= (self.windows[i].x + self.windows[i].width)
                && y >= self.windows[i].y
                && y <= (self.windows[i].y + self.windows[i].height)
            {
                match self.windows[i].w_type {
                    Items::Null => {}
                    _ => return Some(&mut self.windows[i]),
                }
            }
        }
        None
    }

    pub fn find_window_id(&mut self, id: usize) -> Option<&mut Window> {
        for i in 0..self.windows.len() {
            if self.windows[i].id == id {
                let h = self.windows[i].w_type;
                if h != Items::Null {
                    return Some(&mut self.windows[i]);
                }
            }
        }
        None
    }

    pub fn check_id(&self, _rng_seed: u64) -> usize {
        static mut NEXT_ID: usize = 1;
        unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        }
    }

    pub fn add_window(&mut self, mut w: Window) -> usize {
        let wtype = w.w_type;
        if wtype == Items::Wallpaper {
            w.z = 255;
        } else if wtype == Items::Bar {
            w.z = 0;
        } else if wtype == Items::Popup {
            w.z = 0;
        }

        w.id = self.check_id(w.buffer as u64);

        for i in 0..self.windows.len() {
            match self.windows[i].w_type {
                Items::Null => {
                    self.windows[i] = w;
                    break;
                }
                _ => {}
            }
        }

        for i in 0..self.windows.len() {
            if self.windows[i].id != w.id {
                self.windows[i].z = self.windows[i].z.saturating_add(1);
            }
        }

        self.windows.sort_by_key(|w| w.z);
        w.id
    }

    pub fn resize_window(&mut self, w: Window) {
        for i in 0..self.windows.len() {
            if w.id == self.windows[i].id {
                self.windows[i].width = w.width;
                self.windows[i].height = w.height;
                self.windows[i].buffer = w.buffer;
                self.windows[i].can_move = w.can_move;
            }
        }
    }

    pub fn recompose_except(&mut self, except_id: usize) {
        unsafe {
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            if display_server.double_buffer != 0 {
                core::ptr::write_bytes(
                    display_server.double_buffer as *mut u8,
                    0,
                    (display_server.pitch * display_server.height) as usize,
                );
            }

            for i in (0..self.windows.len()).rev() {
                if self.windows[i].id != except_id {
                    match self.windows[i].w_type {
                        Items::Null => {}
                        _ => {
                            display_server.copy_to_db(
                                self.windows[i].width as u32,
                                self.windows[i].height as u32,
                                self.windows[i].buffer,
                                self.windows[i].x as u32,
                                self.windows[i].y as u32,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn remove_window(&mut self, wid: usize) {
        for i in 0..self.windows.len() {
            if self.windows[i].id == wid {
                self.windows[i].w_type = Items::Null;
                self.windows[i].z = 255;
            }
        }

        self.windows.sort_by_key(|w| w.z);

        unsafe {
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            if display_server.double_buffer != 0 {
                core::ptr::write_bytes(
                    display_server.double_buffer as *mut u8,
                    0,
                    (display_server.pitch * display_server.height) as usize,
                );
            }

            for j in (0..self.windows.len()).rev() {
                match self.windows[j].w_type {
                    Items::Null => {}
                    _ => {
                        (*(&raw mut DISPLAY_SERVER)).copy_to_db(
                            self.windows[j].width as u32,
                            self.windows[j].height as u32,
                            self.windows[j].buffer,
                            self.windows[j].x as u32,
                            self.windows[j].y as u32,
                        );
                    }
                }
            }

            (*(&raw mut DISPLAY_SERVER)).copy();
        }
    }
}