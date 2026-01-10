use crate::wasm::{Value, interpreter::Interpreter};
use crate::wasm::leb128::Leb128;
use crate::rust_alloc::vec::Vec;

impl Interpreter {
    pub(crate) fn execute_variable(&mut self, opcode: u8, pc: &mut usize, bytecode: &[u8], locals: &mut Vec<Value>) -> Result<(), &'static str> {
        match opcode {
            0x1A => { self.stack.pop().ok_or("pop error")?; }
            0x1B => { 
                let c = self.pop_i32()?; 
                let v2 = self.stack.pop().ok_or("select v2 missing")?; 
                let v1 = self.stack.pop().ok_or("select v1 missing")?; 
                self.stack.push(if c != 0 { v1 } else { v2 }); 
            }
            0x1C => {
                let count = Leb128::decode_u32(bytecode, pc);
                for _ in 0..count { Leb128::decode_u32(bytecode, pc); } // skip types
                let c = self.pop_i32()?;
                let v2 = self.stack.pop().ok_or("select v2 missing")?;
                let v1 = self.stack.pop().ok_or("select v1 missing")?;
                self.stack.push(if c != 0 { v1 } else { v2 });
            }
            0x20 => { 
                let idx = Leb128::decode_u32(bytecode, pc); 
                self.stack.push(*locals.get(idx as usize).ok_or("local OOB")?); 
            }
            0x21 => { 
                let idx = Leb128::decode_u32(bytecode, pc); 
                *locals.get_mut(idx as usize).ok_or("local OOB")? = self.stack.pop().ok_or("pop error")?; 
            }
            0x22 => { 
                let idx = Leb128::decode_u32(bytecode, pc); 
                *locals.get_mut(idx as usize).ok_or("local OOB")? = *self.stack.last().ok_or("pop error")?; 
            }
            0x23 => { 
                let idx = Leb128::decode_u32(bytecode, pc); 
                self.stack.push(*self.globals.get(idx as usize).ok_or("global OOB")?); 
            }
            0x24 => { 
                let idx = Leb128::decode_u32(bytecode, pc); 
                *self.globals.get_mut(idx as usize).ok_or("global OOB")? = self.stack.pop().ok_or("pop error")?; 
            }
            0x25 => {
                let idx = Leb128::decode_u32(bytecode, pc) as usize;
                let entry_idx = self.pop_i32()? as usize;
                if idx >= self.tables.len() { return Err("table.get OOB table index"); }
                let func_idx = *self.tables[idx].get(entry_idx).ok_or("table.get OOB entry")?;
                self.stack.push(Value::I32(func_idx as i32));
            }
            0x26 => {
                let idx = Leb128::decode_u32(bytecode, pc) as usize;
                let func_idx = self.pop_i32()? as u32;
                let entry_idx = self.pop_i32()? as usize;
                if idx >= self.tables.len() { return Err("table.set OOB table index"); }
                let table = &mut self.tables[idx];
                if entry_idx >= table.len() { return Err("table.set OOB entry"); }
                table[entry_idx] = func_idx;
            }
            _ => return Err("Invalid variable opcode"),
        }
        Ok(())
    }
}
