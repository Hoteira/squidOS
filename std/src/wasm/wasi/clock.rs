use crate::wasm::{Value, interpreter::Interpreter};

pub fn register(interpreter: &mut Interpreter, mod_name: &str) {
    interpreter.add_host_function(mod_name, "clock_res_get", |interp, args| {
        let res_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        if res_ptr + 8 <= interp.memory.len() {
            interp.memory[res_ptr..res_ptr+8].copy_from_slice(&1_000_000u64.to_le_bytes()); // 1ms resolution
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "clock_time_get", |interp, args| {
        let time_ptr = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let nanos = crate::os::get_system_ticks() * 1_000_000;
        if time_ptr + 8 <= interp.memory.len() { interp.memory[time_ptr..time_ptr+8].copy_from_slice(&nanos.to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "random_get", |interp, args| {
        let buf_ptr = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let buf_len = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        if buf_ptr + buf_len <= interp.memory.len() {
            let mut seed = crate::os::get_system_ticks();
            for i in 0..buf_len { 
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1); 
                interp.memory[buf_ptr + i] = (seed >> 32) as u8; 
            }
        }
        Some(Value::I32(0))
    });
}