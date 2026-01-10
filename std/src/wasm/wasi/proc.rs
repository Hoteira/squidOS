use crate::wasm::{Value, interpreter::Interpreter};
use crate::rust_alloc::string::String;
use crate::rust_alloc::vec::Vec;

pub fn register(interpreter: &mut Interpreter, mod_name: &str) {
    interpreter.add_host_function(mod_name, "args_sizes_get", |interp, args| {
        let argc_ptr = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let buf_size_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };

        let count = crate::env::args().count();
        let mut size = 0;
        for arg in crate::env::args() {
            size += arg.as_bytes().len() + 1;
        }

        if argc_ptr + 4 <= interp.memory.len() { interp.memory[argc_ptr..argc_ptr+4].copy_from_slice(&(count as u32).to_le_bytes()); }
        if buf_size_ptr + 4 <= interp.memory.len() { interp.memory[buf_size_ptr..buf_size_ptr+4].copy_from_slice(&(size as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "args_get", |interp, args| {
        let argv_ptr = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let argv_buf_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        
        let mut offset = 0;
        let mut argv_offset = 0;
        for arg in crate::env::args() {
            let bytes = arg.as_bytes();
            let len = bytes.len();
            
            if argv_buf_ptr + offset + len + 1 > interp.memory.len() { return Some(Value::I32(28)); }
            interp.memory[argv_buf_ptr + offset..argv_buf_ptr + offset + len].copy_from_slice(bytes);
            interp.memory[argv_buf_ptr + offset + len] = 0;
            
            if argv_ptr + argv_offset + 4 > interp.memory.len() { return Some(Value::I32(28)); }
            let ptr = (argv_buf_ptr + offset) as u32;
            interp.memory[argv_ptr + argv_offset..argv_ptr + argv_offset + 4].copy_from_slice(&ptr.to_le_bytes());
            
            offset += len + 1;
            argv_offset += 4;
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "environ_sizes_get", |interp, args| {
        let count_ptr = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let size_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };

        let mut count = 0;
        let mut size = 0;
        for (k, v) in crate::env::vars() {
            count += 1;
            size += k.len() + v.len() + 2; // key=val\0
        }

        if count_ptr + 4 <= interp.memory.len() { interp.memory[count_ptr..count_ptr+4].copy_from_slice(&(count as u32).to_le_bytes()); }
        if size_ptr + 4 <= interp.memory.len() { interp.memory[size_ptr..size_ptr+4].copy_from_slice(&(size as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "environ_get", |interp, args| {
        let environ_ptr = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let environ_buf_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };

        let mut offset = 0;
        let mut env_offset = 0;
        for (k, v) in crate::env::vars() {
            let s = crate::rust_alloc::format!("{}={}", k, v);
            let bytes = s.as_bytes();
            let len = bytes.len();

            if environ_buf_ptr + offset + len + 1 > interp.memory.len() { return Some(Value::I32(28)); }
            interp.memory[environ_buf_ptr + offset..environ_buf_ptr + offset + len].copy_from_slice(bytes);
            interp.memory[environ_buf_ptr + offset + len] = 0;

            if environ_ptr + env_offset + 4 > interp.memory.len() { return Some(Value::I32(28)); }
            let ptr = (environ_buf_ptr + offset) as u32;
            interp.memory[environ_ptr + env_offset..environ_ptr + env_offset + 4].copy_from_slice(&ptr.to_le_bytes());

            offset += len + 1;
            env_offset += 4;
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "proc_exit", |_interp, args| {
        let code = match args.get(0) { Some(Value::I32(v)) => *v as u64, _ => 0 };
        crate::os::exit(code);
        None
    });

    interpreter.add_host_function(mod_name, "proc_raise", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
    interpreter.add_host_function(mod_name, "sched_yield", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
}
