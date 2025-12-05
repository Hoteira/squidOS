#![no_std]
#![no_main]

use core::fmt;
use std::{print, println};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let mut buffer = [0u8; 32];

    println!("Type something (Userland):");

    loop {
        let bytes_read = std::os::read(&mut buffer);

        if bytes_read > 0 {

            let s = unsafe { core::str::from_utf8_unchecked(&buffer[..bytes_read]) };
            print!("{}", s);

        } else {

            std::os::yield_task();
        }
    }
}