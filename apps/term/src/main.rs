#![no_std]
#![no_main]

extern crate alloc;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkui::{Color, Size, Widget, Window};
use std::fs::File;

static mut TERM_READ_FD: usize = 0;
static mut TERM_WRITE_FD: usize = 0;

struct TerminalBuffer {
    lines: Vec<Vec<char>>,
    alt_lines: Vec<Vec<char>>,
    is_alt: bool,
    cursor_row: usize,
    cursor_col: usize,
    cursor_visible: bool,
}

impl TerminalBuffer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            alt_lines: Vec::new(),
            is_alt: false,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
        }
    }

    fn clear(&mut self) {
        if self.is_alt {
            self.alt_lines.clear();
        } else {
            self.lines.clear();
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    fn ensure_row(&mut self) {
        let current = if self.is_alt { &mut self.alt_lines } else { &mut self.lines };
        while current.len() <= self.cursor_row {
            current.push(Vec::new());
        }
    }

    fn write_char(&mut self, c: char) {
        /*
        if self.cursor_row < 5 && self.cursor_col < 10 {
             std::debugln!("[term] write_char('{}') at {},{}", c, self.cursor_row, self.cursor_col);
        }
        */
        self.ensure_row();
        let current = if self.is_alt { &mut self.alt_lines } else { &mut self.lines };
        let line = &mut current[self.cursor_row];
        
        while line.len() <= self.cursor_col {
            line.push(' ');
        }
        line[self.cursor_col] = c;
        self.cursor_col += 1;
    }

    fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\x1B' {
                // Should not happen if filtered correctly
                continue;
            }
            self.write_char(c);
        }
    }

    fn newline(&mut self) {
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn clear_line(&mut self) {
        let current = if self.is_alt { &mut self.alt_lines } else { &mut self.lines };
        if self.cursor_row < current.len() {
            current[self.cursor_row].truncate(self.cursor_col);
        }
    }

    fn render(&self) -> String {
        let current = if self.is_alt { &self.alt_lines } else { &self.lines };

        let mut s = String::new();
        for (i, line) in current.iter().enumerate() {
            if i > 0 { s.push('\n'); }
            for &c in line {
                s.push(c);
            }
        }

        if self.cursor_visible {
            // Placeholder for cursor position if we had one in the string
            // But label widget renders everything. 
            // We'll just stick to the simple render for now.
        }
        s
    }

    fn switch_screen(&mut self, alt: bool) {
        if self.is_alt != alt {
            self.is_alt = alt;
            if self.is_alt {
                self.alt_lines.clear();
                self.cursor_row = 0;
                self.cursor_col = 0;
            } else {
                self.cursor_row = self.lines.len().saturating_sub(1);
                self.cursor_col = 0;
            }
        }
    }
}

