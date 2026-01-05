#![no_std]
#![no_main]

extern crate alloc;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut buf = [0u8; 1024];
    loop {
        let n = std::os::file_read(0, &mut buf);
        if n == 0 { break; }
        std::os::file_write(1, &buf[0..n]);
    }

    0
}
