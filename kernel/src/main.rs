#![feature(abi_x86_interrupt)]
#![no_std]
#![no_main]

pub mod boot;
mod interrupts;
mod drivers;
mod fs;
mod memory;
mod tss;
pub mod debug;
pub mod display;
pub mod composer;

extern crate alloc;

use crate::boot::{BootInfo, BOOT_INFO};
use crate::fs::vfs::FileSystem; 
use crate::fs::ext2::fs::Ext2; // Added Ext2 for font loading
use core::arch::asm;

// MSRs for SYSCALL/SYSRET and PAT
const EFER_MSR: u32 = 0xC0000080; // Extended Feature Enable Register
const STAR_MSR: u32 = 0xC0000081; // SYSCALL Target Address Register
const LSTAR_MSR: u32 = 0xC0000082; // Long Mode SYSCALL Target Address Register
const SFMASK_MSR: u32 = 0xC0000084; // SYSCALL Flag Mask
const PAT_MSR: u32 = 0x277;       // Page Attribute Table MSR

#[unsafe(no_mangle)]
#[unsafe(link_section = ".start")]
pub extern "C" fn _start(bootinfo_ptr: *const BootInfo) -> ! {
    unsafe {
        asm!("cli" );
    }

    unsafe { 
        *(&raw mut BOOT_INFO) = bootinfo_ptr.read();
    };

    // Initialize Memory Subsystem
    memory::init();
    
    // Configure PAT for Write-Combining on PAT4
    init_pat();

    fs::dma::init();
    // Move heap to 48MB to avoid collision with PMM Bitmap (10MB-43MB)
    std::memory::heap::init_heap(0x03000000 as *mut u8, 0x100_0000);
    
    crate::fs::vfs::init();

    tss::init_ists();

    interrupts::task::TASK_MANAGER.lock().init();

    unsafe { (*(&raw mut composer::DISPLAY_SERVER)).init(); }

    let first_user_task = interrupts::task::TASK_MANAGER.lock()
        .tasks.iter()
        .find(|t| t.state == interrupts::task::TaskState::Ready && t.kernel_stack != 0)
        .map(|t| t.kernel_stack);

    if let Some(kstack) = first_user_task {
        tss::set_tss(kstack);
    }

    drivers::periferics::mouse::init_mouse();


    crate::fs::vfs::mount(0xE0, Ext2::new(0xE0, 16384).unwrap());

    let path = crate::fs::vfs::Path::new("@0xE0/user").unwrap();
    let mut node = crate::fs::vfs::open(&path).unwrap();
    let mut buf = alloc::vec![0u8; node.size() as usize];
    let n = node.read(0, &mut buf).unwrap();

    unsafe {
        let pml4 = memory::vmm::new_user_pml4();
        let entry = crate::fs::elf::load_elf(&buf[0..n], pml4).unwrap();
        let _ = interrupts::task::TASK_MANAGER.lock().add_user_task(entry, pml4, None);
    }


    load_idt();

    init_syscall_msrs(); // Initialize SYSCALL MSRs

    // Enable interrupts only AFTER all drivers are initialized
    unsafe { asm!("sti"); }

    loop {}
}

fn init_pat() {
    unsafe {
        let mut pat = rdmsr(PAT_MSR);

        // PAT Layout: 8 entries of 1 byte each (bits 0-2 are memory type, others reserved/0).
        // We want Index 4 (Bits 32-39) to be Write-Combining (0x01).
        // Clear index 4
        pat &= !(0xFFu64 << 32);
        // Set index 4 to 0x01 (WC)
        pat |= (0x01u64 << 32);

        wrmsr(PAT_MSR, pat);

        // Flush TLB to ensure new attributes take effect
        let cr3: u64;
        asm!("mov {}, cr3", out(reg) cr3);
        asm!("mov cr3, {}", in(reg) cr3);
    }
}

/// Reads an MSR.
unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);
    unsafe { asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high) };
    ((high as u64) << 32) | (low as u64)
}

/// Writes an MSR.
unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe { asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high) };
}

fn init_syscall_msrs() {
    unsafe {
        // 1. Enable SYSCALL/SYSRET in EFER MSR
        let mut efer = rdmsr(EFER_MSR);
        efer |= 1;
        wrmsr(EFER_MSR, efer);

        let sysret_cs_base = 0x20;

        let syscall_cs_base = 0x28;

        let star_value = ((sysret_cs_base as u64) << 48) | ((syscall_cs_base as u64) << 32);
        wrmsr(STAR_MSR, star_value);
        wrmsr(LSTAR_MSR, interrupts::syscalls::syscall_entry as u64);

        let rflags_mask = (1 << 9) | (1 << 8); // IF and TF
        wrmsr(SFMASK_MSR, rflags_mask);
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
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