fn update_term_size(win: &Window) {
    if let Some(widget) = win.find_widget_by_id(2) {
        if let inkui::widget::Widget::Label { text, geometry, .. } = widget {
            let padding = 10;
            let width = geometry.width.saturating_sub(padding * 2);
            let height = geometry.height.saturating_sub(padding * 2);

                                            let char_width = (text.size as f32 * 1.0) as usize;            let line_height = (text.size as f32 * 1.2) as usize;

            if char_width > 0 && line_height > 0 {
                let cols = (width / char_width) as u16;
                let rows = ((height / line_height) as u16).saturating_sub(1);
                
                let ws = std::os::WinSize {
                    ws_row: rows,
                    ws_col: cols,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                
                std::os::ioctl(0, std::os::TIOCSWINSZ, &ws as *const _ as u64);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let width = 800;
    let height = 400;

    // Manually calculate and set initial terminal size so spawned shell inherits it
    let avail_w = width - (width * 4 / 100);
    let avail_h = height - (height * 5 / 100);
    let font_size = 12.0f32;
    let char_w = (font_size * 0.8) as usize;
    let line_h = (font_size * 1.5) as usize;

    if char_w > 0 && line_h > 0 {
        let cols = (avail_w / char_w) as u16;
        let rows = (avail_h / line_h) as u16;
        let rows = rows.saturating_sub(1); // Subtract 1 row to account for extra margin in scroll logic

        let ws = std::os::WinSize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        std::os::ioctl(0, std::os::TIOCSWINSZ, &ws as *const _ as u64);
    }

    let screen_w = std::graphics::get_screen_width();
    let screen_h = std::graphics::get_screen_height();
    let x = (screen_w / 2).saturating_sub(width / 2);
    let y = (screen_h / 2).saturating_sub(height / 2);

    let mut win = Window::new("krakeOS Term", width, height);
    win.x = x as isize;
    win.y = y as isize;

    {
        if let Ok(mut file) = File::open("@0xE0/sys/fonts/CaskaydiaNerd.ttf") {
            let size = file.size();
            let buffer_addr = std::memory::malloc(size);
            let buffer = unsafe { core::slice::from_raw_parts_mut(buffer_addr as *mut u8, size) };
            if file.read(buffer).is_ok() {
                let static_buf = unsafe { core::slice::from_raw_parts(buffer_addr as *const u8, size) };
                win.load_font(static_buf);
            }
        }
    }


    let mut fds_out = [0i32; 2];
    std::os::pipe(&mut fds_out);
    unsafe { TERM_READ_FD = fds_out[0] as usize; }

    let mut fds_in = [0i32; 2];
    std::os::pipe(&mut fds_in);
    unsafe { TERM_WRITE_FD = fds_in[1] as usize; }

    let fds_map = [
        (0, fds_in[0] as u8),
        (1, fds_out[1] as u8),
        (2, fds_out[1] as u8),
    ];

    std::os::spawn_with_fds("@0xE0/sys/bin/shell.elf", &[], &fds_map);


    std::os::file_close(fds_in[0] as usize);
    std::os::file_close(fds_out[1] as usize);


    let mut root = Widget::frame(1)
        .width(Size::Relative(100))
        .height(Size::Relative(100))
        .background_color(Color::rgba(0, 0, 0, 164));

    let term_display = Widget::label(2, "")
        .width(Size::Relative(92))
        .height(Size::Relative(90))
        .x(Size::Relative(4))
        .y(Size::Relative(5))
        .padding(Size::Absolute(10))
        .set_text_color(Color::rgb(255, 255, 255))
        .set_text_size(14.0)
        .background_color(Color::rgba(0, 0, 0, 0));

    root = root.add_child(term_display);
    win.children.push(root);
    update_term_size(&win);
    win.show();
    win.draw();
    win.update();

    let mut term_buffer = TerminalBuffer::new();
    let mut pipe_buf = [0u8; 4096];

    loop {
        use inkui::Event;
        let mut events: [Event; 16] = [Event::None; 16];
        unsafe {
            std::os::syscall(104, win.id as u64, events.as_mut_ptr() as u64, 16);
        }

        for event in events.iter() {
            match event {
                Event::Keyboard(e) => {
                    if e.pressed {
                        if let Some(c) = core::char::from_u32(e.key) {
                            for _ in 0..e.repeat {
                                let mut buf = [0u8; 4];
                                let s = c.encode_utf8(&mut buf);
                                std::os::file_write(unsafe { TERM_WRITE_FD }, s.as_bytes());
                            }
                        } else {
                            // Special keys
                            let seq = match e.key {
                                0x110003 => Some("\x1B[A"), // Up
                                0x110004 => Some("\x1B[B"), // Down
                                0x110002 => Some("\x1B[C"), // Right
                                0x110001 => Some("\x1B[D"), // Left
                                0x110007 => None, // Shift
                                0x110005 => None, // Ctrl
                                0x110006 => None, // Alt
                                _ => None,
                            };
                            if let Some(s) = seq {
                                for _ in 0..e.repeat {
                                    std::os::file_write(unsafe { TERM_WRITE_FD }, s.as_bytes());
                                }
                            }
                        }
                    }
                }
                Event::Mouse(e) => {
                    if e.scroll != 0 {
                        if let Some(widget) = win.find_widget_by_id_mut(2) {
                            widget.handle_scroll(e.scroll);
                            win.draw();
                            win.update();
                        }
                    }
                }
                Event::Resize(e) => {
                    win.resize(e.width, e.height, true);
                    update_term_size(&win);
                    win.draw();
                    win.update();
                }
                _ => {}
            }
        }

        let old_cursor_row = term_buffer.cursor_row;
        let mut old_scroll_y = 0;
        if let Some(w) = win.find_widget_by_id(2) {
            old_scroll_y = w.geometry().scroll_offset_y;
        }

        let n = std::os::file_read(unsafe { TERM_READ_FD }, &mut pipe_buf);
        if n > 0 && n != usize::MAX {
            let mut i = 0;
            let mut has_newline = false;
            while i < n {
                // ... (processing loop remains unchanged) ...
                let b = pipe_buf[i];
                if b == 0x08 {
                    term_buffer.backspace();
                    i += 1;
                } else if b == b'\r' {
                    term_buffer.cursor_col = 0;
                    i += 1;
                } else if b == b'\n' {
                    term_buffer.newline();
                    has_newline = true;
                    i += 1;
                } else if b == 0x1B {
                    if i + 1 < n && pipe_buf[i + 1] == b'[' {
                        let mut j = i + 2;
                        let mut end_found = false;
                        while j < n && j < i + 32 {
                            let c = pipe_buf[j];
                            if c >= 0x40 && c <= 0x7E {
                                end_found = true;
                                break;
                            }
                            j += 1;
                        }

                        if end_found {
                            let cmd = pipe_buf[j];
                            let seq = &pipe_buf[i + 2..j];

                            match cmd {
                                b'A' => { // Up
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_row = term_buffer.cursor_row.saturating_sub(n);
                                }
                                b'B' => { // Down
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_row += n;
                                }
                                b'C' => { // Forward
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_col += n;
                                }
                                b'D' => { // Backward
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_col = term_buffer.cursor_col.saturating_sub(n);
                                }
                                b'G' => { // Horizontal Absolute
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_col = n.saturating_sub(1);
                                }
                                b'J' => {
                                    if seq == b"2" {
                                        term_buffer.clear();
                                    }
                                }
                                b'H' => {
                                    if seq.is_empty() {
                                        term_buffer.cursor_row = 0;
                                        term_buffer.cursor_col = 0;
                                    } else {
                                        let s = unsafe { core::str::from_utf8_unchecked(seq) };
                                        let parts: Vec<&str> = s.split(';').collect();
                                        if parts.len() >= 2 {
                                            if let Ok(r) = parts[0].parse::<usize>() {
                                                term_buffer.cursor_row = r.saturating_sub(1);
                                            }
                                            if let Ok(c) = parts[1].parse::<usize>() {
                                                term_buffer.cursor_col = c.saturating_sub(1);
                                            }
                                        } else if !parts.is_empty() {
                                            if let Ok(r) = parts[0].parse::<usize>() {
                                                term_buffer.cursor_row = r.saturating_sub(1);
                                            }
                                            term_buffer.cursor_col = 0;
                                        }
                                    }
                                }
                                b'd' => { // Vertical Absolute
                                    let n = if seq.is_empty() { 1 } else { unsafe { core::str::from_utf8_unchecked(seq) }.parse::<usize>().unwrap_or(1) };
                                    term_buffer.cursor_row = n.saturating_sub(1);
                                }
                                b'K' => {
                                    if seq == b"1" { // Start to cursor
                                        let current = if term_buffer.is_alt { &mut term_buffer.alt_lines } else { &mut term_buffer.lines };
                                        if term_buffer.cursor_row < current.len() {
                                            for i in 0..core::cmp::min(term_buffer.cursor_col + 1, current[term_buffer.cursor_row].len()) {
                                                current[term_buffer.cursor_row][i] = ' ';
                                            }
                                        }
                                    } else if seq == b"2" { // Whole line
                                        let current = if term_buffer.is_alt { &mut term_buffer.alt_lines } else { &mut term_buffer.lines };
                                        if term_buffer.cursor_row < current.len() {
                                            current[term_buffer.cursor_row].clear();
                                        }
                                    } else { // Cursor to end (0 or empty)
                                        term_buffer.clear_line();
                                    }
                                }
                                b'm' => {
                                    let seq_full = unsafe { core::str::from_utf8_unchecked(&pipe_buf[i..j + 1]) };
                                    term_buffer.write_str(seq_full);
                                }
                                b'h' => {
                                    if seq.len() > 1 && seq[0] == b'?' {
                                        let param = unsafe { core::str::from_utf8_unchecked(&seq[1..]) };
                                        if param == "25" {
                                            term_buffer.cursor_visible = true;
                                        } else if param == "1049" {
                                            term_buffer.switch_screen(true);
                                        }
                                    }
                                }
                                b'l' => {
                                    if seq.len() > 1 && seq[0] == b'?' {
                                        let param = unsafe { core::str::from_utf8_unchecked(&seq[1..]) };
                                        if param == "25" {
                                            term_buffer.cursor_visible = false;
                                        } else if param == "1049" {
                                            term_buffer.switch_screen(false);
                                        }
                                    }
                                }
                                b't' => {
                                    if seq == b"18" {
                                        if let Some(widget) = win.find_widget_by_id(2) {
                                            if let inkui::widget::Widget::Label { text, geometry, .. } = widget {
                                                let padding = 10;
                                                let width = geometry.width.saturating_sub(padding * 2);
                                                let height = geometry.height.saturating_sub(padding * 2);

                                                let char_width = (text.size as f32 * 0.8) as usize;
                                                let line_height = (text.size as f32 * 1.2) as usize;

                                                if char_width > 0 && line_height > 0 {
                                                    let cols = width / char_width;
                                                    let rows = height / line_height;
                                                    let resp = format!("\x1B[8;{};{}t", rows, cols);
                                                    std::os::file_write(unsafe { TERM_WRITE_FD }, resp.as_bytes());
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                            i = j + 1;
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                } else {
                    let mut len = 1;
                    if (b & 0xE0) == 0xC0 { len = 2; } else if (b & 0xF0) == 0xE0 { len = 3; } else if (b & 0xF8) == 0xF0 { len = 4; }

                    if i + len <= n {
                        let s = unsafe { core::str::from_utf8_unchecked(&pipe_buf[i..i + len]) };
                        term_buffer.write_str(s);
                        i += len;
                    } else {
                        i += 1;
                    }
                }
            }

            if let Some(widget) = win.find_widget_by_id_mut(2) {
                if let inkui::widget::Widget::Label { text, geometry, .. } = widget {
                    text.text = term_buffer.render();

                    if term_buffer.is_alt {
                        geometry.scroll_offset_y = 0;
                    } else {
                        let padding = 10;
                        let width = geometry.width.saturating_sub(padding * 2);
                        let height = geometry.height.saturating_sub(padding * 2);

                        if width > 0 {
                                                            let char_width = (text.size as f32 * 1.0) as usize;                            if char_width > 0 {
                                let chars_per_line = width / char_width;
                                let mut visual_lines = 0;

                                let current_lines = &term_buffer.lines;
                                for line in current_lines {
                                    let len = line.len();
                                    if len == 0 {
                                        visual_lines += 1;
                                    } else {
                                        visual_lines += (len + chars_per_line - 1) / chars_per_line;
                                    }
                                }

                                let line_height = (text.size as f32 * 1.2) as usize;
                                let content_height = visual_lines * line_height;
                                let extra_margin = line_height;

                                if content_height + extra_margin > height {
                                    geometry.scroll_offset_y = (content_height + extra_margin) - height;
                                } else {
                                    geometry.scroll_offset_y = 0;
                                }
                            }
                        }
                    }
                }
            }
            win.draw();
            win.update();
        }

        std::os::yield_task();
    }
}
