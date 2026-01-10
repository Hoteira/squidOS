use crate::wasm::{Value, interpreter::Interpreter};
use crate::rust_alloc::string::{String, ToString};
use crate::rust_alloc::vec::Vec;

pub fn register(interpreter: &mut Interpreter, mod_name: &str) {
    interpreter.add_host_function(mod_name, "fd_close", |_interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        crate::os::file_close(fd);
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_datasync", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
    interpreter.add_host_function(mod_name, "fd_sync", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
    
    interpreter.add_host_function(mod_name, "fd_fdstat_get", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let stat_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        if stat_ptr + 24 <= interp.memory.len() {
            for j in 0..24 { interp.memory[stat_ptr + j] = 0; }
            let rights: u64 = 0xFFFFFFFFFFFFFFFF;
            interp.memory[stat_ptr+8..stat_ptr+16].copy_from_slice(&rights.to_le_bytes());
            interp.memory[stat_ptr+16..stat_ptr+24].copy_from_slice(&rights.to_le_bytes());
            interp.memory[stat_ptr] = if fd <= 2 { 2 } else { 4 };
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_fdstat_set_flags", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
    interpreter.add_host_function(mod_name, "fd_fdstat_set_rights", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });

    interpreter.add_host_function(mod_name, "fd_filestat_get", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let stat_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let size = unsafe { crate::os::syscall(5, fd as u64, 0, 0) };
        if stat_ptr + 64 <= interp.memory.len() {
            for j in 0..64 { interp.memory[stat_ptr + j] = 0; }
            interp.memory[stat_ptr+32..stat_ptr+40].copy_from_slice(&size.to_le_bytes());
            interp.memory[stat_ptr+16] = if fd <= 2 { 2 } else { 4 };
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_filestat_set_size", |_interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let size = match args[1] { Value::I64(v) => v as u64, _ => return Some(Value::I32(28)) };
        let res = crate::os::file_truncate(fd, size);
        Some(Value::I32(if res == 0 { 0 } else { 1 }))
    });

    interpreter.add_host_function(mod_name, "fd_filestat_set_times", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });

    interpreter.add_host_function(mod_name, "fd_pread", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let offset = match args[3] { Value::I64(v) => v, _ => return Some(Value::I32(28)) };
        let nread_ptr = match args[4] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let old_off = crate::os::file_seek(fd, 0, 1);
        crate::os::file_seek(fd, offset, 0);
        let mut total_read = 0;
        for i in 0..iovs_len {
            let base_ptr = iovs_ptr + (i * 8);
            if base_ptr + 8 > interp.memory.len() { break; }
            let buf_ptr = u32::from_le_bytes(interp.memory[base_ptr..base_ptr+4].try_into().unwrap()) as usize;
            let buf_len = u32::from_le_bytes(interp.memory[base_ptr+4..base_ptr+8].try_into().unwrap()) as usize;
            if buf_ptr + buf_len > interp.memory.len() { break; }
            let n = crate::os::file_read(fd, &mut interp.memory[buf_ptr..buf_ptr+buf_len]);
            if n == usize::MAX || n == 0 { break; }
            total_read += n;
        }
        crate::os::file_seek(fd, old_off as i64, 0);
        if nread_ptr + 4 <= interp.memory.len() { interp.memory[nread_ptr..nread_ptr+4].copy_from_slice(&(total_read as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_pwrite", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let offset = match args[3] { Value::I64(v) => v, _ => return Some(Value::I32(28)) };
        let nwritten_ptr = match args[4] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let old_off = crate::os::file_seek(fd, 0, 1);
        crate::os::file_seek(fd, offset, 0);
        let mut total_written = 0;
        for i in 0..iovs_len {
            let base_ptr = iovs_ptr + (i * 8);
            if base_ptr + 8 > interp.memory.len() { break; }
            let buf_ptr = u32::from_le_bytes(interp.memory[base_ptr..base_ptr+4].try_into().unwrap()) as usize;
            let buf_len = u32::from_le_bytes(interp.memory[base_ptr+4..base_ptr+8].try_into().unwrap()) as usize;
            if buf_ptr + buf_len > interp.memory.len() { break; }
            let n = crate::os::file_write(fd, &interp.memory[buf_ptr..buf_ptr+buf_len]);
            if n == usize::MAX { break; }
            total_written += n;
        }
        crate::os::file_seek(fd, old_off as i64, 0);
        if nwritten_ptr + 4 <= interp.memory.len() { interp.memory[nwritten_ptr..nwritten_ptr+4].copy_from_slice(&(total_written as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_read", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let nread_ptr = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let mut total_read = 0;
        for i in 0..iovs_len {
            let base_ptr = iovs_ptr + (i * 8);
            if base_ptr + 8 > interp.memory.len() { break; }
            let buf_ptr = u32::from_le_bytes(interp.memory[base_ptr..base_ptr+4].try_into().unwrap()) as usize;
            let buf_len = u32::from_le_bytes(interp.memory[base_ptr+4..base_ptr+8].try_into().unwrap()) as usize;
            if buf_ptr + buf_len > interp.memory.len() { break; }
            let n = crate::os::file_read(fd, &mut interp.memory[buf_ptr..buf_ptr+buf_len]);
            if n == usize::MAX || n == 0 { break; }
            total_read += n;
            if n < buf_len { break; }
        }
        if nread_ptr + 4 <= interp.memory.len() { interp.memory[nread_ptr..nread_ptr+4].copy_from_slice(&(total_read as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_readdir", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let buf_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let buf_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let _cookie = match args[3] { Value::I64(v) => v as usize, _ => 0 };
        let nused_ptr = match args[4] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let mut krake_buf = [0u8; 512];
        let res = unsafe { crate::os::syscall(78, fd as u64, krake_buf.as_mut_ptr() as u64, 512) };
        if res == u64::MAX || res == 0 {
            if nused_ptr + 4 <= interp.memory.len() { interp.memory[nused_ptr..nused_ptr+4].copy_from_slice(&0u32.to_le_bytes()); }
            return Some(Value::I32(0));
        }
        let mut wasi_used = 0;
        let mut krake_offset = 0;
        let krake_bytes = res as usize;
        while krake_offset + 2 < krake_bytes && wasi_used + 24 < buf_len {
            let d_type = krake_buf[krake_offset];
            let name_len = krake_buf[krake_offset + 1] as usize;
            let name = &krake_buf[krake_offset + 2..krake_offset + 2 + name_len];
            if wasi_used + 24 + name_len > buf_len { break; }
            let next_cookie = (wasi_used + 24 + name_len) as u64;
            interp.memory[buf_ptr + wasi_used..buf_ptr + wasi_used + 8].copy_from_slice(&next_cookie.to_le_bytes());
            interp.memory[buf_ptr + wasi_used + 8..buf_ptr + wasi_used + 16].copy_from_slice(&0u64.to_le_bytes());
            interp.memory[buf_ptr + wasi_used + 16..buf_ptr + wasi_used + 20].copy_from_slice(&(name_len as u32).to_le_bytes());
            interp.memory[buf_ptr + wasi_used + 20] = match d_type { 1 => 8, 2 => 4, _ => 0 };
            interp.memory[buf_ptr + wasi_used + 24..buf_ptr + wasi_used + 24 + name_len].copy_from_slice(name);
            wasi_used += 24 + name_len;
            krake_offset += 2 + name_len;
        }
        if nused_ptr + 4 <= interp.memory.len() { interp.memory[nused_ptr..nused_ptr+4].copy_from_slice(&(wasi_used as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_renumber", |_interp, _args| Some(Value::I32(58)));

    interpreter.add_host_function(mod_name, "fd_seek", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let offset = match args[1] { Value::I64(v) => v, _ => return Some(Value::I32(28)) };
        let whence = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let newoff_ptr = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let res = crate::os::file_seek(fd, offset, whence);
        if res == u64::MAX { return Some(Value::I32(29)); }
        if newoff_ptr + 8 <= interp.memory.len() { interp.memory[newoff_ptr..newoff_ptr+8].copy_from_slice(&res.to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_tell", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let offset_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let res = crate::os::file_seek(fd, 0, 1);
        if offset_ptr + 8 <= interp.memory.len() { interp.memory[offset_ptr..offset_ptr+8].copy_from_slice(&res.to_le_bytes()); }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "fd_write", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let iovs_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let nwritten_ptr = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let mut total_written = 0;
        for i in 0..iovs_len {
            let base_ptr = iovs_ptr + (i * 8);
            if base_ptr + 8 > interp.memory.len() { return Some(Value::I32(21)); }
            let buf_ptr = u32::from_le_bytes(interp.memory[base_ptr..base_ptr+4].try_into().unwrap()) as usize;
            let buf_len = u32::from_le_bytes(interp.memory[base_ptr+4..base_ptr+8].try_into().unwrap()) as usize;
            if buf_ptr + buf_len > interp.memory.len() { return Some(Value::I32(21)); }
            let data = &interp.memory[buf_ptr..buf_ptr+buf_len];
            if fd == 1 || fd == 2 {
                if let Ok(s) = core::str::from_utf8(data) { crate::os::debug_print(s); }
                total_written += buf_len;
            } else {
                let n = crate::os::file_write(fd, data);
                total_written += n;
            }
        }
        if nwritten_ptr + 4 <= interp.memory.len() { interp.memory[nwritten_ptr..nwritten_ptr+4].copy_from_slice(&(total_written as u32).to_le_bytes()); }
        Some(Value::I32(0))
    });

    // --- Path Operations ---

    interpreter.add_host_function(mod_name, "path_create_directory", |interp, args| {
        let path_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_raw = &interp.memory[path_ptr..path_ptr+path_len];
        let path = core::str::from_utf8(path_raw).unwrap_or("");
        let mut krake_path = String::from(path);
        if krake_path.starts_with('/') { krake_path = String::from("@0xE0") + &krake_path; }
        else if !krake_path.starts_with('@') { krake_path = String::from("@0xE0/") + &krake_path; }
        let res = unsafe { crate::os::syscall(83, krake_path.as_ptr() as u64, krake_path.len() as u64, 0) };
        Some(Value::I32(if res == 0 { 0 } else { 1 }))
    });

    interpreter.add_host_function(mod_name, "path_filestat_get", |interp, args| {
        let path_ptr = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_len = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let stat_ptr = match args[4] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_raw = &interp.memory[path_ptr..path_ptr+path_len];
        let path_str = core::str::from_utf8(path_raw).unwrap_or("");
        let mut krake_path = String::from(path_str);
        if krake_path.starts_with('/') { krake_path = String::from("@0xE0") + &krake_path; }
        else if !krake_path.starts_with('@') { krake_path = String::from("@0xE0/") + &krake_path; }
        let fd = unsafe { crate::os::syscall(2, krake_path.as_ptr() as u64, krake_path.len() as u64, 0) };
        if fd == u64::MAX { return Some(Value::I32(44)); }
        let size = unsafe { crate::os::syscall(5, fd, 0, 0) };
        unsafe { crate::os::syscall(3, fd, 0, 0); }
        if stat_ptr + 64 <= interp.memory.len() {
            for j in 0..64 { interp.memory[stat_ptr + j] = 0; }
            interp.memory[stat_ptr+32..stat_ptr+40].copy_from_slice(&size.to_le_bytes());
            interp.memory[stat_ptr+16] = 4;
        }
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "path_filestat_set_times", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
    interpreter.add_host_function(mod_name, "path_link", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
    
    interpreter.add_host_function(mod_name, "path_open", |interp, args| {
        let path_ptr = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_len = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let oflags = match args[4] { Value::I32(v) => v, _ => return Some(Value::I32(28)) };
        let rights_base = match args[5] { Value::I64(v) => v, _ => 0 };
        let opened_fd_ptr = match args[8] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_raw = &interp.memory[path_ptr..path_ptr+path_len];
        let path_str = core::str::from_utf8(path_raw).unwrap_or("");
        let mut krake_path = String::from(path_str);
        if krake_path == "." || krake_path == "" { krake_path = String::from("@0xE0/"); }
        else if krake_path.starts_with('/') { krake_path = String::from("@0xE0") + &krake_path; }
        else if !krake_path.starts_with('@') { krake_path = String::from("@0xE0/") + &krake_path; }
                    let mut krake_flags = 0;
                    if (rights_base & 64) != 0 { krake_flags = 2; }
                    if (oflags & 1) != 0 { unsafe { crate::os::syscall(85, krake_path.as_ptr() as u64, krake_path.len() as u64, 0); } }
                    
                    let fd = unsafe { crate::os::syscall(2, krake_path.as_ptr() as u64, krake_path.len() as u64, krake_flags) };
                    if fd == u64::MAX { return Some(Value::I32(44)); }
        
                    if (oflags & 8) != 0 { // O_TRUNC
                        unsafe { crate::os::syscall(77, fd, 0, 0); } // SYS_FTRUNCATE to 0
                    }
        
                    if opened_fd_ptr + 4 <= interp.memory.len() { interp.memory[opened_fd_ptr..opened_fd_ptr+4].copy_from_slice(&(fd as u32).to_le_bytes()); }
        
        Some(Value::I32(0))
    });

    interpreter.add_host_function(mod_name, "path_readlink", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });

    interpreter.add_host_function(mod_name, "path_remove_directory", |interp, args| {
        let path_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_raw = &interp.memory[path_ptr..path_ptr+path_len];
        let path = core::str::from_utf8(path_raw).unwrap_or("");
        let mut krake_path = String::from(path);
        if krake_path.starts_with('/') { krake_path = String::from("@0xE0") + &krake_path; }
        else if !krake_path.starts_with('@') { krake_path = String::from("@0xE0/") + &krake_path; }
        let res = unsafe { crate::os::syscall(87, krake_path.as_ptr() as u64, krake_path.len() as u64, 0) };
        Some(Value::I32(if res == 0 { 0 } else { 1 }))
    });

    interpreter.add_host_function(mod_name, "path_rename", |interp, args| {
        let old_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let old_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let new_ptr = match args[3] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let new_len = match args[4] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let mut old_path = core::str::from_utf8(&interp.memory[old_ptr..old_ptr+old_len]).unwrap_or("").to_string();
        let mut new_path = core::str::from_utf8(&interp.memory[new_ptr..new_ptr+new_len]).unwrap_or("").to_string();
        if old_path.starts_with('/') { old_path = String::from("@0xE0") + &old_path; }
        if new_path.starts_with('/') { new_path = String::from("@0xE0") + &new_path; }
        let res = unsafe { crate::os::syscall4(82, old_path.as_ptr() as u64, old_path.len() as u64, new_path.as_ptr() as u64, new_path.len() as u64) };
        Some(Value::I32(if res == 0 { 0 } else { 1 }))
    });

    interpreter.add_host_function(mod_name, "path_symlink", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });

    interpreter.add_host_function(mod_name, "path_unlink_file", |interp, args| {
        let path_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_len = match args[2] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        let path_raw = &interp.memory[path_ptr..path_ptr+path_len];
        let path = core::str::from_utf8(path_raw).unwrap_or("");
        let mut krake_path = String::from(path);
        if krake_path.starts_with('/') { krake_path = String::from("@0xE0") + &krake_path; }
        else if !krake_path.starts_with('@') { krake_path = String::from("@0xE0/") + &krake_path; }
        let res = unsafe { crate::os::syscall(87, krake_path.as_ptr() as u64, krake_path.len() as u64, 0) };
        Some(Value::I32(if res == 0 { 0 } else { 1 }))
    });

    // --- Pre-opened Directories ---

    interpreter.add_host_function(mod_name, "fd_prestat_get", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v, _ => return Some(Value::I32(28)) };
        let prestat_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        if fd == 3 {
            if prestat_ptr + 8 < interp.memory.len() { interp.memory[prestat_ptr] = 0; interp.memory[prestat_ptr+4..prestat_ptr+8].copy_from_slice(&1u32.to_le_bytes()); }
            return Some(Value::I32(0));
        }
        Some(Value::I32(8))
    });

    interpreter.add_host_function(mod_name, "fd_prestat_dir_name", |interp, args| {
        let fd = match args[0] { Value::I32(v) => v, _ => return Some(Value::I32(28)) };
        let path_ptr = match args[1] { Value::I32(v) => v as usize, _ => return Some(Value::I32(28)) };
        if fd == 3 && path_ptr < interp.memory.len() { interp.memory[path_ptr] = b'/'; return Some(Value::I32(0)); }
        Some(Value::I32(8))
    });
}