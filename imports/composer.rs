use alloc::collections::VecDeque;
use alloc::vec::Vec;
use crate::display::{Color, DisplayServer, Mouse, State};
use core::sync::atomic::{AtomicU16, Ordering};
use libk::{print, println};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MouseEvent {
    pub wid: u32,
    pub x: usize,
    pub y: usize,
    pub buttons: [bool; 3],
    pub scroll: i8,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct KeyboardEvent {
    pub wid: u32,
    pub char: char,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ResizeEvent {
    pub wid: u32,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RedrawEvent {
    pub wid: u32,
    pub to_fb: bool,
    pub to_db: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
    depth: 8,

    framebuffer: 0xFD000000,
    double_buffer: 0x00,
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
                // Found a matching event, add to result
                result.push(self.queue[read_idx]);
                read_idx += 1;
            } else {
                // Keep non-matching event by moving it to write_idx
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

    pub fn remove_event(&mut self, event: Event) {
        for i in 0..self.queue.len() {
            if self.queue[i] == event {
                self.queue.remove(i);

                break;
            }
        }
    }

    pub fn get_next_owned(&self, window_id: u32) -> Option<Event> {
        for i in 0..self.queue.len() {
           if self.queue[i].get_window_id() == window_id {
               return Some(self.queue[i]);
           }
        }

        None
    }

}

const O: u32 = 0x0000_0000;
const B: u32 = 0x0000_00FF;
const T: u32 = 0xFFFF_FFFF;

const MOUSE_CURSOR: [u32; 96] = [
    B, O, O, O, O, O, O, O,
    B, B, O, O, O, O, O, O,
    B, T, B, O, O, O, O, O,
    B, T, T, B, O, O, O, O,
    B, T, T, T, B, O, O, O,
    B, T, T, T, T, B, O, O,
    B, T, T, T, T, T, B, O,
    B, T, T, T, T, T, T, B,
    B, T, T, T, B, B, B, B,
    B, T, B, B, T, B, O, O,
    B, B, O, O, B, T, B, O,
    B, O, O, O, O, B, B, O,
];

impl DisplayServer {
    pub fn init(&mut self) {
        let vbe = unsafe { crate::BOOTINFO.mode };
        self.width = vbe.width as u64;
        self.pitch = vbe.pitch as u64;
        self.height = vbe.height as u64;
        self.depth = vbe.bpp as usize;

        unsafe {
            crate::display::DEPTH = vbe.bpp;
        }

        self.framebuffer = vbe.framebuffer;
        unsafe {
            (*(&raw mut crate::pmm::PADDR))
                .add_fb(self.framebuffer, self.pitch as u32 * self.height as u32);

            self.double_buffer = (*(&raw mut crate::pmm::PADDR))
                .malloc(self.pitch as u32 * self.height as u32)
                .unwrap();
        }
    }

    fn generate_scaled_cursor(&self) -> Vec<u32> {
        let cursor_width = (self.width as u32 / 50).max(1); // At least 1 pixel
        let cursor_height = ((cursor_width as f32 * 1.5) as u32).max(1); // Maintain 2:3 ratio
        let mut scaled = Vec::with_capacity((cursor_width * cursor_height) as usize);

        // Scale the 8x12 cursor to cursor_width x cursor_height using nearest-neighbor
        for i in 0..cursor_height {
            let src_y = ((i as f32 * 12.0) / cursor_height as f32) as usize;
            for j in 0..cursor_width {
                let src_x = ((j as f32 * 8.0) / cursor_width as f32) as usize;
                let pixel = MOUSE_CURSOR[src_y * 8 + src_x];
                scaled.push(pixel);
            }
        }

        scaled
    }

    pub fn copy(&self) {
        let buffer_size = self.pitch as u32 * self.height as u32;
        unsafe {
            core::ptr::copy(
                self.double_buffer as *const u8,
                self.framebuffer as *mut u8,
                buffer_size as usize,
            );
        }
    }

    pub fn copy_to_fb(&self, x: u32, y: u32, width: u32, height: u32) {
        let bytes_per_pixel = match self.depth {
            32 => 4,
            24 => 3,
            _  => return,
        };

        let src = self.double_buffer as *const u8;
        let dst = self.framebuffer   as *mut u8;
        let pitch = self.pitch as u32;

        unsafe {
            for row in 0..height {
                let line_start = (y + row) * pitch;
                let offset = line_start + x * bytes_per_pixel;

                core::ptr::copy(
                    src.add(offset as usize),
                    dst.add(offset as usize),
                    (width * bytes_per_pixel) as usize,
                );
            }
        }
    }

    pub fn copy_to_db(&self, width: u32, height: u32, buffer: u32, x: u32, y: u32) {
        let dst_bpp = self.depth;
        if dst_bpp != 32 && dst_bpp != 24 {
            return;
        }

        // Source is always 32-bit RGBA/BGRA
        const SRC_BYTES_PER_PIXEL: usize = 4;
        let dst_bytes_per_pixel = if dst_bpp == 32 { 4 } else { 3 };

        let dst_pitch = self.pitch as usize;
        let src_pitch = (width as usize) * SRC_BYTES_PER_PIXEL;

        // Calculate clipping bounds
        let max_x = (self.width as u32).min(x + width);
        let max_y = (self.height as u32).min(y + height);

        if x >= self.width as u32 || y >= self.height as u32 {
            return;
        }

        let copy_width = (max_x - x) as usize;
        let copy_height = (max_y - y) as usize;

        unsafe {
            let src_ptr = buffer as *const u8;
            let dst_ptr = self.double_buffer as *mut u8;

            if dst_bpp == 32 {
                // 32-bit to 32-bit: Copy only non-transparent pixels
                for row in 0..copy_height {
                    for col in 0..copy_width {
                        let src_offset = row * src_pitch + col * SRC_BYTES_PER_PIXEL;
                        let dst_offset = (y as usize + row) * dst_pitch + (x as usize + col) * 4;

                        // Check alpha channel (byte 3)
                        let src_a = *src_ptr.add(src_offset + 3);
                        if src_a == 0 {
                            continue; // Skip fully transparent pixels
                        }

                        // Copy all channels (including alpha) for non-transparent pixels
                        core::ptr::copy(
                            src_ptr.add(src_offset),
                            dst_ptr.add(dst_offset),
                            4,
                        );
                    }
                }
            } else {
                // 32-bit to 24-bit: Copy only non-transparent pixels, drop alpha
                for row in 0..copy_height {
                    for col in 0..copy_width {
                        let src_offset = row * src_pitch + col * SRC_BYTES_PER_PIXEL;
                        let dst_offset = (y as usize + row) * dst_pitch + (x as usize + col) * 3;

                        // Check alpha channel (byte 3)
                        let src_a = *src_ptr.add(src_offset + 3);
                        if src_a == 0 {
                            continue; // Skip fully transparent pixels
                        }

                        // Copy RGB channels, skip alpha
                        *dst_ptr.add(dst_offset) = *src_ptr.add(src_offset);         // R/B
                        *dst_ptr.add(dst_offset + 1) = *src_ptr.add(src_offset + 1); // G
                        *dst_ptr.add(dst_offset + 2) = *src_ptr.add(src_offset + 2); // B/R
                    }
                }
            }
        }
    }


    pub fn copy_to_fb_a(&self, width: u32, height: u32, buffer: u32, x: u32, y: u32) {
        let dst_bpp = self.depth;
        if dst_bpp != 32 && dst_bpp != 24 {
            return;
        }

        // Source is always 32-bit RGBA/BGRA
        const SRC_BYTES_PER_PIXEL: usize = 4;
        let dst_bytes_per_pixel = if dst_bpp == 32 { 4 } else { 3 };

        let dst_pitch = self.pitch as usize;
        let src_pitch = (width as usize) * SRC_BYTES_PER_PIXEL;

        // Calculate clipping bounds
        let max_x = (self.width as u32).min(x + width);
        let max_y = (self.height as u32).min(y + height);

        if x >= self.width as u32 || y >= self.height as u32 {
            return;
        }

        let copy_width = (max_x - x) as usize;
        let copy_height = (max_y - y) as usize;

        unsafe {
            let src_ptr = buffer as *const u8;
            let dst_ptr = self.framebuffer as *mut u8;

            for row in 0..copy_height {
                for col in 0..copy_width {
                    let src_offset = row * src_pitch + col * SRC_BYTES_PER_PIXEL;
                    let dst_offset = (y as usize + row) * dst_pitch + (x as usize + col) * dst_bytes_per_pixel;

                    // Read source pixel (always 32-bit RGBA)
                    let src_r = *src_ptr.add(src_offset);
                    let src_g = *src_ptr.add(src_offset + 1);
                    let src_b = *src_ptr.add(src_offset + 2);
                    let src_a = *src_ptr.add(src_offset + 3);

                    if dst_bpp == 32 {
                        // 32-bit destination with alpha blending
                        if src_a == 255 {
                            // Fully opaque: direct copy (optimization)
                            *dst_ptr.add(dst_offset) = src_r;
                            *dst_ptr.add(dst_offset + 1) = src_g;
                            *dst_ptr.add(dst_offset + 2) = src_b;
                            *dst_ptr.add(dst_offset + 3) = src_a;
                        } else if src_a == 0 {
                            // Fully transparent: skip (optimization)
                            continue;
                        } else {
                            // Alpha blending required
                            let alpha = src_a as u16;
                            let inv_alpha = 255 - alpha;

                            // Read destination pixel
                            let dst_r = *dst_ptr.add(dst_offset) as u16;
                            let dst_g = *dst_ptr.add(dst_offset + 1) as u16;
                            let dst_b = *dst_ptr.add(dst_offset + 2) as u16;

                            // Blend and write back
                            *dst_ptr.add(dst_offset) = ((src_r as u16 * alpha + dst_r * inv_alpha) / 255) as u8;
                            *dst_ptr.add(dst_offset + 1) = ((src_g as u16 * alpha + dst_g * inv_alpha) / 255) as u8;
                            *dst_ptr.add(dst_offset + 2) = ((src_b as u16 * alpha + dst_b * inv_alpha) / 255) as u8;
                            // Keep destination alpha or set to opaque
                            *dst_ptr.add(dst_offset + 3) = 255;
                        }
                    } else {
                        // 24-bit destination: convert RGBA to RGB
                        if src_a == 0 {
                            // Fully transparent: skip
                            continue;
                        } else if src_a == 255 {
                            // Fully opaque: direct copy RGB channels
                            *dst_ptr.add(dst_offset) = src_r;
                            *dst_ptr.add(dst_offset + 1) = src_g;
                            *dst_ptr.add(dst_offset + 2) = src_b;
                        } else {
                            // Alpha blending with 24-bit destination
                            let alpha = src_a as u16;
                            let inv_alpha = 255 - alpha;

                            // Read destination RGB
                            let dst_r = *dst_ptr.add(dst_offset) as u16;
                            let dst_g = *dst_ptr.add(dst_offset + 1) as u16;
                            let dst_b = *dst_ptr.add(dst_offset + 2) as u16;

                            // Blend and write back RGB only
                            *dst_ptr.add(dst_offset) = ((src_r as u16 * alpha + dst_r * inv_alpha) / 255) as u8;
                            *dst_ptr.add(dst_offset + 1) = ((src_g as u16 * alpha + dst_g * inv_alpha) / 255) as u8;
                            *dst_ptr.add(dst_offset + 2) = ((src_b as u16 * alpha + dst_b * inv_alpha) / 255) as u8;
                        }
                    }
                }
            }
        }
    }

    pub fn write_pixel(&self, row: u32, col: u32, color: Color) {
        if col < self.width as u32 && row < self.height as u32 {
            unsafe {
                match self.depth {
                    16 => {
                        *((self.framebuffer as *mut u16).add((row * self.width as u32 + col) as usize)) = color.to_u16();
                    },

                    24 => {
                        let color = color.to_u24();
                        *((self.framebuffer as *mut u8).add(((row * self.width as u32 + col) * 3 + 0) as usize)) = color[0];
                        *((self.framebuffer as *mut u8).add(((row * self.width as u32 + col) * 3 + 1) as usize)) = color[1];
                        *((self.framebuffer as *mut u8).add(((row * self.width as u32 + col) * 3 + 2) as usize)) = color[2];
                    }

                    32 => {
                        *((self.framebuffer as *mut u32).add((row * self.width as u32 + col) as usize)) = color.to_u32();
                    }

                    _ => {}
                }
            }
        }
    }

    pub fn draw_mouse(&self, x: u16, y: u16) {

        for i in 0..12 {
            for j in 0..8 {
                let color = MOUSE_CURSOR[(i * 8 + j) as usize];
                if color != O {
                    self.write_pixel(
                        y.wrapping_add(i) as u32,
                        x.wrapping_add(j) as u32,
                        Color::from_u32(color),
                    );
                }
            }
        }
    }
}

pub static mut LAST_INPUT: u8 = 0;
pub static mut DRAGS: u8 = 0;
pub static mut DRAG: bool = false;
pub static mut DRAGGING_WINDOW: AtomicU16 = AtomicU16::new(0);
pub static mut RESIZING_WINDOW: AtomicU16 = AtomicU16::new(0);

pub static mut W_WIDTH: usize = 0;
pub static mut W_HEIGHT: usize = 0;

impl Mouse {
    pub fn cursor(&mut self, data: [u8; 4]) {
        unsafe { (*(&raw mut DISPLAY_SERVER)).copy_to_fb(self.x as u32, self.y as u32, 8, 12) };

        let x_vec = (data[1] as i8) as i16;
        let y_vec = (data[2] as i8) as i16;

        self.x = self.clamp_mx(x_vec);
        self.y = self.clamp_my(-y_vec);

        self.left = (data[0] & 0b00000001) != 0;
        self.right = (data[0] & 0b00000010) != 0;
        self.center = (data[0] & 0b00000100) != 0;

        unsafe {
            LAST_INPUT = data[0];
        }

        unsafe { (*(&raw mut DISPLAY_SERVER)).draw_mouse(self.x, self.y) };

        unsafe {
            if (LAST_INPUT & 0b00000001) != 0 {
                DRAGS = DRAGS.wrapping_add(1);

                if DRAGS > 1 {
                    DRAG = true;
                }
            } else {
                DRAGS = 0;
                DRAG = false;

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


                } else if (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) != 0 {
                    (*(&raw mut DRAGGING_WINDOW)).store(0, Ordering::Relaxed);
                    (*(&raw mut RESIZING_WINDOW)).store(0, Ordering::Relaxed);
                    W_WIDTH = 0;
                    W_HEIGHT = 0;

                    for i in (0..(*(&raw mut COMPOSER)).windows.len()).rev() {
                        let ty = COMPOSER.windows[i].w_type;
                        if ty != Items::Null {
                            (*(&raw mut DISPLAY_SERVER)).copy_to_db(
                                COMPOSER.windows[i].width as u32,
                                COMPOSER.windows[i].height as u32,
                                COMPOSER.windows[i].buffer,
                                COMPOSER.windows[i].x as u32,
                                COMPOSER.windows[i].y as u32,
                            );
                        }
                    }
                    (*(&raw mut DISPLAY_SERVER)).copy();
                }

                return;
            }
        }

        if self.left {
            let w = unsafe { (*(&raw mut COMPOSER)).find_window(self.x as usize, self.y as usize) };

            if unsafe { (*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) != 0 } {

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
                    if W_WIDTH <= final_width && W_HEIGHT <= final_height {
                        (*(&raw mut DISPLAY_SERVER)).copy_to_fb(
                            w.x as u32,
                            w.y as u32,
                            final_width as u32,
                            final_height as u32,
                        );
                    } else {
                        let mut ww = final_width;
                        let mut wh = final_height;

                        if W_WIDTH > final_width {
                            ww = W_WIDTH + 1;
                        }

                        if W_HEIGHT > final_height {
                            wh = W_HEIGHT + 1;
                        }

                        (*(&raw mut DISPLAY_SERVER))
                            .copy_to_fb(w.x as u32, w.y as u32, ww as u32, wh as u32);
                    }
                }

                unsafe {
                    W_WIDTH = cap(
                        final_width as usize,
                        ((*(&raw mut DISPLAY_SERVER)).width - w.x as u64) as usize,
                    );

                    W_HEIGHT = cap(
                        final_height as usize,
                        ((*(&raw mut DISPLAY_SERVER)).height - w.y as u64) as usize,
                    );
                }

                self.draw_square_outline(
                    w.y as u16,
                    w.x as u16,
                    unsafe { W_HEIGHT as u16 },
                    unsafe { W_WIDTH as u16 },
                    Color::rgb(245, 245, 247),
                );

                return;

            } else if unsafe { (*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) != 0 } {
                let composer = unsafe { &raw mut COMPOSER };
                let display_server = unsafe { &raw mut DISPLAY_SERVER };

                let window_opt = unsafe {
                    (*composer)
                        .find_window_id((*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) as usize)
                };
                let w = match window_opt {
                    Some(w) => w,
                    None => return,
                };

                let old_x = w.x;
                let old_y = w.y;

                let new_x = add_delta(old_x as u16, x_vec);
                let new_y = add_delta(old_y as u16, -y_vec);

                let mut updated_x = old_x;
                let mut updated_y = old_y;

                if (new_x as i16 + w.width as i16)
                    <= unsafe { ((*display_server).width - 1) as i16 }
                {
                    updated_x = new_x as usize;
                }

                if (new_y as i16 + w.height as i16)
                    <= unsafe { ((*display_server).height + 24) as i16 }
                {
                    updated_y = new_y as usize;
                }

                let reset_rect = self.union_rect(
                    old_x as u32,
                    old_y as u32,
                    w.width as u32,
                    w.height as u32,
                    updated_x as u32,
                    updated_y as u32,
                );

                unsafe {
                    (*display_server).copy_to_fb(
                        reset_rect.0,
                        reset_rect.1,
                        reset_rect.2,
                        reset_rect.3,
                    );
                }

                w.x = updated_x;
                w.y = updated_y;

                unsafe {
                    (*composer)
                        .copy_window_fb((*(&raw mut DRAGGING_WINDOW)).load(Ordering::Relaxed) as usize)
                };
                return;
            }

            if let Some(ws) = w {
                let w_type = ws.w_type;
                if w_type == Items::Window && ws.z != 0 && unsafe { DRAG == false } {
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
                    }

                    unsafe {
                        (*(&raw mut DISPLAY_SERVER)).copy_to_fb(
                            x as u32,
                            y as u32,
                            width as u32,
                            height as u32,
                        )
                    };
                } else {
                    if ws.can_move && self.y as usize >= ws.y && self.y as usize <= ws.y + 25 {
                        if unsafe { DRAG == true } {
                            unsafe {
                                (*(&raw mut DRAGGING_WINDOW)).store(0, Ordering::Relaxed);
                                W_WIDTH = 0;
                                W_HEIGHT = 0;
                                
                                (*(&raw mut DRAGGING_WINDOW)).store(ws.id as u16, Ordering::Relaxed)
                            };
                            return;
                        } else if ws.event_handler != 0 {
                            let xc = ws.x;
                            let yc = ws.y;
                            let id = ws.id;

                            unsafe {
                                (*(&raw mut GLOBAL_EVENT_QUEUE)).add_event(Event::Mouse(MouseEvent {
                                    wid: id as u32,
                                    x: self.x as usize - xc,
                                    y: self.y as usize - yc,
                                    buttons: [self.left, self.center, self.right],
                                    scroll: data[3] as i8,
                                }));
                            }
                        }
                    } else if ws.can_move
                        && (self.is_bottom_right(ws.x as u16, ws.y as u16, ws.width as u16, ws.height as u16, self.x, self.y))
                    {
                        if unsafe { DRAG } == true {
                            if unsafe { (*(&raw mut RESIZING_WINDOW)).load(Ordering::Relaxed) == 0 }
                            {
                                unsafe {
                                    W_WIDTH = ws.width;
                                    W_HEIGHT = ws.height;
                                    (*(&raw mut RESIZING_WINDOW)).store(ws.id as u16, Ordering::Relaxed);
                                };
                            }
                        }
                    } else {
                        if ws.event_handler != 0 && unsafe { DRAG == false } {
                            let xc = ws.x;
                            let yc = ws.y;
                            let id = ws.id;
                            let mouse = ws.event_handler;
                            

                            unsafe {
                                (*(&raw mut GLOBAL_EVENT_QUEUE)).add_event(Event::Mouse(MouseEvent {
                                    wid: id as u32,
                                    x: self.x as usize - xc,
                                    y: self.y as usize - yc,
                                    buttons: [self.left, self.center, self.right],
                                    scroll: data[3] as i8,
                                }));
                            }
                        }
                    }
                }
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

    fn clamp_mx(&self, n: i16) -> u16 {
        let mx_0 = self.x as i16;
        let sx = unsafe { (*(&raw mut DISPLAY_SERVER)).width } as u16;

        if n + mx_0 >= (sx as i16 - 8) {
            sx.wrapping_sub(8)
        } else if n + mx_0 <= 0 {
            0
        } else {
            (n + mx_0) as u16
        }
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

    pub fn clamp_my(&self, n: i16) -> u16 {
        let my_0 = self.y as i16;
        let sy = unsafe { (*(&raw mut DISPLAY_SERVER)).height } as u16;

        if n + my_0 >= (sy as i16 - 12) {
            sy.wrapping_sub(12)
        } else if n + my_0 <= 0 {
            return 0;
        } else {
            (n + my_0) as u16
        }
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
pub struct Window {
    pub id: usize,
    pub buffer: u32,

    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub width: usize,
    pub height: usize,

    pub can_move: bool,
    pub can_resize: bool,
    pub min_width: usize,
    pub min_height: usize,

    pub event_handler: u32,
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
                            self.windows[i].buffer as u32,
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
                            self.windows[i].buffer as u32,
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

    pub fn check_id(&self, mut rng: libk::rng::LcgRng) -> usize {
        loop {
            let wid = rng.range(0, 65545) as usize;

            let mut is_used = false;
            for i in 0..self.windows.len() {
                if self.windows[i].id == wid {
                    is_used = true;
                    break;
                }
            }

            if !is_used {
                return wid;
            }
        }
    }

    pub fn add_window(&mut self, mut w: Window) -> u32 {
        let wtype = w.w_type;
        if wtype == Items::Wallpaper {
            w.z = 255;
        } else if wtype == Items::Bar {
            w.z = 0;
        } else if wtype == Items::Popup {
            w.z = 0;
        }

        let rng = libk::rng::LcgRng::new(w.buffer as u64);
        w.id = self.check_id(rng) as usize;

        // Initialize the window's buffer to fully transparent (rgba(0,0,0,0))
        unsafe {
            if w.buffer != 0 {
                let buffer_size = (w.width * w.height * 4) as usize; // 4 bytes per pixel (RGBA)
                core::ptr::write_bytes(w.buffer as *mut u8, 0, buffer_size);
            }
        }

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
        unsafe {
            (*(&raw mut DRAGGING_WINDOW)).store(0, Ordering::Relaxed);
            (*(&raw mut RESIZING_WINDOW)).store(0, Ordering::Relaxed);
            W_WIDTH = 0;
            W_HEIGHT = 0;
        }

        w.id as u32
    }

    pub fn write_kb(&mut self, char: char) {
        for i in 0..self.windows.len() {
            let y = self.windows[i].w_type;

            if self.windows[i].z == 0
                && y != Items::Bar
                && y != Items::Wallpaper
                && y != Items::Null
                && self.windows[i].event_handler != 0
            {
                if self.windows[i].event_handler != 0 {
                    unsafe {
                        (*(&raw mut GLOBAL_EVENT_QUEUE)).add_event(Event::Keyboard(KeyboardEvent {
                            wid: self.windows[i].id as u32,
                            char,
                        }));
                    }
                }
            }
        }
    }

    pub fn resize_window(&mut self, w: Window) {;
        for i in 0..self.windows.len() {
            if w.id == self.windows[i].id {
                self.windows[i].width = w.width;
                self.windows[i].height = w.height;
                self.windows[i].buffer = w.buffer;
                self.windows[i].can_move = w.can_move;
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
            // Optional: Clear the double buffer to prevent stale data
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            core::ptr::write_bytes(
                display_server.double_buffer as *mut u8,
                0,
                (display_server.pitch * display_server.height) as usize,
            );

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
