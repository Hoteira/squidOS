use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::max;
use libk::{print, println};
use crate::display::{Display, FlexDirection};
use crate::gui::{Align, Color, Size, Text};
use crate::{ceil_f32, EventHandler, WidgetId};
use crate::widget::Widget::Frame;
use crate::window::Window;

#[derive(Debug, Clone)]
pub struct WidgetGeometry {
    pub id: WidgetId,

    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub margin: usize,
    pub padding: usize,

    pub user_x: Size,
    pub user_y: Size,
    pub user_width: Size,
    pub user_height: Size,
    pub user_margin: Size,
    pub user_padding: Size,

    pub border_color: Color,
    pub border_radius: Size,
    pub border_size: Size,
}

impl WidgetGeometry {
    pub fn new(id: WidgetId) -> Self {
        WidgetGeometry {
            id,

            x: 0,
            y: 0,
            width: 0,
            height: 0,
            margin: 0,
            padding: 0,

            user_x: Size::Absolute(0),
            user_y: Size::Absolute(0),
            user_width: Size::Absolute(100),
            user_height: Size::Absolute(30),
            user_margin: Size::Absolute(0),
            user_padding: Size::Absolute(0),

            border_color: Color::rgba(0, 0, 0,0 ),
            border_radius: Size::Absolute(0),
            border_size: Size::Absolute(0),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Widget {
    Frame {
        geometry: WidgetGeometry,
        children: Vec<Widget>,
        display: Display,
        background_color: Color,
    },

    Button {
        geometry: WidgetGeometry,
        text: Text,
        background_color: Color,
        on_click: Option<EventHandler>,
    },

    Label {
        geometry: WidgetGeometry,
        text: Text,
        background_color: Color,
        writable: bool,
    },

    Canvas {
        geometry: WidgetGeometry,
        framebuffer: Vec<u32>,
        background_color: Color,
    },

    Image {
        geometry: WidgetGeometry,
        image_path: String,
        loaded: bool,
        image_ptr: u32,
        image_size: usize,
    }


}


impl Widget {
    // Factory methods for creating widgets
    pub fn frame(id: WidgetId) -> Self {
        Widget::Frame {
            geometry: WidgetGeometry::new(id),
            children: Vec::new(),
            display: Display::None,
            background_color: Color::rgb(255, 255, 255),
        }
    }

    pub fn image(id: WidgetId, path: &str) -> Self {
        println!("Searching {}",path);
        let (ptr, size) = tga::load_tga_image(path);

        println!("{} -> {}", ptr, size);

        Widget::Image {
            geometry: WidgetGeometry::new(id),
            image_path: String::from(path),
            loaded: true,
            image_ptr: ptr,
            image_size: size,
        }
    }

    pub fn button(id: WidgetId, text: &str) -> Self {
        Widget::Button {
            geometry: WidgetGeometry::new(id),
            text: Text::new(text),
            background_color: Color::rgb(200, 200, 200),
            on_click: None,
        }
    }

    pub fn label(id: WidgetId, text: &str) -> Self {
        Widget::Label {
            geometry: WidgetGeometry::new(id),
            text: Text::new(text),
            background_color: Color::rgb(255, 255, 255),
            writable: false,
        }
    }

    pub fn canvas(id: WidgetId) -> Self {
        Widget::Canvas {
            geometry: WidgetGeometry::new(id),
            framebuffer: Vec::new(),
            background_color: Color::rgb(100, 100, 100),
        }
    }

    // Fluent API for setting properties
    pub fn width(mut self, width: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_width = width;
            }
        }
        self
    }

    pub fn padding(mut self, padding: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_padding = padding;
            }
        }
        self
    }

    pub fn margin(mut self, margin: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_margin = margin;
            }
        }
        self
    }

    pub fn set_border_color(mut self, color: Color) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.border_color = color;
            }
        }

        self
    }

    pub fn display(mut self, d: Display) -> Self {
        match &mut self {
            Widget::Frame { display, .. } => {
                *display = d;
            }

            _ => {},
        }

        self
    }

    pub fn set_border_radius(mut self, radius: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.border_radius = radius;
            }
        }

        self
    }

    pub fn set_border_size(mut self, size: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.border_size = size;
            }
        }

        self
    }

    pub fn height(mut self, height: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_height = height;
            }
        }
        self
    }

    pub fn x(mut self, x: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_x = x;
            }
        }
        self
    }

    pub fn y(mut self, y: Size) -> Self {
        match &mut self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => {
                geometry.user_y = y;
            }
        }
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        match &mut self {
            Widget::Frame { background_color, .. } |
            Widget::Button { background_color, .. } |
            Widget::Label { background_color, .. } |
            Widget::Canvas { background_color, .. } => {
                *background_color = color;
            }

            _ => {}
        }
        self
    }

    pub fn on_click(mut self, handler: fn(&mut Window, WidgetId)) -> Self {
        if let Widget::Button { on_click, .. } = &mut self {
            *on_click = Some(handler);
        }
        self
    }

    // Add child to frame
    pub fn add_child(mut self, child: Widget) -> Self {
        if let Widget::Frame { children, .. } = &mut self {
            children.push(child);
        }
        self
    }

    pub fn get_text(&self) -> Option<&str> {
        match self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                Some(&text.text)
            }
            _ => { None }
        }
    }

    pub fn append_text(&mut self, s: &str) {
        match self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                if text.can_modify {
                    if text.max_len.is_some() {
                        let available = text.max_len.unwrap().saturating_sub(text.text.len());
                        if available > 0 {
                            let chars_to_take = s.chars()
                                .take(available)
                                .collect::<String>();
                            text.text.push_str(&chars_to_take); // Append only what fits
                        }
                    } else {
                        text.text.push_str(s);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn pop_text_char(&mut self) {
        match self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                if text.can_modify && text.text.len() > text.min_len {
                    text.text.pop();  // Removes last char; no allocation, but capacity stays the same
                }
            }
            _ => {}
        }
    }

    // Getters for geometry - no borrowing needed!
    pub fn geometry(&self) -> &WidgetGeometry {
        match self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => geometry,
        }
    }

    pub fn geometry_mut(&mut self) -> &mut WidgetGeometry {
        match self {
            Widget::Frame { geometry, .. } |
            Widget::Image { geometry, .. } |
            Widget::Button { geometry, .. } |
            Widget::Label { geometry, .. } |
            Widget::Canvas { geometry, .. } => geometry,
        }
    }

    pub fn get_children(&self) -> Option<&Vec<Widget>> {
        match self {
            Widget::Frame { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn get_children_mut(&mut self) -> Option<&mut Vec<Widget>> {
        match self {
            Widget::Frame { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn get_event_handler(&self) -> Option<EventHandler> {
        match self {
            Widget::Button { on_click, .. } => *on_click,
            _ => None,
        }
    }

    // Update widget geometry based on parent constraints
    pub fn update_layout(
        &mut self,
        parent_x: usize,
        parent_y: usize,
        parent_width: usize,
        parent_height: usize,
        parent_padding: usize,
        display: &Display,
    ) {
        // Update self's geometry
        let geometry = self.geometry_mut();

        // Calculate margins and padding
        geometry.margin = match geometry.user_margin {
            Size::Absolute(size) => size,
            Size::Relative(size) => ceil_f32((parent_height as f32 * size as f32 / 100.0)) as usize,
            _ => 0,
        };

        geometry.padding = match geometry.user_padding {
            Size::Absolute(size) => size,
            Size::Relative(size) => ceil_f32((parent_height as f32 * size as f32 / 100.0)) as usize,
            _ => 0,
        };

        // Calculate available space within parent (accounting for parent's padding/margin)
        let available_x = parent_x + parent_padding;
        let available_y = parent_y + parent_padding;
        let available_width = parent_width.saturating_sub(2 * parent_padding);
        let available_height = parent_height.saturating_sub(2 * parent_padding);

        // Calculate position (including this widget's margin)
        geometry.x = match geometry.user_x {
            Size::Absolute(size) => available_x + geometry.margin + size,
            Size::Relative(size) => {
                available_x + geometry.margin + ceil_f32((available_width as f32 * size as f32 / 100.0)) as usize
            }
            Size::FromRight(size) => parent_x + parent_width - parent_padding - size - geometry.margin,
            Size::FromLeft(size) => available_x + size + geometry.margin,
            _ => available_x + geometry.margin,
        };

        geometry.y = match geometry.user_y {
            Size::Absolute(size) => available_y + geometry.margin + size,
            Size::Relative(size) => {
                available_y + geometry.margin + ceil_f32((available_height as f32 * size as f32 / 100.0)) as usize
            }
            Size::FromUp(size) => available_y + size + geometry.margin,
            Size::FromDown(size) => parent_y + parent_height - parent_padding - size - geometry.margin,
            _ => available_y + geometry.margin,
        };

        // Calculate size (accounting for margins)
        let max_width = available_width.saturating_sub(geometry.margin * 2);
        let max_height = available_height.saturating_sub(geometry.margin * 2);

        geometry.width = match geometry.user_width {
            Size::Absolute(size) => size.min(max_width),
            Size::Relative(size) => ceil_f32((max_width as f32 * size as f32 / 100.0)) as usize,
            Size::Auto => max_width,
            _ => max_width,
        };

        geometry.height = match geometry.user_height {
            Size::Absolute(size) => size.min(max_height),
            Size::Relative(size) => ceil_f32((max_height as f32 * size as f32 / 100.0)) as usize,
            Size::Auto => max_height,
            _ => max_height,
        };

        // Store geometry values before the match to avoid borrow conflicts
        let widget_x = geometry.x;
        let widget_y = geometry.y;
        let widget_width = geometry.width;
        let widget_height = geometry.height;
        let widget_padding = geometry.padding;
        let widget_margin = geometry.margin;

        // Handle child layout for Frame variant only
        match self {
            Widget::Frame { children, .. } => {
                match display {
                    Display::Grid { rows, cols } => {
                        if *rows == 0 || *cols == 0 {
                            return; // Invalid grid
                        }

                        // Content area inside this widget (minus padding)
                        let content_width = widget_width.saturating_sub(widget_padding * 2);
                        let content_height = widget_height.saturating_sub(widget_padding * 2);

                        let cell_width = content_width / *cols;
                        let cell_height = content_height / *rows;

                        for (i, child) in children.iter_mut().enumerate() {
                            let row = i / *cols;
                            let col = i % *cols;
                            if row >= *rows {
                                break; // Skip children that don't fit
                            }

                            let child_parent_x = widget_x + widget_padding + col * cell_width;
                            let child_parent_y = widget_y + widget_padding + row * cell_height;

                            // Each child gets its own cell as its parent bounds
                            child.update_layout(
                                child_parent_x,
                                child_parent_y,
                                cell_width,
                                cell_height,
                                0, // No additional padding from grid parent
                                &Display::None, // Individual cells don't have their own display mode
                            );
                        }
                    }
                    Display::Flex { direction, wrap } => {
                        // Content area inside this widget (minus padding)
                        let content_width = widget_width.saturating_sub(widget_padding * 2);
                        let content_height = widget_height.saturating_sub(widget_padding * 2);
                        let content_x = widget_x + widget_padding;
                        let content_y = widget_y + widget_padding;

                        let children_vec = core::mem::take(children);
                        let mut current_x = 0; // Offset from content_x
                        let mut current_y = 0; // Offset from content_y
                        let mut line_height = 0;
                        let mut line_width = 0;

                        for mut child in children_vec.into_iter() {
                            // First, calculate child's preferred size with generous bounds
                            child.update_layout(
                                content_x,
                                content_y,
                                content_width,
                                content_height,
                                0,
                                &Display::None,
                            );

                            let child_geometry = child.geometry();
                            let child_total_width = child_geometry.width + child_geometry.margin * 2;
                            let child_total_height = child_geometry.height + child_geometry.margin * 2;

                            match direction {
                                FlexDirection::Row => {
                                    // Check if we need to wrap
                                    if *wrap && current_x + child_total_width > content_width && current_x > 0 {
                                        current_x = 0;
                                        current_y += line_height;
                                        line_height = 0;
                                    }

                                    // Now position child in its allocated space
                                    let child_x = content_x + current_x;
                                    let child_y = content_y + current_y;
                                    let available_width = (content_width - current_x).min(child_total_width);
                                    let available_height = content_height.saturating_sub(current_y);

                                    child.update_layout(
                                        child_x,
                                        child_y,
                                        available_width,
                                        available_height.min(child_total_height),
                                        0,
                                        &Display::None,
                                    );

                                    current_x += child_total_width;
                                    line_height = line_height.max(child_total_height);
                                }
                                FlexDirection::Column => {
                                    // Check if we need to wrap
                                    if *wrap && current_y + child_total_height > content_height && current_y > 0 {
                                        current_y = 0;
                                        current_x += line_width;
                                        line_width = 0;
                                    }

                                    // Now position child in its allocated space
                                    let child_x = content_x + current_x;
                                    let child_y = content_y + current_y;
                                    let available_width = content_width.saturating_sub(current_x);
                                    let available_height = (content_height - current_y).min(child_total_height);

                                    child.update_layout(
                                        child_x,
                                        child_y,
                                        available_width.min(child_total_width),
                                        available_height,
                                        0,
                                        &Display::None,
                                    );

                                    current_y += child_total_height;
                                    line_width = line_width.max(child_total_width);
                                }
                            }
                            children.push(child);
                        }
                    }
                    Display::None => {
                        // Simple stacking layout - children can position themselves freely
                        let content_width = widget_width.saturating_sub(widget_padding * 2);
                        let content_height = widget_height.saturating_sub(widget_padding * 2);

                        let children_vec = core::mem::take(children);
                        for mut child in children_vec.into_iter() {
                            child.update_layout(
                                widget_x + widget_padding,
                                widget_y + widget_padding,
                                content_width,
                                content_height,
                                0,
                                &Display::None,
                            );
                            children.push(child);
                        }
                    }
                }
            }
            _ => {} // Handle other Widget variants if needed
        }
    }



    pub fn can_edit(&mut self, arg: bool) {
        match self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                text.can_modify = arg;
            }
            _ => {}
        }
    }

    // Draw the widget to framebuffer
    pub fn draw(&self, framebuffer: &mut [u32], buffer_width: usize) {

        if buffer_width == 0 { return; }

        let geometry = self.geometry();
        let buffer_height = framebuffer.len() / buffer_width;

        match self {
            Widget::Frame { background_color, geometry, .. } => {
                // Draw frame background
                crate::draw_2d::square::draw_square(
                    framebuffer,
                    buffer_width,
                    geometry.x + geometry.margin,
                    geometry.y + geometry.margin,
                    geometry.width,
                    geometry.height,
                    geometry.border_radius,
                    *background_color
                );
            }

            Widget::Image { loaded, image_ptr, image_size, .. } => {
                // Draw frame background
                if *loaded {
                    crate::draw_2d::image::draw_image(
                        framebuffer,
                        buffer_width,
                        *image_ptr,
                        (geometry.x + geometry.margin),
                        geometry.y + geometry.margin,
                        geometry.width,
                        geometry.height,
                        *image_size,
                    );
                }
            }

            Widget::Button { background_color, geometry, text, .. } => {
                // Draw button background
                crate::draw_2d::square::draw_square(
                    framebuffer,
                    buffer_width,
                    geometry.x + geometry.margin,
                    geometry.y + geometry.margin,
                    geometry.width,
                    geometry.height,
                    geometry.border_radius,
                    *background_color
                );

                // Draw button text centered
                crate::draw_2d::text::draw_text_in_rect(
                    framebuffer,
                    buffer_width,
                    buffer_height,
                    text.clone(),
                    geometry.x + geometry.margin + geometry.padding,
                    geometry.y + geometry.margin + geometry.padding,
                    geometry.width.saturating_sub(2 * geometry.padding),
                    geometry.height.saturating_sub(2 * geometry.padding),
                );
            }

            Widget::Label { background_color, geometry, text, .. } => {

                // Draw label background if it's not transparent
                if background_color.a > 0 {
                    crate::draw_2d::square::draw_square(
                        framebuffer,
                        buffer_width,
                        geometry.x + geometry.margin,
                        geometry.y + geometry.margin,
                        geometry.width,
                        geometry.height,
                        geometry.border_radius,
                        *background_color
                    );
                }

                // Create text with the label's text color
                let mut label_text = text.clone();

                // Draw label text
                crate::draw_2d::text::draw_text_in_rect(
                    framebuffer,
                    buffer_width,
                    buffer_height,
                    label_text,
                    geometry.x + geometry.margin + geometry.padding,
                    geometry.y + geometry.margin + geometry.padding,
                    geometry.width.saturating_sub(2 * geometry.padding),
                    geometry.height.saturating_sub(2 * geometry.padding),
                );
            }

            Widget::Canvas { framebuffer: widget_buffer, .. } => {
                // Copy canvas buffer to main framebuffer
                if !widget_buffer.is_empty() {
                    for row in 0..geometry.height {
                        let dest_start = (geometry.y + row) * buffer_width + geometry.x;
                        let dest_end = dest_start + geometry.width;
                        let src_start = row * geometry.width;
                        let src_end = src_start + geometry.width;

                        if dest_end <= framebuffer.len() && src_end <= widget_buffer.len() {
                            framebuffer[dest_start..dest_end]
                                .copy_from_slice(&widget_buffer[src_start..src_end]);
                        }
                    }
                }
            }
        }
    }

    pub fn set_text(mut self, string: &str) -> Self {
        match &mut self {
            Widget::Button { text: text, .. } |
            Widget::Label { text: text, .. } => {
                text.text = String::from(string);
            }
            _ => {} // Other widgets don't have text
        }
        self
    }

    pub fn set_font(mut self, font_path: &str) -> Self {
        match &mut self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                text.font = String::from(font_path);
            }
            _ => {}
        }
        self
    }

    pub fn set_text_size(mut self, size: usize) -> Self {
        match &mut self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                text.size = size;
            }
            _ => {}
        }
        self
    }

    pub fn set_text_color(mut self, color: Color) -> Self {
        match &mut self {
            Widget::Button { text, .. } => {
                text.color = color;
            }
            Widget::Label { text, .. } => {
                text.color = color;
            }
            _ => {}
        }
        self
    }

    pub fn set_text_align(mut self, align: Align) -> Self {
        match &mut self {
            Widget::Button { text, .. } |
            Widget::Label { text, .. } => {
                text.align = align;
            }
            _ => {}
        }
        self
    }

    // Find widget at given coordinates - returns widget ID
    pub fn find_widget_at(&self, x: usize, y: usize) -> Option<WidgetId> {
        let geometry = self.geometry();

        // Check if point is within this widget's bounds
        if x >= geometry.x && x < geometry.x + geometry.width &&
            y >= geometry.y && y < geometry.y + geometry.height {

            // If this is a frame with children, check children first (front to back)
            if let Some(children) = self.get_children() {
                for child in children.iter().rev() {
                    if let Some(child_id) = child.find_widget_at(x, y) {
                        return Some(child_id);
                    }
                }
            }

            // Return this widget's ID if no child was hit
            Some(geometry.id)
        } else {
            None
        }
    }

    // Find widget by ID in the tree
    pub fn find_widget_by_id(&self, target_id: WidgetId) -> Option<&Widget> {
        if self.geometry().id == target_id {
            return Some(self);
        }

        if let Some(children) = self.get_children() {
            for child in children {
                if let Some(found) = child.find_widget_by_id(target_id) {
                    return Some(found);
                }
            }
        }

        None
    }

    // Find widget by ID (mutable reference)
    pub fn find_widget_by_id_mut(&mut self, target_id: WidgetId) -> Option<&mut Widget> {
        if self.geometry().id == target_id {
            return Some(self);
        }

        if let Some(children) = self.get_children_mut() {
            for child in children {
                if let Some(found) = child.find_widget_by_id_mut(target_id) {
                    return Some(found);
                }
            }
        }

        None
    }

    pub fn get_display(&self) -> Display {
        match self {
            Frame { display, .. } => *display,

            _ => Display::None,
        }
    }

    pub fn get_id(&self) -> WidgetId {
        self.geometry().id
    }

    pub fn get_x(&self) -> usize {
        self.geometry().x
    }

    pub fn get_y(&self) -> usize {
        self.geometry().y
    }

    pub fn get_width(&self) -> usize {
        self.geometry().width
    }

    pub fn get_height(&self) -> usize {
        self.geometry().height
    }

    pub fn get_margin(&self) -> usize {
        self.geometry().margin
    }

    pub fn get_padding(&self) -> usize {
        self.geometry().padding
    }


    pub fn get_user_x(&self) -> Size {
        self.geometry().user_x
    }

    pub fn get_user_y(&self) -> Size {
        self.geometry().user_y
    }

    pub fn get_user_width(&self) -> Size {
        self.geometry().user_width
    }

    pub fn get_user_height(&self) ->  Size {
        self.geometry().user_height
    }

    pub fn get_user_margin(&self) ->  Size {
        self.geometry().user_margin
    }

    pub fn get_user_padding(&self) ->  Size {
        self.geometry().user_padding
    }


    pub fn set_x(&mut self, val: usize) {
        self.geometry_mut().x = val;
    }

    pub fn set_y(&mut self, val: usize) {
        self.geometry_mut().y = val;
    }

    pub fn set_width(&mut self, val: usize) {
        self.geometry_mut().width = val;
    }

    pub fn set_height(&mut self, val: usize) {
        self.geometry_mut().height = val;
    }

    pub fn set_margin(&mut self, val: usize) {
        self.geometry_mut().margin = val;
    }

    pub fn set_padding(&mut self, val: usize) {
        self.geometry_mut().padding = val;
    }
}
