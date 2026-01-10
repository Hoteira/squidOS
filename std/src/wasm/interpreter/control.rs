use crate::wasm::{WasmModule, Value, ValueType, interpreter::{Interpreter, ControlFrame}};
use crate::wasm::leb128::Leb128;
use crate::rust_alloc::vec::Vec;

impl Interpreter {
    pub(crate) fn execute_control(&mut self, opcode: u8, pc: &mut usize, bytecode: &[u8], control_stack: &mut Vec<ControlFrame>, module: &WasmModule, locals: &mut Vec<Value>) -> Result<(), &'static str> {
        match opcode {
            0x02 | 0x03 | 0x04 => { // block | loop | if
                let bt_raw = Leb128::decode_i64(bytecode, pc);
                let rt = match bt_raw {
                    -1 => Some(ValueType::I32),
                    -2 => Some(ValueType::I64),
                    -3 => Some(ValueType::F32),
                    -4 => Some(ValueType::F64),
                    _ => None,
                };
                let frame = ControlFrame { opcode, pc: *pc, stack_depth: self.stack.len(), result_type: rt };
                if opcode == 0x04 {
                    if self.pop_i32()? == 0 {
                        let (else_pc, end_pc) = self.find_block_ends(*pc, bytecode)?;
                        if let Some(epc) = else_pc {
                            *pc = epc + 1;
                            control_stack.push(frame);
                        } else {
                            *pc = end_pc;
                        }
                    } else {
                        control_stack.push(frame);
                    }
                } else {
                    control_stack.push(frame);
                }
            }
            0x05 => { // else
                let (_, end_pc) = self.find_block_ends(*pc, bytecode)?;
                *pc = end_pc - 1; // Put PC at 0x0B
            }
            0x0C | 0x0D => { // br | br_if
                let label = Leb128::decode_u32(bytecode, pc);
                if opcode == 0x0C || (match self.stack.pop() { Some(Value::I32(v)) => v != 0, _ => false }) {
                    let frame_idx = control_stack.len().checked_sub(1 + label as usize).ok_or("br OOB")?;
                    let frame = control_stack[frame_idx].clone();
                    if frame.opcode == 0x03 { // loop
                        *pc = frame.pc; 
                        self.stack.truncate(frame.stack_depth);
                    } else if frame.opcode == 0x00 { // function body
                        *pc = bytecode.len();
                        if let Some(_rt) = frame.result_type {
                            let res = self.stack.pop().ok_or("br res missing (func)")?;
                            self.stack.truncate(frame.stack_depth);
                            self.stack.push(res);
                        } else { self.stack.truncate(frame.stack_depth); }
                        control_stack.truncate(frame_idx);
                    } else { // block/if
                        let (_, end_pc) = self.find_block_ends(frame.pc, bytecode)?;
                        *pc = end_pc;
                        if let Some(_rt) = frame.result_type {
                            let res = self.stack.pop().ok_or("br res missing (block)")?;
                            self.stack.truncate(frame.stack_depth);
                            self.stack.push(res);
                        } else { self.stack.truncate(frame.stack_depth); }
                        control_stack.truncate(frame_idx);
                    }
                }
            }
            0x0E => { // br_table
                let count = Leb128::decode_u32(bytecode, pc);
                let mut targets = Vec::new();
                for _ in 0..count { targets.push(Leb128::decode_u32(bytecode, pc)); }
                let default_target = Leb128::decode_u32(bytecode, pc);
                let index = self.pop_i32()? as usize;
                let label = if index < targets.len() { targets[index] } else { default_target };
                
                let frame_idx = control_stack.len().checked_sub(1 + label as usize).ok_or("br_table OOB")?;
                let frame = control_stack[frame_idx].clone();
                if frame.opcode == 0x03 { // loop
                    *pc = frame.pc; 
                    self.stack.truncate(frame.stack_depth);
                } else if frame.opcode == 0x00 { // function body
                    *pc = bytecode.len();
                    if let Some(_rt) = frame.result_type {
                        let res = self.stack.pop().ok_or("br_table res missing (func)")?;
                        self.stack.truncate(frame.stack_depth);
                        self.stack.push(res);
                    } else { self.stack.truncate(frame.stack_depth); }
                    control_stack.truncate(frame_idx);
                } else { // block/if
                    let (_, end_pc) = self.find_block_ends(frame.pc, bytecode)?;
                    *pc = end_pc;
                    if let Some(_rt) = frame.result_type {
                        let res = self.stack.pop().ok_or("br_table res missing (block)")?;
                        self.stack.truncate(frame.stack_depth);
                        self.stack.push(res);
                    } else { self.stack.truncate(frame.stack_depth); }
                    control_stack.truncate(frame_idx);
                }
            }
            0x0F => { *pc = bytecode.len(); control_stack.clear(); }
            0x10 => { // call
                let idx = Leb128::decode_u32(bytecode, pc);
                let arg_count = self.get_arg_count(module, idx)?;
                let mut call_args = Vec::new();
                for _ in 0..arg_count { call_args.push(self.stack.pop().ok_or("underflow")?); }
                call_args.reverse();
                let res = self.call(module, idx, call_args)?;
                let type_idx = self.get_type_idx(module, idx)?;
                if !module.types[type_idx as usize].results.is_empty() { self.stack.push(res); }
            }
            0x11 => { // call_indirect
                let type_idx = Leb128::decode_u32(bytecode, pc);
                let _table_idx = Leb128::decode_u32(bytecode, pc);
                let entry_idx = self.pop_i32()? as usize;
                let func_idx = *self.tables[0].get(entry_idx).ok_or("Table OOB")?;
                if func_idx == 0xFFFFFFFF { return Err("Indirect call to null"); }
                if self.get_type_idx(module, func_idx)? != type_idx { return Err("Type mismatch"); }
                let arg_count = self.get_arg_count(module, func_idx)?;
                let mut call_args = Vec::new();
                for _ in 0..arg_count { call_args.push(self.stack.pop().ok_or("underflow")?); }
                call_args.reverse();
                let res = self.call(module, func_idx, call_args)?;
                let actual_type_idx = self.get_type_idx(module, func_idx)?;
                if !module.types[actual_type_idx as usize].results.is_empty() { self.stack.push(res); }
            }
            _ => return Err("Invalid control opcode"),
        }
        Ok(())
    }
}
