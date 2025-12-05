use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::arch::asm;

pub static mut PID: u16 = 0;

#[inline(never)]
pub extern "C" fn syscall(index: u32, ebx: u32, ecx: u32, edx: u32) -> u32 {
    unsafe {
        let mut register: u32 = 0;

        asm!(
            "int 0x80",

            in("ebx") ebx,
            in("ecx") ecx,
            in("edx") edx,
            inlateout("eax") index => register,
        );

        return register;
    }
}

pub fn malloc(size: u32) -> u32 { syscall(5, size, 0, 0) }

pub fn free(base: u32) {
    syscall(6, base, 0, 0);
}

pub fn expand(base: u32, size: u32) -> u32 { syscall(10, base, size, 0) }

pub fn get_dub_buffer() -> u32 {
    syscall(7, 0, 0, 0)
}

pub fn write_to_screen(buffer: u32, c: Coordiates) {
    syscall(8, buffer, &c as *const _ as u32, 0);
}

pub fn write_wid_to_screen(wid: u32) {
    syscall(9, wid, 0, 0);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum Items {
    Wallpaper,
    Bar,
    Popup,
    Window,
}

#[derive(Debug, Copy, Clone)]
pub struct Window {
    pub id: usize,
    pub buffer: u32,

    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub width: usize,
    pub height: usize,

    pub can_move: bool,
    pub can_resize: bool,
    pub min_width: usize,
    pub min_height: usize,

    pub event_handler: u32,
    pub w_type: Items,
}

pub fn add_window(w: Window) -> u32 {
    let val = syscall(22, &w as *const _ as u32, 0, 0);
    return val;
}

pub fn remove_window(id: u32) {
    syscall(23, id, 0, 0);
}

/*pub fn change_window(w: Window) {

    syscall(24, 0, 0, 0);
}*/

pub fn add_task(base: u32, args: Option<&[u32]>) {
    let mut args_ptr = 0;
    if args.is_some() {
        args_ptr = args.unwrap().as_ptr() as u32;
    }
    syscall(25, base, 0, args_ptr);
}

pub fn exit() -> ! {
    syscall(26, 0, 0, 0);

    loop {}
}

pub fn poll_event(window_id: u32, event_ptr: u32) {
    syscall(50, event_ptr, window_id, 0);
}

pub fn thread_yield() {
    unsafe {
        asm!("int 0x20");
    }
}

pub fn move_window(id: u32, x: u32, y: u32) {
    syscall(43, id, x ,y);
}

pub fn get_screen_width() -> u32 {
    syscall(44, 0, 0, 0)
}

pub fn get_screen_height() -> u32 {
    syscall(45, 0, 0, 0)
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Process {
    id: u16,
    draw: u32,
    mouse: u32,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Coordiates {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}
