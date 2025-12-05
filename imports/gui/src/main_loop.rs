use alloc::string::{String, ToString};
use libk::{print, println};
use crate::event::Event;
use crate::window::Window;

impl Window {
    pub fn event_loop(&mut self) {
        let mut key_buffer = String::with_capacity(64);

        let mut events: [Event; 64] = [Event::None; 64];

        libk::syscall::poll_event(self.id as u32, events.as_mut_ptr() as u32);

        for event in events.iter() {
            match event {
                Event::Resize(e) => {
                    if e.width > 0 && e.height > 0 && self.can_resize {
                        self.resize(e.width, e.height, self.can_resize);
                    }
                },

                Event::Mouse(e) => {
                    let btn = self.find_interactive_widget_at(e.x, e.y);

                    if let Some(btn) = btn {
                        if let Some(handler) = btn.get_event_handler() {
                            let btn_id = btn.get_id();
                            handler(self, btn_id);
                        } else {
                            self.focus = btn.get_id();
                        }
                    }
                },

                Event::Keyboard(e) => {
                    let w = self.width;
                    let h = self.height;
                    let b = self.buffer.address;

                    match e.char {
                        '\x08' => { // Backspace
                            if key_buffer.len() > 0 {
                                key_buffer.pop();
                            } else if let Some(label) = self.find_widget_by_id(self.focus) {
                                let old_text = label.get_text().unwrap_or("").to_string();
                                label.pop_text_char();
                                let new_text = label.get_text().unwrap_or("");

                                let framebuffer = unsafe { core::slice::from_raw_parts_mut(b, w * h + 1) };

                                if old_text != new_text {
                                    label.draw(framebuffer, w);
                                    libk::syscall::syscall(41, self.id as u32, 0, 0);
                                }
                            }
                        }

                        _ => {
                            key_buffer.push(e.char);
                        }
                    }
                },

                _ => { break; },
            }
        }

        // Handle accumulated key input
        if !key_buffer.is_empty() {
            let w = self.width;
            let h = self.height;
            let b = self.buffer.address;

            if let Some(label) = self.find_widget_by_id(self.focus) {

                let old_text = label.get_text().unwrap_or("").to_string();
                label.append_text(&key_buffer);
                let new_text = label.get_text().unwrap_or("");

                let framebuffer = unsafe { core::slice::from_raw_parts_mut(b, w * h + 1) };

                if old_text != new_text {
                    label.draw(framebuffer, w);
                    libk::syscall::syscall(41, self.id as u32, 0, 0);
                }
            }
        }

        // Prevent busy waiting
        libk::syscall::thread_yield();
    }
}