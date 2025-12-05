#![no_std]

extern crate alloc;

pub mod display;
pub mod draw_2d;
pub mod draw_3d;

pub mod style;
pub mod window;
pub mod widget;
pub mod gui;

pub  mod event;

pub mod font_manager;
mod main_loop;

use crate::window::Window;
use ttf;

pub fn ceil_f32(x: f32) -> f32 {
    let xi = x as i32;
    if x > xi as f32 {
        (xi + 1) as f32
    } else {
        xi as f32
    }
}

pub fn ceil_f64(x: f64) -> f64 {
    let xi = x as i64;
    if x > xi as f64 {
        (xi + 1) as f64
    } else {
        xi as f64
    }
}

pub fn min_f32(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

pub fn min_f64(a: f64, b: f64) -> f64 {
    if a < b { a } else { b }
}

pub fn sqrt_f64(x: f64) -> f64 {
    if x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 || x == f64::INFINITY {
        return x;
    }

    let mut guess = x;
    let mut prev = 0.0;

    for _ in 0..10 {
        prev = guess;
        guess = 0.5 * (guess + x / guess);

        if (guess - prev).abs() < 1e-14 {
            break;
        }
    }

    guess
}

pub fn floor_f64(x: f64) -> f64 {
    let xi = x as i64;
    let xf = xi as f64;

    if xf > x {
        (xi - 1) as f64
    } else {
        xf
    }
}

pub type WidgetId = usize;
pub type EventHandler = fn(&mut Window, WidgetId);

pub struct FrameBuffer { pub address: *mut u32, size: usize }

impl FrameBuffer {
    pub fn new(size: usize) -> Self {
        let address = libk::syscall::malloc(size as u32);

        Self { address: address as *mut u32, size }
    }

    pub fn drop(&self) {
        libk::syscall::free(self.address as u32);
    }

    pub fn resize(&mut self, size: usize) {
        if self.size < size {
            let address = libk::syscall::expand(self.address as u32, size as u32);
            self.address = address as *mut u32;
        }

    }

    pub fn clear(&self) {
        if self.address as usize != 0 {

        }
    }
}

pub fn init_gui() {
    font_manager::FontManager::init();
}

