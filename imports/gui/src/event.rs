use alloc::vec::Vec;
use libk::println;
use crate::event::Event::{Keyboard, Mouse, Redraw, Resize};

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
