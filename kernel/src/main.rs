#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

mod boot;
mod interrupts;
mod drivers;
mod fs;
mod memory;

use alloc::boxed::Box;
use std::println;
use crate::boot::BootInfo;
use crate::fs::ext2::fs::FileSystem;
use crate::fs::vfs::Vfs;
use core::arch::asm;

extern crate alloc;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".start")]
pub extern "C" fn _start(bootinfo_ptr: *const BootInfo) -> ! {
    unsafe {
        asm!("cli" );
    }

    //fs::dma::init();
    std::memory::heap::init_heap(0x30_0000 as *mut u8, 0x10_0000);

    println!("[KERNEL] Hello World!");

    load_idt();
    //keyboard::init();

    let mut vfs = Vfs::new();

    println!("\nMounting Ext2 (Disk 0xE0) to '/'...");
    let ext2_fs = FileSystem::mount(0xE0);
    vfs.mount("/", Box::new(ext2_fs));

    // List Root
    println!("\n[VFS] Listing '/':");
    match vfs.list_dir("/") {
        Ok(files) => {
            for f in files {
                println!("  - {}", f);
            }
        },
        Err(e) => println!("Error: {}", e),
    }

    let create_path = "/rtc_test.txt";
    println!("\n[VFS] Creating '{}'...", create_path);
    match vfs.create_file(create_path) {
        Ok(_) => {
            println!("  Success! File created.");
            let content = "This file was created with a real timestamp!";
            let _ = vfs.write_file(create_path, content.as_bytes());
        },
        Err(e) => println!("Error: {}", e),
    }

    let read_path = "/thnx.txt";
    println!("\n[VFS] Reading '{}'...", read_path);
    match vfs.read_file(read_path) {
        Ok(data) => {
             match alloc::str::from_utf8(&data) {
                 Ok(s) => println!("  Content:\n\"{}", s),
                 Err(_) => println!("  (Binary data, {} bytes)", data.len()),
             }
        },
        Err(e) => println!("Error: {}", e),
    }

    loop {}
}

pub fn load_idt() {
    unsafe {
        (*(&raw mut interrupts::idt::IDT)).init();

        (*(&raw mut interrupts::idt::IDT)).processor_exceptions();
        (*(&raw mut interrupts::idt::IDT)).hardware_interrupts();

        // 3. Load the IDT into the CPU

        (*(&raw mut interrupts::idt::IDT)).load();
        (*(&raw mut interrupts::pic::PICS)).init();

        //core::arch::asm!("sti"); // Interrupts disabled for stability during mounting
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("[KERNEL_PANIC]\n{}", info);
    loop {}
}


