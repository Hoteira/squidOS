use crate::display::{Display, FlexDirection};
use crate::gui::{Align, Color, Size};
use crate::widget::Widget;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use libk::syscall::Items;
use libk::{print, println, syscall};
use crate::{window, FrameBuffer, WidgetId};

pub struct Window {
    pub id: usize,
    pub title: String,
    pub buffer: FrameBuffer,

    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,

    pub children: Vec<Widget>,

    pub status_bar: bool,
    pub can_move: bool,
    pub can_resize: bool,
    pub min_width: usize,
    pub min_height: usize,

    pub w_type: Items,
    pub focus: WidgetId,
}

impl Window {
    pub fn new(title: &str, width: usize, height: usize) -> Self {
        Window {
            id: 0,
            title: String::from(title),
            buffer: FrameBuffer::new(width * height * 4 + 4),

            x: 0,
            y: 0,
            width: width,
            height: height,

            children: Vec::new(),

            status_bar: false,
            can_move: true,
            can_resize: true,
            min_width: 0,
            min_height: 0,

            w_type: Items::Window,
            focus: 0,
        }
    }

    pub fn set_type(&mut self, w_type: Items) {
        self.w_type = w_type;
    }

    pub fn new_with_action_bar(title: &str, width: usize, height: usize) -> Self {
        let mut window = Window {
            id: 0,
            title: String::from(title),
            buffer: FrameBuffer::new(width * height * 4 + 4),

            x: 0,
            y: 0,
            width: width,
            height: height,

            children: Vec::new(),

            status_bar: false,
            can_move: true,
            can_resize: true,
            min_width: 0,
            min_height: 0,

            w_type: Items::Window,
            focus: 0xF003,
        };

        let b1 = Widget::button(0xF000, "\u{200A}x")
            .set_text_color(Color::rgb(0, 0, 0))
            .set_text_size(12)
            .width(Size::Absolute(20))
            .height(Size::Absolute(20))
            .y(Size::Absolute(3))
            .x(Size::FromRight(22))
            .background_color(Color::rgb(255, 0 ,0))
            .on_click(exit_fn);

        let b2 = Widget::button(0xF001, "[]")
            .width(Size::Absolute(20))
            .height(Size::Absolute(20))
            .y(Size::Absolute(3))
            .x(Size::FromRight(45))
            .background_color(Color::rgb(255, 0 ,0))
            .on_click(maximise_fn);

        let b3 = Widget::button(0xF002, "\u{200A}_")
            .width(Size::Absolute(20))
            .height(Size::Absolute(20))
            .y(Size::Absolute(3))
            .x(Size::FromRight(67))
            .background_color(Color::rgb(255, 0 ,0))
            .on_click(minimise_fn);

        let l = Widget::label(0xF003, title)
            .set_text_align(Align::Left)
            .width(Size::Relative(100))
            .height(Size::Relative(100))
            .background_color(Color::rgb(255, 0 ,255));

        // Add your content frame
        let f = Widget::frame(0xF004)
            .width(Size::Relative(100))
            .height(Size::Absolute(26))
            .background_color(Color::rgb(0, 255, 255))
            .add_child(l)
            .add_child(b1)
            .add_child(b2)
            .add_child(b3);

        window.children.push(f);

        window
    }

    pub fn draw(&mut self) {
        crate::font_manager::FontManager::init();

        for child in &mut self.children {
            let buffer = unsafe {
                &mut core::slice::from_raw_parts_mut(
                    self.buffer.address,
                    self.width * self.height + 1
                )
            };

            // Start recursive drawing from the window's bounds
            draw_recursive(buffer, self.width, child, 0, 0, self.width, self.height, 0, 0);
        }
    }

