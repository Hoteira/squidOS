use core::arch::naked_asm;
use crate::println;

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "xor rbp, rbp",      // Mark outermost frame
        "mov rdi, rsp",      // Original stack pointer (points to argc)
        "and rsp, -16",      // 16-byte alignment
        "call rust_start",
        "1: hlt",            // Should not be reached
        "jmp 1b",
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_start(stack: *const usize) -> ! {
    let heap_size = 10 * 1024 * 1024; // 10 MiB
    let heap_ptr = crate::memory::malloc(heap_size);
    if heap_ptr == 0 || heap_ptr == usize::MAX {
        crate::os::exit(1);
    }
    crate::memory::heap::init_heap(heap_ptr as *mut u8, heap_size);

    crate::os::print("[DEBUG] Runtime started\n");

    unsafe extern "C" {
        fn main(argc: i32, argv: *const *const u8, envp: *const *const u8) -> i32;
    }

    let argc = *stack as i32;
    let argv = stack.add(1) as *const *const u8;
    let envp = stack.add(argc as usize + 2) as *const *const u8;

    let result = main(argc, argv, envp);
    crate::os::exit(result as u64);
}
