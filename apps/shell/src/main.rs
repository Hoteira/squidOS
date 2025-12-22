#![no_std]
#![no_main]

use alloc::format;
use std::{fs, println};
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const STDIN_FD: usize = 0;
const STDOUT_FD: usize = 1;
const STDERR_FD: usize = 2;

fn resolve_path(cwd: &str, path: &str) -> String {
    let trimmed_path = path.trim();
    if trimmed_path.is_empty() { return String::from(cwd); }

    let mut parts = Vec::new();
    
    // If absolute, start fresh, else start with cwd parts
    if trimmed_path.starts_with('@') {
        // Absolute
    } else {
        for part in cwd.split('/') {
            if !part.is_empty() {
                parts.push(part);
            }
        }
    }

    for part in trimmed_path.split('/') {
        if part.is_empty() || part == "." {
            continue;
        } else if part == ".." {
            if parts.len() > 1 { // Prevent popping the drive identifier (e.g. "@0xE0")
                parts.pop();
            }
        } else {
            parts.push(part);
        }
    }
    
    if parts.is_empty() {
         // Should usually have at least the drive. Default to @0xE0 if something went wrong?
         return String::from("@0xE0");
    }

    // Join
    let mut res = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 { res.push('/'); }
        res.push_str(p);
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let heap_size = 1024 * 1024; 
    let heap_ptr = std::memory::malloc(heap_size);
    std::memory::heap::init_heap(heap_ptr as *mut u8, heap_size);

    println!("Shell: Started");

    let msg = "\nWelcome to krakeOS \u{E8F0}\n> ";
    std::os::file_write(STDOUT_FD, msg.as_bytes());

    let mut cwd = String::from("@0xE0");
    let mut cmd_buffer = String::new();

    loop {
        let mut buf = [0u8; 1];
        let n = std::os::file_read(STDIN_FD, &mut buf);
        if n > 0 {
            let b = buf[0];
            let c = b as char;

            if b == b'\r' || b == b'\n' {
                std::os::file_write(STDOUT_FD, b"\n");
                
                let cmd_line = cmd_buffer.trim();
                if !cmd_line.is_empty() {
                    let mut parts = cmd_line.split_whitespace();
                    let cmd = parts.next().unwrap_or("");
                    let arg = parts.next().unwrap_or(""); // Basic single arg support for now

                    if cmd == "help" {
                        std::os::file_write(STDOUT_FD, b"Available commands: help, hello, clear, echo, ls, cd, pwd\n");
                    } else if cmd == "hello" {
                        std::os::file_write(STDOUT_FD, b"Hello from Shell!\n");
                    } else if cmd == "clear" {
                        std::os::file_write(STDOUT_FD, b"\x1B[2J\x1B[H");
                    } else if cmd == "echo" {
                        if cmd_line.len() > 5 {
                            std::os::file_write(STDOUT_FD, cmd_line[5..].as_bytes());
                        }
                        std::os::file_write(STDOUT_FD, b"\n");
                    } else if cmd == "pwd" {
                         std::os::file_write(STDOUT_FD, cwd.as_bytes());
                         std::os::file_write(STDOUT_FD, b"\n");
                    } else if cmd == "ls" {
                        let target = if arg.is_empty() { &cwd } else { arg };
                        let full_path = resolve_path(&cwd, target);
                        
                        match std::fs::read_dir(&full_path) {
                            Ok(entries) => {
                                for entry in entries {
                                    let mut line = String::new();
                                    line.push_str("  "); // 2 spaces padding
                                    
                                    if entry.file_type == std::fs::FileType::Directory {
                                        line.push_str("\x1B[1;94m"); // Bold Bright Blue (Light Blue)
                                        line.push_str("\u{F07B} "); // Closed Folder Icon
                                        line.push_str(&entry.name);
                                        line.push('/');
                                        line.push_str("\x1B[0m"); // Reset
                                    } else {
                                        line.push_str("\x1B[37m"); // White
                                        line.push_str("\u{F016} "); // File Icon
                                        line.push_str(&entry.name);
                                        line.push_str("\x1B[0m"); // Reset
                                    }
                                    line.push('\n');
                                    std::os::file_write(STDOUT_FD, line.as_bytes());
                                }
                            },
                            Err(_) => {
                                let err = format!("ls: cannot access '{}': No such file or directory\n", full_path);
                                std::os::file_write(STDOUT_FD, err.as_bytes());
                            }
                        }
                    } else if cmd == "cd" {
                        if arg.is_empty() {
                             // Default to root? or do nothing?
                             // let's do nothing or go to root @0xE0
                             cwd = String::from("@0xE0");
                        } else {
                            let new_path = resolve_path(&cwd, arg);
                            // Verify existence by trying to read dir (simple check)
                            if std::fs::read_dir(&new_path).is_ok() {
                                cwd = new_path;
                            } else {
                                let err = format!("cd: {}: No such file or directory\n", new_path);
                                std::os::file_write(STDOUT_FD, err.as_bytes());
                            }
                        }
                    } else {
                         // External command execution
                         let prog_path = if cmd.starts_with('@') {
                             String::from(cmd)
                         } else {
                             // Search in PATH (@0xE0/sys/bin/)
                             // Assume .elf extension if not present? Requested: "without extension (assumes they are elfs)"
                             let mut p = String::from("@0xE0/sys/bin/");
                             p.push_str(cmd);
                             if !cmd.ends_with(".elf") {
                                 p.push_str(".elf");
                             }
                             p
                         };

                         // Try to spawn
                         // We need to inherit FDs? std::os::spawn usually inherits (or we implemented spawn_with_fds)
                         // The standard `spawn` uses syscall 66 which I implemented to inherit FDs by default if no map passed.
                         // Wait, check syscall 66 implementation in kernel/src/interrupts/syscalls.rs: 
                         // "Inherit all if no map provided" -> Yes.
                         
                         let pid = std::os::spawn(&prog_path);
                         if pid != usize::MAX {
                             std::os::waitpid(pid);
                             std::os::file_write(STDOUT_FD, b"\n[Process Finished]\n");
                         } else {
                             let err = format!("Unknown command: {}\n", cmd);
                             std::os::file_write(STDOUT_FD, err.as_bytes());
                         }
                    }
                }
                
                cmd_buffer.clear();
                std::os::file_write(STDOUT_FD, b"> ");
            } else if b == 0x08 || b == 0x7F { // Backspace
                if !cmd_buffer.is_empty() {
                    cmd_buffer.pop();
                    std::os::file_write(STDOUT_FD, b"\x08 \x08"); 
                }
            } else if c >= ' ' && c != '\x7F' { // Filter non-printable
                cmd_buffer.push(c);
                std::os::file_write(STDOUT_FD, &[b]); // Echo
            }
        } else {
            std::os::yield_task();
        }
    }
}