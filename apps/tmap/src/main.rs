#![no_std]
#![no_main]

use std::{fs, println};
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::ToString;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const STDIN_FD: usize = 0;
const STDOUT_FD: usize = 1;

struct AppState {
    current_path: String,
    entries: Vec<fs::DirEntry>,
    selected_index: usize,
}

impl AppState {
    fn new(start_path: &str) -> Self {
        let mut app = AppState {
            current_path: String::from(start_path),
            entries: Vec::new(),
            selected_index: 0,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.selected_index = 0;
        
        // Add parent directory option if not at root (assumed @0xE0 root for now)
        if self.current_path != "@0xE0" && self.current_path.len() > 5 { // simple check
             self.entries.push(fs::DirEntry { 
                 name: String::from(".."), 
                 file_type: fs::FileType::Directory 
             });
        }

        if let Ok(items) = fs::read_dir(&self.current_path) {
            for item in items {
                self.entries.push(item);
            }
        }
    }

    fn draw(&self) {
        // Clear screen
        std::os::file_write(STDOUT_FD, b"\x1B[2J\x1B[H");
        
        // Header
        let header = format!("\x1B[1;37;44m TMAP - {}\x1B[0m\n\n", self.current_path);
        std::os::file_write(STDOUT_FD, header.as_bytes());

        // List
        for (i, entry) in self.entries.iter().enumerate() {
            if i == self.selected_index {
                std::os::file_write(STDOUT_FD, b"\x1B[7m > "); // Invert colors
            } else {
                std::os::file_write(STDOUT_FD, b"   ");
            }

            if entry.file_type == fs::FileType::Directory {
                std::os::file_write(STDOUT_FD, "\x1B[1;94m\u{F07B} ".as_bytes()); // Blue Folder
            } else {
                std::os::file_write(STDOUT_FD, "\x1B[37m\u{F016} ".as_bytes()); // White File
            }
            
            std::os::file_write(STDOUT_FD, entry.name.as_bytes());
            std::os::file_write(STDOUT_FD, b"\x1B[0m\n");
        }
        
        // Footer help
        std::os::file_write(STDOUT_FD, b"\n\x1B[90m[W/S] Move  [Enter] Open  [Q] Quit\x1B[0m");
    }

    fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected_index + 1 < self.entries.len() {
            self.selected_index += 1;
        }
    }

    fn enter(&mut self) {
        if self.selected_index < self.entries.len() {
            let entry = &self.entries[self.selected_index];
            if entry.name == ".." {
                // Go up
                if let Some(last_slash) = self.current_path.rfind('/') {
                    self.current_path.truncate(last_slash);
                } else {
                    // Try to reset to root if weird path
                    self.current_path = String::from("@0xE0");
                }
                self.refresh();
            } else if entry.file_type == fs::FileType::Directory {
                // Go down
                if !self.current_path.ends_with('/') {
                    self.current_path.push('/');
                }
                self.current_path.push_str(&entry.name);
                self.refresh();
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let heap_size = 1024 * 1024 * 2; 
    let heap_ptr = std::memory::malloc(heap_size);
    std::memory::heap::init_heap(heap_ptr as *mut u8, heap_size);

    let mut app = AppState::new("@0xE0");
    let mut needs_redraw = true;
    
    // Enter Alternate Screen & Hide Cursor
    std::os::file_write(STDOUT_FD, b"\x1B[?1049h\x1B[?25l");
    
    loop {
        if needs_redraw {
            app.draw();
            needs_redraw = false;
        }

        let mut buf = [0u8; 1];
        if std::os::file_read(STDIN_FD, &mut buf) > 0 {
            let c = buf[0] as char;
            match c {
                'w' | 'W' => { app.move_up(); needs_redraw = true; },
                's' | 'S' => { app.move_down(); needs_redraw = true; },
                '\n' | '\r' => { app.enter(); needs_redraw = true; },
                'q' | 'Q' => {
                    // Exit Alternate Screen (restores main screen) & Show Cursor
                    std::os::file_write(STDOUT_FD, b"\x1B[?1049l\x1B[?25h");
                    std::os::exit(0);
                },
                _ => {}
            }
        } else {
            std::os::yield_task();
        }
    }
}