use core::arch::{asm, naked_asm};
use crate::interrupts::task::CPUState;
use crate::debugln;
use crate::drivers::periferics::keyboard::KEYBOARD_BUFFER; // Import keyboard buffer for SYS_READ

// Syscall Numbers
pub const SYS_READ: u64 = 0;
pub const SYS_PRINT: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_EXIT: u64 = 60;

#[unsafe(naked)]
pub extern "C" fn int80_handler(_stack_frame: *mut crate::interrupts::exceptions::StackFrame) {
    unsafe {
        naked_asm!(

            
            "push rbp",
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",

            "mov rdi, rsp", 
            "call syscall_dispatcher",

            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            "pop rbp",

            "iretq"
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatcher(context: &mut CPUState) {
    
    let syscall_num = context.rax;

    match syscall_num {
        SYS_READ => {
            let user_ptr = context.rdi as *mut u8;
            let user_len = context.rsi as usize;
            let mut bytes_read = 0;


            if user_ptr.is_null() {
                context.rax = 0;
                return;
            }

            let mut keyboard_buffer = KEYBOARD_BUFFER.lock();
            while bytes_read < user_len {
                if let Some(char_code) = keyboard_buffer.pop_front() {

                    unsafe {
                        *user_ptr.add(bytes_read) = char_code as u8;
                    }
                    bytes_read += 1;
                } else {

                    break;
                }
            }
            context.rax = bytes_read as u64;
        }

        SYS_PRINT => {
            let ptr = context.rdi as *const u8;
            let len = context.rsi as usize;

            if ptr as u64 >= 0x8000000000 {
                let s = unsafe { core::slice::from_raw_parts(ptr, len) };
                let str_val = String::from_utf8_lossy(s);

                crate::debug_print!("{}", str_val);
            }
            context.rax = len as u64;
        }
        SYS_YIELD => {
             unsafe {
                 asm!("int 32"); // Trigger timer interrupt to force a context switch
             }
             context.rax = 0;
        }
        SYS_EXIT => {
            debugln!("[Syscall] Process exited with code {}", context.rdi + 0);

            loop { unsafe { asm!("hlt") } }
        }
        _ => {
            debugln!("[Syscall] Unknown syscall #{}", syscall_num);
            context.rax = u64::MAX;
        }
    }
}

use alloc::string::String;
use alloc::string::ToString;