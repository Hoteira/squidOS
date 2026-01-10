use crate::wasm::{Value, interpreter::Interpreter};
use crate::rust_alloc::vec::Vec;
use crate::rust_alloc::string::{String, ToString};
use crate::{print, debugln};

pub struct Wasi;

impl Wasi {
    pub fn register(interpreter: &mut Interpreter) {
        let mod_name = "wasi_snapshot_preview1";

        // --- Process & Environment ---

        interpreter.add_host_function(mod_name, "args_sizes_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "args_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "environ_sizes_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "environ_get", |_interp, _args| Some(Value::I32(0)));

        interpreter.add_host_function(mod_name, "proc_exit", |_interp, args| {
            let code = match args.get(0) { Some(Value::I32(v)) => *v as u64, _ => 0 };
            crate::os::exit(code);
            None
        });

        interpreter.add_host_function(mod_name, "proc_raise", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "sched_yield", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });

        // --- Clocks & Random ---

        interpreter.add_host_function(mod_name, "clock_res_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "clock_time_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "random_get", |_interp, _args| Some(Value::I32(0)));

        // --- Filesystem: File Descriptors ---

        interpreter.add_host_function(mod_name, "fd_close", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_datasync", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "fd_sync", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "fd_fdstat_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_fdstat_set_flags", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "fd_fdstat_set_rights", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "fd_filestat_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_filestat_set_size", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_filestat_set_times", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "fd_pread", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "fd_pwrite", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "fd_read", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_readdir", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_renumber", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "fd_seek", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_tell", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_write", |_interp, _args| Some(Value::I32(0)));

        // --- Filesystem: Path Operations ---

        interpreter.add_host_function(mod_name, "path_create_directory", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "path_filestat_get", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "path_filestat_set_times", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(0)) });
        interpreter.add_host_function(mod_name, "path_link", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "path_open", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "path_readlink", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "path_remove_directory", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "path_rename", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "path_symlink", |_interp, _args| Some(Value::I32(58)));
        interpreter.add_host_function(mod_name, "path_unlink_file", |_interp, _args| Some(Value::I32(0)));

        // --- Polling & Sockets ---

        interpreter.add_host_function(mod_name, "poll_oneoff", |_interp, _args| Some(Value::I32(0)));
        interpreter.add_host_function(mod_name, "fd_prestat_get", |_interp, _args| Some(Value::I32(8)));
        interpreter.add_host_function(mod_name, "fd_prestat_dir_name", |_interp, _args| Some(Value::I32(8)));
        interpreter.add_host_function(mod_name, "sock_recv", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
        interpreter.add_host_function(mod_name, "sock_send", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
        interpreter.add_host_function(mod_name, "sock_shutdown", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
    }
}