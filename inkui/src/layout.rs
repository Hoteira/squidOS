use crate::types::Size;
use crate::widget::Widget;
use crate::math::ceil_f32;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    None,
    Grid { rows: usize, cols: usize }, // Number of rows and columns
    Flex { direction: FlexDirection, wrap: bool },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    Row,
    Column,
}

pub fn make_flex_display(parent: &mut Widget) {
    // Step 1: Extract parent dimensions
    let (p_w, p_h, p_x, p_y) = {
        (
            parent.get_width(),
            parent.get_height(),
            parent.get_x(),
            parent.get_y()
        )
    };

    // Step 2: Clone the children Vec with explicit type
    let mut children: Vec<Widget> = {
        match parent.get_children_mut() {
            Some(children) => children.clone(),
            None => return,
        }
    };

    let mut height_buffer = 0;
    let mut row_height = 0;
    let mut row_width = 0;

    // Step 3: Now we can safely iterate and modify children
    for child in children.iter_mut() {
        // Explicitly re-borrow as immutable for calling getters
        let user_margin = (&*child).get_user_margin();
        let user_width = (&*child).get_user_width();
        let user_height = (&*child).get_user_height();

        // Calculate dimensions
        let margin = match user_margin {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(p_w as f32 * rv as f32 / 100.0) as usize,
            _ => 0,
        };

        let cw = match user_width {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(p_w as f32 * rv as f32 / 100.0) as usize,
            _ => 0,
        };

        let ch = match user_height {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(p_h as f32 * rv as f32 / 100.0) as usize,
            _ => 0,
        };

        // Handle row wrapping
        if row_width + cw > p_w {
            height_buffer += row_height;
            row_width = 0;
            row_height = 0;
        }

        // Update child with calculated values
        {
            child.set_width(cw);
            child.set_height(ch);
            child.set_x(p_x + row_width);
            child.set_y(p_y + height_buffer);
        }

        // Update layout tracking
        row_width += cw + margin;
        if ch + margin > row_height {
            row_height = ch + margin;
        }
    }
}

pub fn make_grid_display(parent: &mut Widget, rows: usize, cols: usize) {
    if rows == 0 || cols == 0 {
        return;
    }

    // Step 1: Extract parent data
    let (p_w, p_h, p_x, p_y) = {
        (
            parent.get_width(),
            parent.get_height(),
            parent.get_x(),
            parent.get_y()
        )
    };

    // Step 2: Clone children Vec with explicit type
    let mut children: Vec<Widget> = {
        match parent.get_children_mut() {
            Some(children) => children.clone(),
            None => return,
        }
    };

    let cell_w = p_w / cols;
    let cell_h = p_h / rows;

    // Step 3: Process each child safely
    for (i, child) in children.iter_mut().enumerate() {
        let r = i / cols;
        let c = i % cols;

        // Explicitly re-borrow as immutable for calling getters
        let user_width = (&*child).get_user_width();
        let user_height = (&*child).get_user_height();
        let user_margin = (&*child).get_user_margin();

        // Calculate dimensions
        let cw = match user_width {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(cell_w as f32 * rv as f32 / 100.0) as usize,
            _ => cell_w, // Default to full cell width
        };

        let ch = match user_height {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(cell_h as f32 * rv as f32 / 100.0) as usize,
            _ => cell_h, // Default to full cell height
        };

        let margin = match user_margin {
            Size::Absolute(v) => v,
            Size::Relative(rv) => ceil_f32(cell_w as f32 * rv as f32 / 100.0) as usize,
            _ => 0,
        };

        // Apply calculated values
        {
            child.set_width(cw);
            child.set_height(ch);
            child.set_x(p_x + c * cell_w + margin);
            child.set_y(p_y + r * cell_h + margin);
        }
    }
}
