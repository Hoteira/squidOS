#![feature(c_variadic)]
#![no_std]
#![feature(naked_functions)]

extern crate alloc;
#[macro_use]
extern crate std;

use core::ffi::{c_char, c_int};

pub mod string;
pub mod ctype;
pub mod stdlib;
pub mod stdio;
pub mod math;
pub mod unistd;
pub mod sys;
pub mod curses;
pub mod dirent;
pub mod locale;

unsafe extern "C" {
    fn main(argc: c_int, argv: *mut *mut c_char) -> c_int;
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "mov rdi, rsp",
        "and rsp, -16",
        "call rust_start",
        "hlt",
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_start(stack: *const usize) -> ! {
    let size = 10 * 1024 * 1024;
    let ptr_raw = std::memory::malloc(size);
    if ptr_raw == usize::MAX || ptr_raw == 0 {
        loop { unsafe { core::arch::asm!("hlt"); } }
    }
    let ptr = ptr_raw as *mut u8;
    std::memory::heap::init_heap(ptr, size);

    let argc = *stack as c_int;
    let argv = stack.add(1) as *mut *mut c_char;

    let result = main(argc, argv);
    stdlib::exit(result);
}

#[unsafe(no_mangle)]
pub static mut errno: c_int = 0;

#[panic_handler]
pub fn panic(i: &core::panic::PanicInfo) -> ! {
    std::println!("[USER PANIC] {}", i);
    loop {}
}
