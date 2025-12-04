#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

pub mod boot;
mod interrupts;
mod drivers;
mod fs;
mod memory;
mod tss;

use alloc::boxed::Box;
use std::println;
use crate::boot::{BootInfo, BOOT_INFO};
use core::arch::asm;

extern crate alloc;

#[unsafe(no_mangle)]
#[unsafe(link_section = ".start")]
pub extern "C" fn _start(bootinfo_ptr: *const BootInfo) -> ! {
    unsafe {
        asm!("cli" );
    }

    unsafe { 
        *(&raw mut BOOT_INFO) = bootinfo_ptr.read();
    };

    memory::init();

    fs::dma::init();
    std::memory::heap::init_heap(0x30_0000 as *mut u8, 0x10_0000);

    println!("[KERNEL] Hello World!");

    interrupts::task::TASK_MANAGER.lock().init();

    // Kernel Task (can use println!)
    interrupts::task::TASK_MANAGER.lock().add_task(test_task as u64, None);
    
    // User Task (MUST be self-contained, no kernel calls)
    interrupts::task::TASK_MANAGER.lock().add_task(user_task as u64, None);

    load_idt();
    //keyboard::init();

    println!("[KERNEL] Initializing Ext2...");
    
    // Mount Ext2 at offset 16384 (where make.bat puts it)
    // Disk 0
    match crate::fs::ext2::fs::Ext2::new(0, 16384) {
        Ok(fs) => {
            println!("[EXT2] Superblock found!");
            println!(" - Magic: {:#x}", fs.superblock.magic + 0);
            println!(" - Inodes Count: {}", fs.superblock.inodes_count + 0);
            println!(" - Blocks Count: {}", fs.superblock.blocks_count + 0);
            println!(" - Block Size: 1024 << {} = {}", fs.superblock.log_block_size + 0, 1024 << fs.superblock.log_block_size + 0);
            println!(" - First Data Block: {}", fs.superblock.first_data_block + 0);
            println!(" - Mount Count: {}", fs.superblock.mount_count + 0);
        }
        Err(e) => println!("[EXT2] Failed to mount Disk 0 @ 16384: {}", e),
    }

    println!("[KERNEL] Starting Kernel Tasks...");

    loop {}
}

fn test_task() {
    loop {
        println!("Task A (Kernel)");
        for _ in 0..10000000 { unsafe { core::arch::asm!("nop") } }
    }
}

fn user_task() {
    loop {
        unsafe {
            // Write 'U' to serial port 0x3F8
            core::arch::asm!(
                "mov dx, 0x3F8",
                "mov al, 0x55", // 'U'
                "out dx, al",
                options(nomem, nostack, preserves_flags)
            );

            for _ in 0..10000000 { 
                core::arch::asm!("nop");
            }
        }
    }
}


pub fn load_idt() {
    unsafe {
        (*(&raw mut interrupts::idt::IDT)).init();

        (*(&raw mut interrupts::idt::IDT)).processor_exceptions();
        (*(&raw mut interrupts::idt::IDT)).hardware_interrupts();

        // 3. Load the IDT into the CPU

        (*(&raw mut interrupts::idt::IDT)).load();
        (*(&raw mut interrupts::pic::PICS)).init();

        core::arch::asm!("sti"); // Interrupts disabled for stability during mounting
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("[KERNEL_PANIC]\n{}", info);
    loop {}
}


