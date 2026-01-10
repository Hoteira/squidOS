use crate::wasm::{Value, interpreter::Interpreter};

pub fn register(interpreter: &mut Interpreter, mod_name: &str) {
    interpreter.add_host_function(mod_name, "poll_oneoff", |_interp, _args| Some(Value::I32(0)));

    interpreter.add_host_function(mod_name, "sock_recv", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
    interpreter.add_host_function(mod_name, "sock_send", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
    interpreter.add_host_function(mod_name, "sock_shutdown", |_interp, _args| { crate::os::yield_task(); Some(Value::I32(58)) });
}