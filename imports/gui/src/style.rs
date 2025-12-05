use crate::gui::{ Size, Color };

pub struct Style {
    pub background_color: Color,

    pub text_color: Color,
    pub text_size: usize,

    pub border_radius: Size,
    pub border_color: Color,
    pub border_width: Size,
    
}

impl Style {
    pub fn new() -> Self {
        Style {
            background_color: Color::rgb(255, 255, 255),

            text_color: Color::rgb(0, 0, 0),
            text_size: 12,

            border_radius: Size::Absolute(0),
            border_color: Color::rgb(0, 0, 0,),
            border_width: Size::Absolute(0),

        }
    }

    pub fn bg_color(mut self, arg: Color) -> Self {
        self.background_color = arg;

        self
    }

    pub fn text_color(mut self, arg: Color) -> Self {
        self.text_color = arg;

        self
    }

    pub fn text_size(mut self, arg: usize) -> Self {
        self.text_size = arg;

        self
    }

    pub fn border_radius(mut self, arg: Size) -> Self {
        self.border_radius = arg;

        self
    }

    pub fn border_color(mut self, arg: Color) -> Self {
        self.border_color = arg;

        self
    }

    pub fn border_width(mut self, arg:Size) -> Self {
        self.border_width = arg;

        self
    }
}