use alloc::string::String;

#[derive(Debug, Copy, Clone)]
pub enum Size {
    Absolute(usize),
    Relative(usize),

    FromRight(usize),
    FromLeft(usize),

    FromUp(usize),
    FromDown(usize),
    Auto,
}

#[derive(Debug, Copy, Clone)]
pub enum Align {
    Center,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub text: String,
    pub size: usize,
    pub color: Color,
    pub align: Align,
    pub font: String,
    pub max_len: Option<usize>,
    pub can_modify: bool,
    pub min_len: usize,
}

impl Text {
    pub  fn new(text: &str) -> Self {
        Text {
            text: String::from(text),
            size: 12,
            color: Color::rgb(0, 0, 0),
            align: Align::Left,
            font: String::from("/SYS/FONT/EXCAL.TTF"),
            max_len: None,
            can_modify: false,
            min_len: 0,
        }
    }

    pub fn set_max(mut self, arg: usize) -> Self {
        self.max_len = Some(arg);

        self
    }

    pub fn set_min(mut self, arg: usize) -> Self {
        self.min_len = arg;

        self
    }

    pub fn set_align(mut self, arg: Align) -> Self {
        self.align = arg;

        self
    }

    pub fn set_size(mut self, arg: usize) -> Self {
        self.size = arg;

        self
    }

    pub fn set_color(mut self, arg: Color) -> Self {
        self.color = arg;

        self
    }

    pub fn set_font(mut self, arg: &str) -> Self {
        self.font = String::from(arg);

        self
    }
}

#[derive(Debug, Copy, Clone)]
pub struct  Color {
    pub r: usize,
    pub g: usize,
    pub b: usize,
    pub a: usize,
}

impl Color {
    pub const fn rgb(r: usize, g: usize, b: usize) -> Color {
        Color {
            r,
            g,
            b,
            a: 255,
        }
    }

    pub const fn rgba(r: usize, g: usize, b: usize, a: usize) -> Color {
        Color {
            r,
            g,
            b,
            a,
        }
    }

    pub fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn from_u32(color: u32) -> Color {
        Color::rgba(
            ((color >> 16) & 0xFF) as usize, // Red
            ((color >> 8)  & 0xFF) as usize, // Green
            (color         & 0xFF) as usize, // Blue
            ((color >> 24) & 0xFF) as usize, // Alpha
        )
    }

    pub fn to_u24(&self) -> [u8; 4] {
        [self.r as u8, self.g as u8, self.b as u8, self.a as u8]
    }
}