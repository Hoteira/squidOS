use crate::drivers::port::{inb, outb};
#[allow(unused_imports)]
use crate::debugln;
use alloc::vec::Vec;

pub const MOUSE_INT: u8 = 44;
pub static mut MOUSE_PACKET: [u8; 4] = [0; 4];
pub static mut MOUSE_IDX: usize = 0;
pub static mut MOUSE_PACKET_SIZE: usize = 3; 

// Cursor definitions
const O: u32 = 0x0000_0000;
const B: u32 = 0x0000_00FF;
const T: u32 = 0xFFFF_FFFF;

// Original 8x12 cursor pattern
pub const CURSOR_BUFFER: [u32; 1024] = [
    B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, T, B, B, B, B, B, B, B, B, B, B, B, B, B, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, T, B, B, B, B, B, B, B, B, B, B, B, B, B, B, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, T, B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, T, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
    B, B, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O, O,
];

// Global cursor buffer - 32x32 max size
pub const CURSOR_WIDTH: usize = 32;
pub const CURSOR_HEIGHT: usize = 32;

// Initialize mouse driver and generate the scaled cursor
pub fn init_mouse() {
    debugln!("[MOUSE] Initializing...");


    // --- PS/2 Initialization ---
    wait();
    outb(0x64, 0xA8);
    
    wait();
    outb(0x64, 0x20); 
    wait_input();
    let mut status = inb(0x60);
    
    status |= 0b10; 
    status &= !0x20;
    
    wait();
    outb(0x64, 0x60);
    wait();
    outb(0x60, status);

    mouse_write(0xF6);
    let _ack = mouse_read();

    mouse_write(0xF3); let _ = mouse_read(); mouse_write(200); let _ = mouse_read();
    mouse_write(0xF3); let _ = mouse_read(); mouse_write(100); let _ = mouse_read();
    mouse_write(0xF3); let _ = mouse_read(); mouse_write(80);  let _ = mouse_read();

    mouse_write(0xF2);
    let _ack_id = mouse_read();
    let id = mouse_read();
    debugln!("[MOUSE] Device ID: {:#x}", id);

    unsafe {
        if id == 3 || id == 4 {
            MOUSE_PACKET_SIZE = 4;
            debugln!("[MOUSE] IntelliMouse Mode Enabled (4-byte packets)");
        } else {
            MOUSE_PACKET_SIZE = 3;
            debugln!("[MOUSE] Standard PS/2 Mode (3-byte packets)");
        }
    }

    mouse_write(0xF4);
    let ack_enable = mouse_read();
    debugln!("[MOUSE] Enable Reporting ACK: {:#x}", ack_enable);
}

fn mouse_write(value: u8) {
    wait();
    outb(0x64, 0xD4);
    wait();
    outb(0x60, value);
}

fn mouse_read() -> u8 {
    wait_input();
    inb(0x60)
}

fn wait() {
    let mut time = 100_000;
    while time > 0 {
        if (inb(0x64) & 0b10) == 0 {
            return;
        }
        time -= 1;
    }
}

fn wait_input() {
    let mut time = 100_000;
    while time > 0 {
        if (inb(0x64) & 0b1) == 1 {
            return;
        }
        time -= 1;
    }
}