    pub fn render(&mut self) {
        let w2 = libk::syscall::Window {
            id: 0,
            buffer: self.buffer.address as u32,
            x: self.x,
            y: self.y,
            z: 0,
            width: self.width,
            height: self.height,
            can_move: self.can_move,
            can_resize: self.can_resize,
            min_width: self.min_width,
            min_height: self.min_height,
            event_handler: 1,
            w_type: self.w_type,
        };

        self.id = libk::syscall::add_window(w2) as usize;
    }

    pub fn resize(&mut self, width: usize, height: usize, can_move: bool) {

        if self.can_resize == false { return; }
        self.width = width;
        self.height = height;
        self.buffer.resize(width * height * 4 + 4);

        self.draw();

        let w2 = libk::syscall::Window {
            id: self.id,
            buffer: self.buffer.address as u32,
            x: self.x,
            y: self.y,
            z: 0,
            width: self.width,
            height: self.height,
            can_move: can_move,
            can_resize: self.can_resize,
            min_width: self.min_width,
            min_height: self.min_height,
            event_handler: 1,
            w_type: self.w_type,
        };

        libk::syscall::syscall(51, &w2 as *const _ as u32, 0, 0);

    }
}

pub fn draw_recursive(
    buffer: &mut [u32],
    width0: usize,
    widget: &mut Widget,
    parent_x: usize,
    parent_y: usize,
    parent_width: usize,
    parent_height: usize,
    parent_padding: usize,
    parent_margin: usize,
) {
    // First update this widget's layout
    widget.update_layout(parent_x, parent_y, parent_width, parent_height, parent_padding, &Display::None);

    // Draw the widget
    widget.draw(buffer, width0);

    let widget_x = widget.get_x();
    let widget_y = widget.get_y();
    let widget_width = widget.get_width();
    let widget_height = widget.get_height();
    let widget_padding = widget.get_padding();
    let widget_margin = widget.get_margin();
    let display = widget.get_display();

    // Handle children if this is a Frame
    if let Some(children) = widget.get_children_mut() {
        // Get this widget's properties for positioning children

        match display {
            Display::Flex { direction, wrap } => {
                // Handle flex layout - all children need to be processed together
                let content_x = widget_x + widget_padding;
                let content_y = widget_y + widget_padding;
                let content_width = widget_width.saturating_sub(widget_padding * 2);
                let content_height = widget_height.saturating_sub(widget_padding * 2);

                // First pass: calculate all child sizes
                let mut child_info = Vec::new();
                for child in children.iter_mut() {
                    // Give child generous bounds to calculate preferred size
                    child.update_layout(content_x, content_y, content_width, content_height, 0, &Display::None);
                    let child_geom = child.geometry();
                    let total_width = child_geom.width + child_geom.margin * 2;
                    let total_height = child_geom.height + child_geom.margin * 2;
                    child_info.push((total_width, total_height));
                }

                // Second pass: position children
                let mut current_x = 0;
                let mut current_y = 0;
                let mut line_height = 0;
                let mut line_width = 0;

                for (i, child) in children.iter_mut().enumerate() {
                    let (child_total_width, child_total_height) = child_info[i];

                    let (child_x, child_y, child_w, child_h) = match direction {
                        FlexDirection::Row => {
                            // Check wrapping
                            if wrap && current_x + child_total_width > content_width && current_x > 0 {
                                current_x = 0;
                                current_y += line_height;
                                line_height = 0;
                            }

                            let x = content_x + current_x;
                            let y = content_y + current_y;
                            let w = child_total_width.min(content_width - current_x);
                            let h = child_total_height.min(content_height - current_y);

                            current_x += child_total_width;
                            line_height = line_height.max(child_total_height);

                            (x, y, w, h)
                        }
                        FlexDirection::Column => {
                            // Check wrapping
                            if wrap && current_y + child_total_height > content_height && current_y > 0 {
                                current_y = 0;
                                current_x += line_width;
                                line_width = 0;
                            }

                            let x = content_x + current_x;
                            let y = content_y + current_y;
                            let w = child_total_width.min(content_width - current_x);
                            let h = child_total_height.min(content_height - current_y);

                            current_y += child_total_height;
                            line_width = line_width.max(child_total_width);

                            (x, y, w, h)
                        }
                    };

                    // Recursively draw child in its allocated space
                    draw_recursive(buffer, width0, child, child_x, child_y, child_w, child_h, 0, widget_margin);
                }
            }

            Display::Grid { rows, cols } => {
                if rows > 0 && cols > 0 {
                    let content_width = widget_width.saturating_sub(widget_padding * 2);
                    let content_height = widget_height.saturating_sub(widget_padding * 2);
                    let cell_width = content_width / cols;
                    let cell_height = content_height / rows;

                    for (i, child) in children.iter_mut().enumerate() {
                        let row = i / cols;
                        let col = i % cols;
                        if row >= rows {
                            break;
                        }

                        let child_x = widget_x + widget_padding + col * cell_width;
                        let child_y = widget_y + widget_padding + row * cell_height;

                        draw_recursive(buffer, width0, child, child_x, child_y, cell_width, cell_height, 0, widget_margin);
                    }
                }
            }

            Display::None => {
                // Free positioning - each child can position itself
                let content_x = widget_x + widget_padding;
                let content_y = widget_y + widget_padding;
                let content_width = widget_width.saturating_sub(widget_padding * 2);
                let content_height = widget_height.saturating_sub(widget_padding * 2);

                for child in children.iter_mut() {
                    draw_recursive(buffer, width0, child, content_x, content_y, content_width, content_height, 0, widget_margin);
                }
            }
        }
    }
}

