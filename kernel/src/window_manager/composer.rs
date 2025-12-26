use super::window::{Window, Items, NULL_WINDOW};
use crate::window_manager::display::DISPLAY_SERVER;
use crate::window_manager::events::{Event, ResizeEvent, GLOBAL_EVENT_QUEUE};
use crate::debugln;

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
                        (*(&raw mut DISPLAY_SERVER)).copy_to_db(
                            self.windows[i].width as u32,
                            self.windows[i].height as u32,
                            self.windows[i].buffer,
                            self.windows[i].x as i32,
                            self.windows[i].y as i32,
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
                            self.windows[i].x as i32,
                            self.windows[i].y as i32,
                        )
                    },
                }
            }
        }
    }

    pub fn find_window(&mut self, x: usize, y: usize) -> Option<&mut Window> {
        let mx = x as isize;
        let my = y as isize;
        
        for i in 0..self.windows.len() {
            if mx >= self.windows[i].x
                && mx <= (self.windows[i].x + self.windows[i].width as isize)
                && my >= self.windows[i].y
                && my <= (self.windows[i].y + self.windows[i].height as isize)
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

    pub fn update_tiling(&mut self) {
        let mut tiled_windows = [0usize; 16];
        let mut tiled_count = 0;
        
        let mut bar_height = 0;
        let mut bar_position_top = true;

        for i in 0..self.windows.len() {
            match self.windows[i].w_type {
                Items::Window => {
                     if tiled_count < 16 {
                        tiled_windows[tiled_count] = self.windows[i].id;
                        tiled_count += 1;
                     }
                },
                Items::Bar => {
                     bar_height = self.windows[i].height;
                     if self.windows[i].y > 0 {
                         bar_position_top = false;
                     }
                }
                _ => {}
            }
        }

        if tiled_count == 0 { return; }

        let gap = 5;
        let outer_gap = 5;
        
        let (screen_w, screen_h) = unsafe {
            ((*(&raw mut DISPLAY_SERVER)).width as usize, (*(&raw mut DISPLAY_SERVER)).height as usize)
        };

        let mut work_x = outer_gap;
        let mut work_y = outer_gap;
        let mut work_w = screen_w.saturating_sub(outer_gap * 2);
        let mut work_h = screen_h.saturating_sub(outer_gap * 2);

        if bar_height > 0 {
            if bar_position_top {
                work_y += bar_height;
                work_h = work_h.saturating_sub(bar_height);
            } else {
                work_h = work_h.saturating_sub(bar_height);
            }
        }

        debugln!("Tiling: Count={}, Work={}x{} @ {},{}", tiled_count, work_w, work_h, work_x, work_y);

        let count = tiled_count;
        
        for i in 0..count {
            let wid = tiled_windows[i];
            
            let (tx, ty, tw, th) = if count == 1 {
                (work_x, work_y, work_w.max(1), work_h.max(1))
            } else {
                let master_width = (work_w / 2).max(1);
                let stack_width = work_w.saturating_sub(master_width);
                
                if i == 0 {
                    let safe_w = (master_width.saturating_sub(gap / 2)).max(1);
                    (work_x, work_y, safe_w, work_h.max(1))
                } else {
                    let stack_count = count - 1;
                    let stack_index = i - 1;
                    let stack_h = work_h / stack_count;
                    let this_h = if stack_index == stack_count - 1 {
                        work_h - (stack_h * (stack_count - 1))
                    } else {
                        stack_h
                    };
                    
                    let sx = work_x + master_width + (gap / 2);
                    let sy = work_y + (stack_index * stack_h);
                    
                    let (final_sy, final_h) = if stack_count > 1 {
                        if stack_index == 0 {
                            (sy, this_h.saturating_sub(gap/2))
                        } else if stack_index == stack_count - 1 {
                            (sy + gap/2, this_h.saturating_sub(gap/2))
                        } else {
                            (sy + gap/2, this_h.saturating_sub(gap))
                        }
                    } else {
                        (sy, this_h)
                    };

                    // Ensure dimensions are at least 1x1
                    let safe_w = (stack_width.saturating_sub(gap / 2)).max(1);
                    let safe_h = final_h.max(1);

                    (sx, final_sy, safe_w, safe_h)
                }
            };

            debugln!("  Win {}: ID={} -> {}x{} @ {},{}", i, wid, tw, th, tx, ty);

            // Apply to window
            let mut win_idx = None;
            for idx in 0..self.windows.len() {
                if self.windows[idx].id == wid {
                    win_idx = Some(idx);
                    break;
                }
            }

            if let Some(idx) = win_idx {
                debugln!("    Applying to idx {}", idx);
                let current_w = self.windows[idx].width;
                let current_h = self.windows[idx].height;

                // Update Position Only
                self.windows[idx].x = tx as isize;
                self.windows[idx].y = ty as isize;
                self.windows[idx].can_move = false;
                self.windows[idx].can_resize = false;

                // Send Event if size TARGET changed
                if current_w != tw || current_h != th {
                    debugln!("    Size changed ({}x{} -> {}x{}), sending event...", current_w, current_h, tw, th);
                    debugln!("    Acquiring Event Queue Lock...");
                    // let mut queue = GLOBAL_EVENT_QUEUE.lock();
                    debugln!("    Lock acquired. Adding event...");
                    // queue.add_event(Event::Resize(ResizeEvent {
                    //     wid: wid as u32,
                    //     width: tw,
                    //     height: th,
                    // }));
                    debugln!("    Event added. Lock dropping...");
                    // drop(queue);
                    debugln!("    Event sent (SKIPPED).");
                }
            } else {
                debugln!("    Window ID {} not found!", wid);
            }
        }
        
        
        // Batch Redraw: Update the entire tiling workspace once.
        // This clears the background and redraws all windows at their new positions.
        self.update_window_area_rect(work_x as i32, work_y as i32, work_w as u32, work_h as u32);
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
        
        // Determine initial Z
        if wtype == Items::Wallpaper {
            w.z = 255;
        } else if wtype == Items::Bar || wtype == Items::Popup {
            w.z = 0;
        } else {
            w.z = 1; // Standard windows start behind system bars
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
            if self.windows[i].id == w.id { continue; }

            match self.windows[i].w_type {
                Items::Bar | Items::Popup => {
                    self.windows[i].z = 0; // Keep system bars at top
                },
                Items::Null => {},
                _ => {
                    if wtype == Items::Bar || wtype == Items::Popup {
                        if self.windows[i].z == 0 { self.windows[i].z = 1; }
                    } else {
                         self.windows[i].z = self.windows[i].z.saturating_add(1);
                    }
                }
            }
        }

        self.windows.sort_by_key(|w| w.z);
        debugln!("add_window: sorted, updating tiling...");
        self.update_tiling();
        debugln!("add_window: tiling updated, returning ID.");
        w.id
    }

    pub fn resize_window(&mut self, w: Window) {
        for i in 0..self.windows.len() {
            if w.id == self.windows[i].id {
                let old_w = self.windows[i].width;
                let old_h = self.windows[i].height;
                
                // Update Size and Buffer
                self.windows[i].width = w.width;
                self.windows[i].height = w.height;
                self.windows[i].buffer = w.buffer;
                
                // For Tiled windows, IGNORE position from user/app to prevent snapping
                if self.windows[i].w_type != Items::Window {
                    self.windows[i].x = w.x;
                    self.windows[i].y = w.y;
                    self.windows[i].can_move = w.can_move;
                } else {
                    self.windows[i].can_move = false;
                    // Keep existing X/Y set by update_tiling
                }

                // Redraw
                let current_x = self.windows[i].x;
                let current_y = self.windows[i].y;
                let u_max_x = (current_x + old_w as isize).max(current_x + w.width as isize);
                let u_max_y = (current_y + old_h as isize).max(current_y + w.height as isize);
                
                let dirty_w = (u_max_x - current_x).max(0) as u32;
                let dirty_h = (u_max_y - current_y).max(0) as u32;

                if dirty_w > 0 && dirty_h > 0 {
                    self.update_window_area_rect(current_x as i32, current_y as i32, dirty_w, dirty_h);
                }
            }
        }
    }

    pub fn update_window_area_rect(&mut self, dirty_x: i32, dirty_y: i32, dirty_w: u32, dirty_h: u32) {
        unsafe {
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            
            // 1. Clear the area in the double buffer (fill with black or background)
            if display_server.double_buffer != 0 {
                let db_ptr = display_server.double_buffer as *mut u32;
                let pitch_u32 = (display_server.pitch / 4) as usize;
                let height = display_server.height as i32;
                let width = display_server.width as i32;

                let start_x = dirty_x.max(0);
                let start_y = dirty_y.max(0);
                let end_x = (dirty_x + dirty_w as i32).min(width);
                let end_y = (dirty_y + dirty_h as i32).min(height);

                if end_x > start_x && end_y > start_y {
                    for y in start_y..end_y {
                        let row_offset = y as usize * pitch_u32;
                        let start_ptr = db_ptr.add(row_offset + start_x as usize);
                        let count = (end_x - start_x) as usize;
                        // Fill with black (0xFF000000 for opaque black) or 0 for transparent/black
                        core::ptr::write_bytes(start_ptr, 0, count);
                    }
                }
            }
            
            // 2. Redraw all windows overlapping this rect
            for i in (0..self.windows.len()).rev() {
                match self.windows[i].w_type {
                    Items::Null => {}
                    _ => {
                        let w = &self.windows[i];
                        display_server.copy_to_db_clipped(
                            w.width as u32,
                            w.height as u32,
                            w.buffer,
                            w.x as i32,
                            w.y as i32,
                            dirty_x, dirty_y, dirty_w, dirty_h
                        );
                    }
                }
            }

            // 3. Present
            display_server.present_rect(dirty_x, dirty_y, dirty_w, dirty_h);
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
                                self.windows[i].x as i32,
                                self.windows[i].y as i32,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn update_window_area(&mut self, id: usize) {
        let (dirty_x, dirty_y, dirty_w, dirty_h) = {
            let mut found = None;
            for i in 0..self.windows.len() {
                if self.windows[i].id == id {
                    let w = &self.windows[i];
                    found = Some((w.x as i32, w.y as i32, w.width as u32, w.height as u32));
                    break;
                }
            }
            match found {
                Some(rect) => rect,
                None => return,
            }
        };

        unsafe {
            let display_server = &mut *(&raw mut DISPLAY_SERVER);
            
            if display_server.double_buffer != 0 {
                let db_ptr = display_server.double_buffer as *mut u32;
                let pitch_u32 = (display_server.pitch / 4) as usize;
                let height = display_server.height as i32;
                let width = display_server.width as i32;

                let start_x = dirty_x.max(0);
                let start_y = dirty_y.max(0);
                let end_x = (dirty_x + dirty_w as i32).min(width);
                let end_y = (dirty_y + dirty_h as i32).min(height);

                if end_x > start_x && end_y > start_y {
                    for y in start_y..end_y {
                        let row_offset = y as usize * pitch_u32;
                        let _start_ptr = db_ptr.add(row_offset + start_x as usize);
                        let _count = (end_x - start_x) as usize;
                        // core::ptr::write_bytes(start_ptr, 0, count * 4);
                    }
                }
            }
            
            for i in (0..self.windows.len()).rev() {
                match self.windows[i].w_type {
                    Items::Null => {}
                    _ => {
                        let w = &self.windows[i];
                        display_server.copy_to_db_clipped(
                            w.width as u32,
                            w.height as u32,
                            w.buffer,
                            w.x as i32,
                            w.y as i32,
                            dirty_x, dirty_y, dirty_w, dirty_h
                        );
                    }
                }
            }

            display_server.present_rect(dirty_x, dirty_y, dirty_w, dirty_h);
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
                            self.windows[j].x as i32,
                            self.windows[j].y as i32,
                        );
                    }
                }
            }

            (*(&raw mut DISPLAY_SERVER)).copy();
        }
        self.update_tiling();
    }
}