pub fn exit_fn(w: &mut Window, id: WidgetId) {
    println!("Exit");

    crate::font_manager::FontManager::cleanup();

    for widget in w.children.iter() {
        match widget {
            Widget::Image{ image_ptr, .. } => { libk::syscall::free(*image_ptr as u32); },
            _ => {}
        }
    }

    libk::syscall::remove_window(w.id as u32);
    libk::syscall::free(w.buffer.address as u32);
}

pub fn maximise_fn(w: &mut Window, id: WidgetId) {
    println!("Maximise");
    libk::syscall::move_window(w.id as u32, 0, 0);
    let screen_width = libk::syscall::get_screen_width() as usize;
    let screen_height = libk::syscall::get_screen_height() as usize;

    if w.width == screen_width && w.height == screen_height {
        w.resize(200, 200, true);
    } else {
        w.resize(screen_width, screen_height, false);
    }
}

pub fn minimise_fn(w: &mut Window, id: WidgetId) {
    println!("Minimise");
    w.resize(0, 0, w.can_move);
}

impl Window {
    pub fn find_interactive_widget_at(&self, x: usize, y: usize) -> Option<&Widget> {
        for child in &self.children {
            if let Some(widget) = find_interactive_widget_recursive(child, x, y) {
                return Some(widget);
            }
        }
        None
    }

    pub fn find_interactive_widget_id_at(&self, x: usize, y: usize) -> Option<crate::WidgetId> {
        self.find_interactive_widget_at(x, y)
            .map(|widget| widget.geometry().id)
    }

    pub fn find_widget_by_id(&mut self, id: WidgetId) -> Option<&mut Widget> {
        for child in self.children.iter_mut() {
            if let Some(found) = child.find_widget_by_id_mut(id) {
                return Some(found);
            }
        }
        None
    }
}

fn find_interactive_widget_recursive(widget: &Widget, x: usize, y: usize) -> Option<&Widget> {
    let geometry = widget.geometry();

    if x < geometry.x || x >= geometry.x + geometry.width ||
        y < geometry.y || y >= geometry.y + geometry.height {
        return None;
    }

    if let Some(children) = widget.get_children() {
        for child in children.iter().rev() {
            if let Some(found_widget) = find_interactive_widget_recursive(child, x, y) {
                return Some(found_widget);
            }
        }

        None
    } else {

        Some(widget)
    }
}